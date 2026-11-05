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
        not_shipped: None,
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

#[test]
fn a_watched_run_stopped_before_it_starts_runs_nothing() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    let dir = tempfile::tempdir().expect("tmp");
    let runner = PipelineRunner::new(Arc::new(super::super::runner::NullSink));
    let cancel = AtomicBool::new(true);
    let mut ticks = 0;
    let mut tick = || ticks += 1;
    let mut watch = Watch {
        cancel: &cancel,
        tick: &mut tick,
    };
    let err = ScriptRunner::run_watched(
        &runner,
        &Toolbox::default(),
        dir.path(),
        ("Recipe bake", ("x.py", "")),
        &[],
        &mut watch,
    )
    .expect_err("cancelled");
    assert_eq!(err.to_string(), CANCELLED);
    assert!(!dir.path().join("x.py").exists(), "nothing was written");
}

#[test]
fn a_watched_run_without_blender_says_so() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    let dir = tempfile::tempdir().expect("tmp");
    let runner = PipelineRunner::new(Arc::new(super::super::runner::NullSink));
    let cancel = AtomicBool::new(false);
    let mut tick = || {};
    let mut watch = Watch {
        cancel: &cancel,
        tick: &mut tick,
    };
    let err = ScriptRunner::run_watched(
        &runner,
        &Toolbox::default(),
        dir.path(),
        ("Recipe bake", ("x.py", "")),
        &[],
        &mut watch,
    )
    .expect_err("no Blender");
    assert_eq!(err.code(), "unavailable");
}

/// A long script in a real Blender is killed part way once cancel is set.
#[test]
fn a_watched_run_is_killed_part_way_when_cancelled() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("tmp");
    let runner = PipelineRunner::new(Arc::new(super::super::runner::NullSink));
    let mut tools = Toolbox {
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        os: Some(crate::toolchain::HostOs::current()),
        ..Toolbox::default()
    };
    tools.tools.insert(
        ToolId::Blender,
        crate::pipeline::tools::ResolvedTool {
            program: blender,
            prefix: Vec::new(),
            version: "test".into(),
        },
    );
    let cancel = AtomicBool::new(false);
    let started = Instant::now();
    let mut ticks = 0;
    let mut tick = || {
        ticks += 1;
        if started.elapsed() > Duration::from_secs(2) {
            cancel.store(true, Ordering::SeqCst);
        }
    };
    let mut watch = Watch {
        cancel: &cancel,
        tick: &mut tick,
    };
    let script = "import time\nprint('[sleep] progress 0/1', flush=True)\ntime.sleep(60)\n";
    let err = ScriptRunner::run_watched(
        &runner,
        &tools,
        dir.path(),
        ("Sleep", ("sleep.py", script)),
        &[],
        &mut watch,
    )
    .expect_err("cancelled");
    assert_eq!(err.to_string(), CANCELLED);
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "stopped part way, after {:?}",
        started.elapsed()
    );
    assert!(ticks > 2);
}
