use std::path::Path;

use super::*;
use crate::fixture::{self, HumanoidSpec, Naming};

fn read_with(spec: &HumanoidSpec, overrides: &BoneOverrides) -> VrmReadiness {
    readiness(&fixture::humanoid(spec), Path::new("."), overrides).expect("readiness")
}

fn descriptive() -> HumanoidSpec {
    HumanoidSpec {
        naming: Naming::Descriptive,
        ..HumanoidSpec::default()
    }
}

fn finding<'a>(r: &'a VrmReadiness, id: &str) -> Option<&'a VrmFinding> {
    r.findings.iter().find(|f| f.id == id)
}

fn joint<'a>(r: &'a VrmReadiness, bone: &str) -> Option<&'a str> {
    r.bones
        .iter()
        .find(|b| b.bone == bone)
        .and_then(|b| b.joint.as_deref())
}

fn joints(names: &[&str]) -> Vec<(usize, String)> {
    names
        .iter()
        .enumerate()
        .map(|(i, n)| (i, (*n).to_string()))
        .collect()
}

#[test]
fn exact_namings_report_their_convention_and_stay_confident() {
    for (naming, want) in [
        (Naming::Mixamo, BoneNaming::Mixamo),
        (Naming::Vrm, BoneNaming::Vrm),
        (Naming::Unreal, BoneNaming::Unreal),
        (Naming::Tripo, BoneNaming::Tripo),
    ] {
        let r = read_with(
            &HumanoidSpec {
                naming,
                ..HumanoidSpec::default()
            },
            &BoneOverrides::new(),
        );
        let n = &r.naming;
        assert_eq!(n.convention, Some(want), "{naming:?}");
        assert_eq!(n.convention_required, 15, "{naming:?}");
        assert!(n.confident, "{naming:?}: {n:?}");
        assert!(n.ambiguous.is_empty(), "{naming:?}: {n:?}");
        assert_eq!(n.descriptive, 0, "{naming:?}");
        assert!(finding(&r, "bone-naming").is_none(), "{naming:?}");
        // Every skin joint is offered for picking by hand.
        assert_eq!(r.joints.len(), 65, "{naming:?}");
    }
}

#[test]
fn no_exact_name_in_the_table_matches_two_bones() {
    let human = bones::human_bones();
    for bone in &human {
        for (_, name) in &bone.names {
            let owners: Vec<&str> = human
                .iter()
                .filter(|b| b.names.iter().any(|(_, n)| n == name))
                .map(|b| b.name.as_str())
                .collect();
            assert_eq!(owners.len(), 1, "{name}: {owners:?}");
        }
    }
}

#[test]
fn descriptive_names_map_fully_and_ask_for_a_check() {
    let r = read_with(&descriptive(), &BoneOverrides::new());
    assert_eq!((r.mapped, r.required), (52, 15), "{:#?}", r.findings);
    assert!(r.ready, "{:#?}", r.findings);
    assert_eq!(joint(&r, "leftUpperLeg"), Some("Left leg"));
    assert_eq!(joint(&r, "leftLowerLeg"), Some("Left knee"));
    assert_eq!(joint(&r, "rightLowerArm"), Some("Right elbow"));
    assert_eq!(joint(&r, "leftHand"), Some("Left wrist"));
    assert_eq!(joint(&r, "upperChest"), Some("Upper chest"));
    // VRM 0's thumb names, counted from the metacarpal.
    assert_eq!(joint(&r, "leftThumbMetacarpal"), Some("ThumbProximal_L"));
    assert_eq!(joint(&r, "leftThumbProximal"), Some("ThumbIntermediate_L"));
    assert_eq!(joint(&r, "rightLittleDistal"), Some("LittleDistal_R"));
    let n = &r.naming;
    assert_eq!(n.convention, Some(BoneNaming::Descriptive));
    assert!(n.confident, "{n:?}");
    // Hips, spine, chest, neck and head match VRM's own names exactly.
    assert_eq!(n.descriptive, 47);
    let found = finding(&r, "bone-naming").expect("naming finding");
    assert_eq!(found.level, FindingLevel::Warning);
    assert_eq!(found.title, "47 bones matched by descriptive names");
    assert_eq!(
        found.detail.as_deref(),
        Some(
            "47 bones were read from descriptive names such as Left elbow, which is a guess. \
             Check them in the Bone map and set any that are wrong."
        )
    );
    let hand = r.bones.iter().find(|b| b.bone == "leftHand").expect("hand");
    assert_eq!(hand.naming, Some(BoneNaming::Descriptive));
}

#[test]
fn an_unknown_humanoid_naming_is_unclear_and_never_guessed() {
    // Spelled so neither the exact namings nor the descriptive reader
    // knows the limbs: the rig looks humanoid, the map is a person's job.
    let mut overrides = BoneOverrides::new();
    let bytes = fixture::humanoid(&HumanoidSpec::default());
    let base = readiness(&bytes, Path::new("."), &overrides).expect("readiness");
    assert!(base.naming.confident);
    let gltf = gltf::Gltf::from_slice(&bytes).expect("parse");
    let renamed: Vec<(usize, String)> = gltf
        .document
        .nodes()
        .map(|n| {
            let name = n.name().unwrap_or_default();
            let odd = name
                .replace("mixamorig:Left", "Lft")
                .replace("mixamorig:Right", "Rgt")
                .replace("mixamorig:", "");
            (n.index(), odd)
        })
        .collect();
    let human = bones::human_bones();
    let mapped = bones::map_joints(&human, &renamed, &renamed, &overrides);
    let report = bones::naming_report(&human, &renamed, &mapped, &overrides);
    assert!(!report.confident, "{report:?}");
    assert_eq!(report.missing_required, 12);
    assert!(report.humanoid_joints >= HUMANOID_JOINTS_MIN, "{report:?}");
    let found = findings::naming_finding(&report).expect("finding");
    assert_eq!(found.title, "Bone names are unclear");
    let detail = found.detail.as_deref().unwrap_or_default();
    assert!(
        detail.contains("only 3 of the 15 required bones match Mixamo names"),
        "{detail}"
    );
    assert!(
        detail.contains(
            "Assign the bones in the Bone map, or have an agent set them with set_avatar."
        ),
        "{detail}"
    );
    // Overrides are how a person settles it.
    for (bone, joint) in [("leftUpperArm", "LftArm"), ("rightUpperArm", "RgtArm")] {
        overrides.insert(bone.to_string(), Some(joint.to_string()));
    }
    let mapped = bones::map_joints(&human, &renamed, &renamed, &overrides);
    let report = bones::naming_report(&human, &renamed, &mapped, &overrides);
    assert_eq!(report.missing_required, 10);
}

#[test]
fn two_joints_for_one_bone_are_ambiguous_and_left_for_a_person() {
    let human = bones::human_bones();
    let rig = joints(&[
        "Hips",
        "Spine",
        "Head",
        "Left leg",
        "Left upper leg",
        "Left knee",
    ]);
    let none = BoneOverrides::new();
    let mapped = bones::map_joints(&human, &rig, &rig, &none);
    // Neither leg joint is taken: guessing one would be silent.
    assert!(!mapped.iter().any(|m| m.bone == "leftUpperLeg"));
    assert!(mapped.iter().any(|m| m.bone == "leftLowerLeg"));
    let report = bones::naming_report(&human, &rig, &mapped, &none);
    assert_eq!(
        report.ambiguous,
        ["2 joints match leftUpperLeg: Left leg and Left upper leg."]
    );
    assert!(!report.confident);
    let detail = findings::naming_finding(&report)
        .and_then(|f| f.detail)
        .unwrap_or_default();
    assert!(
        detail.contains("2 joints match leftUpperLeg: Left leg and Left upper leg."),
        "{detail}"
    );

    // Two exact matches are named too: the first is mapped, as before.
    let rig = joints(&["mixamorig:Hips", "J_Bip_C_Hips", "Spine", "Head"]);
    let mapped = bones::map_joints(&human, &rig, &rig, &none);
    let report = bones::naming_report(&human, &rig, &mapped, &none);
    assert_eq!(
        report.ambiguous,
        ["2 joints match hips: mixamorig:Hips and J_Bip_C_Hips."]
    );

    // An override settles the bone.
    let mut picked = BoneOverrides::new();
    picked.insert("leftUpperLeg".to_string(), Some("Left leg".to_string()));
    let rig = joints(&["Hips", "Left leg", "Left upper leg"]);
    let mapped = bones::map_joints(&human, &rig, &rig, &picked);
    let report = bones::naming_report(&human, &rig, &mapped, &picked);
    assert!(report.ambiguous.is_empty(), "{report:?}");
}

#[test]
fn descriptive_names_never_take_a_bone_an_exact_name_has() {
    let human = bones::human_bones();
    let rig = joints(&["mixamorig:LeftForeArm", "Left elbow", "Left wrist"]);
    let mapped = bones::map_joints(&human, &rig, &rig, &BoneOverrides::new());
    let got: Vec<(&str, usize, BoneNaming)> = mapped
        .iter()
        .map(|m| (m.bone.as_str(), m.node, m.naming))
        .collect();
    assert_eq!(
        got,
        [
            ("leftLowerArm", 0, BoneNaming::Mixamo),
            ("leftHand", 2, BoneNaming::Descriptive),
        ]
    );
}

#[test]
fn overrides_feed_the_readiness_and_the_writer() {
    // Unmap the left wrist, then point the hand at it by hand: the
    // readiness and the written humanoid both follow the override.
    let bytes = fixture::humanoid(&descriptive());
    let mut overrides = BoneOverrides::new();
    overrides.insert("leftHand".to_string(), None);
    let r = readiness(&bytes, Path::new("."), &overrides).expect("readiness");
    assert!(!r.ready);
    assert_eq!(joint(&r, "leftHand"), None);
    overrides.insert("leftHand".to_string(), Some("Left wrist".to_string()));
    let r = readiness(&bytes, Path::new("."), &overrides).expect("readiness");
    assert!(r.ready, "{:#?}", r.findings);
    let hand = r.bones.iter().find(|b| b.bone == "leftHand").expect("hand");
    assert_eq!(hand.naming, Some(BoneNaming::Override));
    let spec = WriteSpec {
        meta: VrmMeta {
            name: "Box".to_string(),
            authors: vec!["Tester".to_string()],
            ..VrmMeta::default()
        },
        overrides,
        ..WriteSpec::default()
    };
    let written = write(&bytes, &spec, Path::new(".")).expect("write");
    let gltf = gltf::Gltf::from_slice(&written.bytes).expect("parse");
    let wrist = gltf
        .document
        .nodes()
        .find(|n| n.name() == Some("Left wrist"))
        .expect("wrist")
        .index();
    let (json, _) = container::split(&written.bytes).expect("split");
    assert_eq!(
        json["extensions"]["VRMC_vrm"]["humanoid"]["humanBones"]["leftHand"]["node"],
        serde_json::json!(wrist)
    );
}
