//! Normalising a rigged model's rest pose for VRM: Blender turns a copy of
//! one asset version to face +Z, centres and grounds it, lifts the arms to
//! level and makes that pose the rest ([`SCRIPT`]), and draws pose sheets
//! before and after.
//!
//! The result is derived, never a new version: it lives in
//! `<workspace cache>/avatar/<project>/<asset>/v<version>/`, with the
//! normalised GLB, the script's report, the two sheets and the readiness
//! read again on the output. The run is keyed like a pipeline step (the
//! source's content, the script, the bone roles and the options) and
//! recorded with the step cache, so asking again with nothing changed
//! skips Blender.
//!
//! Changing a rest pose needs a person's yes. A fresh result is pending;
//! [`approve`] records the yes against that exact output, and the writer
//! path ([`approved_glb`], [`write_vrm`]) refuses a result that is not
//! approved, or whose file changed since.

mod approval;
mod report;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::assets;
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::inbox::vrm::{
    self, ARM_SLACK_DEG, Axis, BoneOverrides, FACING_SLACK_DEG, FindingLevel, VrmReadiness,
};
use crate::json::write_json;
use crate::pipeline::cache::{self, CacheRecord, FileHasher};
use crate::pipeline::tools::Toolbox;
use crate::project;
use crate::workspace::WorkspaceConfig;

use approval::APPROVAL_FILE;
pub use approval::{approve, approved_glb, load, write_vrm};
pub use report::{
    ArmLift, NormalizeReport, NormalizeSheets, NormalizeState, Normalized, PoseMeasures,
    WeightChecks,
};

/// The script, run headless on a copy of the model.
pub const SCRIPT: &str = include_str!("avatar_normalize.py");
const SCRIPT_FILE: &str = "avatar_normalize.py";
/// The cache folder every asset's normalised models live under.
pub const AVATAR_DIR: &str = "avatar";
const JOB_FILE: &str = "job.json";
const OUTPUT_FILE: &str = "normalized.glb";
const REPORT_FILE: &str = "report.json";
const SHEET_BEFORE: &str = "sheet_before.png";
const SHEET_AFTER: &str = "sheet_after.png";
const RESULT_FILE: &str = "result.json";
/// The step cache record's name inside the folder (`normalize.json`).
const STEP: &str = "normalize";
/// The skill's copy of [`SCRIPT`], for a person or an agent to run by hand.
pub const SKILL_COPY: &str =
    "plugins/maquette/skills/maquette-avatar-vrm/scripts/avatar_normalize.py";
const KEY_VERSION: &str = "maquette avatar normalize v1";

/// The findings normalising fixes; any other blocker stops it first.
const FIXED_HERE: [&str; 2] = ["facing", "arms"];

/// What happens at the elbows when the arms are lifted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Elbows {
    /// The whole arm turns at the shoulder; a bent elbow stays bent.
    Keep,
    /// The lower arm is levelled too, so the arm lies straight along X.
    #[default]
    Straighten,
}

/// `<workspace cache>/avatar/<project>/<asset>/v<version>`.
pub fn work_dir(
    workspace_root: &Path,
    config: &WorkspaceConfig,
    project: &str,
    asset: &str,
    version: u32,
) -> PathBuf {
    workspace_root
        .join(&config.layout.cache)
        .join(AVATAR_DIR)
        .join(project)
        .join(asset)
        .join(format!("v{version}"))
}

/// Which asset version to normalise, and how.
pub struct NormalizeRequest<'a> {
    pub workspace_root: &'a Path,
    pub config: &'a WorkspaceConfig,
    pub project: &'a str,
    pub asset: &'a str,
    /// The newest version when `None`.
    pub version: Option<u32>,
    /// The avatar spec's bone map overrides.
    pub overrides: &'a BoneOverrides,
    pub elbows: Elbows,
}

/// A planned run: what it reads, where it writes and its cache key.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizePlan {
    pub project: String,
    pub asset: String,
    pub version: u32,
    /// The version's model file, only read.
    pub source: PathBuf,
    pub dir: PathBuf,
    /// VRM bone name to joint name, as the readiness mapped them.
    pub bones: BTreeMap<String, String>,
    pub overrides: BoneOverrides,
    pub elbows: Elbows,
    /// The source's readiness, before.
    pub readiness: VrmReadiness,
    pub key: String,
}

/// What the script reads.
#[derive(Debug, Serialize)]
struct Job<'a> {
    input: String,
    output: &'a str,
    report: &'a str,
    sheets: BTreeMap<&'a str, &'a str>,
    bones: &'a BTreeMap<String, String>,
    elbows: Elbows,
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

/// Plans normalising one asset version: finds the file, reads its
/// readiness and bone map, and keys the run.
///
/// # Errors
///
/// Fails for an unknown project, asset or version, a file that is not
/// glTF, or a rig with a blocker normalising cannot fix (no skin, missing
/// or misplaced bones, swapped sides, a mirrored or uneven scale).
pub fn plan(req: &NormalizeRequest<'_>) -> CoreResult<NormalizePlan> {
    let manifest = project::load(req.workspace_root, &req.config.layout, req.project)?;
    let asset = assets::find(&manifest, req.asset)?;
    let version = match req.version {
        Some(n) => asset.versions.iter().find(|v| v.version == n),
        None => asset.latest(),
    }
    .ok_or_else(|| {
        CoreError::NotFound(match req.version {
            Some(n) => format!("version {n} of {}", asset.name),
            None => format!("a model of {}", asset.name),
        })
    })?;
    let project_dir = project::dir(req.workspace_root, &req.config.layout, req.project)?;
    let source = project_dir.join(&version.file);
    let bytes = fs::read(&source).map_err(|e| CoreError::io(&source, e))?;
    let base = source.parent().unwrap_or(&project_dir);
    let readiness = vrm::readiness(&bytes, base, req.overrides)?;
    let stoppers: Vec<&str> = readiness
        .findings
        .iter()
        .filter(|f| f.level == FindingLevel::Blocker && !FIXED_HERE.contains(&f.id.as_str()))
        .map(|f| f.title.as_str())
        .collect();
    if !stoppers.is_empty() {
        return Err(invalid(format!(
            "Normalising the rest pose cannot fix this rig: {}. Fix the rig first.",
            stoppers.join("; ")
        )));
    }
    let bones: BTreeMap<String, String> = readiness
        .bones
        .iter()
        .filter_map(|b| b.joint.clone().map(|j| (b.bone.clone(), j)))
        .collect();
    let dir = work_dir(
        req.workspace_root,
        req.config,
        req.project,
        &asset.id,
        version.version,
    );
    let avatar_root = req
        .workspace_root
        .join(&req.config.layout.cache)
        .join(AVATAR_DIR);
    let mut hasher = FileHasher::with_memo(&avatar_root);
    let source_hash = hasher
        .hash(&source)
        .map_err(|e| CoreError::io(&source, e))?;
    hasher.flush();
    let key = key(&source_hash, &bones, req.elbows);
    Ok(NormalizePlan {
        project: req.project.to_string(),
        asset: asset.id.clone(),
        version: version.version,
        source,
        dir,
        bones,
        overrides: req.overrides.clone(),
        elbows: req.elbows,
        readiness,
        key,
    })
}

/// The run's key: everything that decides the output.
fn key(source_hash: &str, bones: &BTreeMap<String, String>, elbows: Elbows) -> String {
    let mut hasher = Sha256::new();
    let bones = serde_json::to_string(bones).unwrap_or_default();
    let elbows = serde_json::to_string(&elbows).unwrap_or_default();
    for part in [
        KEY_VERSION,
        source_hash,
        &SCRIPT.replace("\r\n", "\n"),
        &bones,
        &elbows,
    ] {
        hasher.update(part.as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())
}

/// True when the folder holds this key's outputs, untouched.
fn fresh(plan: &NormalizePlan) -> bool {
    cache::load_record(&plan.dir, STEP)
        .is_some_and(|r| cache::is_fresh(&r, &plan.key, &plan.dir, &outputs()))
        && plan.dir.join(RESULT_FILE).is_file()
}

/// Normalises the planned version, or reads the result of an earlier run
/// with the same key. Blocks while Blender runs.
///
/// # Errors
///
/// Fails without Blender, when the script fails or leaves an output out,
/// or when the normalised model still misses the VRM rest pose (arms more
/// than 3 degrees off level, or not facing +Z).
pub fn run(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    plan: &NormalizePlan,
) -> CoreResult<Normalized> {
    if fresh(plan)
        && let Some(found) = load(&plan.dir)?
    {
        return Ok(found);
    }
    let dir = &plan.dir;
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    cache::forget(dir, STEP);
    for old in [RESULT_FILE, APPROVAL_FILE]
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
        elbows: plan.elbows,
    };
    write_json(&dir.join(JOB_FILE), &job)?;
    let started = Instant::now();
    let extra = ["--job".to_string(), JOB_FILE.to_string()];
    runner.run_script(tools, dir, (SCRIPT_FILE, SCRIPT), &extra)?;
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

/// Reads the report and the output's readiness, and holds the output to
/// the VRM rest pose.
fn finish(plan: &NormalizePlan) -> CoreResult<Normalized> {
    let dir = &plan.dir;
    let report_file = dir.join(REPORT_FILE);
    let text = fs::read_to_string(&report_file).map_err(|e| CoreError::io(&report_file, e))?;
    let report: NormalizeReport =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&report_file, e))?;
    let glb = dir.join(OUTPUT_FILE);
    let bytes = fs::read(&glb).map_err(|e| CoreError::io(&glb, e))?;
    let readiness = vrm::readiness(&bytes, dir, &plan.overrides)?;
    check_rest_pose(&readiness)?;
    Ok(Normalized {
        project: plan.project.clone(),
        asset: plan.asset.clone(),
        source_version: plan.version,
        state: NormalizeState::Pending,
        glb: glb.display().to_string(),
        sheets: NormalizeSheets {
            before: dir.join(SHEET_BEFORE).display().to_string(),
            after: dir.join(SHEET_AFTER).display().to_string(),
        },
        report,
        readiness,
        key: plan.key.clone(),
        approved_at: None,
    })
}

/// The bar a normalised model must clear: both arms within
/// [`ARM_SLACK_DEG`] of level and the front within [`FACING_SLACK_DEG`]
/// of +Z.
///
/// # Errors
///
/// Says which measure missed.
pub fn check_rest_pose(readiness: &VrmReadiness) -> CoreResult<()> {
    let pose = &readiness.pose;
    let mut missed = Vec::new();
    for (side, deg) in [("left", pose.arm_left_deg), ("right", pose.arm_right_deg)] {
        match deg {
            Some(d) if d.abs() <= ARM_SLACK_DEG => {}
            Some(d) => missed.push(format!(
                "the {side} arm is {:.1} degrees off level",
                d.abs()
            )),
            None => missed.push(format!("the {side} arm could not be measured")),
        }
    }
    match (pose.facing, pose.facing_deg) {
        (Some(Axis::PlusZ), Some(d)) if d.abs() <= FACING_SLACK_DEG => {}
        (_, Some(d)) => missed.push(format!("the front is turned {d:.1} degrees from +Z")),
        _ => missed.push("the facing could not be read".to_string()),
    }
    if missed.is_empty() {
        Ok(())
    } else {
        Err(invalid(format!(
            "The normalised model still misses the VRM rest pose: {}.",
            missed.join(", ")
        )))
    }
}

#[cfg(test)]
mod blender_tests;
#[cfg(test)]
mod tests;
