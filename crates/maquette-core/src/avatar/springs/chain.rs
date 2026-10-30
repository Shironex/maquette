//! Adding the confirmed springs' chains: Blender adds a chain of joints
//! along each part, weights the part to it and derives colliders, on a
//! copy. The result is keyed by the model, the script, the bone map and
//! each spring's part and joint count (not its swing settings, which only
//! the writer reads), and it waits for a person's yes like the other
//! avatar steps.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use super::{ChainReport, Chained, JOB_FILE, SCRIPT, SCRIPT_FILE, SpringsInput, invalid, key};
use crate::avatar::SpringSpec;
use crate::avatar::gate::{APPROVAL_FILE, DENIAL_FILE, Gate, Gated};
use crate::avatar::normalize::{NormalizeSheets, NormalizeState};
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::inbox::vrm::{self, BoneOverrides, SpringBones, SpringChain};
use crate::json::write_json;
use crate::pipeline::cache::{self, CacheRecord};
use crate::pipeline::tools::Toolbox;

const CHAIN_DIR: &str = "chain";
const OUTPUT_FILE: &str = "springs.glb";
const REPORT_FILE: &str = "report.json";
const SHEET_BEFORE: &str = "sheet_before.png";
const SHEET_AFTER: &str = "sheet_after.png";
const RESULT_FILE: &str = "result.json";
/// The step cache record's name inside the folder (`chain.json`).
const STEP: &str = "chain";
/// The Blender step as a run shows it.
const STEP_NAME: &str = "Springs chain";
const KEY_VERSION: &str = "maquette avatar springs chain v1";

const GATE: Gate = Gate {
    result: RESULT_FILE,
    step: STEP,
    output: OUTPUT_FILE,
    outputs,
    missing: "No chains were made here yet, or their files changed since. Make the chains again.",
    waiting: "The new chains wait for a person's yes. Look at the before and after sheets, then approve them.",
};

impl Gated for Chained {
    fn key(&self) -> &str {
        &self.key
    }

    fn decided(&mut self, state: NormalizeState, approved_at: Option<String>) {
        self.state = state;
        self.approved_at = approved_at;
    }
}

fn outputs() -> Vec<String> {
    [OUTPUT_FILE, REPORT_FILE, SHEET_BEFORE, SHEET_AFTER]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// `<springs folder>/chain`.
pub fn dir(input: &SpringsInput) -> PathBuf {
    dir_in(&input.dir)
}

/// The chain folder inside a version's springs folder.
pub fn dir_in(springs_dir: &Path) -> PathBuf {
    springs_dir.join(CHAIN_DIR)
}

/// What decides a spring's chain: its part and its joints.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChainJob<'a> {
    id: &'a str,
    root_bone: &'a str,
    line: &'a [[f64; 3]],
    #[serde(skip_serializing_if = "Option::is_none")]
    joints: Option<u32>,
}

fn chain_jobs(springs: &[SpringSpec]) -> Vec<ChainJob<'_>> {
    springs
        .iter()
        .map(|s| ChainJob {
            id: &s.id,
            root_bone: &s.root_bone,
            line: &s.line,
            joints: s.joints,
        })
        .collect()
}

/// What the script reads in chain mode.
#[derive(Debug, Serialize)]
struct Job<'a> {
    mode: &'a str,
    input: String,
    output: &'a str,
    report: &'a str,
    sheets: BTreeMap<&'a str, &'a str>,
    bones: &'a BTreeMap<String, String>,
    springs: Vec<ChainJob<'a>>,
}

/// A planned run.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainPlan {
    pub input: SpringsInput,
    pub springs: Vec<SpringSpec>,
    pub dir: PathBuf,
    pub key: String,
}

impl ChainPlan {
    /// True when an earlier run with this key left its outputs untouched.
    pub fn cached(&self) -> bool {
        fresh(&self.dir, &self.key)
    }
}

/// Plans chaining `springs` on the input.
///
/// # Errors
///
/// Refuses no springs, and fails when the model cannot be read to key the
/// run.
pub fn plan(input: &SpringsInput, springs: &[SpringSpec]) -> CoreResult<ChainPlan> {
    if springs.is_empty() {
        return Err(invalid(
            "No spring is confirmed. Pick the dangling parts that should swing first.",
        ));
    }
    let bones = serde_json::to_string(&input.bones).unwrap_or_default();
    let chains = serde_json::to_string(&chain_jobs(springs)).unwrap_or_default();
    let key = key(&[
        KEY_VERSION,
        &input.hash()?,
        &SCRIPT.replace("\r\n", "\n"),
        &bones,
        &chains,
    ]);
    Ok(ChainPlan {
        input: input.clone(),
        springs: springs.to_vec(),
        dir: dir(input),
        key,
    })
}

fn fresh(dir: &Path, key: &str) -> bool {
    cache::load_record(dir, STEP).is_some_and(|r| cache::is_fresh(&r, key, dir, &outputs()))
        && dir.join(RESULT_FILE).is_file()
}

/// Chains the planned springs, or reads the result of an earlier run with
/// the same key. Blocks while Blender runs.
///
/// # Errors
///
/// Fails without Blender, when the script fails or leaves an output out,
/// when a vertex is left without weight or with more than 4 bones, or when
/// the humanoid bones changed.
pub fn run(runner: &dyn ScriptRunner, tools: &Toolbox, plan: &ChainPlan) -> CoreResult<Chained> {
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
        mode: "chain",
        input: plan.input.glb.display().to_string(),
        output: OUTPUT_FILE,
        report: REPORT_FILE,
        sheets: BTreeMap::from([("before", SHEET_BEFORE), ("after", SHEET_AFTER)]),
        bones: &plan.input.bones,
        springs: chain_jobs(&plan.springs),
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

/// Reads the report and holds the output to whole weights and the same
/// humanoid bones.
fn finish(plan: &ChainPlan) -> CoreResult<Chained> {
    let dir = &plan.dir;
    let report_file = dir.join(REPORT_FILE);
    let text = fs::read_to_string(&report_file).map_err(|e| CoreError::io(&report_file, e))?;
    let report: ChainReport =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&report_file, e))?;
    if report.weights.unweighted > 0 || report.weights.max_influences > 4 {
        return Err(invalid(format!(
            "The chained model is not whole: {} vertices without weight, up to {} bones on one.",
            report.weights.unweighted, report.weights.max_influences
        )));
    }
    let none = BoneOverrides::new();
    let glb = dir.join(OUTPUT_FILE);
    let read = |path: &Path| -> CoreResult<Vec<(String, Option<String>)>> {
        let bytes = fs::read(path).map_err(|e| CoreError::io(path, e))?;
        let base = path.parent().unwrap_or(dir);
        Ok(vrm::readiness(&bytes, base, &none)?
            .bones
            .into_iter()
            .map(|b| (b.bone, b.joint))
            .collect())
    };
    if read(&glb)? != read(&plan.input.glb)? {
        return Err(invalid(
            "The chained model maps its humanoid bones differently; nothing was kept.",
        ));
    }
    Ok(Chained {
        project: plan.input.project.clone(),
        asset: plan.input.asset.clone(),
        source_version: plan.input.version,
        state: NormalizeState::Pending,
        glb: glb.display().to_string(),
        sheets: NormalizeSheets {
            before: dir.join(SHEET_BEFORE).display().to_string(),
            after: dir.join(SHEET_AFTER).display().to_string(),
        },
        report,
        key: plan.key.clone(),
        approved_at: None,
    })
}

/// The result in `dir`, with its approval state, or `None`.
///
/// # Errors
///
/// Fails when the result file is there but cannot be read from disk.
pub fn load(dir: &Path) -> CoreResult<Option<Chained>> {
    GATE.load(dir)
}

/// Records a person's yes to the chains in `dir`, for this exact output.
///
/// # Errors
///
/// Fails when there is no intact result to approve.
pub fn approve(dir: &Path) -> CoreResult<Chained> {
    GATE.approve(dir)
}

/// Records a person's no: the VRM is written without springs until
/// someone approves them after all, or a new run replaces them.
///
/// # Errors
///
/// Fails when there is no intact result to deny.
pub fn deny(dir: &Path) -> CoreResult<Chained> {
    GATE.deny(dir)
}

/// The chained model's bytes, only once a person approved them.
///
/// # Errors
///
/// Refuses a result that is pending, missing or changed since the yes.
pub fn approved_glb(dir: &Path) -> CoreResult<Vec<u8>> {
    GATE.approved_bytes::<Chained>(dir)
}

/// The springs and colliders to write: each chain with its spec's
/// settings (its kind's preset when the spec no longer names it).
pub fn spring_bones(report: &ChainReport, springs: &[SpringSpec]) -> SpringBones {
    let springs = report
        .chains
        .iter()
        .map(|chain| {
            let spec = springs.iter().find(|s| s.id == chain.id);
            let settings = spec.map_or_else(
                || {
                    chain
                        .kind
                        .unwrap_or(vrm::SpringKind::Accessory)
                        .preset()
                        .settings()
                },
                SpringSpec::settings,
            );
            SpringChain {
                name: chain.id.clone(),
                joints: chain.joints.clone(),
                settings,
                collider_groups: chain.collider_groups.clone(),
            }
        })
        .collect();
    SpringBones {
        springs,
        colliders: report.colliders.clone(),
    }
}
