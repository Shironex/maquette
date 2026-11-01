//! A later round against an earlier draft, with a script runner that writes
//! what the fix would, so no Blender is needed.

use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use serde_json::json;

use super::*;
use crate::pipeline::tools::Toolbox;
use crate::workspace::WorkspaceConfig;

/// A fix that overwrites its output and reports no vertex moved.
struct MovesNothing;

impl crate::base::ports::ScriptRunner for MovesNothing {
    fn run_script(
        &self,
        _: &Toolbox,
        _: &Path,
        _: (&str, &str),
        args: &[String],
    ) -> crate::error::CoreResult<()> {
        let output = &args[1];
        std::fs::write(output, b"what the fix wrote").expect("output");
        std::fs::write(
            format!("{output}.report.json"),
            json!({ "changed": 0 }).to_string(),
        )
        .expect("report");
        Ok(())
    }
}

/// A person's case after a first round that made a draft.
fn after_round_one(root: &Path, change: impl FnOnce(&mut RigRecord)) -> RigRecord {
    let config = WorkspaceConfig::default();
    let dir = work_dir(root, &config, "hero-v1");
    std::fs::create_dir_all(&dir).expect("dir");
    let draft = dir.join("draft.glb");
    std::fs::write(&draft, b"round one").expect("draft");
    std::fs::write(dir.join("in.glb"), b"model").expect("in");
    let mut c = record::blank(
        "hero-v1".to_string(),
        "Hero".to_string(),
        &dir,
        &dir.join("in.glb"),
        RigSource {
            inbox: None,
            project: Some("p".to_string()),
            asset: Some("hero".to_string()),
            version: Some(1),
        },
    );
    c.route = Some(RigRoute::Humanoid);
    c.state = RigState::Fixed;
    c.step = RigStep::Land;
    c.round = 1;
    c.draft = Some(draft.display().to_string());
    c.before = Some(RigSheet::default());
    c.after = Some(RigSheet::default());
    change(&mut c);
    save(&c).expect("save");
    c
}

/// Runs round 2 and answers with the case once the run is over.
fn round_two(root: &Path) -> RigRecord {
    let signal = Arc::new((Mutex::new(false), Condvar::new()));
    let seen = Arc::clone(&signal);
    let doctors = RigDoctors::new(Arc::new(MovesNothing), move |case: &RigRecord| {
        if !case.state.running() {
            *seen.0.lock().expect("lock") = true;
            seen.1.notify_all();
        }
    });
    let config = WorkspaceConfig::default();
    let tools = Toolbox {
        tools: std::collections::BTreeMap::new(),
        path_env: std::ffi::OsString::new(),
        os: None,
    };
    doctors
        .run(root, &config, tools, "hero-v1", RigRun::Round2)
        .expect("started");
    let (done, _) = signal
        .1
        .wait_timeout_while(
            signal.0.lock().expect("lock"),
            Duration::from_secs(30),
            |over| !*over,
        )
        .expect("wait");
    assert!(*done, "the round did not finish");
    drop(done);
    // The thread reports, then drops the case from the running set.
    for _ in 0..200 {
        if doctors.active().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    doctors.get(root, &config, "hero-v1").expect("case")
}

#[test]
fn a_second_round_that_moves_nothing_keeps_the_first_draft() {
    let dir = tempfile::tempdir().expect("temp");
    let before = after_round_one(dir.path(), |_| {});

    let after = round_two(dir.path());

    assert_eq!(after.error, None);
    let draft = after.draft.as_deref().expect("the draft is still named");
    assert_eq!(draft, before.draft.as_deref().expect("draft"));
    assert_eq!(std::fs::read(draft).expect("the file"), b"round one");
    assert!(!Path::new(&after.dir).join("attempt.glb").exists());
    assert_eq!(after.state, RigState::Fixed, "it can still be added");
    assert_eq!(after.step, RigStep::Land);
    assert_eq!(after.round, 1, "a round that made nothing does not count");
    assert!(after.after.is_some(), "its sheet stays");
    let stop = after.stop.expect("it says so");
    assert_eq!(stop.kind, RigStopKind::NoChange);
    assert_eq!(stop.title, "Round 2 changed nothing");
    assert!(stop.detail.contains("draft from round 1 stays"), "{stop:?}");
}

#[test]
fn a_held_draft_stays_held_when_the_next_round_moves_nothing() {
    let dir = tempfile::tempdir().expect("temp");
    after_round_one(dir.path(), |c| {
        c.state = RigState::Stopped;
        c.stop = Some(RigStop {
            kind: RigStopKind::Worse,
            title: "A check got worse after the fix".to_string(),
            detail: "The mesh changed.".to_string(),
            row: None,
        });
    });

    let after = round_two(dir.path());

    assert_eq!(after.state, RigState::Stopped);
    let stop = after.stop.expect("still says why");
    assert_eq!(stop.kind, RigStopKind::Worse);
    assert!(stop.detail.starts_with("The mesh changed."), "{stop:?}");
    assert!(stop.detail.contains("draft from round 1 stays"), "{stop:?}");
}
