use std::fs;
use std::path::Path;

use serde_json::json;

use super::*;
use crate::fixture;

#[test]
fn key_rates_snap_to_the_common_ones() {
    assert!((clips::key_rate(33, 32.0 / 30.0) - 30.0).abs() < f32::EPSILON);
    assert!((clips::key_rate(121, 4.0) - 30.0).abs() < f32::EPSILON);
    assert!((clips::key_rate(25, 1.0) - 24.0).abs() < f32::EPSILON);
    assert!((clips::key_rate(3, 2.0) - 1.0).abs() < f32::EPSILON);
    assert!(clips::key_rate(1, 0.0).abs() < f32::EPSILON);
}

#[test]
fn the_fixture_character_has_one_idle_clip_on_a_mixamo_rig() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("figure.glb");
    fs::write(&file, fixture::character(false)).expect("figure");
    let found = read_file(&file).expect("read");
    assert_eq!(found.clips.len(), 1);
    let idle = &found.clips[0];
    assert_eq!(idle.name, "Idle");
    assert_eq!((idle.keys, idle.channels, idle.targets), (3, 1, 1));
    assert!((idle.duration - 2.0).abs() < 1e-6);
    // Three times and three positions, four bytes a number.
    assert!((idle.bytes - 48.0).abs() < f64::EPSILON, "{}", idle.bytes);
    assert_eq!(idle.root_travel, None, "the hips dip and come back");
    assert_eq!(idle.loops, Some(true));
    let rig = found.rig.expect("rig");
    assert_eq!(rig.kind, RigKind::Mixamo);
    assert!(rig.humanoid);
    assert!(
        rig.map.iter().all(|m| m.how == BoneMatch::Exact),
        "{:?}",
        rig.map
    );
    assert_eq!(rig.max_influences, Some(1));
    assert_eq!(found.parts, ["Body"]);
}

#[test]
fn keys_packed_by_meshopt_leave_travel_and_looping_unknown() {
    // The web app's mock Elsheria: a Mixamo rig with two clips, shipped packed.
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/web/src/assets/mock/models/elsheria.glb");
    let found = read_file(&file).expect("read");
    assert_eq!(found.clips.len(), 2);
    for clip in &found.clips {
        assert_eq!(
            (clip.loops, clip.root_travel),
            (None, None),
            "{}",
            clip.name
        );
        assert!(clip.bytes > 0.0 && clip.duration >= 0.5);
    }
    let rig = found.rig.expect("rig");
    assert_eq!((rig.joints, rig.kind), (119, RigKind::Mixamo));
    assert_eq!(rig.max_influences, None, "packed weights are not read");
}

#[test]
fn other_rig_names_map_onto_the_humanoid_bones() {
    let names: Vec<String> = [
        "hips",
        "spine_01",
        "spine_02",
        "spine_03",
        "neck",
        "head",
        "clavicle_l",
        "upperarm_l",
        "lowerarm_l",
        "hand_l",
        "clavicle_r",
        "upperarm_r",
        "lowerarm_r",
        "hand_r",
        "thigh_l",
        "calf_l",
        "foot_l",
        "thigh_r",
        "calf_r",
        "foot_r",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    let map = rig::humanoid_map(&names);
    assert!(rig::is_humanoid(&map));
    let of = |bone: &str| map.iter().find(|m| m.bone == bone).expect(bone).clone();
    assert_eq!(of("LeftForeArm").joint.as_deref(), Some("lowerarm_l"));
    assert_eq!(of("LeftForeArm").how, BoneMatch::ByName);
    assert_eq!(of("RightLeg").joint.as_deref(), Some("calf_r"));
    let missing: Vec<&str> = map
        .iter()
        .filter(|m| m.how == BoneMatch::Missing)
        .map(|m| m.bone.as_str())
        .collect();
    assert_eq!(missing, ["LeftToeBase", "RightToeBase"]);

    let rigify: Vec<String> = ["upper_arm.L", "forearm.L", "hand.R", "Left_Hand"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let map = rig::humanoid_map(&rigify);
    assert!(!rig::is_humanoid(&map));
    let of = |bone: &str| {
        map.iter()
            .find(|m| m.bone == bone)
            .expect(bone)
            .joint
            .clone()
    };
    assert_eq!(of("LeftArm").as_deref(), Some("upper_arm.L"));
    assert_eq!(of("RightHand").as_deref(), Some("hand.R"));
    assert_eq!(of("LeftHand").as_deref(), Some("Left_Hand"));
}

#[test]
fn compressed_clips_count_their_share_of_the_packed_view() {
    let doc = json!({
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 10, "type": "SCALAR" },
            { "bufferView": 0, "componentType": 5126, "count": 10, "type": "VEC4" },
            { "bufferView": 1, "componentType": 5126, "count": 10, "type": "VEC3" }
        ],
        "bufferViews": [
            { "byteLength": 200, "extensions": { "EXT_meshopt_compression": { "byteLength": 50 } } },
            { "byteLength": 120 }
        ],
        "animations": [
            { "name": "Spin", "samplers": [{ "input": 0, "output": 1 }] },
            { "samplers": [{ "input": 0, "output": 2 }] }
        ]
    });
    let (per, total) = clips::clip_bytes(&doc);
    // 40 and 160 of the 200 packed into 50.
    assert!((per["Spin"] - 50.0).abs() < 1e-9, "{per:?}");
    assert!(
        (per["Animation 2"] - (10.0 + 120.0)).abs() < 1e-9,
        "{per:?}"
    );
    assert!((total - 170.0).abs() < 1e-9, "the shared input counts once");
}
