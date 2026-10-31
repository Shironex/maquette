//! The whole doctor against real headless Blender, on small rigs built in
//! the test. No outside models are used.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use super::*;
use crate::inbox::analyse;
use crate::pipeline::runner::{NullSink, PipelineRunner};
use crate::workspace::WorkspaceConfig;

const BUILD: &str = include_str!("fixtures/rig_models.py");

pub(super) fn build(blender: &Path, dir: &Path, which: &str) -> PathBuf {
    let script = dir.join("rig_models.py");
    fs::write(&script, BUILD).expect("script");
    let out = dir.join(format!("{which}.glb"));
    crate::test_blender::run(
        blender,
        &[
            "-P",
            script.to_str().expect("path"),
            "--",
            which,
            out.to_str().expect("path"),
        ],
    );
    out
}

/// A doctor and a way to wait until a run is over.
pub(super) fn doctors() -> (RigDoctors, Arc<(Mutex<bool>, Condvar)>) {
    let signal = Arc::new((Mutex::new(false), Condvar::new()));
    let seen = Arc::clone(&signal);
    let runner = Arc::new(PipelineRunner::new(Arc::new(NullSink)));
    let doctors = RigDoctors::new(runner, move |case| {
        if !case.state.running() {
            *seen.0.lock().expect("lock") = true;
            seen.1.notify_all();
        }
    });
    (doctors, signal)
}

pub(super) fn wait(signal: &Arc<(Mutex<bool>, Condvar)>) {
    let mut done = signal.0.lock().expect("lock");
    while !*done {
        let (next, timeout) = signal
            .1
            .wait_timeout(done, Duration::from_secs(600))
            .expect("wait");
        assert!(!timeout.timed_out(), "the run did not finish");
        done = next;
    }
    *done = false;
}

pub(super) fn at_v1(name: &str) -> RigSource {
    RigSource {
        inbox: None,
        project: Some("p".to_string()),
        asset: Some(name.to_string()),
        version: Some(1),
    }
}

/// Runs `what` and waits for it.
pub(super) fn go(
    (doctors, signal): &(RigDoctors, Arc<(Mutex<bool>, Condvar)>),
    (root, config, blender): (&Path, &WorkspaceConfig, &Path),
    id: &str,
    what: RigRun,
) -> RigRecord {
    *signal.0.lock().expect("lock") = false;
    let tools = crate::test_blender::toolbox(blender);
    doctors.run(root, config, tools, id, what).expect("run");
    wait(signal);
    // The thread clears `active` just after the last change.
    std::thread::sleep(Duration::from_millis(200));
    doctors.get(root, config, id).expect("case")
}

#[test]
fn a_real_humanoid_is_tested_fixed_and_only_its_weights_change() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "humanoid");
    let original = fs::read(&file).expect("read");
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            "Hero",
            &file,
            at_v1("hero"),
            RigStarter::Person,
        )
        .expect("open");
    assert_eq!(open.route, Some(RigRoute::Humanoid));
    assert_eq!(open.state, RigState::Ready);
    assert!(open.model.arm_far > 0 || open.model.hand_far > 0);

    let at = (root, &config, blender.as_path());
    let tested = go(&both, at, &open.id, RigRun::PoseTest);
    assert_eq!(tested.state, RigState::Tested, "{:?}", tested.error);
    let before = tested.before.as_ref().expect("before");
    assert!(before.rows.len() >= 7, "{}", before.rows.len());
    assert!(Path::new(before.rows[0].front.as_ref().expect("cell")).is_file());
    assert!(before.rows[0].xray.is_some());
    assert!(!tested.defects.is_empty());

    let fixed = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(
        fixed.state,
        RigState::Fixed,
        "{:?} {:?}",
        fixed.error,
        fixed.stop
    );
    assert!(fixed.checks.iter().all(|c| c.ok), "{:?}", fixed.checks);
    assert!(fixed.joint_move.is_none());
    assert!(Path::new(&fixed.after.as_ref().expect("after").file).is_file());

    let draft = PathBuf::from(fixed.draft.as_ref().expect("draft"));
    let probe = analyse::analyse(&draft)
        .expect("draft")
        .rig_probe
        .expect("probe");
    assert_eq!(probe.hand_far + probe.arm_far, 0, "{probe:?}");
    assert!(got_worse(&file, &draft).expect("compare").is_none());
    assert_eq!(fs::read(&file).expect("read"), original, "the source stays");
}

#[test]
fn the_read_counts_what_the_probe_script_counts() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    let file = build(&blender, root, "humanoid");
    let installed = scripts::install(root).expect("install");
    let out = root.join("probe.json");
    let python = format!(
        "import runpy,sys; sys.argv=['probe.py',{:?},'--json',{:?}]; sys.path.insert(0,{:?}); runpy.run_path({:?}, run_name='__main__')",
        file.display().to_string(),
        out.display().to_string(),
        installed.join("humanoid").display().to_string(),
        installed
            .join("humanoid")
            .join("probe.py")
            .display()
            .to_string(),
    );
    crate::test_blender::run(&blender, &["--python-expr", &python]);
    let script: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out).expect("json")).expect("json");
    let probe = analyse::analyse(&file)
        .expect("read")
        .rig_probe
        .expect("probe");
    let sum = |k: &str| {
        script[format!("Left_{k}")].as_u64().unwrap_or(0)
            + script[format!("Right_{k}")].as_u64().unwrap_or(0)
    };
    assert_eq!(u64::from(probe.hand_far), sum("hand_joint"));
    assert_eq!(u64::from(probe.arm_far), sum("arm_far"));
    assert_eq!(
        u64::from(probe.islands),
        script["islands"].as_u64().unwrap_or(0)
    );
}

#[test]
fn a_misplaced_arm_only_offers_its_joint_move() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "misplaced");
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            "Hero",
            &file,
            at_v1("hero"),
            RigStarter::Person,
        )
        .expect("open");
    let at = (root, &config, blender.as_path());
    let tested = go(&both, at, &open.id, RigRun::PoseTest);
    assert!(
        tested
            .defects
            .iter()
            .any(|d| d.kind == DefectKind::NeedsYes),
        "{:?}",
        tested.defects
    );
    let fixed = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(
        fixed.state,
        RigState::Fixed,
        "{:?} {:?}",
        fixed.error,
        fixed.stop
    );
    let offer = fixed.joint_move.as_ref().expect("offer");
    assert_eq!(offer.state, JointMoveState::Asked);
    assert!(!offer.breaks_clips.is_empty(), "names the clips: {offer:?}");
    let draft = fixed.draft.clone().expect("draft");
    assert_ne!(draft, offer.draft, "the move is not the default");
    let approved = both
        .0
        .decide_joints(root, &config, &open.id, true)
        .expect("yes");
    assert_eq!(approved.draft.as_deref(), Some(offer.draft.as_str()));
}

#[test]
fn one_continuous_mesh_gets_the_narrow_reweight_and_says_so() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "one-island");
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            "Shell",
            &file,
            at_v1("shell"),
            RigStarter::Person,
        )
        .expect("open");
    assert!(open.model.one_island, "{:?}", open.model);
    let at = (root, &config, blender.as_path());
    go(&both, at, &open.id, RigRun::PoseTest);
    let fixed = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(
        fixed.state,
        RigState::Fixed,
        "{:?} {:?}",
        fixed.error,
        fixed.stop
    );
    assert_eq!(fixed.fix_steps[0].title, "Narrow reweight");
    assert!(fixed.fix_steps[0].detail.contains("would tear"));
    assert!(fixed.checks.iter().all(|c| c.ok), "{:?}", fixed.checks);
}

#[test]
fn a_real_creature_reads_its_chains_and_loses_the_junk_weight() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "creature");
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            "Hound",
            &file,
            at_v1("hound"),
            RigStarter::Agent,
        )
        .expect("open");
    assert_eq!(open.route, Some(RigRoute::Creature));
    let at = (root, &config, blender.as_path());
    let tested = go(&both, at, &open.id, RigRun::PoseTest);
    assert_eq!(tested.state, RigState::Tested, "{:?}", tested.error);
    let table = tested.creature.as_ref().expect("chains");
    assert_eq!(table.legs, 4);
    assert_eq!(table.front, "-X");
    assert!(!table.junk.is_empty());
    assert!(table.weight_map.is_some());
    let fixed = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(
        fixed.state,
        RigState::Fixed,
        "{:?} {:?}",
        fixed.error,
        fixed.stop
    );
    assert!(fixed.checks.iter().all(|c| c.ok), "{:?}", fixed.checks);
    assert!(Path::new(&fixed.after.as_ref().expect("after").file).is_file());
}

#[test]
fn a_real_second_round_widens_the_seam_and_holds_the_checks() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "humanoid");
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            "Hero",
            &file,
            at_v1("hero"),
            RigStarter::Person,
        )
        .expect("open");
    let at = (root, &config, blender.as_path());
    go(&both, at, &open.id, RigRun::PoseTest);
    let first = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(first.round, 1);
    assert_eq!(
        first.state,
        RigState::Fixed,
        "{:?} {:?}",
        first.error,
        first.stop
    );
    let second = go(&both, at, &open.id, RigRun::Round2);
    assert_eq!(second.round, 2);
    assert_eq!(
        second.state,
        RigState::Fixed,
        "{:?} {:?}",
        second.error,
        second.stop
    );
    assert!(second.checks.iter().all(|c| c.ok), "{:?}", second.checks);
    let band = second
        .fix_steps
        .iter()
        .find(|s| s.title == "Smooth the seams")
        .expect("band");
    assert_eq!(band.value, "8 cm");
    let draft = PathBuf::from(second.draft.as_ref().expect("draft"));
    assert!(got_worse(&file, &draft).expect("compare").is_none());
    // A third round is refused.
    let tools = crate::test_blender::toolbox(&blender);
    assert!(
        both.0
            .run(root, &config, tools, &open.id, RigRun::Round2)
            .is_err()
    );
}

#[test]
fn a_model_without_a_skin_is_refused_and_cannot_run() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "no-skin");
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            "Bare",
            &file,
            at_v1("bare"),
            RigStarter::Person,
        )
        .expect("open");
    assert_eq!(open.state, RigState::Refused);
    let tools = crate::test_blender::toolbox(&blender);
    assert!(
        both.0
            .run(root, &config, tools, &open.id, RigRun::PoseTest)
            .is_err()
    );
}
