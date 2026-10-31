//! What the doctor reads off a model and says about it, against real headless
//! Blender: cloth that is fine and cloth that is not, a quadruped's chains,
//! and a fix with nothing to do.

use std::path::{Path, PathBuf};

use super::tests_blender::{at_v1, build, doctors, go};
use super::*;
use crate::workspace::WorkspaceConfig;

/// The pose test of the fixture `which`, with the record and what the run needs.
fn tested(which: &str, blender: &Path, root: &Path) -> RigRecord {
    let config = WorkspaceConfig::default();
    let file = build(blender, root, which);
    let both = doctors();
    let open = both
        .0
        .open(
            root,
            &config,
            which,
            &file,
            at_v1(which),
            RigStarter::Person,
        )
        .expect("open");
    let case = go(&both, (root, &config, blender), &open.id, RigRun::PoseTest);
    assert_eq!(case.state, RigState::Tested, "{:?}", case.error);
    case
}

fn row<'a>(case: &'a RigRecord, id: &str) -> &'a RigRow {
    let rows = &case.before.as_ref().expect("before").rows;
    rows.iter().find(|r| r.id == id).expect(id)
}

#[test]
fn a_blended_robe_is_not_flagged_though_it_stretches_far() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let case = tested("long-robe", &blender, dir.path());
    let walk = row(&case, "clip:walk:0.5");
    // Its worst edge has one nearest bone at both ends, so the rigid body alone left this flagged.
    assert!(walk.stretch_mm >= TEAR_MM, "{walk:?}");
    assert!(walk.beyond_body_mm.expect("number") < TEAR_MM, "{walk:?}");
    assert_eq!(verdict::problem(walk), None, "{walk:?}");
}

#[test]
fn a_torn_robe_is_flagged() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let case = tested("robe-torn", &blender, dir.path());
    let walk = row(&case, "clip:walk:0.5");
    assert!(walk.beyond_body_mm.expect("number") >= TEAR_MM, "{walk:?}");
    assert!(verdict::problem(walk).is_some(), "{walk:?}");
}

#[test]
fn a_hem_blended_onto_a_far_arm_is_flagged() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let case = tested("robe-arm", &blender, dir.path());
    let raise = row(&case, "raise");
    assert!(
        raise.beyond_body_mm.expect("number") >= TEAR_MM,
        "{raise:?}"
    );
}

#[test]
fn a_few_edges_stepped_onto_the_leg_say_look_here_and_are_no_tear() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let case = tested("robe-patch", &blender, dir.path());
    let walk = row(&case, "clip:walk:0.5");
    // Real, long edges at a hard step, on too little of the surface to call a tear.
    assert!(walk.worst_edge_mm.expect("number") >= TEAR_MM, "{walk:?}");
    assert!(walk.beyond_body_mm.expect("number") < TEAR_MM, "{walk:?}");
    assert_eq!(walk.note, None, "{walk:?}");
    let look = walk.look.as_deref().expect("look");
    assert!(look.ends_with("look here"), "{look}");
    assert!(look.contains("near "), "{look}");
}

const TOE_WEIGHTS: &str = r#"
import bpy, sys
bpy.ops.import_scene.gltf(filepath=sys.argv[sys.argv.index("--") + 1])
me = next(o for o in bpy.data.objects if o.type == "MESH" and o.vertex_groups)
names = {g.index: g.name for g in me.vertex_groups}
used = {names[g.group] for v in me.data.vertices for g in v.groups if g.weight > 0.01}
print("TOES", sum(1 for n in used if n in {f"bone_{a}{b}" for a in range(4) for b in (1, 2, 3)}))
"#;

#[test]
fn a_tripo_quadruped_reads_four_legs_and_keeps_its_toes() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "hound");
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
    let case = go(&both, at, &open.id, RigRun::PoseTest);
    assert_eq!(case.state, RigState::Tested, "{:?}", case.error);
    let table = case.creature.as_ref().expect("chains");
    assert_eq!(table.legs, 4, "{:?}", table.chains);
    assert!(table.junk.is_empty(), "{:?}", table.junk);
    let legs: Vec<_> = table
        .chains
        .iter()
        .filter(|c| c.chain.starts_with("leg_"))
        .collect();
    assert_eq!(legs.len(), 4, "{legs:?}");
    // Three leg bones and three toes each; the front one has its nub too.
    assert!(legs.iter().all(|l| l.bones >= 6), "{legs:?}");
    let tails: Vec<_> = table
        .chains
        .iter()
        .filter(|c| c.chain.starts_with("tail"))
        .collect();
    assert_eq!(tails.len(), 1, "{tails:?}");
    assert_eq!(tails[0].bones, 6, "{tails:?}");
    assert_eq!(
        table
            .chains
            .iter()
            .find(|c| c.chain == "spikes")
            .map(|c| c.bones),
        Some(3),
        "{:?}",
        table.chains
    );

    let fixed = go(&both, at, &open.id, RigRun::Fix);
    let draft = fixed.draft.as_ref().map_or(file.clone(), PathBuf::from);
    let said = crate::test_blender::run(
        &blender,
        &[
            "--python-expr",
            TOE_WEIGHTS,
            "--",
            draft.to_str().expect("path"),
        ],
    );
    assert!(said.contains("TOES 12"), "{said}");
}

#[test]
fn a_fix_that_changes_nothing_says_so_and_leaves_no_draft() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "hound-shell");
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
    let at = (root, &config, blender.as_path());
    let case = go(&both, at, &open.id, RigRun::PoseTest);
    assert_eq!(case.state, RigState::Tested, "{:?}", case.error);
    let table = case.creature.as_ref().expect("chains");
    assert!(table.one_shell, "{table:?}");
    let before = case.before.as_ref().expect("before");
    assert!(before.seconds > 0.0, "a real time: {}", before.seconds);
    // Islands of one shell are not rigid pieces, whatever a bend does to them.
    let says_bend = |r: &RigRow| r.note.as_deref().is_some_and(|n| n.contains("bend"));
    assert!(!before.rows.iter().any(says_bend), "{:?}", before.rows);
    assert!(
        !case.defects.iter().any(|d| d.title.starts_with("Rigid")),
        "{:?}",
        case.defects
    );
    assert_eq!(table.rigid_pieces, 0);

    let round = case.round;
    let fixed = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(fixed.error, None);
    assert_eq!(fixed.state, RigState::Tested, "{:?}", fixed.stop);
    assert_eq!(
        fixed.stop.as_ref().map(|s| s.kind),
        Some(RigStopKind::NoChange)
    );
    assert_eq!(fixed.round, round, "it does not use up a round");
    assert_eq!(fixed.draft, None);
    assert!(fixed.after.is_none());
    assert!(fixed.checks.is_empty());
    assert!(
        fixed.defects.iter().all(|d| !d.fixed),
        "{:?}",
        fixed.defects
    );
    assert!(!Path::new(&fixed.dir).join("draft.glb").exists());
}

#[test]
fn a_creature_sheet_after_a_fix_reports_real_time() {
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
            "Beast",
            &file,
            at_v1("beast"),
            RigStarter::Person,
        )
        .expect("open");
    let at = (root, &config, blender.as_path());
    let case = go(&both, at, &open.id, RigRun::PoseTest);
    assert!(case.before.as_ref().expect("before").seconds > 0.0);
    // The doctor bends the split spikes, and the tag counts as the rows count.
    let bent = case
        .before
        .as_ref()
        .expect("before")
        .rows
        .iter()
        .filter(|r| r.note.is_some())
        .count();
    assert!(bent > 0);
    let rigid = case
        .defects
        .iter()
        .find(|d| d.title.starts_with("Rigid"))
        .expect("rigid");
    assert!(!rigid.tags[0].starts_with("0 "), "{rigid:?}");

    let fixed = go(&both, at, &open.id, RigRun::Fix);
    assert_eq!(
        fixed.state,
        RigState::Fixed,
        "{:?} {:?}",
        fixed.error,
        fixed.stop
    );
    assert!(fixed.after.as_ref().expect("after").seconds > 0.0);
    let flagged = fixed
        .stats
        .iter()
        .find(|s| s.label == "Chains flagged")
        .expect("stat");
    assert_eq!(flagged.before.as_deref(), Some(bent.to_string().as_str()));
}

#[test]
fn an_agent_reopening_a_persons_case_does_not_become_its_starter() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let file = build(&blender, root, "humanoid");
    let both = doctors();
    let mine = both
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
    assert_eq!(mine.started_by, RigStarter::Person);
    let again = both
        .0
        .open(
            root,
            &config,
            "Hero",
            &file,
            at_v1("hero"),
            RigStarter::Agent,
        )
        .expect("again");
    assert_eq!(again.id, mine.id);
    assert_eq!(again.started_by, RigStarter::Person);
    assert!(again.agent_opened);
    let read = both.0.get(root, &config, &mine.id).expect("read");
    assert_eq!(read.started_by, RigStarter::Person);
}
