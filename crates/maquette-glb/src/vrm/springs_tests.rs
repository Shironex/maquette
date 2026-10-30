use std::path::Path;

use serde_json::Value;

use super::*;
use crate::fixture::{self, HumanoidSpec};

fn written() -> Vec<u8> {
    let spec = WriteSpec {
        meta: VrmMeta {
            name: "Hero".to_string(),
            authors: vec!["Test Author".to_string()],
            ..VrmMeta::default()
        },
        ..WriteSpec::default()
    };
    let model = fixture::humanoid(&HumanoidSpec {
        tail: true,
        ..HumanoidSpec::default()
    });
    write(&model, &spec, Path::new(".")).expect("write").bytes
}

/// A spring along the spine (three joints, each the child of the one
/// before) and colliders on the head and the left upper arm.
fn bones() -> SpringBones {
    SpringBones {
        springs: vec![SpringChain {
            name: "hair-1".to_string(),
            joints: ["Spine", "Spine1", "Spine2"]
                .iter()
                .map(|j| format!("mixamorig:{j}"))
                .collect(),
            settings: SpringKind::Hair.preset().settings(),
            collider_groups: vec!["Head".to_string(), "Arms".to_string(), "Gone".to_string()],
        }],
        colliders: vec![
            SpringCollider {
                group: "Head".to_string(),
                bone: "head".to_string(),
                joint: "mixamorig:Head".to_string(),
                shape: ColliderShape::Sphere {
                    offset: [0.0, 0.09, 0.0],
                    radius: 0.1,
                },
            },
            SpringCollider {
                group: "Arms".to_string(),
                bone: "leftUpperArm".to_string(),
                joint: "mixamorig:LeftArm".to_string(),
                shape: ColliderShape::Capsule {
                    offset: [0.0; 3],
                    radius: 0.05,
                    tail: [0.0, 0.25, 0.0],
                },
            },
        ],
    }
}

fn ext(bytes: &[u8]) -> Value {
    let (json, _) = container::split(bytes).expect("split");
    json["extensions"][VRMC_SPRING_BONE].clone()
}

fn error(bytes: &[u8], bones: &SpringBones) -> String {
    match add_springs(bytes, bones) {
        Ok(_) => panic!("expected a refusal"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn springs_are_written_as_vrmc_spring_bone_and_read_back() {
    let vrm = written();
    let out = add_springs(&vrm, &bones()).expect("springs");
    let found = ext(&out);
    assert_eq!(found["specVersion"], "1.0");
    assert_eq!(found["colliders"].as_array().map(Vec::len), Some(2));
    assert!(found["colliders"][0]["shape"]["sphere"]["radius"].is_number());
    assert!(found["colliders"][1]["shape"]["capsule"]["tail"].is_array());
    assert_eq!(found["colliderGroups"][0]["name"], "Head");
    assert_eq!(found["colliderGroups"][1]["colliders"][0], 1);
    let spring = &found["springs"][0];
    assert_eq!(spring["name"], "hair-1");
    assert_eq!(spring["joints"].as_array().map(Vec::len), Some(3));
    let joint = &spring["joints"][0];
    for key in [
        "hitRadius",
        "stiffness",
        "gravityPower",
        "gravityDir",
        "dragForce",
    ] {
        assert!(!joint[key].is_null(), "{key}");
    }
    // A group the colliders do not have is left out.
    assert_eq!(spring["colliderGroups"], serde_json::json!([0, 1]));
    let (json, _) = container::split(&out).expect("split");
    assert!(
        json["extensionsUsed"]
            .as_array()
            .is_some_and(|l| l.iter().any(|e| e == VRMC_SPRING_BONE))
    );
    let back = round_trip(&out);
    assert!(back.passed, "{:?}", back.problems);
    assert_eq!(back.springs, 1);
    assert_eq!(round_trip(&vrm).springs, 0);
}

#[test]
fn presets_differ_by_kind() {
    let hair = SpringKind::Hair.preset().settings();
    let cloth = SpringKind::Cloth.preset().settings();
    let charm = SpringKind::Accessory.preset().settings();
    assert!(cloth.drag_force > hair.drag_force);
    assert!(cloth.gravity_power > hair.gravity_power);
    assert!(charm.stiffness > cloth.stiffness);
    assert!(charm.hit_radius < hair.hit_radius);
    assert_eq!(hair.gravity_dir, [0.0, -1.0, 0.0]);
}

#[test]
fn the_writer_refuses_springs_the_file_cannot_hold() {
    let vrm = written();
    let mut missing = bones();
    missing.springs[0].joints[1] = "spring_nowhere_1".to_string();
    assert!(error(&vrm, &missing).contains("no node named spring_nowhere_1"));

    let mut unchained = bones();
    unchained.springs[0].joints = vec![
        "mixamorig:Spine".to_string(),
        "mixamorig:Spine2".to_string(),
    ];
    assert!(error(&vrm, &unchained).contains("not the child"));

    let mut short = bones();
    short.springs[0].joints.truncate(1);
    assert!(error(&vrm, &short).contains("at least two joints"));

    let mut wild = bones();
    wild.springs[0].settings.drag_force = 3.0;
    assert!(error(&vrm, &wild).contains("out of range"));
}

#[test]
fn a_broken_spring_bone_fails_the_round_trip() {
    let out = add_springs(&written(), &bones()).expect("springs");
    let (mut json, bin) = container::split(&out).expect("split");
    let ext = &mut json["extensions"][VRMC_SPRING_BONE];
    ext["springs"][0]["joints"][1]["node"] = serde_json::json!(9999);
    ext["colliderGroups"][0]["colliders"] = serde_json::json!([7]);
    ext["colliders"][1]["shape"] = serde_json::json!({ "box": {} });
    let broken = container::join(&json, bin).expect("join");
    let found = round_trip(&broken);
    assert!(!found.passed);
    let text = found.problems.join(" ");
    assert!(text.contains("two or more joints"), "{text}");
    assert!(text.contains("lists a collider"), "{text}");
    assert!(text.contains("neither a sphere"), "{text}");
}

#[test]
fn a_spring_whose_joints_do_not_chain_fails_the_round_trip() {
    let out = add_springs(&written(), &bones()).expect("springs");
    let (mut json, bin) = container::split(&out).expect("split");
    let spring = &mut json["extensions"][VRMC_SPRING_BONE]["springs"][0];
    let first = spring["joints"][0]["node"].clone();
    spring["joints"][2]["node"] = first;
    spring["colliderGroups"] = serde_json::json!([5]);
    let broken = container::join(&json, bin).expect("join");
    let text = round_trip(&broken).problems.join(" ");
    assert!(text.contains("not each the child"), "{text}");
    assert!(text.contains("collider group the file"), "{text}");
}
