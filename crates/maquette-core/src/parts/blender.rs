//! Running one of the parts scripts in headless Blender, as a one step
//! pipeline inside its own folder (the way the fix-up previews run), so
//! the run is logged and a failure says the script's last error line.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::{self, LogLevel, RunState};
use crate::pipeline::runner::{NullSink, PipelineRunner, RunSetup};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec};

/// The runner lane the parts scripts share.
const LANE: &str = "parts";

/// One script to run: its file name and text, what it does in a few words
/// for the errors, its arguments after `--` and the files it writes.
pub(crate) struct Script<'a> {
    pub file: &'a str,
    pub text: &'a str,
    pub what: &'a str,
    pub args: Vec<String>,
    pub outputs: Vec<String>,
}

/// Blender's full arguments for `script`, always headless. Paths are
/// whole arguments, so a space in a folder name stays one.
pub(crate) fn args(script: &Script<'_>) -> Vec<String> {
    let mut args: Vec<String> = [
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
    args.extend(script.args.iter().cloned());
    args
}

/// Runs `script` inside `dir` and blocks until Blender is done.
///
/// # Errors
///
/// Fails without Blender, or when the script fails (with the last error
/// line of its log).
pub(crate) fn run(dir: &Path, script: &Script<'_>, tools: &Toolbox) -> CoreResult<()> {
    if !tools.has(ToolId::Blender) {
        return Err(CoreError::Unavailable(format!(
            "Blender is not installed, so {} cannot run. Install it, then try again.",
            script.what
        )));
    }
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    let path = dir.join(script.file);
    crate::fs::write_atomic(&path, script.text.as_bytes()).map_err(|e| CoreError::io(&path, e))?;
    let def = PipelineDef {
        version: PIPELINE_VERSION,
        root: None,
        steps: vec![StepDef {
            id: "parts".into(),
            name: script.what.to_string(),
            kind: StepKind::Blender,
            command: None,
            script: Some(script.file.into()),
            args: args(script),
            inputs: Vec::new(),
            outputs: script.outputs.clone(),
            depends_on: Vec::new(),
            compress: None,
        }],
        presets: Vec::new(),
    };
    let cache_dir = dir.join("runs");
    let record = PipelineRunner::new(Arc::new(NullSink)).run_blocking(RunSetup {
        project: LANE.to_string(),
        project_dir: dir.to_path_buf(),
        cache_dir: cache_dir.clone(),
        def,
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
    })?;
    if record.state == RunState::Done {
        return Ok(());
    }
    let lines = history::load_log(&cache_dir, record.number);
    let last = lines
        .iter()
        .rev()
        .find(|l| l.level == LogLevel::Error || l.text.contains("Error"))
        .map(|l| l.text.trim().to_string());
    let step = record.steps.iter().find_map(|s| s.message.clone());
    Err(CoreError::Invalid(format!(
        "{} did not finish. {}",
        capitalized(script.what),
        last.or(step)
            .unwrap_or_else(|| "Blender stopped without saying why.".to_string())
    )))
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}
