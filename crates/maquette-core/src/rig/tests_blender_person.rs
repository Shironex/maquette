//! A person's part on a real creature and a real route change, against real
//! headless Blender.

use std::fs;
use std::path::{Path, PathBuf};

use super::tests_blender::{at_v1, build, doctors, go};
use super::*;
use crate::workspace::WorkspaceConfig;

#[test]
fn a_person_corrects_a_real_creature_and_walks_the_draft() {
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
            RigStarter::Person,
        )
        .expect("open");
    let at = (root, &config, blender.as_path());
    let tested = go(&both, at, &open.id, RigRun::PoseTest);
    let table = tested.creature.clone().expect("chains");
    let junk_before = table.junk.len();
    let tail = table
        .chains
        .iter()
        .find(|c| c.chain == "tail")
        .expect("tail")
        .key
        .clone();
    let leg = table
        .chains
        .iter()
        .find(|c| c.chain.starts_with("leg_") && c.side.starts_with("left"))
        .expect("a left leg")
        .key
        .clone();
    let edits = vec![
        RigChainEdit {
            key: tail.clone(),
            read_as: Some("junk".to_string()),
            side: None,
        },
        RigChainEdit {
            key: leg.clone(),
            read_as: None,
            side: Some("right".to_string()),
        },
    ];
    let edited = both
        .0
        .confirm_chains(root, &config, &open.id, edits)
        .expect("edit");
    assert_eq!(edited.state, RigState::Ready);

    let again = go(&both, at, &open.id, RigRun::PoseTest);
    assert_eq!(again.state, RigState::Tested, "{:?}", again.error);
    let table = again.creature.as_ref().expect("chains");
    assert_eq!(table.junk.len(), junk_before + 1, "{:?}", table.junk);
    assert!(
        table
            .chains
            .iter()
            .any(|c| c.key == tail && c.read_as == "junk")
    );
    let row = table.chains.iter().find(|c| c.key == leg).expect("the leg");
    assert_eq!(row.side, "right, by hand");
    assert_eq!(
        table
            .chains
            .iter()
            .filter(|c| c.chain.starts_with("leg_"))
            .count(),
        4
    );

    let fixed = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(
        fixed.state,
        RigState::Fixed,
        "{:?} {:?}",
        fixed.error,
        fixed.stop
    );
    assert!(fixed.checks.iter().all(|c| c.ok), "{:?}", fixed.checks);
    assert!(waits_for_chains(&fixed).is_none());
    let draft = PathBuf::from(fixed.draft.as_ref().expect("draft"));
    let before_proof = fs::read(&draft).expect("draft");

    let walked = go(&both, at, &open.id, RigRun::WalkProof);
    assert_eq!(walked.state, RigState::Fixed, "{:?}", walked.error);
    assert!(walked.error.is_none(), "{:?}", walked.error);
    let strip = walked.walk_proof.as_ref().expect("strip");
    assert!(Path::new(strip).is_file());
    assert_eq!(
        fs::read(&draft).expect("draft"),
        before_proof,
        "the clip stays out of the draft"
    );
    assert_eq!(walked.round, 1, "a proof is no round");
}

#[test]
fn a_real_humanoid_can_take_the_creature_route_by_hand() {
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
    assert!(open.model.humanoid_ok);
    let at = (root, &config, blender.as_path());
    go(&both, at, &open.id, RigRun::PoseTest);
    let turned = both
        .0
        .set_route(root, &config, &open.id, RigRoute::Creature)
        .expect("route");
    assert_eq!(turned.route, Some(RigRoute::Creature));
    assert!(
        turned.before.is_none(),
        "the old sheet was for the other route"
    );
    let tested = go(&both, at, &open.id, RigRun::PoseTest);
    assert_eq!(tested.state, RigState::Tested, "{:?}", tested.error);
    assert!(tested.creature.is_some());
    let back = both
        .0
        .set_route(root, &config, &open.id, RigRoute::Humanoid)
        .expect("back");
    assert!(back.creature.is_none());
}

#[test]
fn a_model_with_its_own_names_cannot_take_the_humanoid_route() {
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
            RigStarter::Person,
        )
        .expect("open");
    assert!(!open.model.humanoid_ok);
    assert!(
        both.0
            .set_route(root, &config, &open.id, RigRoute::Humanoid)
            .is_err()
    );
}

/// Fails unless every picture's border is empty: the model fits its frame.
const IN_FRAME: &str = r#"
import bpy, sys
import numpy as np
for path in sys.argv[sys.argv.index("--") + 1:]:
    im = bpy.data.images.load(path)
    w, h = im.size
    a = np.array(im.pixels[:], np.float32).reshape(h, w, 4)[..., :3]
    edge = np.concatenate([a[0], a[-1], a[:, 0], a[:, -1]])
    ground = a[0, 0]
    assert abs(ground - np.array([49, 47, 43]) / 255).max() < 0.02, (path, ground)
    assert abs(edge - ground).max() < 0.02, path + " runs off its frame"
print("FRAMED")
"#;

fn assert_in_frame(blender: &Path, files: &[String]) {
    let mut args = vec!["--python-expr", IN_FRAME, "--"];
    args.extend(files.iter().map(String::as_str));
    let said = crate::test_blender::run(blender, &args);
    assert!(said.contains("FRAMED"), "{said}");
}

#[test]
fn a_clean_rigs_cloth_stretches_with_the_pose_and_is_not_flagged() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "webbed");
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            "Web",
            &file,
            at_v1("web"),
            RigStarter::Person,
        )
        .expect("open");
    let at = (root, &config, blender.as_path());
    let tested = go(&both, at, &open.id, RigRun::PoseTest);
    assert_eq!(tested.state, RigState::Tested, "{:?}", tested.error);
    let rows = &tested.before.as_ref().expect("before").rows;
    let raise = rows.iter().find(|r| r.id == "raise").expect("raise");
    assert!(raise.stretch_mm > 150.0, "{raise:?}");
    assert!(raise.beyond_body_mm.expect("number") < TEAR_MM, "{raise:?}");
    assert_eq!(verdict::problem(raise), None, "{raise:?}");
}

#[test]
fn a_robe_that_tears_closes_its_seams_and_every_pose_fits_its_frame() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "robed");
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            "Robe",
            &file,
            at_v1("robe"),
            RigStarter::Person,
        )
        .expect("open");
    let at = (root, &config, blender.as_path());
    let tested = go(&both, at, &open.id, RigRun::PoseTest);
    assert_eq!(tested.state, RigState::Tested, "{:?}", tested.error);
    let before = tested.before.as_ref().expect("before");
    let raise = before.rows.iter().find(|r| r.id == "raise").expect("raise");
    assert!(raise.seam_gap_mm > 100.0, "{raise:?}");
    assert!(raise.note.is_some());
    assert!(before.rows.iter().any(|r| r.id == "clip:fall:0.5"));
    // The stats say which pose their worst number is at.
    assert!(
        tested
            .stats
            .iter()
            .any(|s| s.label.starts_with("Seam gaps, "))
    );

    let fixed = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(
        fixed.state,
        RigState::Fixed,
        "{:?} {:?}",
        fixed.error,
        fixed.stop
    );
    let after = fixed.after.as_ref().expect("after");
    let raised = after.rows.iter().find(|r| r.id == "raise").expect("raise");
    assert!(raised.seam_gap_mm < 1.0, "{raised:?}");
    let change = raised.change.as_ref().expect("change");
    assert_eq!(change.seam_gap, RigTrend::Better);
    assert!(
        raised
            .note
            .as_deref()
            .is_some_and(|n| n.contains(" to 0 mm, better"))
    );

    let mut args: Vec<String> = Vec::new();
    for row in &after.rows {
        args.extend(row.front.clone());
        args.extend(row.side.clone());
        args.extend(row.before_front.clone());
    }
    assert_in_frame(&blender, &args);
    assert!(Path::new(&after.rows[0].before_front.clone().expect("before")).is_file());
}

#[test]
fn a_creature_is_judged_and_framed_the_same_way() {
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
            RigStarter::Person,
        )
        .expect("open");
    let at = (root, &config, blender.as_path());
    let tested = go(&both, at, &open.id, RigRun::PoseTest);
    assert_eq!(tested.state, RigState::Tested, "{:?}", tested.error);
    let rows = &tested.before.as_ref().expect("before").rows;
    assert!(rows.iter().all(|r| r.beyond_body_mm.is_some()), "{rows:?}");
    assert!(
        tested
            .stats
            .iter()
            .any(|s| s.label.starts_with("Past the body"))
    );
    let fixed = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(
        fixed.state,
        RigState::Fixed,
        "{:?} {:?}",
        fixed.error,
        fixed.stop
    );
    let after = fixed.after.as_ref().expect("after");
    assert!(after.rows.iter().all(|r| r.change.is_some()), "{after:?}");
    let mut args: Vec<String> = Vec::new();
    for row in &after.rows {
        args.extend(row.front.clone());
    }
    assert_in_frame(&blender, &args);
}
