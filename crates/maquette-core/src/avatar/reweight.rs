//! Freeing cloth and hair from the arms before the arms are lifted: auto
//! rigs often weight a cape, a coat's skirt or long hair beside the arms to
//! the arm bones, so lifting the arms to a T-pose drags or tears them.
//! Blender reads a copy of one asset version, tells the arm (skin, sleeves,
//! gloves) from what only hangs beside it, and moves the weight of the
//! rest to the trunk ([`SCRIPT`]). Only the skin weights change; the mesh,
//! UVs, textures, bone names and rest pose stay as they were, except that
//! with [`BridgeFaces::Hide`] the faces fused between an arm and the body
//! (the bridges) leave the index buffers, so they are not drawn.
//!
//! The result is derived, never a new version: it lives in the version's
//! avatar folder under `reweight/`, with the reweighted GLB, the script's
//! report and before and after sheets. It is keyed and cached like
//! normalising, and it needs a person's yes the same way. Once approved,
//! normalising the same source lifts the arms of the reweighted model
//! instead ([`approved`]).

mod report;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::BridgeFaces;
use super::gate::{APPROVAL_FILE, DENIAL_FILE, Gate, Gated};
use super::normalize::{self, NormalizeSheets, NormalizeState};
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::inbox::vrm::{self, BoneOverrides, VrmReadiness};
use crate::json::write_json;
use crate::pipeline::cache::{self, CacheRecord, FileHasher};
use crate::pipeline::tools::Toolbox;
use crate::workspace::WorkspaceConfig;

pub use report::{ReweightBridge, ReweightChecks, ReweightReport, ReweightSide, Reweighted};

/// The script, run headless on a copy of the model.
pub const SCRIPT: &str = include_str!("avatar_reweight.py");
const SCRIPT_FILE: &str = "avatar_reweight.py";
/// The folder inside a version's avatar folder.
pub const REWEIGHT_DIR: &str = "reweight";
const JOB_FILE: &str = "job.json";
const OUTPUT_FILE: &str = "reweighted.glb";
const REPORT_FILE: &str = "report.json";
const SHEET_BEFORE: &str = "sheet_before.png";
const SHEET_AFTER: &str = "sheet_after.png";
const RESULT_FILE: &str = "result.json";
/// The step cache record's name inside the folder (`reweight.json`).
const STEP: &str = "reweight";
/// The Blender step as a run shows it.
const STEP_NAME: &str = "Reweight";
/// The skill's copy of [`SCRIPT`], for a person or an agent to run by hand.
pub const SKILL_COPY: &str =
    "plugins/maquette/skills/maquette-avatar-vrm/scripts/avatar_reweight.py";
const KEY_VERSION: &str = "maquette avatar reweight v1";
/// The readiness finding a reweight answers.
pub const FINDING: &str = "arm-weights";

const GATE: Gate = Gate {
    result: RESULT_FILE,
    step: STEP,
    output: OUTPUT_FILE,
    outputs,
    missing: "Nothing reweighted here yet, or its files changed since. Free the arms again.",
    waiting: "The new weights wait for a person's yes. Look at the before and after sheets, then approve them.",
};

impl Gated for Reweighted {
    fn key(&self) -> &str {
        &self.key
    }

    fn decided(&mut self, state: NormalizeState, approved_at: Option<String>) {
        self.state = state;
        self.approved_at = approved_at;
    }
}

/// Whether a build frees cloth and hair from the arms before it lifts them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ReweightMode {
    /// When the readiness finds cloth or hair on the arms and the arms are
    /// lifted, the reweight runs first and waits for a yes of its own.
    #[default]
    Auto,
    /// The arms are lifted without a new reweight; one a person already
    /// approved still counts until they turn it down.
    Off,
}

/// `<version's avatar folder>/reweight`.
pub fn work_dir(version_dir: &Path) -> PathBuf {
    version_dir.join(REWEIGHT_DIR)
}

/// Which asset version to reweight.
pub struct ReweightRequest<'a> {
    pub workspace_root: &'a Path,
    pub config: &'a WorkspaceConfig,
    pub project: &'a str,
    pub asset: &'a str,
    /// The newest version when `None`.
    pub version: Option<u32>,
    /// The avatar spec's bone map overrides.
    pub overrides: &'a BoneOverrides,
    /// Hide the bridge faces in the output, or keep them.
    pub bridges: BridgeFaces,
}

/// A planned run: what it reads, where it writes and its cache key.
#[derive(Debug, Clone, PartialEq)]
pub struct ReweightPlan {
    pub project: String,
    pub asset: String,
    pub version: u32,
    /// The version's model file, only read.
    pub source: PathBuf,
    pub dir: PathBuf,
    /// VRM bone name to joint name, as the readiness mapped them.
    pub bones: BTreeMap<String, String>,
    /// The source's readiness.
    pub readiness: VrmReadiness,
    pub bridges: BridgeFaces,
    pub key: String,
}

impl ReweightPlan {
    /// True when an earlier run with this key left its outputs untouched,
    /// so [`run`] answers without Blender.
    pub fn cached(&self) -> bool {
        fresh(&self.dir, &self.key)
    }
}

/// What the script reads.
#[derive(Debug, Serialize)]
struct Job<'a> {
    input: String,
    output: &'a str,
    report: &'a str,
    sheets: BTreeMap<&'a str, &'a str>,
    bones: &'a BTreeMap<String, String>,
    bridges: BridgeFaces,
}

fn outputs() -> Vec<String> {
    [OUTPUT_FILE, REPORT_FILE, SHEET_BEFORE, SHEET_AFTER]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

/// True when the readiness found cloth or hair weighted to the arms, which
/// a reweight frees before the arms are lifted.
pub fn needs_reweight(readiness: &VrmReadiness) -> bool {
    readiness.findings.iter().any(|f| f.id == FINDING)
}

/// Plans reweighting one asset version: finds the file, reads its
/// readiness and bone map, and keys the run.
///
/// # Errors
///
/// Fails for an unknown project, asset or version, a file that is not
/// glTF, or a rig with a blocker beyond its rest pose.
pub fn plan(req: &ReweightRequest<'_>) -> CoreResult<ReweightPlan> {
    let found = normalize::locate(
        req.workspace_root,
        req.config,
        req.project,
        req.asset,
        req.version,
    )?;
    let bytes = fs::read(&found.source).map_err(|e| CoreError::io(&found.source, e))?;
    let base = found.source.parent().unwrap_or(&found.project_dir);
    let readiness = vrm::readiness(&bytes, base, req.overrides)?;
    let bones = normalize::fixable_bones(&readiness, "Freeing cloth and hair from the arms")?;
    let version_dir = normalize::work_dir(
        req.workspace_root,
        req.config,
        req.project,
        &found.asset,
        found.version,
    );
    let avatar_root = req
        .workspace_root
        .join(&req.config.layout.cache)
        .join(normalize::AVATAR_DIR);
    let mut hasher = FileHasher::with_memo(&avatar_root);
    let source_hash = hasher
        .hash(&found.source)
        .map_err(|e| CoreError::io(&found.source, e))?;
    hasher.flush();
    Ok(ReweightPlan {
        project: req.project.to_string(),
        asset: found.asset,
        version: found.version,
        key: key(&source_hash, &bones, req.bridges),
        source: found.source,
        dir: work_dir(&version_dir),
        bones,
        readiness,
        bridges: req.bridges,
    })
}

/// The run's key: everything that decides the output.
pub(crate) fn key(
    source_hash: &str,
    bones: &BTreeMap<String, String>,
    bridges: BridgeFaces,
) -> String {
    let mut hasher = Sha256::new();
    let bones = serde_json::to_string(bones).unwrap_or_default();
    let bridges = serde_json::to_string(&bridges).unwrap_or_default();
    for part in [
        KEY_VERSION,
        source_hash,
        &SCRIPT.replace("\r\n", "\n"),
        &bones,
        &bridges,
    ] {
        hasher.update(part.as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())
}

/// True when `dir` holds `key`'s outputs, untouched.
fn fresh(dir: &Path, key: &str) -> bool {
    cache::load_record(dir, STEP).is_some_and(|r| cache::is_fresh(&r, key, dir, &outputs()))
        && dir.join(RESULT_FILE).is_file()
}

/// Reweights the planned version, or reads the result of an earlier run
/// with the same key. Blocks while Blender runs.
///
/// # Errors
///
/// Fails without Blender, when the script fails or leaves an output out,
/// or when the reweighted model leaves a vertex without weight.
pub fn run(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    plan: &ReweightPlan,
) -> CoreResult<Reweighted> {
    if fresh(&plan.dir, &plan.key)
        && let Some(found) = load(&plan.dir)?
    {
        return Ok(found);
    }
    let dir = &plan.dir;
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    cache::forget(dir, STEP);
    for old in [RESULT_FILE, APPROVAL_FILE, DENIAL_FILE]
        .iter()
        .copied()
        .chain(outputs().iter().map(String::as_str))
    {
        let _ = fs::remove_file(dir.join(old));
    }
    let job = Job {
        input: plan.source.display().to_string(),
        output: OUTPUT_FILE,
        report: REPORT_FILE,
        sheets: BTreeMap::from([("before", SHEET_BEFORE), ("after", SHEET_AFTER)]),
        bones: &plan.bones,
        bridges: plan.bridges,
    };
    write_json(&dir.join(JOB_FILE), &job)?;
    let started = Instant::now();
    let extra = ["--job".to_string(), JOB_FILE.to_string()];
    runner.run_step(tools, dir, STEP_NAME, (SCRIPT_FILE, SCRIPT), &extra)?;
    if let Some(missing) = outputs().iter().find(|o| !dir.join(o).is_file()) {
        return Err(invalid(format!(
            "Blender finished without writing {missing}."
        )));
    }
    let found = finish(plan)?;
    write_json(&dir.join(RESULT_FILE), &found)?;
    cache::save_record(
        dir,
        STEP,
        &CacheRecord {
            key: plan.key.clone(),
            outputs: cache::stamp_outputs(dir, &outputs()),
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
    )?;
    Ok(found)
}

/// Reads the report and holds the output to whole weights.
fn finish(plan: &ReweightPlan) -> CoreResult<Reweighted> {
    let dir = &plan.dir;
    let report_file = dir.join(REPORT_FILE);
    let text = fs::read_to_string(&report_file).map_err(|e| CoreError::io(&report_file, e))?;
    let report: ReweightReport =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&report_file, e))?;
    if report.weights.unweighted > 0 || report.weights.max_influences > 4 {
        return Err(invalid(format!(
            "The reweighted model is not whole: {} vertices without weight, up to {} bones on one.",
            report.weights.unweighted, report.weights.max_influences
        )));
    }
    Ok(Reweighted {
        project: plan.project.clone(),
        asset: plan.asset.clone(),
        source_version: plan.version,
        state: NormalizeState::Pending,
        glb: dir.join(OUTPUT_FILE).display().to_string(),
        sheets: NormalizeSheets {
            before: dir.join(SHEET_BEFORE).display().to_string(),
            after: dir.join(SHEET_AFTER).display().to_string(),
        },
        report,
        bridges: plan.bridges,
        key: plan.key.clone(),
        approved_at: None,
    })
}

/// The result in `dir`, with its approval state, or `None` when nothing
/// was reweighted there, its outputs changed since, or an older version
/// wrote it in a shape this one cannot read.
///
/// # Errors
///
/// Fails when the result file is there but cannot be read from disk.
pub fn load(dir: &Path) -> CoreResult<Option<Reweighted>> {
    GATE.load(dir)
}

/// Records a person's yes to the new weights in `dir`, for this exact
/// output.
///
/// # Errors
///
/// Fails when there is no intact result to approve.
pub fn approve(dir: &Path) -> CoreResult<Reweighted> {
    GATE.approve(dir)
}

/// Records a person's no to the new weights in `dir`: the arms are lifted
/// from the source as it is until someone approves them after all.
///
/// # Errors
///
/// Fails when there is no intact result to deny.
pub fn deny(dir: &Path) -> CoreResult<Reweighted> {
    GATE.deny(dir)
}

/// The approved reweight in `dir` of the source with `source_hash` and
/// these bones, with the bridges as the person chose, still intact; `None`
/// when there is none.
///
/// # Errors
///
/// Fails when a result file is there but cannot be read from disk.
pub fn approved(
    dir: &Path,
    source_hash: &str,
    bones: &BTreeMap<String, String>,
) -> CoreResult<Option<Reweighted>> {
    // Approved means the yes holds for the output's bytes as they are now.
    Ok(load(dir)?.filter(|found| {
        found.key == key(source_hash, bones, found.bridges)
            && found.state == NormalizeState::Approved
    }))
}

#[cfg(test)]
mod blender_tests;
#[cfg(test)]
pub(crate) mod tests;
