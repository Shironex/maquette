//! One Blender script run headless on the runner's own lane, in a scratch
//! folder, waited for: the animation library reads and retargets clips with
//! it, and the provider sends turn a model file into a GLB with it through
//! the script runner port, which the runner implements here.

use std::path::Path;
use std::time::Duration;

use super::history::{self, LogLevel, RunState};
use super::runner::{PipelineRunner, RunOptions, RunSetup};
use super::tools::Toolbox;
use super::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec};

/// The runner lane the library's Blender jobs share, one at a time.
pub const LANE: &str = "animations";
const TIMEOUT: Duration = Duration::from_secs(20 * 60);
const LOG_TAIL: usize = 30;

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

impl ScriptRunner for PipelineRunner {
    fn run_script(
        &self,
        tools: &Toolbox,
        work: &Path,
        script: (&str, &str),
        extra: &[String],
    ) -> CoreResult<()> {
        run_script(self, tools, work, script, extra)
    }
}
