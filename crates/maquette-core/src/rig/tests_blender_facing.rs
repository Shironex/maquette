//! Which way a creature faces, read in real headless Blender. An upright
//! creature's head points up, so its facing comes from its feet.

use super::tests_blender::{at_v1, build, doctors, go};
use super::*;
use crate::workspace::WorkspaceConfig;

#[test]
fn an_upright_creature_facing_plus_x_reads_plus_x_with_its_head_on_top() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "biped-x");
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            "Strider",
            &file,
            at_v1("strider"),
            RigStarter::Person,
        )
        .expect("open");
    assert_eq!(open.route, Some(RigRoute::Creature));
    let tested = go(
        &both,
        (root, &config, blender.as_path()),
        &open.id,
        RigRun::PoseTest,
    );
    assert_eq!(tested.state, RigState::Tested, "{:?}", tested.error);
    let table = tested.creature.as_ref().expect("chains");
    // In glTF and three.js axes, as the model stands in the file.
    assert_eq!(table.front, "+X", "{:?}", table.chains);
    assert_eq!(table.legs, 2);
    let head = table
        .chains
        .iter()
        .find(|c| c.chain == "neck, head")
        .expect("head row");
    // The neck and head, not an arm with its hand and fingers.
    assert_eq!(head.bones, 2, "{:?}", table.chains);
    let side = |key: &str| {
        table
            .chains
            .iter()
            .find(|c| c.key == key)
            .map(|c| c.side.clone())
            .expect(key)
    };
    assert!(side("J_LeftUpLeg").starts_with("left"));
    assert!(side("J_RightUpLeg").starts_with("right"));
}
