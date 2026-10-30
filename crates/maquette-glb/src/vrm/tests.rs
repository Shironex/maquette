use std::path::Path;

use super::*;
use crate::fixture::{self, HumanoidSpec, Naming};

fn read(spec: &HumanoidSpec) -> VrmReadiness {
    readiness(
        &fixture::humanoid(spec),
        Path::new("."),
        &BoneOverrides::new(),
    )
    .expect("readiness")
}

fn ids(r: &VrmReadiness, level: FindingLevel) -> Vec<&str> {
    r.findings
        .iter()
        .filter(|f| f.level == level)
        .map(|f| f.id.as_str())
        .collect()
}

fn joint<'a>(r: &'a VrmReadiness, bone: &str) -> Option<&'a str> {
    r.bones
        .iter()
        .find(|b| b.bone == bone)
        .and_then(|b| b.joint.as_deref())
}

#[test]
fn the_table_names_55_bones_15_required_parents_first() {
    let human = bones::human_bones();
    assert_eq!(human.len(), HUMAN_BONES);
    assert_eq!(human.iter().filter(|b| b.required).count(), REQUIRED_BONES);
    for (i, b) in human.iter().enumerate() {
        if let Some(parent) = &b.parent {
            let at = human.iter().position(|p| &p.name == parent);
            assert!(at.is_some_and(|p| p < i), "{} before {parent}", b.name);
        }
    }
    let names: Vec<&str> = human.iter().map(|b| b.name.as_str()).collect();
    for bone in [
        "upperChest",
        "leftEye",
        "jaw",
        "rightToes",
        "leftThumbMetacarpal",
        "rightLittleDistal",
    ] {
        assert!(names.contains(&bone), "{bone}");
    }
}

#[test]
fn a_clean_mixamo_t_pose_is_ready() {
    let r = read(&HumanoidSpec::default());
    assert!(r.ready, "{:#?}", r.findings);
    // 25 body bones less eyes and jaw, and all 30 finger bones.
    assert_eq!((r.mapped, r.required), (52, 15));
    assert_eq!(joint(&r, "leftUpperArm"), Some("mixamorig:LeftArm"));
    assert_eq!(joint(&r, "upperChest"), Some("mixamorig:Spine2"));
    assert_eq!(
        joint(&r, "leftThumbMetacarpal"),
        Some("mixamorig:LeftHandThumb1")
    );
    assert_eq!(
        joint(&r, "rightLittleDistal"),
        Some("mixamorig:RightHandPinky3")
    );
    assert_eq!(joint(&r, "leftEye"), None);
    assert!(
        r.bones
            .iter()
            .filter(|b| b.joint.is_some())
            .all(|b| b.naming == Some(BoneNaming::Mixamo))
    );
    let p = &r.pose;
    assert_eq!(p.facing, Some(Axis::PlusZ));
    assert!(p.facing_deg.is_some_and(|d| d.abs() < 1e-3));
    assert!(p.arm_left_deg.is_some_and(|d| d.abs() < 1e-3));
    assert!(p.arm_right_deg.is_some_and(|d| d.abs() < 1e-3));
    assert!(
        p.ground_y.is_some_and(|y| y.abs() < 1e-4),
        "{:?}",
        p.ground_y
    );
    assert!(p.asymmetry.is_some_and(|a| a < 1e-4));
    assert!(p.uniform_scale && !p.sides_swapped);
    // The idle clip and the missing eyes are notes, ranked after the rest.
    assert_eq!(ids(&r, FindingLevel::Note), ["bones", "animations"]);
}

#[test]
fn every_naming_maps_the_required_bones() {
    for naming in [Naming::Vrm, Naming::Unreal, Naming::Tripo] {
        let r = read(&HumanoidSpec {
            naming,
            ..HumanoidSpec::default()
        });
        assert_eq!(r.required, 15, "{naming:?}: {:#?}", r.findings);
        assert!(r.ready, "{naming:?}: {:#?}", r.findings);
    }
    let vrm = read(&HumanoidSpec {
        naming: Naming::Vrm,
        ..HumanoidSpec::default()
    });
    assert_eq!(vrm.mapped, 52);
    assert_eq!(
        joint(&vrm, "leftIndexIntermediate"),
        Some("leftIndexIntermediate")
    );
    let unreal = read(&HumanoidSpec {
        naming: Naming::Unreal,
        ..HumanoidSpec::default()
    });
    assert_eq!(joint(&unreal, "chest"), Some("spine_02"));
    assert_eq!(joint(&unreal, "leftThumbProximal"), Some("thumb_02_l"));
    assert_eq!(unreal.mapped, 52);
    let tripo = read(&HumanoidSpec {
        naming: Naming::Tripo,
        ..HumanoidSpec::default()
    });
    assert_eq!(joint(&tripo, "neck"), Some("NeckTwist01"));
    assert_eq!(joint(&tripo, "rightToes"), Some("R_ToeBase"));
    // Tripo's finger names are unknown, so its fingers stay unmapped.
    assert_eq!(tripo.mapped, 22);
}

#[test]
fn vroid_and_prefixed_names_map_too() {
    let bones = bones::human_bones();
    let joints: Vec<(usize, String)> = [
        "J_Bip_C_Hips",
        "J_Bip_L_UpperArm",
        "J_Bip_R_Little2",
        "J_Adj_L_FaceEye",
        "mixamorig1_LeftForeArm",
    ]
    .iter()
    .enumerate()
    .map(|(i, n)| (i, (*n).to_string()))
    .collect();
    let mapped = bones::map_joints(&bones, &joints, &joints, &BoneOverrides::new());
    let got: Vec<(&str, usize)> = mapped.iter().map(|m| (m.bone.as_str(), m.node)).collect();
    assert_eq!(
        got,
        [
            ("hips", 0),
            ("leftEye", 3),
            ("leftUpperArm", 1),
            ("leftLowerArm", 4),
            ("rightLittleIntermediate", 2),
        ]
    );
}

#[test]
fn an_a_pose_facing_x_is_not_ready_and_says_why() {
    let r = read(&HumanoidSpec {
        arm_drop: 50.0,
        yaw: 90.0,
        ..HumanoidSpec::default()
    });
    assert!(!r.ready);
    assert_eq!(ids(&r, FindingLevel::Blocker), ["facing", "arms"]);
    let p = &r.pose;
    assert_eq!(p.facing, Some(Axis::PlusX));
    assert!(p.facing_deg.is_some_and(|d| (d - 90.0).abs() < 1e-2));
    assert!(
        p.arm_left_deg.is_some_and(|d| (d - 50.0).abs() < 1e-2),
        "{p:?}"
    );
    assert!(p.arm_right_deg.is_some_and(|d| (d - 50.0).abs() < 1e-2));
    let facing = &r.findings[0];
    assert_eq!(facing.title, "Faces +X, VRM wants +Z");
    let arms = &r.findings[1];
    assert_eq!(arms.title, "Arms hang below horizontal, VRM wants a T-pose");
    assert!(
        arms.detail
            .as_deref()
            .is_some_and(|d| d.contains("left 50 degrees below and right 50 degrees below")),
        "{arms:?}"
    );
    // Turning does not break the mirror.
    assert!(p.asymmetry.is_some_and(|a| a < 1e-3), "{p:?}");
}

#[test]
fn lift_scale_and_noise_are_measured() {
    let lifted = read(&HumanoidSpec {
        lift: 0.05,
        ..HumanoidSpec::default()
    });
    assert!(
        lifted
            .pose
            .ground_y
            .is_some_and(|y| (y - 0.05).abs() < 1e-4)
    );
    assert_eq!(ids(&lifted, FindingLevel::Warning), ["ground"]);
    assert!(lifted.ready, "a warning does not block");

    let noisy = read(&HumanoidSpec {
        scale_noise: true,
        ..HumanoidSpec::default()
    });
    assert!(noisy.ready);
    assert_eq!(noisy.pose.scale_noise, 1);
    assert!(ids(&noisy, FindingLevel::Note).contains(&"scale-noise"));

    let small = read(&HumanoidSpec {
        armature_scale: 0.01,
        ..HumanoidSpec::default()
    });
    assert_eq!(small.pose.scaled, ["Armature (0.01)"]);
    assert!(ids(&small, FindingLevel::Warning).contains(&"scaled"));
}

#[test]
fn missing_bones_overrides_and_wrong_parents_are_blockers() {
    let bytes = fixture::humanoid(&HumanoidSpec::default());
    let mut overrides = BoneOverrides::new();
    overrides.insert("leftHand".to_string(), None);
    let r = readiness(&bytes, Path::new("."), &overrides).expect("readiness");
    assert!(!r.ready);
    let missing = r
        .findings
        .iter()
        .find(|f| f.id == "required-bones")
        .expect("row");
    assert_eq!(missing.title, "1 bone VRM needs not found");
    // An unmapped bone is skipped on both trees, so the fingers still sit
    // in order under the lower arm.
    assert!(!ids(&r, FindingLevel::Blocker).contains(&"parents"));

    // An override can put a bone back by any node name.
    overrides.insert(
        "leftHand".to_string(),
        Some("mixamorig:LeftHand".to_string()),
    );
    let back = readiness(&bytes, Path::new("."), &overrides).expect("readiness");
    assert!(back.ready, "{:#?}", back.findings);
    let hand = back
        .bones
        .iter()
        .find(|b| b.bone == "leftHand")
        .expect("hand");
    assert_eq!(hand.naming, Some(BoneNaming::Override));

    // Pointing the chest at the head breaks the order both ways.
    overrides.clear();
    overrides.insert("chest".to_string(), Some("mixamorig:Head".to_string()));
    overrides.insert("head".to_string(), Some("mixamorig:Spine1".to_string()));
    let swapped = readiness(&bytes, Path::new("."), &overrides).expect("readiness");
    let parents = swapped
        .findings
        .iter()
        .find(|f| f.id == "parents")
        .expect("row");
    assert!(
        parents
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("head sits under spine, not neck")),
        "{parents:?}"
    );
}

#[test]
fn a_finger_without_the_segment_above_is_a_gap() {
    let bytes = fixture::humanoid(&HumanoidSpec::default());
    let mut overrides = BoneOverrides::new();
    overrides.insert("leftIndexProximal".to_string(), None);
    let r = readiness(&bytes, Path::new("."), &overrides).expect("readiness");
    let gap = r
        .findings
        .iter()
        .find(|f| f.id == "bone-gaps")
        .expect("gap");
    assert!(
        gap.detail
            .as_deref()
            .is_some_and(|d| d.contains("leftIndexIntermediate without leftIndexProximal")),
        "{gap:?}"
    );
}

#[test]
fn swapped_sides_and_an_unrigged_model_are_blockers() {
    // Name the left limbs right and the right limbs left: the toes still
    // point to +Z, so the front read from the sides points backwards.
    let bytes = fixture::humanoid(&HumanoidSpec::default());
    let gltf = gltf::Gltf::from_slice(&bytes).expect("parse");
    let mut overrides = BoneOverrides::new();
    for (bone, joint) in [
        ("leftUpperLeg", "RightUpLeg"),
        ("rightUpperLeg", "LeftUpLeg"),
        ("leftLowerLeg", "RightLeg"),
        ("rightLowerLeg", "LeftLeg"),
        ("leftFoot", "RightFoot"),
        ("rightFoot", "LeftFoot"),
        ("leftToes", "RightToeBase"),
        ("rightToes", "LeftToeBase"),
        ("leftUpperArm", "RightArm"),
        ("rightUpperArm", "LeftArm"),
    ] {
        overrides.insert(bone.to_string(), Some(format!("mixamorig:{joint}")));
    }
    let r = readiness_of(&gltf.document, std::slice::from_ref(&gltf.blob), &overrides);
    assert_eq!(r.pose.facing, Some(Axis::MinusZ));
    assert!(r.pose.sides_swapped);
    assert!(ids(&r, FindingLevel::Blocker).contains(&"sides"));

    let prop = readiness(
        &fixture::clean_prop(),
        Path::new("."),
        &BoneOverrides::new(),
    )
    .expect("readiness");
    assert!(!prop.ready);
    assert_eq!(
        ids(&prop, FindingLevel::Blocker),
        ["skin", "required-bones", "facing"]
    );
}

#[test]
fn a_turned_model_off_centre_reads_its_offsets_in_its_own_frame() {
    for yaw in [90.0, -90.0, 180.0] {
        let r = read(&HumanoidSpec {
            yaw,
            offset: [0.02, 0.26],
            ..HumanoidSpec::default()
        });
        let p = &r.pose;
        assert!(
            p.hips_offset.is_some_and(|o| (o - 0.2608).abs() < 1e-3),
            "{yaw}: {p:?}"
        );
        assert!(
            p.hips_forward.is_some_and(|f| (f - 0.26).abs() < 1e-3),
            "{yaw}: {p:?}"
        );
        assert!(
            p.hips_side.is_some_and(|s| (s - 0.02).abs() < 1e-3),
            "{yaw}: {p:?}"
        );
        // Moving and turning the whole body keeps it mirrored.
        assert!(p.asymmetry.is_some_and(|a| a < 1e-3), "{yaw}: {p:?}");
        let centre = r
            .findings
            .iter()
            .find(|f| f.id == "centre")
            .expect("centre");
        assert_eq!(centre.title, "Hips sit 26 cm off the centre");
        assert!(
            centre
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("26 cm forward and 2 cm to its left")),
            "{centre:?}"
        );
        assert!(!ids(&r, FindingLevel::Warning).contains(&"symmetry"));
    }
    let behind = read(&HumanoidSpec {
        offset: [-0.1, -0.1],
        ..HumanoidSpec::default()
    });
    let centre = behind
        .findings
        .iter()
        .find(|f| f.id == "centre")
        .expect("centre");
    assert!(
        centre
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("10 cm back and 10 cm to its right")),
        "{centre:?}"
    );
}

#[test]
fn the_widest_mirror_gap_names_its_bone() {
    // The left hand read from the index finger's first joint: only that
    // pair is out of mirror.
    let bytes = fixture::humanoid(&HumanoidSpec {
        yaw: 90.0,
        ..HumanoidSpec::default()
    });
    let mut overrides = BoneOverrides::new();
    overrides.insert(
        "leftHand".to_string(),
        Some("mixamorig:LeftHandIndex1".to_string()),
    );
    let r = readiness(&bytes, Path::new("."), &overrides).expect("readiness");
    assert_eq!(r.pose.asymmetry_bone.as_deref(), Some("leftHand"));
    assert!(r.pose.asymmetry.is_some_and(|a| a > 0.05), "{:?}", r.pose);
}

#[test]
fn scale_noise_from_an_export_is_not_a_scale() {
    // Float noise as exports leave it, up to about 2e-5.
    for noise in [1.000_010_6, 1.000_017_4, 0.999_982_6] {
        let r = read(&HumanoidSpec {
            armature_scale: noise,
            ..HumanoidSpec::default()
        });
        assert!(r.pose.scaled.is_empty(), "{noise}: {:?}", r.pose.scaled);
        assert_eq!(r.pose.scale_noise, 1, "{noise}");
        assert!(!ids(&r, FindingLevel::Warning).contains(&"scaled"));
    }
    let real = read(&HumanoidSpec {
        armature_scale: 1.001,
        ..HumanoidSpec::default()
    });
    assert_eq!(real.pose.scaled, ["Armature (1.001)"]);
}

#[test]
fn findings_are_ranked_blockers_first() {
    let r = read(&HumanoidSpec {
        arm_drop: 40.0,
        lift: 0.1,
        ..HumanoidSpec::default()
    });
    let levels: Vec<FindingLevel> = r.findings.iter().map(|f| f.level).collect();
    let mut sorted = levels.clone();
    sorted.sort();
    assert_eq!(levels, sorted);
    assert_eq!(r.count(FindingLevel::Blocker), 1);
    assert_eq!(r.count(FindingLevel::Warning), 1);
}

#[test]
fn a_cape_weighted_to_the_arm_is_a_warning() {
    let clean = read(&HumanoidSpec::default());
    assert!(!ids(&clean, FindingLevel::Warning).contains(&"arm-weights"));
    for arm_drop in [0.0, 45.0] {
        let caped = read(&HumanoidSpec {
            cape: true,
            arm_drop,
            ..HumanoidSpec::default()
        });
        let found = caped
            .findings
            .iter()
            .find(|f| f.id == "arm-weights")
            .expect("arm weights");
        assert_eq!(found.level, FindingLevel::Warning);
        assert_eq!(
            found.title,
            "24 vertices of cloth or hair weighted to the arms"
        );
        let detail = found.detail.as_deref().unwrap_or_default();
        assert!(detail.contains("leftUpperArm (24)"), "{detail}");
        assert!(detail.contains("reweight"), "{detail}");
        // A warning only: the rig stays as ready as it was.
        assert_eq!(caped.ready, arm_drop == 0.0);
    }
}

/// A real model from disk, for a local look. Set `MAQUETTE_VRM_PROBE` to a
/// `.glb` and run with `--ignored --nocapture`.
#[test]
#[ignore = "reads a model from MAQUETTE_VRM_PROBE"]
fn probe_a_model_from_disk() {
    let Ok(file) = std::env::var("MAQUETTE_VRM_PROBE") else {
        return;
    };
    let bytes = std::fs::read(&file).expect("read");
    let r = readiness(&bytes, Path::new("."), &BoneOverrides::new()).expect("readiness");
    let mapped: Vec<String> = r
        .bones
        .iter()
        .filter_map(|b| b.joint.as_ref().map(|j| format!("{} <- {j}", b.bone)))
        .collect();
    println!("{}", mapped.join("\n"));
    println!(
        "ready {}, mapped {}, required {}",
        r.ready, r.mapped, r.required
    );
    println!("{:#?}", r.naming);
    println!("{} joints to pick from", r.joints.len());
    println!("{:#?}", r.pose);
    for f in &r.findings {
        println!(
            "[{:?}] {}: {}",
            f.level,
            f.title,
            f.detail.as_deref().unwrap_or("")
        );
    }
}
