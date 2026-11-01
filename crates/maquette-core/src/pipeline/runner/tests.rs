use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use super::execution::UNCHANGED_MESSAGE;
use super::*;
use crate::pipeline::history::{self, LogLevel, RunState, StepState};
use crate::pipeline::template::character_preset;
use crate::pipeline::tools::{ResolvedTool, Toolbox};
use crate::pipeline::{
    CompressDefaults, CompressPreset, CompressSpec, PIPELINE_VERSION, PipelineDef, StepKind,
};
use crate::toolchain::{HostOs, ToolId};
use crate::workspace::{MeshoptLevel, TextureCodec};

/// Collects every event, for assertions.
#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<PipelineEvent>>,
}

impl RunSink for Recorder {
    fn event(&self, event: PipelineEvent) {
        lock(&self.events).push(event);
    }
}

impl Recorder {
    fn lines(&self) -> Vec<String> {
        lock(&self.events)
            .iter()
            .filter_map(|e| match e {
                PipelineEvent::Log { entry } => Some(entry.text.clone()),
                PipelineEvent::Run { .. } => None,
            })
            .collect()
    }

    fn wait_for_line(&self, needle: &str, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if self.lines().iter().any(|l| l.contains(needle)) {
                return true;
            }
            thread::sleep(Duration::from_millis(20));
        }
        false
    }
}

fn shell_step(id: &str, script: &str, outputs: &[&str], deps: &[&str]) -> StepDef {
    StepDef {
        id: id.into(),
        name: format!("Step {id}"),
        kind: StepKind::Shell,
        command: Some("sh".into()),
        script: Some(script.into()),
        args: vec!["{script}".into(), "{root}".into()],
        inputs: vec!["in/*.txt".into()],
        outputs: outputs.iter().map(|o| o.to_string()).collect(),
        depends_on: deps.iter().map(|d| d.to_string()).collect(),
        compress: None,
    }
}

fn setup(dir: &Path, steps: Vec<StepDef>) -> RunSetup {
    let project_dir = dir.join("project");
    fs::create_dir_all(project_dir.join("in")).expect("mkdir");
    fs::create_dir_all(project_dir.join("tools")).expect("mkdir");
    fs::write(project_dir.join("in/a.txt"), b"input").expect("write");
    RunSetup {
        project: "moon".into(),
        project_dir,
        cache_dir: dir.join("cache"),
        def: PipelineDef {
            version: PIPELINE_VERSION,
            root: None,
            steps,
            presets: Vec::new(),
        },
        tools: Toolbox {
            path_env: OsString::from("/bin:/usr/bin"),
            os: Some(HostOs::current()),
            ..Toolbox::default()
        },
        defaults: CompressDefaults {
            texture_size: 2048,
            codec: TextureCodec::Etc1s,
            meshopt: MeshoptLevel::High,
        },
        vars: BTreeMap::new(),
        shipped_names: BTreeMap::new(),
        materials: Default::default(),
        from: None,
        force: false,
        textures: Default::default(),
        quality: crate::ship::DEFAULT_QUALITY,
        build_only: false,
    }
}

fn script(setup: &RunSetup, name: &str, body: &str) {
    fs::write(setup.project_dir.join("tools").join(name), body).expect("script");
}

fn states(run: &RunRecord) -> Vec<StepState> {
    run.steps.iter().map(|s| s.state).collect()
}

#[test]
fn runs_streams_and_then_caches() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![
            shell_step("build", "tools/build.sh", &["out/build.txt"], &[]),
            shell_step("export", "tools/export.sh", &["out/export.txt"], &["build"]),
        ],
    );
    script(
        &s,
        "build.sh",
        "echo building in \"$1\"\necho oops >&2\nmkdir -p out\ncat in/a.txt > out/build.txt\n",
    );
    script(
        &s,
        "export.sh",
        "echo exporting\nmkdir -p out\necho glb > out/export.txt\n",
    );
    let sink = Arc::new(Recorder::default());
    let runner = PipelineRunner::new(sink.clone());

    let first = runner.run_blocking(s.clone()).expect("first run");
    assert_eq!(first.state, RunState::Done, "{:?}", first.message);
    assert_eq!(states(&first), vec![StepState::Done, StepState::Done]);
    assert_eq!(first.number, 1);
    let lines = sink.lines();
    assert!(
        lines.iter().any(|l| l.starts_with("building in /")),
        "{lines:?}"
    );
    assert!(lines.iter().any(|l| l == "oops"));
    assert!(lines.iter().any(|l| l == "exporting"));

    let second = runner.run_blocking(s.clone()).expect("second run");
    assert_eq!(second.number, 2);
    assert_eq!(states(&second), vec![StepState::Cached, StepState::Cached]);

    // A changed input reruns the step that reads it; the next one sees
    // the same input bytes it had before only if the output is unchanged.
    fs::write(s.project_dir.join("in/a.txt"), b"changed").expect("write");
    let third = runner.run_blocking(s.clone()).expect("third run");
    assert_eq!(states(&third), vec![StepState::Done, StepState::Done]);

    let mut forced = s.clone();
    forced.force = true;
    let fourth = runner.run_blocking(forced).expect("forced");
    assert_eq!(states(&fourth), vec![StepState::Done, StepState::Done]);

    let mut from = s.clone();
    from.from = Some("export".into());
    let fifth = runner.run_blocking(from).expect("from here");
    assert_eq!(states(&fifth), vec![StepState::Skipped, StepState::Done]);

    let runs = history::load_runs(&s.cache_dir);
    assert_eq!(runs.len(), 5);
    assert_eq!(runs[0].number, 5);
    assert!(!history::load_log(&s.cache_dir, 5).is_empty());
}

#[test]
fn progress_lines_reach_the_step_record() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![shell_step("bake", "tools/bake.sh", &["out/x.txt"], &[])],
    );
    script(
        &s,
        "bake.sh",
        "echo '[bake] progress 1/4 rock'\nsleep 0.3\necho '[bake] progress 3/4 tree'\nsleep 0.3\nmkdir -p out\necho x > out/x.txt\n",
    );
    let sink = Arc::new(Recorder::default());
    let run = PipelineRunner::new(sink.clone())
        .run_blocking(s)
        .expect("run");
    assert_eq!(run.state, RunState::Done);
    assert_eq!(run.steps[0].progress, Some(1.0));
    let seen: Vec<f64> = lock(&sink.events)
        .iter()
        .filter_map(|e| match e {
            PipelineEvent::Run { run } if run.steps[0].state == StepState::Running => {
                run.steps[0].progress
            }
            _ => None,
        })
        .collect();
    assert!(seen.contains(&0.25), "{seen:?}");
    assert!(seen.contains(&0.75), "{seen:?}");
}

#[test]
fn exit_codes_fail_the_run_and_skip_the_rest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![
            shell_step("build", "tools/build.sh", &["out/build.txt"], &[]),
            shell_step("export", "tools/export.sh", &["out/export.txt"], &["build"]),
        ],
    );
    script(
        &s,
        "build.sh",
        "echo 'Error: camera poster not found'\nexit 3\n",
    );
    script(&s, "export.sh", "echo never\n");
    let sink = Arc::new(Recorder::default());
    let runner = PipelineRunner::new(sink.clone());
    let run = runner.run_blocking(s).expect("run");
    assert_eq!(run.state, RunState::Failed);
    assert_eq!(states(&run), vec![StepState::Failed, StepState::Skipped]);
    assert_eq!(run.steps[0].exit_code, Some(3));
    assert!(
        run.message
            .as_deref()
            .unwrap_or_default()
            .contains("exited with code 3")
    );
    let errors: Vec<LogEntry> = lock(&sink.events)
        .iter()
        .filter_map(|e| match e {
            PipelineEvent::Log { entry } if entry.level == LogLevel::Error => Some(entry.clone()),
            _ => None,
        })
        .collect();
    assert!(errors.iter().any(|e| e.text.contains("camera poster")));
    assert!(!sink.lines().iter().any(|l| l == "never"));
}

#[test]
fn a_step_that_writes_nothing_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![shell_step("build", "tools/build.sh", &["out/*.glb"], &[])],
    );
    script(&s, "build.sh", "echo done\n");
    let run = PipelineRunner::new(Arc::new(NullSink))
        .run_blocking(s)
        .expect("run");
    assert_eq!(run.state, RunState::Failed);
    assert!(
        run.steps[0]
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("wrote nothing to out/*.glb")
    );
}

#[test]
fn missing_script_and_missing_tool_fail_cleanly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut blender = shell_step("bake", "tools/bake.py", &["out/x"], &[]);
    blender.kind = StepKind::Blender;
    blender.command = None;
    blender.args = vec![
        "-b".into(),
        "x.blend".into(),
        "--python".into(),
        "{script}".into(),
    ];
    let s = setup(dir.path(), vec![blender]);
    let run = PipelineRunner::new(Arc::new(NullSink))
        .run_blocking(s.clone())
        .expect("run");
    assert_eq!(run.state, RunState::Failed);
    assert!(
        run.steps[0]
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("Blender is not installed")
    );

    let s = setup(
        dir.path(),
        vec![shell_step("build", "tools/nope.sh", &["out/x"], &[])],
    );
    let run = PipelineRunner::new(Arc::new(NullSink))
        .run_blocking(s)
        .expect("run");
    assert!(
        run.steps[0]
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("tools/nope.sh is missing")
    );
}

#[test]
fn blender_without_background_is_refused_before_spawning() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut step = shell_step("build", "tools/build.py", &["out/x"], &[]);
    step.kind = StepKind::Blender;
    step.command = None;
    step.args = vec!["scene.blend".into(), "--python".into(), "{script}".into()];
    let s = setup(dir.path(), vec![step]);
    let err = PipelineRunner::new(Arc::new(NullSink))
        .submit(s)
        .expect_err("refused");
    assert!(err.to_string().contains("always run headless"));
}

#[test]
fn cancel_kills_the_process_tree() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![shell_step("build", "tools/build.sh", &["out/x"], &[])],
    );
    // A grandchild that would write a marker after a second, and a long
    // sleep in the step itself.
    script(
        &s,
        "build.sh",
        "(sleep 1; echo late > \"$1/marker\") &\necho started\nsleep 30\n",
    );
    let sink = Arc::new(Recorder::default());
    let runner = PipelineRunner::new(sink.clone());
    let started = Instant::now();
    let run = runner.submit(s.clone()).expect("submit");
    assert!(sink.wait_for_line("started", Duration::from_secs(10)));
    // A second run queues behind the first and is cancelled while queued.
    let queued = runner.submit(s.clone()).expect("queue");
    assert_eq!(queued.state, RunState::Queued);
    runner.cancel(&queued.id).expect("cancel queued");
    runner.cancel(&run.id).expect("cancel");
    loop {
        if runner.active("moon").0.is_none() {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "cancel did not stop the run"
        );
        thread::sleep(Duration::from_millis(20));
    }
    assert!(started.elapsed() < Duration::from_secs(10));
    thread::sleep(Duration::from_millis(1500));
    assert!(
        !s.project_dir.join("marker").exists(),
        "the grandchild survived"
    );
    let runs = history::load_runs(&s.cache_dir);
    let first = runs.iter().find(|r| r.id == run.id).expect("first");
    assert_eq!(first.state, RunState::Cancelled);
    assert_eq!(first.steps[0].state, StepState::Cancelled);
    let second = runs.iter().find(|r| r.id == queued.id).expect("second");
    assert_eq!(second.state, RunState::Cancelled);
    assert_eq!(
        runner.cancel("nope").expect_err("unknown").code(),
        "not-found"
    );
}

#[test]
fn runs_queue_one_at_a_time_per_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![shell_step("build", "tools/build.sh", &["out/x.txt"], &[])],
    );
    script(
        &s,
        "build.sh",
        "mkdir -p out\necho start >> out/order.log\nsleep 0.3\necho end >> out/order.log\necho x > out/x.txt\n",
    );
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let mut forced = s.clone();
    forced.force = true;
    let a = runner.submit(forced.clone()).expect("a");
    let b = runner.submit(forced.clone()).expect("b");
    assert_eq!(b.state, RunState::Queued);
    assert_eq!(runner.active("moon").1.len(), 1);
    while runner.active("moon").0.is_some() {
        thread::sleep(Duration::from_millis(20));
    }
    let log = fs::read_to_string(s.project_dir.join("out/order.log")).expect("log");
    assert_eq!(log, "start\nend\nstart\nend\n");
    let runs = history::load_runs(&s.cache_dir);
    assert_eq!(runs.len(), 2);
    assert!(runs.iter().all(|r| r.state == RunState::Done));
    assert_eq!((a.number, b.number), (1, 2));
}

#[test]
fn a_lane_runs_up_to_its_limit_at_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![shell_step("job", "tools/job.sh", &[], &[])],
    );
    // Each job logs its start, then waits until two jobs have started before
    // it ends. With a limit of two the first pair meets and the third can only
    // start once one of them is done. The proof is the order of the log, never
    // the wall clock, so a busy machine only makes it slower.
    script(
        &s,
        "job.sh",
        "echo \"start $$\" >> order.log\n\
         i=0\n\
         while [ \"$(grep -c '^start' order.log)\" -lt 2 ] && [ $i -lt 400 ]; do sleep 0.05; i=$((i+1)); done\n\
         [ \"$(grep -c '^start' order.log)\" -ge 2 ] || echo \"timeout $$\" >> order.log\n\
         echo \"end $$\" >> order.log\n",
    );
    let runner = PipelineRunner::new(Arc::new(NullSink));
    runner.set_limit("moon", 2);
    let mut forced = s.clone();
    forced.force = true;
    for _ in 0..3 {
        runner.submit(forced.clone()).expect("submit");
    }
    while !runner.running("moon").is_empty() || !runner.active("moon").1.is_empty() {
        thread::sleep(Duration::from_millis(20));
    }
    let log = fs::read_to_string(s.project_dir.join("order.log")).expect("log");
    let words: Vec<&str> = log
        .lines()
        .map(|l| l.split(' ').next().unwrap_or(""))
        .collect();
    // The first two overlap (each waited for the other), the third starts only
    // after a slot was freed, and nobody timed out waiting for a partner.
    assert!(!log.contains("timeout"), "{log}");
    assert_eq!(words.iter().filter(|w| **w == "start").count(), 3, "{log}");
    assert_eq!(&words[..2], ["start", "start"], "{log}");
    assert_eq!(words[2], "end", "{log}");
    let third = words.iter().rposition(|w| *w == "start").expect("third");
    assert!(third >= 3, "{log}");
    let done = history::load_runs(&s.cache_dir);
    assert!(done.iter().all(|r| r.state == RunState::Done), "{done:?}");
}

#[test]
fn stdin_reaches_the_program_and_timeouts_stop_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![shell_step(
            "read",
            "tools/read.sh",
            &["out/prompt.txt"],
            &[],
        )],
    );
    script(
        &s,
        "read.sh",
        "mkdir -p out\ncat > out/prompt.txt\necho \"got $(wc -l < out/prompt.txt) lines\"\n",
    );
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let options = RunOptions {
        stdin: Some("Generate an image.\nA small island.\n".into()),
        ..RunOptions::default()
    };
    let run = runner.submit_with(s.clone(), options).expect("submit");
    while runner.active("moon").0.is_some() {
        thread::sleep(Duration::from_millis(20));
    }
    let done = history::load_runs(&s.cache_dir);
    assert_eq!(done[0].id, run.id);
    assert_eq!(done[0].state, RunState::Done, "{:?}", done[0].message);
    assert_eq!(
        fs::read_to_string(s.project_dir.join("out/prompt.txt")).expect("prompt"),
        "Generate an image.\nA small island.\n"
    );

    script(&s, "read.sh", "echo waiting\nsleep 30\n");
    let mut forced = s.clone();
    forced.force = true;
    let options = RunOptions {
        timeout: Some(Duration::from_millis(400)),
        ..RunOptions::default()
    };
    let started = Instant::now();
    runner.submit_with(forced, options).expect("submit");
    while runner.active("moon").0.is_some() {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(started.elapsed() < Duration::from_secs(10));
    let runs = history::load_runs(&s.cache_dir);
    assert_eq!(runs[0].state, RunState::Failed);
    assert!(
        runs[0].steps[0]
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("ran longer than 0.4s"),
        "{:?}",
        runs[0].steps[0].message
    );
}

/// A gate on the "ship" step that answers from a channel.
struct Approval {
    answer: Mutex<mpsc::Receiver<GateDecision>>,
    asked: AtomicBool,
}

impl StepGate for Approval {
    fn applies(&self, step: &StepDef) -> bool {
        step.id == "ship"
    }
    fn waiting_message(&self, _step: &StepDef) -> String {
        "Waiting for approval".into()
    }
    fn wait(&self, _run: &RunRecord, _step: &StepDef, cancelled: &AtomicBool) -> GateDecision {
        self.asked.store(true, Ordering::SeqCst);
        loop {
            if cancelled.load(Ordering::SeqCst) {
                return GateDecision::Deny("Cancelled.".into());
            }
            if let Ok(answer) = lock(&self.answer).recv_timeout(Duration::from_millis(20)) {
                return answer;
            }
        }
    }
}

#[test]
fn a_gate_holds_the_step_until_it_is_approved_or_denied() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![
            shell_step("build", "tools/build.sh", &["out/build.txt"], &[]),
            shell_step("ship", "tools/ship.sh", &[], &["build"]),
        ],
    );
    script(&s, "build.sh", "mkdir -p out\necho b > out/build.txt\n");
    for (round, (answer, state)) in [
        (GateDecision::Allow, RunState::Done),
        (
            GateDecision::Deny("Shipping was denied in Maquette.".into()),
            RunState::Cancelled,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        // A changed script, so the second round has something new to ship.
        script(
            &s,
            "ship.sh",
            &format!("echo shipped >> shipped.log\n# round {round}\n"),
        );
        let (tx, rx) = mpsc::channel();
        let gate = Arc::new(Approval {
            answer: Mutex::new(rx),
            asked: AtomicBool::new(false),
        });
        let sink = Arc::new(Recorder::default());
        let runner = PipelineRunner::new(sink.clone());
        let options = RunOptions {
            gate: Some(gate.clone()),
            ..RunOptions::default()
        };
        runner.submit_with(s.clone(), options).expect("submit");
        assert!(sink.wait_for_line("Waiting for approval", Duration::from_secs(10)));
        assert!(gate.asked.load(Ordering::SeqCst));
        tx.send(answer).expect("answer");
        while runner.active("moon").0.is_some() {
            thread::sleep(Duration::from_millis(20));
        }
        let run = history::load_runs(&s.cache_dir).remove(0);
        assert_eq!(run.state, state, "{:?}", run.message);
        if state == RunState::Cancelled {
            assert_eq!(run.steps[1].state, StepState::Cancelled);
            assert_eq!(
                run.message.as_deref(),
                Some("Shipping was denied in Maquette.")
            );
        }
    }
    // Shipped once: after the approval, never after the denial.
    let shipped = fs::read_to_string(s.project_dir.join("shipped.log")).expect("log");
    assert_eq!(shipped, "shipped\n");
}

#[test]
fn a_ship_step_with_no_repo_is_left_out_without_asking_and_the_run_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut ship = shell_step("ship", "tools/ship.sh", &[], &[]);
    ship.args.extend(["--site".into(), "{siteRepo}".into()]);
    let mut s = setup(dir.path(), vec![ship]);
    s.vars.insert("siteRepo".into(), String::new());
    // An older ship.mjs passes here; the run must not read as shipped either way.
    script(&s, "ship.sh", "echo ran >> ran.log\n");
    let (tx, rx) = mpsc::channel();
    let gate = Arc::new(Approval {
        answer: Mutex::new(rx),
        asked: AtomicBool::new(false),
    });
    let sink = Arc::new(Recorder::default());
    let runner = PipelineRunner::new(sink.clone());
    let options = RunOptions {
        gate: Some(gate.clone()),
        ..RunOptions::default()
    };
    runner.submit_with(s.clone(), options).expect("submit");
    assert!(sink.wait_for_line("nothing shipped", Duration::from_secs(10)));
    while runner.active("moon").0.is_some() {
        thread::sleep(Duration::from_millis(20));
    }
    drop(tx);
    let run = history::load_runs(&s.cache_dir).remove(0);
    assert_eq!(run.state, RunState::Done);
    assert_eq!(states(&run), vec![StepState::Skipped]);
    let reason = run.not_shipped.clone().expect("the reason");
    assert!(reason.contains("No site repo"), "{reason}");
    assert_eq!(run.ship_outcome(), Some((false, Some(reason))));
    assert!(!gate.asked.load(Ordering::SeqCst), "nothing to approve");
    assert!(
        !s.project_dir.join("ran.log").exists(),
        "the script did not run"
    );
}

#[test]
fn a_ship_step_with_a_repo_ships() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut ship = shell_step("ship", "tools/ship.sh", &[], &[]);
    ship.args.extend(["--site".into(), "{siteRepo}".into()]);
    let mut s = setup(dir.path(), vec![ship]);
    s.vars.insert("siteRepo".into(), "/some/site".into());
    script(&s, "ship.sh", "echo ran\n");
    let sink = Arc::new(Recorder::default());
    let runner = PipelineRunner::new(sink.clone());
    runner
        .submit_with(s.clone(), RunOptions::default())
        .expect("submit");
    while runner.active("moon").0.is_some() {
        thread::sleep(Duration::from_millis(20));
    }
    let run = history::load_runs(&s.cache_dir).remove(0);
    assert_eq!(run.not_shipped, None);
    assert_eq!(run.ship_outcome(), Some((true, None)));
}

#[test]
fn a_gated_step_with_nothing_new_is_skipped_without_asking() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![
            shell_step("build", "tools/build.sh", &["out/build.txt"], &[]),
            shell_step("ship", "tools/ship.sh", &[], &["build"]),
        ],
    );
    script(&s, "build.sh", "mkdir -p out\necho b > out/build.txt\n");
    script(&s, "ship.sh", "echo shipped >> shipped.log\n");
    let run_once = |answer: Option<GateDecision>| {
        let (tx, rx) = mpsc::channel();
        let gate = Arc::new(Approval {
            answer: Mutex::new(rx),
            asked: AtomicBool::new(false),
        });
        if let Some(answer) = answer {
            tx.send(answer).expect("answer");
        }
        let runner = PipelineRunner::new(Arc::new(Recorder::default()));
        let options = RunOptions {
            gate: Some(gate.clone()),
            ..RunOptions::default()
        };
        runner.submit_with(s.clone(), options).expect("submit");
        while runner.active("moon").0.is_some() {
            thread::sleep(Duration::from_millis(20));
        }
        (
            gate.asked.load(Ordering::SeqCst),
            history::load_runs(&s.cache_dir).remove(0),
        )
    };
    let (asked, run) = run_once(Some(GateDecision::Allow));
    assert!(asked);
    assert_eq!(run.steps[1].state, StepState::Done);

    // Nothing changed: no question, and Ship counts as cached.
    let (asked, run) = run_once(None);
    assert!(!asked, "the gate was asked although nothing changed");
    assert_eq!(run.state, RunState::Done);
    assert_eq!(run.steps[1].state, StepState::Cached);
    assert_eq!(run.steps[1].message.as_deref(), Some(UNCHANGED_MESSAGE));

    // A changed input means a new ship, so it asks again.
    fs::create_dir_all(s.project_dir.join("in")).expect("in");
    fs::write(s.project_dir.join("in/model.txt"), "v2").expect("input");
    let (asked, _) = run_once(Some(GateDecision::Allow));
    assert!(asked);
    let shipped = fs::read_to_string(s.project_dir.join("shipped.log")).expect("log");
    assert_eq!(shipped, "shipped\nshipped\n");
}

/// A compress-only pipeline over `web/raw/<model>.glb` files, with a fake
/// gltf-transform that logs its args and copies input to output.
fn compress_setup(dir: &Path, models: &[&str]) -> RunSetup {
    let step = StepDef {
        id: "compress".into(),
        name: "Compress".into(),
        kind: StepKind::GltfTransform,
        command: None,
        script: None,
        args: Vec::new(),
        inputs: vec!["web/raw/*.glb".into()],
        outputs: vec!["web/dist/*.glb".into()],
        depends_on: Vec::new(),
        compress: Some(CompressSpec {
            from: "web/raw".into(),
            to: "web/dist".into(),
        }),
    };
    let mut s = setup(dir, vec![step]);
    let raw = s.project_dir.join("web/raw");
    fs::create_dir_all(&raw).expect("mkdir");
    for model in models {
        fs::write(raw.join(format!("{model}.glb")), b"glb").expect("write");
    }
    let fake = dir.join("gltf-transform");
    fs::write(&fake, "#!/bin/sh\necho \"args: $*\"\ncp \"$2\" \"$3\"\n").expect("fake");
    let mut perms = fs::metadata(&fake).expect("meta").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    fs::set_permissions(&fake, perms).expect("chmod");
    s.tools.tools.insert(
        ToolId::GltfTransform,
        ResolvedTool {
            program: fake,
            prefix: Vec::new(),
            version: "4.5.1".into(),
        },
    );
    s
}

#[test]
fn compress_expands_presets_into_gltf_transform_calls() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut s = compress_setup(dir.path(), &["character", "bench"]);
    s.def.presets = vec![character_preset()];
    let sink = Arc::new(Recorder::default());
    let run = PipelineRunner::new(sink.clone())
        .run_blocking(s.clone())
        .expect("run");
    assert_eq!(
        run.state,
        RunState::Done,
        "{:?} {:?}",
        run.message,
        sink.lines()
    );
    let lines = sink.lines();
    let calls: Vec<&String> = lines.iter().filter(|l| l.starts_with("args: ")).collect();
    assert_eq!(calls.len(), 2, "{lines:?}");
    assert!(
        calls[0].contains("meshopt")
            && calls[0].contains("bench.glb")
            && calls[0].ends_with("--level high")
    );
    assert!(calls[1].contains("character.glb") && calls[1].ends_with("--level medium"));
    assert!(lines.iter().any(|l| l.contains("toktx is missing")));
    assert!(s.project_dir.join("web/dist/character.glb").is_file());
}

#[test]
fn a_renamed_model_ships_under_its_new_name_and_the_old_file_goes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut s = compress_setup(dir.path(), &["clay_v1", "bench", "plinth_v1"]);
    s.def.presets = vec![CompressPreset {
        model: "clay_v1".into(),
        label: "Clay".into(),
        texture_size: 512,
        data_texture_size: None,
        codec: TextureCodec::Uastc,
        map_codecs: None,
        meshopt: MeshoptLevel::Low,
        lock: None,
    }];
    // The asset behind clay_v1 is now called "Paper clay". The new name
    // for plinth_v1 would land on the bench file, so it keeps its own.
    s.shipped_names = BTreeMap::from([
        ("clay_v1".to_string(), "paper_clay_v1".to_string()),
        ("plinth_v1".to_string(), "bench".to_string()),
    ]);
    let dist = s.project_dir.join("web/dist");
    fs::create_dir_all(&dist).expect("mkdir");
    fs::write(dist.join("clay_v1.glb"), b"old").expect("old");
    // A model from a layout entry that is gone: it would ship on every Ship.
    fs::write(dist.join("desk.glb"), b"stale").expect("stale");
    let sink = Arc::new(Recorder::default());
    let runner = PipelineRunner::new(sink.clone());
    let run = runner.run_blocking(s.clone()).expect("run");
    assert_eq!(run.state, RunState::Done, "{:?}", sink.lines());
    let present = |name: &str| dist.join(name).is_file();
    assert!(present("paper_clay_v1.glb") && present("bench.glb") && present("plinth_v1.glb"));
    assert!(!present("clay_v1.glb"), "the old name would ship twice");
    assert!(
        !present("desk.glb"),
        "a model nothing makes any more is removed"
    );
    assert!(
        sink.lines()
            .iter()
            .any(|l| l.contains("Removed web/dist/desk.glb")),
        "{:?}",
        sink.lines()
    );
    // The preset still follows the model file.
    let lines = sink.lines();
    assert!(
        lines
            .iter()
            .any(|l| l.contains("clay_v1 geometry quantized"))
    );

    // A rename changes the cache key, so the step runs again.
    // A stale file is cleared even when the step comes from the cache.
    fs::write(dist.join("desk.glb"), b"stale").expect("stale");
    let again = runner.run_blocking(s.clone()).expect("again");
    assert_eq!(again.steps[0].state, StepState::Cached);
    assert!(!present("desk.glb"));
    s.shipped_names
        .insert("clay_v1".into(), "tin_clay_v1".into());
    let renamed = runner.run_blocking(s).expect("renamed");
    assert_eq!(renamed.steps[0].state, StepState::Done);
    assert!(present("tin_clay_v1.glb") && !present("paper_clay_v1.glb"));
}

#[test]
fn compress_reads_a_tweaked_copy_and_reruns_when_the_tweaks_change() {
    use crate::materials::{MaterialTweak, ModelMaterials};
    let dir = tempfile::tempdir().expect("tempdir");
    let mut s = compress_setup(dir.path(), &["bench"]);
    let raw = s.project_dir.join("web/raw/crate_v1.glb");
    let source = crate::inbox::fixture::clean_prop();
    fs::write(&raw, &source).expect("fixture");
    let matte = |metalness: f32| ModelMaterials {
        all: MaterialTweak {
            metalness: Some(metalness),
            ..MaterialTweak::default()
        },
        materials: BTreeMap::new(),
    };
    s.materials = BTreeMap::from([("crate_v1".to_string(), matte(0.0))]);
    let sink = Arc::new(Recorder::default());
    let runner = PipelineRunner::new(sink.clone());
    let run = runner.run_blocking(s.clone()).expect("run");
    assert_eq!(run.state, RunState::Done, "{:?}", sink.lines());
    let lines = sink.lines();
    assert!(
        lines.iter().any(|l| l.contains(
            "crate_v1 material tweaks on 1 material, 1 metal and roughness map replaced"
        )),
        "{lines:?}"
    );
    let calls: Vec<&String> = lines.iter().filter(|l| l.starts_with("args: ")).collect();
    let crate_call = calls
        .iter()
        .find(|l| l.contains("crate_v1"))
        .expect("crate call");
    assert!(crate_call.contains("crate_v1.look.glb"), "{crate_call}");
    let bench_call = calls
        .iter()
        .find(|l| l.contains("bench"))
        .expect("bench call");
    assert!(
        bench_call.contains("web/raw/bench.glb"),
        "an untweaked model reads its raw file"
    );
    assert_eq!(
        fs::read(&raw).expect("raw"),
        source,
        "the raw export never changes"
    );
    let shipped = fs::read(s.project_dir.join("web/dist/crate_v1.glb")).expect("shipped");
    let gltf = gltf::Gltf::from_slice(&shipped).expect("glb");
    let material = gltf.document.materials().next().expect("material");
    assert!(material.pbr_metallic_roughness().metallic_factor().abs() < 1e-6);

    assert_eq!(
        runner.run_blocking(s.clone()).expect("again").steps[0].state,
        StepState::Cached
    );
    s.materials.insert("crate_v1".into(), matte(0.5));
    assert_eq!(
        runner.run_blocking(s).expect("changed").steps[0].state,
        StepState::Done
    );
}

#[test]
fn run_next_moves_queued_runs_to_the_front_of_the_lane() {
    let dir = tempfile::tempdir().expect("tempdir");
    let s = setup(
        dir.path(),
        vec![shell_step("job", "tools/job.sh", &[], &[])],
    );
    // Each run notes its mark, then holds the lane a moment.
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let runs: Vec<RunRecord> = ["a", "b", "c", "d"]
        .iter()
        .map(|mark| {
            let mut forced = s.clone();
            forced.force = true;
            forced.def.steps[0].args =
                vec!["-c".into(), format!("echo {mark} >> order.log; sleep 0.2")];
            forced.def.steps[0].script = None;
            runner.submit(forced).expect("submit")
        })
        .collect();
    let ids = |names: &[usize]| {
        names
            .iter()
            .map(|&i| runs[i].id.clone())
            .collect::<Vec<_>>()
    };
    // The running one and unknown ids are left alone.
    assert_eq!(runner.run_next("moon", &ids(&[0])), 0);
    assert_eq!(runner.run_next("elsewhere", &ids(&[3])), 0);
    assert_eq!(runner.run_next("moon", &ids(&[3, 2])), 2);
    let queued: Vec<String> = runner
        .active("moon")
        .1
        .iter()
        .map(|r| r.id.clone())
        .collect();
    assert_eq!(
        queued,
        ids(&[2, 3, 1]),
        "moved in the order they were queued"
    );
    while runner.active("moon").0.is_some() || !runner.active("moon").1.is_empty() {
        thread::sleep(Duration::from_millis(20));
    }
    let log = fs::read_to_string(s.project_dir.join("order.log")).expect("log");
    assert_eq!(log, "a\nc\nd\nb\n");
}
