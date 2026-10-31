//! The rules that need no Blender: ids, order of runs, approval and defects.

use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::workspace::WorkspaceConfig;

struct NoBlender;

impl crate::base::ports::ScriptRunner for NoBlender {
    fn run_script(
        &self,
        _: &crate::pipeline::tools::Toolbox,
        _: &std::path::Path,
        _: (&str, &str),
        _: &[String],
    ) -> crate::error::CoreResult<()> {
        panic!("no script may run in these tests");
    }
}

fn doctors() -> RigDoctors {
    RigDoctors::new(Arc::new(NoBlender), |_| {})
}

/// A case on disk in the state `change` leaves it.
fn case(root: &std::path::Path, change: impl FnOnce(&mut RigRecord)) -> RigRecord {
    let config = WorkspaceConfig::default();
    let dir = work_dir(root, &config, "hero-v1");
    std::fs::create_dir_all(&dir).expect("dir");
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
    change(&mut c);
    save(&c).expect("save");
    c
}

fn offer() -> JointMove {
    JointMove {
        state: JointMoveState::Asked,
        limb: "left arm".to_string(),
        off_cm: 9.0,
        joints: 3,
        breaks_clips: vec!["walk".to_string()],
        draft: "moved.glb".to_string(),
        sheet: None,
    }
}

#[test]
fn the_fix_needs_its_pose_test_and_a_third_round_is_refused() {
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    let tools = crate::pipeline::tools::Toolbox::default();
    case(root, |_| {});
    let answer = doctors().run(root, &config, tools.clone(), "hero-v1", RigRun::Fix);
    assert!(answer.is_err());
    case(root, |c| {
        c.before = Some(RigSheet::default());
        c.draft = Some("draft.glb".to_string());
        c.round = MAX_ROUNDS;
        c.state = RigState::Stopped;
    });
    let answer = doctors().run(root, &config, tools, "hero-v1", RigRun::Round2);
    assert!(answer.is_err_and(|e| e.to_string().contains("two rounds")));
}

#[test]
fn a_model_still_in_the_inbox_is_not_run() {
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    case(root, |c| c.source.version = None);
    let tools = crate::pipeline::tools::Toolbox::default();
    let answer = doctors().run(root, &config, tools, "hero-v1", RigRun::PoseTest);
    assert!(answer.is_err_and(|e| e.to_string().contains("Accept the model")));
}

#[test]
fn a_declined_joint_move_keeps_the_weight_only_draft() {
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    case(root, |c| {
        c.state = RigState::Fixed;
        c.draft = Some("draft.glb".to_string());
        c.joint_move = Some(offer());
    });
    let no = doctors()
        .decide_joints(root, &config, "hero-v1", false)
        .expect("no");
    assert_eq!(no.draft.as_deref(), Some("draft.glb"));
    assert_eq!(no.joint_move.expect("move").state, JointMoveState::Declined);
    let again = doctors().decide_joints(root, &config, "hero-v1", true);
    assert!(again.is_err(), "an answered move is not asked again");
}

#[test]
fn an_approved_joint_move_uses_the_moved_draft_and_says_the_rest_pose_changed() {
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    case(root, |c| {
        c.state = RigState::Fixed;
        c.draft = Some("draft.glb".to_string());
        c.joint_move = Some(offer());
        c.checks = vec![RigCheck {
            ok: true,
            title: "Bones and rest pose unchanged".to_string(),
            detail: String::new(),
            value: String::new(),
        }];
    });
    let yes = doctors()
        .decide_joints(root, &config, "hero-v1", true)
        .expect("yes");
    assert_eq!(yes.draft.as_deref(), Some("moved.glb"));
    assert!(yes.checks[0].title.contains("changed by your yes"));
}

#[test]
fn a_run_the_app_closed_on_comes_back_as_not_running() {
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    case(root, |c| c.state = RigState::Testing);
    let found = doctors().get(root, &config, "hero-v1").expect("case");
    assert_eq!(found.state, RigState::Ready);
    assert!(found.error.is_some());
}

#[test]
fn one_island_names_the_narrow_reweight() {
    let probe = json!({"Left_arm_far": 10, "places": {"arm": {"hair": 0, "cloth": 10}}});
    let found = defects::humanoid(&probe, None, true);
    assert!(found.iter().any(|d| d.detail.contains("narrow reweight")));
}

#[test]
fn a_misplaced_limb_is_a_needs_yes_defect() {
    let probe = json!({"places": {}});
    let plan = json!({"limbs": {"LeftArm": {
        "misplaced": true,
        "icp_shift_cm": [8.0, 11.0],
        "registered_joints": {"mixamorig:LeftArm": [0, 0, 0], "mixamorig:LeftForeArm": [0, 0, 0]},
    }}});
    let found = defects::humanoid(&probe, Some(&plan), false);
    assert_eq!(found[0].kind, DefectKind::NeedsYes);
    assert_eq!(found[0].title, "Left arm chain off its mesh");
}

#[test]
fn the_verify_checks_fail_on_a_mesh_change() {
    let verify = json!({"changed": ["JOINTS_0", "POSITION"], "accessors": 14, "images_same": true,
        "same_names": true, "same_parents": true, "max_influences": 4, "zero_weight": 0,
        "sum_min": 1.0, "sum_max": 1.0, "bones": 20});
    let checks = steps::checks(&verify);
    assert!(!checks[0].ok);
    assert!(checks[1].ok);
    let stop = steps::stop_of("Hero", &checks, false, None).expect("stop");
    assert_eq!(stop.kind, RigStopKind::Worse);
}

#[test]
fn a_tear_left_stops_only_on_the_last_round() {
    let worst = ("raise".to_string(), 80.0);
    assert!(steps::stop_of("Hero", &[], false, Some(&worst)).is_none());
    let stop = steps::stop_of("Hero", &[], true, Some(&worst)).expect("stop");
    assert_eq!(stop.kind, RigStopKind::Rounds);
    assert_eq!(stop.row.as_deref(), Some("raise"));
}

fn hound(c: &mut RigRecord) {
    c.route = Some(RigRoute::Creature);
    c.state = RigState::Fixed;
    c.before = Some(RigSheet::default());
    c.after = Some(RigSheet::default());
    c.round = 1;
    let row = |chain: &str, key: &str, read_as: &str| RigChain {
        chain: chain.to_string(),
        key: key.to_string(),
        bones: 4,
        side: "left, by position".to_string(),
        read_as: read_as.to_string(),
        note: None,
    };
    c.creature = Some(RigCreature {
        front: "-X".to_string(),
        legs: 4,
        swapped: 0,
        junk: Vec::new(),
        rigid_pieces: 0,
        one_shell: false,
        chains: vec![
            row("body", "", "chain"),
            row("leg_FL", "bone_1", "chain"),
            row("tail", "bone_9", "chain"),
            row("bone_20", "bone_20", "junk"),
        ],
        weight_map: None,
    });
}

fn edit(key: &str, read_as: Option<&str>, side: Option<&str>) -> RigChainEdit {
    RigChainEdit {
        key: key.to_string(),
        read_as: read_as.map(str::to_string),
        side: side.map(str::to_string),
    }
}

#[test]
fn looking_at_the_chains_keeps_the_sheets_and_lets_the_draft_be_added() {
    let dir = tempfile::tempdir().expect("dir");
    let config = WorkspaceConfig::default();
    let draft = dir.path().join("draft.glb");
    std::fs::write(&draft, b"glb").expect("draft");
    let c = case(dir.path(), |c| {
        hound(c);
        c.draft = Some(draft.display().to_string());
    });
    assert!(waits_for_chains(&c).is_some(), "only read so far");
    let seen = doctors()
        .confirm_chains(dir.path(), &config, &c.id, Vec::new())
        .expect("confirm");
    assert!(waits_for_chains(&seen).is_none());
    assert_eq!(seen.state, RigState::Fixed);
    assert!(draft.is_file(), "nothing changed, so the draft stays");
    // Saying what the table says already changes nothing either.
    let same = doctors()
        .confirm_chains(
            dir.path(),
            &config,
            &c.id,
            vec![edit("bone_1", Some("chain"), Some("left"))],
        )
        .expect("confirm");
    assert_eq!(same.state, RigState::Fixed);
}

#[test]
fn a_corrected_chain_starts_the_case_over_and_keeps_the_edit() {
    let dir = tempfile::tempdir().expect("dir");
    let config = WorkspaceConfig::default();
    let draft = dir.path().join("draft.glb");
    std::fs::write(&draft, b"glb").expect("draft");
    let c = case(dir.path(), |c| {
        hound(c);
        c.draft = Some(draft.display().to_string());
    });
    let done = doctors()
        .confirm_chains(
            dir.path(),
            &config,
            &c.id,
            vec![edit("bone_9", Some("junk"), None)],
        )
        .expect("edit");
    assert_eq!(done.state, RigState::Ready);
    assert_eq!(done.step, RigStep::Before);
    assert_eq!(done.round, 0);
    assert!(done.before.is_none() && done.after.is_none() && done.draft.is_none());
    assert!(!draft.exists(), "the draft came from the old chains");
    assert!(done.chains_confirmed);
    assert_eq!(done.chain_edits.len(), 1);
    assert_eq!(done.creature.expect("table").chains[2].read_as, "junk");
}

#[test]
fn an_edit_must_fit_the_table() {
    let dir = tempfile::tempdir().expect("dir");
    let config = WorkspaceConfig::default();
    let c = case(dir.path(), hound);
    for bad in [
        edit("", Some("junk"), None),
        edit("nobody", Some("junk"), None),
        edit("bone_1", Some("wing"), None),
        edit("bone_1", None, Some("up")),
        edit("bone_9", None, Some("left")),
    ] {
        assert!(
            doctors()
                .confirm_chains(dir.path(), &config, &c.id, vec![bad.clone()])
                .is_err(),
            "{bad:?}"
        );
    }
}

#[test]
fn a_draft_holds_the_route_and_a_route_needs_its_bone_names() {
    let dir = tempfile::tempdir().expect("dir");
    let config = WorkspaceConfig::default();
    let c = case(dir.path(), |c| {
        hound(c);
        c.draft = Some("draft.glb".to_string());
    });
    let held = doctors().set_route(dir.path(), &config, &c.id, RigRoute::Humanoid);
    assert!(
        held.expect_err("draft")
            .to_string()
            .contains("Discard the draft")
    );
    let same = doctors()
        .set_route(dir.path(), &config, &c.id, RigRoute::Creature)
        .expect("same");
    assert_eq!(
        same.state,
        RigState::Fixed,
        "the route it has changes nothing"
    );
    let refused = case(dir.path(), |c| {
        hound(c);
        c.state = RigState::Refused;
    });
    assert!(
        doctors()
            .set_route(dir.path(), &config, &refused.id, RigRoute::Humanoid)
            .is_err()
    );
}

#[test]
fn only_a_creature_with_a_draft_walks() {
    let dir = tempfile::tempdir().expect("dir");
    let config = WorkspaceConfig::default();
    let tools = || crate::pipeline::tools::Toolbox::default();
    let human = case(dir.path(), |c| {
        c.route = Some(RigRoute::Humanoid);
        c.state = RigState::Fixed;
        c.draft = Some("d.glb".to_string());
    });
    assert!(
        doctors()
            .run(dir.path(), &config, tools(), &human.id, RigRun::WalkProof)
            .is_err()
    );
    let no_draft = case(dir.path(), hound);
    assert!(
        doctors()
            .run(
                dir.path(),
                &config,
                tools(),
                &no_draft.id,
                RigRun::WalkProof
            )
            .is_err()
    );
}
