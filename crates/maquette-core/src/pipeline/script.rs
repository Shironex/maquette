//! One Blender script run headless on the runner's own lane, in a scratch
//! folder, waited for: the animation library reads and retargets clips with
//! it, and the provider sends and the avatar steps run theirs through the
//! script runner port, which the runner implements here. Each run is one
//! step named for what runs, and a failure carries the script's own error
//! line when it printed one.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::history::{self, LogLevel, LogStream, RunState};
use super::runner::{JobRunner, PipelineRunner, RunOptions, RunSetup};
use super::tools::Toolbox;
use super::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::base::ports::{CANCELLED, ScriptRunner, Watch};
use crate::error::{CoreError, CoreResult};
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec};

/// The runner lane the library's Blender jobs share, one at a time.
pub const LANE: &str = "animations";
const TIMEOUT: Duration = Duration::from_secs(20 * 60);
const LOG_TAIL: usize = 30;
/// The step name of a run that names none: the library's clip jobs.
const CLIPS: &str = "Animation clips";
/// What a script prints before the reason it stopped (`[avatar] Error: ...`).
const SCRIPT_ERROR: &str = "] Error:";
/// How often a watched run checks its cancel flag and ticks.
const WATCH_EVERY: Duration = Duration::from_millis(250);

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

fn def(name: &str, script_file: &str, extra: &[String]) -> PipelineDef {
    PipelineDef {
        version: PIPELINE_VERSION,
        root: None,
        steps: vec![StepDef {
            id: "clips".into(),
            name: name.into(),
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

/// Why a run failed: the script's own error line when it printed one
/// (`[avatar] Error: the spring hair-1 no longer matches ...`), as `name`
/// failed with that reason, else the last error a step logged or the
/// runner's message. The end of the log goes to the app's log.
fn failure(cache_dir: &Path, name: &str, record: &history::RunRecord) -> String {
    let lines = history::load_log(cache_dir, record.number);
    let from_script = |l: &&history::LogEntry| l.stream != LogStream::System;
    let own = lines
        .iter()
        .rev()
        .filter(from_script)
        .find_map(|l| {
            let at = l.text.find(SCRIPT_ERROR)?;
            Some(l.text[at + SCRIPT_ERROR.len()..].trim().to_string())
        })
        .filter(|reason| !reason.is_empty())
        .map(|reason| format!("{name} failed: {}", sentence(&reason)));
    let tail: Vec<String> = lines
        .iter()
        .skip(lines.len().saturating_sub(LOG_TAIL))
        .map(|l| l.text.trim_end().to_string())
        .collect();
    tracing::warn!(step = name, log = %tail.join("\n"), "a Blender script failed");
    own.or_else(|| {
        lines
            .iter()
            .rev()
            .find(|l| l.level == LogLevel::Error && l.step.is_some())
            .map(|l| l.text.trim().to_string())
    })
    .or_else(|| record.steps.iter().find_map(|s| s.message.clone()))
    .unwrap_or_else(|| "Blender stopped without saying why.".to_string())
}

/// `reason` as a sentence: a capital first and a full stop.
fn sentence(reason: &str) -> String {
    let mut chars = reason.chars();
    let mut out: String = chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    if !out.ends_with(['.', '!', '?']) {
        out.push('.');
    }
    out
}

/// Runs `script` (written into `work` as `script_file`) headless with
/// `extra` args after `--`, and waits for it, as the library's clip step.
///
/// # Errors
///
/// Fails without Blender, or with the reason the run gave when it failed.
pub(crate) fn run_script(
    runner: &PipelineRunner,
    tools: &Toolbox,
    work: &Path,
    script: (&str, &str),
    extra: &[String],
) -> CoreResult<()> {
    run_step(runner, tools, work, CLIPS, script, extra)
}

/// As [`run_script`], as a step called `name`.
///
/// # Errors
///
/// As [`run_script`].
fn run_step(
    runner: &PipelineRunner,
    tools: &Toolbox,
    work: &Path,
    name: &str,
    (script_file, script): (&str, &str),
    extra: &[String],
) -> CoreResult<()> {
    let (setup, cache_dir) = setup(tools, work, name, (script_file, script), extra)?;
    let record = runner.run_blocking_with(setup, options())?;
    if record.state != RunState::Done {
        return Err(CoreError::Invalid(failure(&cache_dir, name, &record)));
    }
    Ok(())
}

fn options() -> RunOptions {
    RunOptions {
        timeout: Some(TIMEOUT),
        ..RunOptions::default()
    }
}

/// Writes the script into `work` and answers the run that runs it, and the
/// folder its records go in.
fn setup(
    tools: &Toolbox,
    work: &Path,
    name: &str,
    (script_file, script): (&str, &str),
    extra: &[String],
) -> CoreResult<(RunSetup, std::path::PathBuf)> {
    if !tools.has(ToolId::Blender) {
        return Err(CoreError::Unavailable(
            "Blender is not installed, and this needs it. Install it, then try again.".to_string(),
        ));
    }
    let path = work.join(script_file);
    crate::fs::write_atomic(&path, script.as_bytes()).map_err(|e| CoreError::io(&path, e))?;
    let cache_dir = work.join("runs");
    let setup = RunSetup {
        project: LANE.to_string(),
        project_dir: work.to_path_buf(),
        cache_dir: cache_dir.clone(),
        def: def(name, script_file, extra),
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
    };
    Ok((setup, cache_dir))
}

/// As [`run_step`], watched: queued on the lane like any other run, it is
/// cancelled (Blender killed) once `watch.cancel` turns true, and
/// `watch.tick` is called while it waits.
///
/// # Errors
///
/// As [`run_step`], and [`CANCELLED`] once stopped.
fn run_watched(
    runner: &PipelineRunner,
    tools: &Toolbox,
    work: &Path,
    (name, script): (&str, (&str, &str)),
    extra: &[String],
    watch: &mut Watch<'_>,
) -> CoreResult<()> {
    if watch.cancel.load(Ordering::SeqCst) {
        return Err(CoreError::Invalid(CANCELLED.to_string()));
    }
    let (setup, cache_dir) = setup(tools, work, name, script, extra)?;
    let queued = runner.submit_with(setup, options())?;
    let mut asked = false;
    loop {
        (watch.tick)();
        let (_, waiting) = runner.active(LANE);
        let busy =
            runner.running(LANE).contains(&queued.id) || waiting.iter().any(|r| r.id == queued.id);
        if !busy {
            break;
        }
        if !asked && watch.cancel.load(Ordering::SeqCst) {
            asked = true;
            if let Err(err) = runner.cancel(&queued.id) {
                tracing::warn!(error = %err, "could not cancel a watched Blender run");
            }
        }
        std::thread::sleep(WATCH_EVERY);
    }
    (watch.tick)();
    let record = history::load_runs(&cache_dir)
        .into_iter()
        .find(|r| r.id == queued.id)
        .ok_or_else(|| CoreError::NotFound(format!("the run {}", queued.id)))?;
    if asked || record.state == RunState::Cancelled {
        return Err(CoreError::Invalid(CANCELLED.to_string()));
    }
    if record.state != RunState::Done {
        return Err(CoreError::Invalid(failure(&cache_dir, name, &record)));
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

    fn run_step(
        &self,
        tools: &Toolbox,
        work: &Path,
        name: &str,
        script: (&str, &str),
        extra: &[String],
    ) -> CoreResult<()> {
        run_step(self, tools, work, name, script, extra)
    }

    fn run_watched(
        &self,
        tools: &Toolbox,
        work: &Path,
        step: (&str, (&str, &str)),
        extra: &[String],
        watch: &mut Watch<'_>,
    ) -> CoreResult<()> {
        run_watched(self, tools, work, step, extra, watch)
    }
}

#[cfg(test)]
mod tests;
