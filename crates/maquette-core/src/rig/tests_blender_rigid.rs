//! Wings weighted to the arms, bound rigidly to the chest in real headless
//! Blender: the parts are read on the pose test, the bind is measured on a
//! copy of the draft, and only a person's yes puts it in the draft.

use std::path::PathBuf;
use std::time::Duration;

use super::tests_blender::{at_v1, build, doctors, go, wait};
use super::*;
use crate::inbox::analyse;
use crate::workspace::WorkspaceConfig;

#[test]
fn wings_on_the_arm_bones_bind_to_the_chest_and_stretch_less() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "winged");
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            "Winged imp",
            &file,
            at_v1("winged-imp"),
            RigStarter::Agent,
        )
        .expect("open");
    assert_eq!(open.route, Some(RigRoute::Humanoid));
    assert!(open.model.one_island);
    let at = (root, &config, blender.as_path());
    let tested = go(&both, at, &open.id, RigRun::PoseTest);
    assert_eq!(tested.state, RigState::Tested, "{:?}", tested.error);
    // One part per wing, each held by its own arm.
    assert_eq!(tested.parts.len(), 2, "{:?}", tested.parts);
    let sides: Vec<&str> = tested.parts.iter().map(|p| p.side.as_str()).collect();
    assert!(
        sides.contains(&"left") && sides.contains(&"right"),
        "{sides:?}"
    );
    assert!(
        tested.parts.iter().all(|p| p.across_cm > 25.0),
        "{:?}",
        tested.parts
    );
    assert_eq!(default_bone(&tested).as_deref(), Some("mixamorig:Spine2"));

    let fixed = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(
        fixed.state,
        RigState::Fixed,
        "{:?} {:?}",
        fixed.error,
        fixed.stop
    );
    let draft = fixed.draft.clone().expect("draft");

    let tools = crate::test_blender::toolbox(&blender);
    both.0
        .bind_rigid(
            (root, &config, tools),
            &open.id,
            (&[1, 2], None),
            RigStarter::Agent,
        )
        .expect("bind");
    // The ask itself was published before the run: wait for the run's own end.
    *both.1.0.lock().expect("lock") = false;
    wait(&both.1);
    std::thread::sleep(Duration::from_millis(200));
    let bound = both.0.get(root, &config, &open.id).expect("case");
    assert_eq!(bound.state, RigState::Fixed, "{:?}", bound.error);
    assert_eq!(
        bound.draft.as_deref(),
        Some(draft.as_str()),
        "asked, not used"
    );
    let rigid = bound.rigid.clone().expect("bind");
    assert_eq!(rigid.state, RigidState::Asked);
    assert!(rigid.checks.iter().all(|c| c.ok), "{:?}", rigid.checks);
    let (before, after) = (
        rigid.stretch_before.expect("before"),
        rigid.stretch_after.expect("after"),
    );
    assert!(after < before * 0.5, "{:?}: {before} to {after}", rigid.row);
    assert!(!rigid.sheet.as_ref().expect("sheet").rows.is_empty());

    let yes = both
        .0
        .decide_rigid(root, &config, &open.id, true)
        .expect("yes");
    let used = PathBuf::from(yes.draft.clone().expect("draft"));
    assert_eq!(Some(used.display().to_string()), rigid.file);
    let far = |p: &PathBuf| {
        analyse::analyse(p)
            .expect("read")
            .rig_probe
            .expect("probe")
            .arm_far
    };
    assert!(
        far(&used) < far(&PathBuf::from(&draft)),
        "the wings left the arms"
    );
    assert!(got_worse(&file, &used).expect("compare").is_none());
    let undo = both
        .0
        .decide_rigid(root, &config, &open.id, false)
        .expect("undo");
    assert_eq!(undo.draft.as_deref(), Some(draft.as_str()));
}
