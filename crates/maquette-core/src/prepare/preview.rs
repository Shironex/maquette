//! Running one asset's fix-ups and real size on one model, headless, to
//! show the numbers before and after each fix-up. The Inbox does this
//! before Accept and the Inspector for an asset's newest version. The model
//! is only read; the fixed copy and its numbers go to
//! `<workspace cache>/fixups/<key>/`, keyed by the model's content, the
//! plan and the script, so asking again is instant.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{FixupKind, ModelPrep, SCRIPT, SCRIPT_PATH};
use crate::error::{CoreError, CoreResult};
use crate::layout::Bounds;
use crate::pipeline::cache::FileHasher;
use crate::pipeline::history::{self, LogLevel, RunState};
use crate::pipeline::runner::{NullSink, PipelineRunner, RunSetup};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec};

/// The runner lane previews share, one at a time.
pub const LANE: &str = "fixups";
const SCRIPT_FILE: &str = "fixups.py";
const PLAN_FILE: &str = "plan.json";
const OUTPUT_FILE: &str = "fixed.glb";
const STATS_FILE: &str = "stats.json";

/// A model's numbers at one point of the fix-ups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelNumbers {
    pub triangles: u32,
    /// Mesh objects.
    pub objects: u32,
    /// The longest texture edge in pixels, 0 without textures.
    pub max_texture: u32,
    /// Bounds in glTF metres (Y up); `None` without meshes.
    pub min: Option<[f32; 3]>,
    pub max: Option<[f32; 3]>,
}

/// What one fix-up did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FixupStep {
    pub op: FixupKind,
    pub before: ModelNumbers,
    pub after: ModelNumbers,
    /// The script's one line summary, for example "ratio 0.260".
    pub note: String,
}

/// The fix-ups and real size of one plan on one model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FixupReport {
    pub plan: ModelPrep,
    pub before: ModelNumbers,
    pub after: ModelNumbers,
    pub steps: Vec<FixupStep>,
    /// The real size scale, when the plan has a size.
    pub scale: Option<f32>,
    /// The file sizes of the model and of the fixed copy.
    pub bytes_before: Option<f64>,
    pub bytes_after: Option<f64>,
    /// The fixed copy, an absolute path.
    pub output: String,
}

/// The script's `stats.json`.
#[derive(Debug, Deserialize)]
struct Stats {
    before: ModelNumbers,
    after: ModelNumbers,
    steps: Vec<RawStep>,
    scale: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct RawStep {
    op: String,
    before: ModelNumbers,
    after: ModelNumbers,
    note: String,
}

pub fn previews_dir(workspace_cache: &Path) -> PathBuf {
    workspace_cache.join("fixups")
}

/// The fix-up script a project's pipeline runs: its own `tools/fixups.py`
/// when it has one, else the one this build ships.
pub fn script_for(project_dir: Option<&Path>) -> String {
    project_dir
        .and_then(|dir| fs::read_to_string(dir.join(SCRIPT_PATH)).ok())
        .unwrap_or_else(|| SCRIPT.to_string())
}

/// The folder name for a model's content hash, a plan and a script.
pub fn key(model_hash: &str, plan: &ModelPrep, script: &str) -> String {
    let plan = serde_json::to_string(plan).unwrap_or_default();
    let mut hasher = Sha256::new();
    for part in [model_hash, &plan, &script.replace("\r\n", "\n")] {
        hasher.update(part.as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())[..32].to_string()
}

/// Blender's args for one preview, always headless. Paths are filled in
/// whole, so a space in a folder name stays one argument.
pub fn args(input: &Path) -> Vec<String> {
    [
        "-b",
        "--factory-startup",
        "--python-exit-code",
        "1",
        "--python",
        "{script}",
        "--",
        "--input",
        &input.display().to_string(),
        "--plan",
        PLAN_FILE,
        "--output",
        OUTPUT_FILE,
        "--stats",
        STATS_FILE,
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

/// The one step pipeline that runs a preview inside its own folder.
pub fn def(input: &Path) -> PipelineDef {
    PipelineDef {
        version: PIPELINE_VERSION,
        root: None,
        steps: vec![StepDef {
            id: "fixups".into(),
            name: "Fix-ups".into(),
            kind: StepKind::Blender,
            command: None,
            script: Some(SCRIPT_FILE.into()),
            args: args(input),
            inputs: Vec::new(),
            outputs: vec![OUTPUT_FILE.into(), STATS_FILE.into()],
            depends_on: Vec::new(),
            compress: None,
        }],
        presets: Vec::new(),
    }
}

fn file_bytes(path: &Path) -> Option<f64> {
    #[expect(clippy::cast_precision_loss, reason = "model sizes stay below 2^53")]
    fs::metadata(path).ok().map(|m| m.len() as f64)
}

/// Reads a finished preview's numbers. Steps the script names with a kind
/// this build does not know are left out.
pub fn read_report(dir: &Path, input: &Path, plan: &ModelPrep) -> CoreResult<FixupReport> {
    let file = dir.join(STATS_FILE);
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    let stats: Stats = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    let steps = stats
        .steps
        .into_iter()
        .filter_map(|step| {
            let op = serde_json::from_value(serde_json::Value::String(step.op)).ok()?;
            Some(FixupStep {
                op,
                before: step.before,
                after: step.after,
                note: step.note,
            })
        })
        .collect();
    let output = dir.join(OUTPUT_FILE);
    Ok(FixupReport {
        plan: plan.clone(),
        before: stats.before,
        after: stats.after,
        steps,
        scale: stats.scale,
        bytes_before: file_bytes(input),
        bytes_after: file_bytes(&output),
        output: output.display().to_string(),
    })
}

/// The preview service. Cheap to clone; previews run one at a time.
#[derive(Clone)]
pub struct Previews {
    runner: PipelineRunner,
    one: Arc<Mutex<()>>,
}

impl Default for Previews {
    fn default() -> Self {
        Self {
            runner: PipelineRunner::new(Arc::new(NullSink)),
            one: Arc::new(Mutex::new(())),
        }
    }
}

/// The content hash of `input`, remembered by size and modified time.
fn model_hash(root: &Path, input: &Path) -> CoreResult<String> {
    let mut hasher = FileHasher::with_memo(root);
    let hash = hasher.hash(input).map_err(|e| CoreError::io(input, e))?;
    hasher.flush();
    Ok(hash)
}

/// Where a preview of `input` with `plan` and `script` lives under `root`.
///
/// # Errors
///
/// Fails when the model cannot be read.
pub fn folder(root: &Path, input: &Path, plan: &ModelPrep, script: &str) -> CoreResult<PathBuf> {
    Ok(root.join(key(&model_hash(root, input)?, plan, script)))
}

/// The plan without its size: what the bounds after the fix-ups depend on.
fn fixups_only(plan: &ModelPrep) -> ModelPrep {
    ModelPrep {
        size: None,
        fixups: plan.fixups.clone(),
    }
}

fn bounds_file(root: &Path, input: &Path, plan: &ModelPrep, script: &str) -> CoreResult<PathBuf> {
    let key = key(&model_hash(root, input)?, &fixups_only(plan), script);
    Ok(root.join("bounds").join(format!("{key}.json")))
}

/// The bounds `input` has after `plan`'s fix-ups (before its real size),
/// when a preview of them ran: a removed loose part can change them, so the
/// layout measures from these once they are known.
pub fn fixed_bounds(root: &Path, input: &Path, plan: &ModelPrep, script: &str) -> Option<Bounds> {
    let file = bounds_file(root, input, plan, script).ok()?;
    let numbers: ModelNumbers = serde_json::from_str(&fs::read_to_string(file).ok()?).ok()?;
    Some((numbers.min?, numbers.max?))
}

/// Remembers the bounds after the fix-ups of a finished preview.
fn keep_bounds(root: &Path, input: &Path, report: &FixupReport, script: &str) {
    let after = report
        .steps
        .last()
        .map_or(&report.before, |step| &step.after);
    let Ok(file) = bounds_file(root, input, &report.plan, script) else {
        return;
    };
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(err) = crate::json::write_json(&file, after) {
        tracing::warn!(error = %err, "could not keep the fixed bounds");
    }
}

/// The preview of `input` with `plan`, when it already ran.
pub fn cached(root: &Path, input: &Path, plan: &ModelPrep, script: &str) -> Option<FixupReport> {
    let dir = folder(root, input, plan, script).ok()?;
    read_report(&dir, input, plan).ok()
}

impl Previews {
    /// Runs `plan` on `input` with `script` and returns the numbers; a
    /// preview that already ran comes from its folder under `root`.
    /// Blocks until Blender is done.
    ///
    /// # Errors
    ///
    /// Fails without Blender, when the model cannot be read, or when the
    /// script fails (with the last error line of its log).
    pub fn run(
        &self,
        root: &Path,
        input: &Path,
        plan: &ModelPrep,
        script: &str,
        tools: &Toolbox,
    ) -> CoreResult<FixupReport> {
        let dir = folder(root, input, plan, script)?;
        if let Ok(report) = read_report(&dir, input, plan) {
            keep_bounds(root, input, &report, script);
            return Ok(report);
        }
        if !tools.has(ToolId::Blender) {
            return Err(CoreError::Unavailable(
                "Blender is not installed, so the fix-ups cannot run. Install it, then try again."
                    .to_string(),
            ));
        }
        let _one = self.one.lock().unwrap_or_else(PoisonError::into_inner);
        fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
        let plan_text =
            serde_json::to_string_pretty(plan).map_err(|e| CoreError::Invalid(e.to_string()))?;
        for (name, text) in [(PLAN_FILE, plan_text.as_str()), (SCRIPT_FILE, script)] {
            let path = dir.join(name);
            crate::fs::write_atomic(&path, text.as_bytes()).map_err(|e| CoreError::io(&path, e))?;
        }
        let cache_dir = dir.join("runs");
        let record = self.runner.run_blocking(RunSetup {
            project: LANE.to_string(),
            project_dir: dir.clone(),
            cache_dir: cache_dir.clone(),
            def: def(input),
            tools: tools.clone(),
            defaults: CompressDefaults {
                texture_size: 2048,
                codec: TextureCodec::Etc1s,
                meshopt: MeshoptLevel::Medium,
            },
            vars: Default::default(),
            shipped_names: Default::default(),
            materials: Default::default(),
            from: None,
            force: true,
        })?;
        if record.state != RunState::Done {
            let lines = history::load_log(&cache_dir, record.number);
            let last = lines
                .iter()
                .rev()
                .find(|l| l.level == LogLevel::Error || l.text.contains("Error"))
                .map(|l| l.text.trim().to_string());
            let step = record.steps.iter().find_map(|s| s.message.clone());
            return Err(CoreError::Invalid(format!(
                "The fix-ups did not finish. {}",
                last.or(step)
                    .unwrap_or_else(|| "Blender stopped without saying why.".to_string())
            )));
        }
        let report = read_report(&dir, input, plan)?;
        keep_bounds(root, input, &report, script);
        Ok(report)
    }
}
