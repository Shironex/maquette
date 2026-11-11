//! The rigid bind's rules that need no Blender: when it may start, and that
//! only a person's answer uses it or puts the fix's draft back.

use std::sync::Arc;

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

fn part(n: u32) -> RigPart {
    RigPart {
        part: n,
        vertices: 900,
        side: "left".to_string(),
        bones: vec!["LeftArm".to_string()],
        across_cm: 38.0,
    }
}

/// A fixed humanoid case with two parts, changed by `change`, on disk.
fn fixed(root: &std::path::Path, change: impl FnOnce(&mut RigRecord)) {
    let config = WorkspaceConfig::default();
    let dir = work_dir(root, &config, "moth-v1");
    std::fs::create_dir_all(&dir).expect("dir");
    let mut c = record::blank(
        "moth-v1".to_string(),
        "Moth".to_string(),
        &dir,
        &dir.join("in.glb"),
        RigSource {
            inbox: None,
            project: Some("p".to_string()),
            asset: Some("moth".to_string()),
            version: Some(1),
        },
    );
    c.route = Some(RigRoute::Humanoid);
    c.state = RigState::Fixed;
    c.before = Some(RigSheet::default());
    c.draft = Some("draft.glb".to_string());
    c.parts = vec![part(1), part(2)];
    c.body_bones = vec![
        "mixamorig:Spine2".to_string(),
        "mixamorig:Spine1".to_string(),
        "mixamorig:Hips".to_string(),
    ];
    change(&mut c);
    save(&c).expect("save");
}

fn asked(checks_ok: bool) -> RigRigid {
    RigRigid {
        parts: vec![1, 2],
        bone: "mixamorig:Spine2".to_string(),
        state: RigidState::Asked,
        asked_by: RigStarter::Agent,
        vertices: 1800,
        file: Some("rigid.glb".to_string()),
        earlier: Some("draft.glb".to_string()),
        sheet: None,
        checks: vec![RigCheck {
            ok: checks_ok,
            title: "Only joints and weights changed".to_string(),
            detail: String::new(),
            value: String::new(),
        }],
        row: Some("forward".to_string()),
        stretch_before: Some(139.0),
        stretch_after: Some(25.0),
    }
}

fn bind(root: &std::path::Path, parts: &[u32], bone: Option<&str>) -> String {
    let config = WorkspaceConfig::default();
    let tools = crate::pipeline::tools::Toolbox::default();
    doctors()
        .bind_rigid(
            (root, &config, tools),
            "moth-v1",
            (parts, bone),
            RigStarter::Agent,
        )
        .expect_err("refused")
        .to_string()
}

#[test]
fn a_bind_needs_a_fixed_humanoid_draft_and_real_parts_and_bones() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    fixed(root, |c| c.route = Some(RigRoute::Creature));
    assert!(bind(root, &[1], None).contains("creature"));
    fixed(root, |c| c.state = RigState::Tested);
    assert!(bind(root, &[1], None).contains("Run the fix first"));
    fixed(root, |_| {});
    assert!(bind(root, &[3], None).contains("no part 3"));
    assert!(bind(root, &[], None).contains("at least one part"));
    assert!(bind(root, &[1], Some("LeftArm")).contains("not a body bone"));
    fixed(root, |c| {
        c.rigid = Some(RigRigid {
            state: RigidState::Approved,
            ..asked(true)
        });
    });
    assert!(bind(root, &[1], None).contains("Undo the bound parts first"));
}

#[test]
fn the_bind_follows_spine2_unless_a_bone_is_named() {
    let dir = tempfile::tempdir().expect("dir");
    fixed(dir.path(), |_| {});
    let config = WorkspaceConfig::default();
    let case = doctors().get(dir.path(), &config, "moth-v1").expect("case");
    assert_eq!(default_bone(&case).as_deref(), Some("mixamorig:Spine2"));
    let check = rigid::cannot_bind(&case, &[1, 2], "mixamorig:Hips");
    assert_eq!(check, None);
}

#[test]
fn a_rigid_run_cannot_start_without_a_bind_named() {
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    fixed(root, |_| {});
    let tools = crate::pipeline::tools::Toolbox::default();
    let answer = doctors().run(root, &config, tools, "moth-v1", RigRun::Rigid);
    assert!(answer.is_err_and(|e| e.to_string().contains("Name the parts")));
}

#[test]
fn a_persons_yes_uses_the_bound_draft_and_a_no_after_it_puts_the_fix_back() {
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    fixed(root, |c| c.rigid = Some(asked(true)));
    let d = doctors();
    let yes = d.decide_rigid(root, &config, "moth-v1", true).expect("yes");
    assert_eq!(yes.draft.as_deref(), Some("rigid.glb"));
    let rigid = yes.rigid.expect("bind");
    assert_eq!(rigid.state, RigidState::Approved);
    assert_eq!(rigid.earlier.as_deref(), Some("draft.glb"));
    let undo = d
        .decide_rigid(root, &config, "moth-v1", false)
        .expect("undo");
    assert_eq!(undo.draft.as_deref(), Some("draft.glb"));
    assert_eq!(undo.rigid.expect("bind").state, RigidState::Declined);
    assert!(d.decide_rigid(root, &config, "moth-v1", true).is_err());
}

#[test]
fn a_no_keeps_the_fix_and_a_failed_check_cannot_be_approved() {
    let dir = tempfile::tempdir().expect("dir");
    let (root, config) = (dir.path(), WorkspaceConfig::default());
    fixed(root, |c| c.rigid = Some(asked(true)));
    let no = doctors()
        .decide_rigid(root, &config, "moth-v1", false)
        .expect("no");
    assert_eq!(no.draft.as_deref(), Some("draft.glb"));
    assert_eq!(no.rigid.expect("bind").state, RigidState::Declined);
    fixed(root, |c| c.rigid = Some(asked(false)));
    let yes = doctors().decide_rigid(root, &config, "moth-v1", true);
    assert!(yes.is_err_and(|e| e.to_string().contains("did not hold")));
}
