//! The finger curl measure, its findings, and the arm weights finding once
//! a reweight freed the arms.

use std::path::Path;

use super::*;
use crate::fixture::{self, HumanoidSpec};

fn read(spec: &HumanoidSpec, overrides: &BoneOverrides) -> VrmReadiness {
    readiness(&fixture::humanoid(spec), Path::new("."), overrides).expect("readiness")
}

fn find<'a>(r: &'a VrmReadiness, id: &str) -> Option<&'a VrmFinding> {
    r.findings.iter().find(|f| f.id == id)
}

#[test]
fn straight_fingers_measure_flat_and_find_nothing() {
    let r = read(&HumanoidSpec::default(), &BoneOverrides::new());
    assert_eq!(r.pose.fingers.len(), 10, "{:?}", r.pose.fingers);
    assert!(
        r.pose.finger_curl_deg.is_some_and(|d| d < 0.5),
        "{:?}",
        r.pose
    );
    assert!(r.pose.bunched_hands.is_empty());
    assert!(find(&r, "fingers").is_none());
    assert!(find(&r, "finger-bones").is_none());
}

#[test]
fn curled_fingers_are_a_warning_with_the_worst_named() {
    // 20 degrees at the knuckle and at each of the next two joints.
    for arm_drop in [0.0, 50.0] {
        let r = read(
            &HumanoidSpec {
                finger_curl: 20.0,
                arm_drop,
                ..HumanoidSpec::default()
            },
            &BoneOverrides::new(),
        );
        let index = r
            .pose
            .fingers
            .iter()
            .find(|f| f.finger == "leftIndex")
            .expect("left index");
        assert!((index.curl_deg - 60.0).abs() < 0.5, "{index:?}");
        let thumb = r
            .pose
            .fingers
            .iter()
            .find(|f| f.finger == "rightThumb")
            .expect("right thumb");
        assert!(thumb.curl_deg < 0.5, "{thumb:?}");
        let found = find(&r, "fingers").expect("fingers finding");
        assert_eq!(found.level, FindingLevel::Warning);
        assert_eq!(
            found.title,
            "Fingers curl up to 60 degrees, VRM wants them straight"
        );
        let detail = found.detail.as_deref().unwrap_or_default();
        assert!(detail.contains("(60)"), "{detail}");
        assert!(detail.contains("45 degrees"), "{detail}");
        // Never a blocker: a level rig with curled fingers is still ready.
        assert_eq!(r.ready, arm_drop == 0.0, "{:#?}", r.findings);
    }
}

#[test]
fn a_hand_with_its_knuckles_at_the_wrist_is_bunched() {
    let mut overrides = BoneOverrides::new();
    for finger in ["Index", "Middle", "Ring", "Little"] {
        overrides.insert(
            format!("right{finger}Proximal"),
            Some("mixamorig:RightHand".to_string()),
        );
    }
    let r = read(&HumanoidSpec::default(), &overrides);
    assert_eq!(r.pose.bunched_hands, vec!["right".to_string()]);
    assert!(r.pose.fingers.iter().all(|f| f.finger.starts_with("left")));
    let found = find(&r, "finger-bones").expect("finger bones finding");
    assert_eq!(found.level, FindingLevel::Warning);
    assert_eq!(
        found.title,
        "The right hand's finger bones sit bunched at the wrist"
    );
}

#[test]
fn the_bunched_hand_fixture_reads_as_bunched() {
    let r = read(
        &HumanoidSpec {
            arm_drop: 55.0,
            yaw: 90.0,
            bunched_hand: true,
            ..HumanoidSpec::default()
        },
        &BoneOverrides::new(),
    );
    assert_eq!(r.pose.bunched_hands, vec!["right".to_string()]);
    assert!(find(&r, "finger-bones").is_some());
}

#[test]
fn after_a_reweight_what_is_left_on_the_arms_is_a_note() {
    let bytes = fixture::humanoid(&HumanoidSpec {
        cape: true,
        arm_drop: 45.0,
        ..HumanoidSpec::default()
    });
    let none = BoneOverrides::new();
    let fresh = readiness(&bytes, Path::new("."), &none).expect("readiness");
    let freed = readiness_after_reweight(&bytes, Path::new("."), &none).expect("readiness");
    let before = find(&fresh, "arm-weights").expect("arm weights");
    assert_eq!(before.level, FindingLevel::Warning);
    assert!(
        before
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("Free them first")
    );
    let after = find(&freed, "arm-weights").expect("arm weights");
    assert_eq!(after.level, FindingLevel::Note);
    assert_eq!(
        after.title,
        "24 vertices still follow the arms after the reweight, cloth sewn to the sleeves"
    );
    let detail = after.detail.as_deref().unwrap_or_default();
    assert!(detail.contains("freed the arms"), "{detail}");
    assert!(!detail.contains("Free them first"), "{detail}");
    // Everything else reads the same.
    assert_eq!(fresh.pose, freed.pose);
    assert_eq!(fresh.findings.len(), freed.findings.len());
}
