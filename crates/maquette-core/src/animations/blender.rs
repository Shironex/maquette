//! The Blender side of the animation library: a scratch folder under the
//! workspace cache with the files to read and a script, and one headless
//! run of it on the runner's own lane.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{RestBone, RootMotion};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::{self, LogLevel, RunState};
use crate::pipeline::runner::{PipelineRunner, RunOptions, RunSetup};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec, WorkspaceConfig};

/// The runner lane the library's Blender jobs share, one at a time.
pub const LANE: &str = "animations";
/// Reads clips out of FBX and glTF files, skeleton only.
pub const READ_SCRIPT: &str = include_str!("read_clips.py");
const READ_FILE: &str = "read_clips.py";
pub(super) const JOBS_FILE: &str = "jobs.json";
pub(super) const OUT_DIR: &str = "clips";
pub(super) const REPORT_FILE: &str = "report.json";
const TIMEOUT: Duration = Duration::from_secs(20 * 60);
const LOG_TAIL: usize = 30;

/// One file for the reader.
#[derive(Debug, Clone, Serialize)]
pub(super) struct ReadJob {
    pub id: String,
    /// Relative to the scratch folder, with `/` separators.
    pub file: String,
    /// The file's own name, which a lone take is named after.
    pub name: String,
    /// The clips to keep, for a model that holds others too.
    pub clips: Option<Vec<String>>,
    /// The rate a glTF model's keys were sampled at.
    pub fps: Option<f32>,
}

/// One clip as the reader found it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClipReport {
    pub name: String,
    pub hash: String,
    pub has_keys: bool,
    pub frames: u32,
    pub fps: f32,
    pub length: f32,
    pub root_motion: Option<RootMotion>,
    pub loops: bool,
    /// The written skeleton file's size; none for a clip without keys.
    pub bytes: Option<u64>,
    /// Relative to the clips folder.
    pub out: Option<String>,
}

/// One file as the reader found it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileReport {
    pub id: String,
    pub error: Option<String>,
    #[serde(default)]
    pub skinned: bool,
    #[serde(default)]
    pub bones: Vec<RestBone>,
    #[serde(default)]
    pub hips_height: f32,
    #[serde(default)]
    pub clips: Vec<ClipReport>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ReadReport {
    #[serde(default)]
    pub files: Vec<FileReport>,
}

/// A fresh scratch folder for one job: `<workspace cache>/animations/<kind>-<n>`.
pub(super) fn scratch(root: &Path, config: &WorkspaceConfig, kind: &str) -> CoreResult<PathBuf> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let parent = root.join(&config.layout.cache).join(super::ANIMATIONS_DIR);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let name = format!("{kind}-{stamp}-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let work = parent.join(name);
    fs::create_dir_all(&work).map_err(|e| CoreError::io(&work, e))?;
    Ok(work)
}

/// Blender's args, always headless.
fn args(extra: &[String]) -> Vec<String> {
    let mut out: Vec<String> = [
        "-b",
        "--factory-startup",
        "--python-exit-code",
        "1",
        "--python",
        "{script}",
        "--",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    out.extend(extra.iter().cloned());
    out
}

fn def(script_file: &str, extra: &[String]) -> PipelineDef {
    PipelineDef {
        version: PIPELINE_VERSION,
        root: None,
        steps: vec![StepDef {
            id: "clips".into(),
            name: "Animation clips".into(),
            kind: StepKind::Blender,
            command: None,
            script: Some(script_file.into()),
            args: args(extra),
            inputs: Vec::new(),
            outputs: Vec::new(),
            depends_on: Vec::new(),
            compress: None,
        }],
        presets: Vec::new(),
    }
}

/// Why a run failed, from its last error line, and the end of its log.
fn failure(cache_dir: &Path, record: &history::RunRecord) -> String {
    let lines = history::load_log(cache_dir, record.number);
    let last = lines
        .iter()
        .rev()
        .find(|l| l.text.contains("] Error") || (l.level == LogLevel::Error && l.step.is_some()))
        .map(|l| l.text.trim().to_string());
    let tail: Vec<String> = lines
        .iter()
        .skip(lines.len().saturating_sub(LOG_TAIL))
        .map(|l| l.text.trim_end().to_string())
        .collect();
    tracing::warn!(log = %tail.join("\n"), "the clip job failed");
    last.or_else(|| record.steps.iter().find_map(|s| s.message.clone()))
        .unwrap_or_else(|| "Blender stopped without saying why.".to_string())
}

/// Runs `script` (written into `work` as `script_file`) headless with
/// `extra` args after `--`, and waits for it.
///
/// # Errors
///
/// Fails without Blender, or with the reason the run gave when it failed.
pub(crate) fn run_script(
    runner: &PipelineRunner,
    tools: &Toolbox,
    work: &Path,
    (script_file, script): (&str, &str),
    extra: &[String],
) -> CoreResult<()> {
    if !tools.has(ToolId::Blender) {
        return Err(CoreError::Unavailable(
            "Blender is not installed, and this needs it. Install it, then try again.".to_string(),
        ));
    }
    let path = work.join(script_file);
    crate::fs::write_atomic(&path, script.as_bytes()).map_err(|e| CoreError::io(&path, e))?;
    let cache_dir = work.join("runs");
    let record = runner.run_blocking_with(
        RunSetup {
            project: LANE.to_string(),
            project_dir: work.to_path_buf(),
            cache_dir: cache_dir.clone(),
            def: def(script_file, extra),
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
            textures: Default::default(),
            quality: crate::ship::DEFAULT_QUALITY,
            build_only: false,
        },
        RunOptions {
            timeout: Some(TIMEOUT),
            ..RunOptions::default()
        },
    )?;
    if record.state != RunState::Done {
        return Err(CoreError::Invalid(failure(&cache_dir, &record)));
    }
    Ok(())
}

/// Reads `jobs` (files already in `work`) and returns what was found.
pub(super) fn read(
    runner: &PipelineRunner,
    tools: &Toolbox,
    work: &Path,
    jobs: &[ReadJob],
) -> CoreResult<ReadReport> {
    crate::json::write_json(&work.join(JOBS_FILE), &jobs)?;
    let extra: Vec<String> = [
        "--jobs",
        JOBS_FILE,
        "--out",
        OUT_DIR,
        "--report",
        REPORT_FILE,
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    run_script(runner, tools, work, (READ_FILE, READ_SCRIPT), &extra)?;
    read_report(work)
}

pub(super) fn read_report(work: &Path) -> CoreResult<ReadReport> {
    let file = work.join(REPORT_FILE);
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))
}
