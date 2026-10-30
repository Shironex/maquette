use std::path::Path;

use super::*;
use crate::fixture::{self, HumanoidSpec, SkinFlaw, WeightFormat};

fn read(spec: &HumanoidSpec) -> VrmReadiness {
    readiness(
        &fixture::humanoid(spec),
        Path::new("."),
        &BoneOverrides::new(),
    )
    .expect("readiness")
}

fn flawed(flaw: SkinFlaw, split: bool) -> VrmReadiness {
    read(&HumanoidSpec {
        flaw,
        split,
        ..HumanoidSpec::default()
    })
}

fn ids(r: &VrmReadiness, level: FindingLevel) -> Vec<&str> {
    r.findings
        .iter()
        .filter(|f| f.level == level)
        .map(|f| f.id.as_str())
        .collect()
}

fn finding<'a>(r: &'a VrmReadiness, id: &str) -> &'a VrmFinding {
    r.findings
        .iter()
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("no {id} in {:#?}", r.findings))
}

fn detail<'a>(r: &'a VrmReadiness, id: &str) -> &'a str {
    finding(r, id).detail.as_deref().unwrap_or_default()
}

const SKIN_IDS: [&str; 9] = [
    "node-cycle",
    "skin-joints-twice",
    "skin-joint-index",
    "skin-unweighted",
    "skin-weight-sum",
    "skin-influences",
    "short-bones",
    "side-topology",
    "skin-unread",
];

#[test]
fn a_clean_skin_passes_in_every_weight_format_and_split() {
    for weights in [WeightFormat::Float, WeightFormat::U8, WeightFormat::U16] {
        for split in [false, true] {
            let r = read(&HumanoidSpec {
                weights,
                split,
                ..HumanoidSpec::default()
            });
            assert!(r.ready, "{weights:?} {split}: {:#?}", r.findings);
            for id in SKIN_IDS {
                assert!(
                    !r.findings.iter().any(|f| f.id == id),
                    "{weights:?} {split}: {id}"
                );
            }
        }
    }
}

#[test]
fn bad_joint_indices_zero_and_off_weights_block_in_any_mesh() {
    for split in [false, true] {
        let mesh = if split { "Body 2" } else { "Body" };
        let past = flawed(SkinFlaw::JointPastEnd, split);
        assert!(!past.ready);
        let f = finding(&past, "skin-joint-index");
        assert_eq!(f.level, FindingLevel::Blocker);
        assert_eq!(f.title, "24 vertices use a joint the skin does not have");
        assert!(
            detail(&past, "skin-joint-index").starts_with(&format!("In {mesh} (24).")),
            "{f:?}"
        );

        let zero = flawed(SkinFlaw::Unweighted, split);
        assert_eq!(
            finding(&zero, "skin-unweighted").title,
            "24 vertices carry no weight"
        );
        assert!(ids(&zero, FindingLevel::Blocker).contains(&"skin-unweighted"));

        let off = flawed(SkinFlaw::OffSum, split);
        let f = finding(&off, "skin-weight-sum");
        assert_eq!(f.level, FindingLevel::Blocker);
        assert_eq!(f.title, "24 vertices have weights that do not add up to 1");
        assert!(detail(&off, "skin-weight-sum").contains("0.01"));
    }
}

#[test]
fn normalised_integer_weights_within_rounding_pass() {
    // Rounded to 8 bits the five weights read 1.008 together: a
    // normaliser's rounding, not a flaw.
    for weights in [WeightFormat::U8, WeightFormat::U16] {
        let r = read(&HumanoidSpec {
            weights,
            flaw: SkinFlaw::FiveInfluences,
            ..HumanoidSpec::default()
        });
        assert!(
            !r.findings.iter().any(|f| f.id == "skin-weight-sum"),
            "{weights:?}: {:#?}",
            r.findings
        );
    }
}

#[test]
fn five_influences_over_two_sets_are_a_warning() {
    for weights in [WeightFormat::Float, WeightFormat::U8] {
        let r = read(&HumanoidSpec {
            weights,
            flaw: SkinFlaw::FiveInfluences,
            ..HumanoidSpec::default()
        });
        let f = finding(&r, "skin-influences");
        assert_eq!(f.level, FindingLevel::Warning);
        assert_eq!(f.title, "24 vertices have more than 4 influences");
        assert!(r.ready, "a warning does not block: {:#?}", r.findings);
    }
}

#[test]
fn a_joint_twice_and_a_loop_in_the_tree_block() {
    let twice = flawed(SkinFlaw::JointTwice, false);
    let f = finding(&twice, "skin-joints-twice");
    assert_eq!(f.level, FindingLevel::Blocker);
    assert_eq!(f.title, "Armature lists 1 joint twice");
    assert!(detail(&twice, "skin-joints-twice").starts_with("mixamorig:Hips."));

    let looped = flawed(SkinFlaw::Cycle, false);
    let f = finding(&looped, "node-cycle");
    assert_eq!(f.level, FindingLevel::Blocker);
    assert_eq!(f.title, "The node tree loops");
    assert!(
        detail(&looped, "node-cycle").starts_with(
            "mixamorig:LeftHandThumb2, mixamorig:LeftHandThumb3 and mixamorig:LeftHandThumb4 sit in a loop"
        ),
        "{f:?}"
    );
}

#[test]
fn a_sheared_armature_is_an_uneven_scale() {
    let r = flawed(SkinFlaw::Shear, false);
    assert_eq!(r.pose.uneven_scale, ["Armature (sheared)"]);
    assert!(ids(&r, FindingLevel::Blocker).contains(&"uneven-scale"));
    assert!(detail(&r, "uneven-scale").contains("no shear"));
}

#[test]
fn a_hand_on_its_forearm_joint_is_a_short_bone() {
    let r = flawed(SkinFlaw::ShortForearm, false);
    let f = finding(&r, "short-bones");
    assert_eq!(f.level, FindingLevel::Warning);
    assert_eq!(f.title, "1 limb bone is too short to bend by");
    assert!(
        detail(&r, "short-bones").starts_with("leftLowerArm (0.0 cm)"),
        "{f:?}"
    );
}

#[test]
fn a_finger_on_one_side_only_is_a_topology_warning() {
    let r = flawed(SkinFlaw::OneSidedFinger, false);
    let f = finding(&r, "side-topology");
    assert_eq!(f.level, FindingLevel::Warning);
    assert_eq!(f.title, "Left and right limbs differ");
    let d = detail(&r, "side-topology");
    assert!(
        d.contains("The left hand has 15 finger bones mapped, the right 12."),
        "{d}"
    );
    assert!(
        d.contains(
            "Mapped on one side only: leftLittleProximal, leftLittleIntermediate and leftLittleDistal."
        ),
        "{d}"
    );
    // The joints are all there: only the names differ.
    assert!(!d.contains("Under the hands"), "{d}");
}

#[test]
fn a_flaw_in_the_skin_stops_the_writer() {
    let bytes = fixture::humanoid(&HumanoidSpec {
        flaw: SkinFlaw::OffSum,
        split: true,
        ..HumanoidSpec::default()
    });
    let spec = WriteSpec {
        meta: VrmMeta {
            name: "Box".to_string(),
            authors: vec!["Tester".to_string()],
            ..VrmMeta::default()
        },
        ..WriteSpec::default()
    };
    let err = write(&bytes, &spec, Path::new("."))
        .expect_err("refused")
        .to_string();
    assert!(
        err.contains("24 vertices have weights that do not add up to 1"),
        "{err}"
    );
    // A warning alone writes.
    let warned = fixture::humanoid(&HumanoidSpec {
        flaw: SkinFlaw::FiveInfluences,
        ..HumanoidSpec::default()
    });
    let written = write(&warned, &spec, Path::new(".")).expect("write");
    assert!(written.unresolved.iter().any(|f| f.id == "skin-influences"));
}
