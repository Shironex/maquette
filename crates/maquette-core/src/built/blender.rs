//! The Blender side of a build: a scratch folder under the workspace cache
//! with a copy of the asset's files and [`super::BUILD_SCRIPT`], and one
//! headless run of it on the runner's own lane.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use super::BuiltSource;
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::{self, LogLevel, RunState};
use crate::pipeline::runner::{PipelineRunner, RunOptions, RunSetup};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::workspace::{MeshoptLevel, TextureCodec, WorkspaceConfig};

/// The runner lane builds share, one at a time.
pub const LANE: &str = "built";
const RUNNER_FILE: &str = "build_asset.py";
const SCRIPTS_DIR: &str = "scripts";
const OUTPUT_FILE: &str = "built.glb";
const REPORT_FILE: &str = "report.json";
/// A script still running after this long is stopped.
const TIMEOUT: Duration = Duration::from_secs(10 * 60);
const LOG_TAIL: usize = 30;

/// `<workspace cache>/built/<project>/<asset>`.
pub fn work_dir(
    workspace_root: &Path,
    config: &WorkspaceConfig,
    project: &str,
    asset: &str,
) -> PathBuf {
    workspace_root
        .join(&config.layout.cache)
        .join("built")
        .join(project)
        .join(asset)
}

/// Blender's args, always headless.
fn args(script_name: &str, root: Option<&str>) -> Vec<String> {
    let mut out: Vec<String> = [
        "-b",
        "--factory-startup",
        "--python-exit-code",
        "1",
        "--python",
        "{script}",
        "--",
        "--script",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    out.push(format!("{SCRIPTS_DIR}/{script_name}"));
    for (flag, value) in [("--output", OUTPUT_FILE), ("--report", REPORT_FILE)] {
        out.push(flag.to_string());
        out.push(value.to_string());
    }
    if let Some(root) = root {
        out.push("--root".to_string());
        out.push(root.to_string());
    }
    out
}

fn def(script_name: &str, root: Option<&str>) -> PipelineDef {
    PipelineDef {
        version: PIPELINE_VERSION,
        root: None,
        steps: vec![StepDef {
            id: "build".into(),
            name: "Build asset".into(),
            kind: StepKind::Blender,
            command: None,
            script: Some(RUNNER_FILE.into()),
            args: args(script_name, root),
            inputs: Vec::new(),
            outputs: vec![OUTPUT_FILE.into(), REPORT_FILE.into()],
            depends_on: Vec::new(),
            compress: None,
        }],
        presets: Vec::new(),
    }
}

fn file_name(rel: &str) -> String {
    Path::new(rel)
        .file_name()
        .map_or_else(|| rel.to_string(), |n| n.to_string_lossy().into_owned())
}

/// A fresh scratch folder with the runner and a copy of every file.
pub(super) fn stage_files(project_dir: &Path, work: &Path, source: &BuiltSource) -> CoreResult<()> {
    let scripts = work.join(SCRIPTS_DIR);
    if scripts.exists() {
        fs::remove_dir_all(&scripts).map_err(|e| CoreError::io(&scripts, e))?;
    }
    fs::create_dir_all(&scripts).map_err(|e| CoreError::io(&scripts, e))?;
    for rel in source.files() {
        let from = project_dir.join(rel);
        let to = scripts.join(file_name(rel));
        fs::copy(&from, &to).map_err(|e| CoreError::io(&from, e))?;
    }
    for old in [OUTPUT_FILE, REPORT_FILE] {
        let _ = fs::remove_file(work.join(old));
    }
    let runner = work.join(RUNNER_FILE);
    crate::fs::write_atomic(&runner, super::BUILD_SCRIPT.as_bytes())
        .map_err(|e| CoreError::io(&runner, e))
}

/// The script's `report.json`.
#[derive(Debug, Default, Deserialize)]
pub(super) struct Report {
    #[serde(default)]
    pub(super) objects: Vec<String>,
    #[serde(default)]
    pub(super) materials: Vec<String>,
    #[serde(default)]
    pub(super) triangles: u32,
}

/// Why a run failed, from the last error line of its log, and the log's end.
fn failure(cache_dir: &Path, record: &history::RunRecord) -> (String, Vec<String>) {
    let lines = history::load_log(cache_dir, record.number);
    let tail: Vec<String> = lines
        .iter()
        .skip(lines.len().saturating_sub(LOG_TAIL))
        .map(|l| l.text.trim_end().to_string())
        .collect();
    // The runner script's own line names the exception; a Python or
    // Blender error line comes next; the runner's summary last.
    let find = |wanted: &dyn Fn(&history::LogEntry) -> bool| {
        lines
            .iter()
            .rev()
            .find(|l| wanted(l))
            .map(|l| l.text.trim().trim_start_matches("[built] ").to_string())
    };
    let last = find(&|l| l.text.contains("[built] Error"))
        .or_else(|| find(&|l| l.text.contains("Error") && l.step.is_some()))
        .or_else(|| find(&|l| l.level == LogLevel::Error));
    let step = record.steps.iter().find_map(|s| s.message.clone());
    let reason = last
        .or(step)
        .unwrap_or_else(|| "Blender stopped without saying why.".to_string());
    (reason, tail)
}

/// What one Blender run left, before it is recorded.
pub(super) enum Ran {
    Made { output: PathBuf, report: Report },
    Failed { error: String, log: Vec<String> },
}

pub(super) fn run(
    runner: &PipelineRunner,
    work: &Path,
    source: &BuiltSource,
    tools: &Toolbox,
) -> CoreResult<Ran> {
    let cache_dir = work.join("runs");
    let script = file_name(&source.script);
    let record = runner.run_blocking_with(
        RunSetup {
            project: LANE.to_string(),
            project_dir: work.to_path_buf(),
            cache_dir: cache_dir.clone(),
            def: def(&script, source.root.as_deref()),
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
        },
        RunOptions {
            timeout: Some(TIMEOUT),
            ..RunOptions::default()
        },
    )?;
    if record.state != RunState::Done {
        let (error, log) = failure(&cache_dir, &record);
        return Ok(Ran::Failed { error, log });
    }
    let report = fs::read_to_string(work.join(REPORT_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    Ok(Ran::Made {
        output: work.join(OUTPUT_FILE),
        report,
    })
}
