//! The bones only on a model, baked for real in headless Blender.

use std::fs;
use std::path::Path;

use super::tests_blender::{Setup, elsheria_with_a_dodge, fixture, setup};
use crate::test_blender;
use crate::workspace::WorkspaceConfig;

/// The hand and the socket's distance per frame in a baked clip, measured
/// by Blender itself on the written file.
const MEASURE_FOLLOW: &str = r#"
import json, sys
import bpy
model, clip, out = sys.argv[sys.argv.index("--") + 1 :]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=model)
arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
action = next(a for a in bpy.data.actions if a.name == clip or a.name.startswith(clip + "_"))
arm.animation_data_create().action = action
if action.slots:
    arm.animation_data.action_slot = action.slots[0]
scene = bpy.context.scene
gaps = []
for frame in range(int(action.frame_range[0]), int(action.frame_range[1]) + 1, 5):
    scene.frame_set(frame)
    bpy.context.view_layer.update()
    a, b = arm.pose.bones["sword_socket"].head, arm.pose.bones["hand.L"].head
    gaps.append((a - b).length)
json.dump(gaps, open(out, "w"))
"#;

fn socket_gaps(s: &Setup, draft: &Path) -> Vec<f32> {
    let script = s.root.path().join("measure_follow.py");
    fs::write(&script, MEASURE_FOLLOW).expect("script");
    let out = s.root.path().join("gaps.json");
    let at = |p: &Path| p.display().to_string();
    test_blender::run(
        &s.blender,
        &[
            "--python",
            &at(&script),
            "--",
            &at(draft),
            "dodge_left",
            &at(&out),
        ],
    );
    serde_json::from_str(&fs::read_to_string(&out).expect("gaps")).expect("json")
}

#[test]
fn a_bone_only_on_the_model_follows_the_bone_chosen_for_it() {
    use super::retarget::{self, FollowChoice, RestShape, UseClips};
    let Some(s) = setup() else { return };
    let config = WorkspaceConfig::default();
    let dir = elsheria_with_a_dodge(&s, &config);
    fixture(
        &s.blender,
        &s.root.path().join("scripts"),
        "model_socket",
        &dir.join("models").join("elsheria_v1.glb"),
    );
    let target = ("frostveil", "elsheria");
    let clips = ["dodge-left".to_string()];
    let plan = retarget::plan(s.root.path(), &config, target, &clips, &[]).expect("plan");

    // What the bone map shows, read from the two files.
    let rest = plan.rest.as_ref().expect("both skeletons");
    assert_eq!(rest.clip_shape, RestShape::TPose);
    assert_eq!(rest.model_shape, RestShape::APose);
    assert!((rest.model_arms_deg - 45.0).abs() < 2.0, "{rest:?}");
    assert!(rest.clip_arms_deg.abs() < 2.0, "{rest:?}");
    assert!(rest.model_hips > 0.5, "{rest:?}");
    let arm = rest
        .turns
        .iter()
        .find(|t| t.bone == "LeftArm")
        .expect("arm");
    assert!((arm.deg - 45.0).abs() < 2.0, "{arm:?}");
    let socket = plan
        .only_bones
        .iter()
        .find(|o| o.bone == "sword_socket")
        .expect("only on the model");
    assert_eq!(socket.carried_by.as_deref(), Some("hips"));
    assert!(socket.free);

    let tools = (&s.runner, &s.tools);
    let request = |follow| UseClips {
        project: target.0.into(),
        asset: target.1.into(),
        settings: plan.settings.clone(),
        map: Vec::new(),
        save_map: true,
        follow,
    };
    let plain =
        retarget::draft(s.root.path(), &config, tools, &request(Vec::new())).expect("draft");
    let gaps = socket_gaps(&s, Path::new(&plain.file));
    let spread = |g: &[f32]| {
        g.iter().copied().fold(f32::MIN, f32::max) - g.iter().copied().fold(f32::MAX, f32::min)
    };
    assert!(spread(&gaps) > 0.01, "the hand moves on its own: {gaps:?}");

    let choice = FollowChoice {
        bone: "sword_socket".into(),
        follows: Some("hand.L".into()),
    };
    let held =
        retarget::draft(s.root.path(), &config, tools, &request(vec![choice])).expect("draft");
    let gaps = socket_gaps(&s, Path::new(&held.file));
    assert!(
        spread(&gaps) < 0.005,
        "the socket goes with the hand: {gaps:?}"
    );

    // Saved with the map, and shown again next time.
    let again = retarget::plan(s.root.path(), &config, target, &clips, &[]).expect("again");
    let socket = again
        .only_bones
        .iter()
        .find(|o| o.bone == "sword_socket")
        .expect("socket");
    assert_eq!(socket.follows.as_deref(), Some("hand.L"));
}
