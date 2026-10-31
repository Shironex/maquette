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
