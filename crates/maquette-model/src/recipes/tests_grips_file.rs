//! The grip file: the bones a grip may pose, its mirror, every check a
//! file passes and reading one back as it was written.

use super::grips::{
    GripFile, GripHolds, GripSide, MAX_GRIP_FILE_BYTES, bone_side, check_weapon, grip_bones,
    hands_words, is_grip_bone, mirror_pose, other_side, weapon_group,
};
use super::tests_grips::{dagger, longsword, pose, relaxed};
use super::*;

#[test]
fn a_grip_poses_hands_fingers_and_arms_only() {
    for side in [GripSide::Left, GripSide::Right] {
        let bones = grip_bones(side);
        assert_eq!(bones.len(), 18, "an arm of 3 and 15 finger bones");
        for bone in &bones {
            assert!(is_grip_bone(bone), "{bone}");
            assert!(maquette_glb::vrm::is_human_bone(bone), "{bone} is VRM's");
            assert_eq!(bone_side(bone), Some(side));
            assert_eq!(other_side(&other_side(bone)), *bone);
        }
    }
    for bone in [
        "chest",
        "head",
        "leftShoulder",
        "rightFoot",
        "rightIndex",
        "hand",
    ] {
        assert!(!is_grip_bone(bone), "{bone}");
    }
    assert_eq!(other_side("leftHand"), "rightHand");
    assert_eq!(other_side("chest"), "chest");
}

#[test]
fn a_mirror_swaps_sides_and_turns_the_other_way_about_up_and_ahead() {
    let mirrored = mirror_pose(&dagger().pose);
    assert_eq!(mirrored["leftHand"], [0.0, -30.0, 0.0]);
    assert_eq!(mirrored["leftMiddleIntermediate"], [0.0, 0.0, -80.0]);
    assert_eq!(mirror_pose(&mirrored), dagger().pose, "twice is the pose");
    let zero = mirror_pose(&pose(&[("rightHand", [0.0, 0.0, 0.0])]));
    assert!(
        zero["leftHand"].iter().all(|v| v.to_bits() == 0),
        "no minus zero is written"
    );
}

#[test]
fn a_grip_file_is_checked_whole() {
    let refused = |grip: &GripFile, words: &str| {
        let err = grips::validate(&grips::normalized(grip)).expect_err(words);
        assert!(err.to_string().contains(words), "{err}");
    };
    let good = longsword();
    grips::validate(&good).expect("valid");
    refused(
        &GripFile {
            name: "  ".into(),
            ..good.clone()
        },
        "a name",
    );
    refused(
        &GripFile {
            name: "x".repeat(121),
            ..good.clone()
        },
        "under 120",
    );
    refused(
        &GripFile {
            revision: 0,
            ..good.clone()
        },
        "starts at 1",
    );
    refused(
        &GripFile {
            weapon: "long sword!".into(),
            ..good.clone()
        },
        "not a weapon",
    );
    refused(
        &GripFile {
            pose: BonePose::new(),
            ..good.clone()
        },
        "turns no bone",
    );
    let mut chest = good.clone();
    chest.pose.insert("chest".into(), [0.0, 5.0, 0.0]);
    refused(&chest, "not chest");
    let mut far = good.clone();
    far.pose.insert("rightHand".into(), [0.0, 181.0, 0.0]);
    refused(&far, "more than 180");
    let mut odd = good.clone();
    odd.pose.insert("rightHand".into(), [f32::NAN, 0.0, 0.0]);
    refused(&odd, "more than 180");

    // The hands always agree with the sides the pose turns.
    let one_hand = GripHolds {
        holds: GripSide::Right,
        supports: None,
    };
    refused(
        &GripFile {
            hands: one_hand,
            ..good.clone()
        },
        "poses both hands",
    );
    let both = GripHolds {
        holds: GripSide::Right,
        supports: Some(GripSide::Left),
    };
    refused(
        &GripFile {
            hands: both,
            ..dagger()
        },
        "left hand supports",
    );
    let wrong = GripHolds {
        holds: GripSide::Left,
        supports: None,
    };
    refused(
        &GripFile {
            hands: wrong,
            ..dagger()
        },
        "left hand holds",
    );
    let twice = GripHolds {
        holds: GripSide::Right,
        supports: Some(GripSide::Right),
    };
    refused(
        &GripFile {
            hands: twice,
            ..good.clone()
        },
        "right hand supports",
    );

    check_weapon("scythe").expect("known");
    let unknown = check_weapon("lightsaber").expect_err("unknown");
    assert!(
        unknown.to_string().contains("longsword, sword"),
        "{unknown}"
    );
    assert_eq!(
        weapon_group("lightsaber"),
        "other",
        "a newer weapon still lists"
    );
    assert_eq!(hands_words(&good), "both hands");
    assert_eq!(hands_words(&dagger()), "right hand");
    assert_eq!(hands_words(&relaxed()), "either hand");
}

#[test]
fn a_grip_file_reads_back_as_written_and_refuses_what_is_not_one() {
    let dir = tempfile::tempdir().expect("temp");
    let file = dir.path().join("r1.grip.json");
    let mut grip = longsword();
    grip.name = "  Longsword, two hands ".into();
    let text = grips::text(&grip).expect("text");
    assert!(
        text.ends_with("}\n") && text.contains("\"supports\": \"left\""),
        "{text}"
    );
    std::fs::write(&file, &text).expect("write");
    let read = grips::load(&file).expect("read");
    assert_eq!(read, longsword(), "trimmed on the way in");
    assert_eq!(grips::text(&read).expect("again"), text, "same bytes");
    let one = grips::text(&dagger()).expect("text");
    assert!(
        one.contains("\"supports\": null"),
        "one hand has no support: {one}"
    );

    let missing = grips::load(&dir.path().join("r9.grip.json"));
    assert!(matches!(missing, Err(CoreError::NotFound(_))));
    std::fs::write(&file, text.replace("\"version\": 1", "\"version\": 2")).expect("write");
    assert!(matches!(
        grips::load(&file),
        Err(CoreError::UnsupportedVersion {
            found: 2,
            supported: 1
        })
    ));
    let cap = usize::try_from(MAX_GRIP_FILE_BYTES).expect("cap");
    std::fs::write(&file, format!("{{\"name\": \"{}\"}}", "x".repeat(cap))).expect("write");
    let big = grips::load(&file).expect_err("too big");
    assert!(big.to_string().contains("too big to be a grip"), "{big}");
    std::fs::write(&file, text.replace("rightHand", "chest")).expect("write");
    assert!(matches!(grips::load(&file), Err(CoreError::Invalid(_))));
}
