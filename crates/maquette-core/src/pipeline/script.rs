//! One Blender script run headless on the runner's own lane, in a scratch
//! folder, waited for: the animation library reads and retargets clips with
//! it, and the provider sends and the avatar steps run theirs through the
//! script runner port, which the runner implements here. Each run is one
//! step named for what runs, and a failure carries the script's own error
//! line when it printed one.

use std::path::Path;
use std::time::Duration;

use super::history::{self, LogLevel, LogStream, RunState};
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
/// The step name of a run that names none: the library's clip jobs.
const CLIPS: &str = "Animation clips";
/// What a script prints before the reason it stopped (`[avatar] Error: ...`).
const SCRIPT_ERROR: &str = "] Error:";

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
        },
        RunOptions {
            timeout: Some(TIMEOUT),
            ..RunOptions::default()
        },
    )?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::history::{LogEntry, RunRecord, StepRecord, StepState, save_log};

    fn line(stream: LogStream, level: LogLevel, text: &str) -> LogEntry {
        LogEntry {
            run_id: "animations-run-8".into(),
            step: Some("clips".into()),
            stream,
            level,
            text: text.into(),
            at_ms: 0.0,
        }
    }

    fn record(name: &str) -> RunRecord {
        RunRecord {
            id: "animations-run-8".into(),
            number: 8,
            project: LANE.into(),
            from: None,
            force: true,
            build_only: false,
            state: RunState::Failed,
            queued_at_ms: 0.0,
            started_at_ms: None,
            finished_at_ms: None,
            steps: vec![StepRecord {
                id: "clips".into(),
                name: name.into(),
                state: StepState::Failed,
                started_at_ms: None,
                duration_ms: None,
                exit_code: Some(1),
                message: Some("Blender exited with code 1.".into()),
                progress: None,
            }],
            message: None,
        }
    }

    #[test]
    fn a_failure_carries_the_scripts_own_error_and_the_steps_name() {
        let dir = tempfile::tempdir().expect("tmp");
        let lines = [
            line(
                LogStream::Stdout,
                LogLevel::Error,
                "[avatar] Error: the spring hair-1 no longer matches a part hanging off this model",
            ),
            line(LogStream::Stdout, LogLevel::Info, "Blender 5.2.2 LTS"),
            line(
                LogStream::System,
                LogLevel::Error,
                "Springs chain failed after 0.9s: Blender exited with code 1.",
            ),
        ];
        save_log(dir.path(), 8, &lines).expect("log");
        assert_eq!(
            failure(dir.path(), "Springs chain", &record("Springs chain")),
            "Springs chain failed: The spring hair-1 no longer matches a part hanging off this model."
        );
    }

    #[test]
    fn without_a_script_error_the_runner_says_why() {
        let dir = tempfile::tempdir().expect("tmp");
        let lines = [line(
            LogStream::System,
            LogLevel::Error,
            "Reweight failed after 2.0s: Blender exited with code 1.",
        )];
        save_log(dir.path(), 8, &lines).expect("log");
        assert_eq!(
            failure(dir.path(), "Reweight", &record("Reweight")),
            "Reweight failed after 2.0s: Blender exited with code 1."
        );
        let empty = tempfile::tempdir().expect("tmp");
        assert_eq!(
            failure(empty.path(), "Reweight", &record("Reweight")),
            "Blender exited with code 1."
        );
    }

    #[test]
    fn a_run_is_one_step_named_for_what_runs() {
        let named = def("Springs find", "avatar_springs.py", &[]);
        assert_eq!(named.steps[0].name, "Springs find");
        assert_eq!(named.steps[0].script.as_deref(), Some("avatar_springs.py"));
    }
}
