//! Rig detection: joint names and whether the skeleton is a Mixamo one.

use super::RigKind;

/// The name without a rig prefix such as `mixamorig:` or `mixamorig1_`.
pub fn bare_joint_name(name: &str) -> &str {
    let after = name.rsplit(':').next().unwrap_or(name);
    let lower = after.to_ascii_lowercase();
    if lower.starts_with("mixamorig")
        && let Some(pos) = after.find('_')
    {
        return &after[pos + 1..];
    }
    after
}

pub(super) const MIXAMO_CORE: [&str; 20] = [
    "Hips",
    "Spine",
    "Spine1",
    "Spine2",
    "Neck",
    "Head",
    "LeftShoulder",
    "LeftArm",
    "LeftForeArm",
    "LeftHand",
    "RightShoulder",
    "RightArm",
    "RightForeArm",
    "RightHand",
    "LeftUpLeg",
    "LeftLeg",
    "LeftFoot",
    "RightUpLeg",
    "RightLeg",
    "RightFoot",
];

pub(super) fn rig_kind(joints: &[String]) -> RigKind {
    if joints.is_empty() {
        return RigKind::None;
    }
    let found = MIXAMO_CORE
        .iter()
        .filter(|core| joints.iter().any(|j| bare_joint_name(j) == **core))
        .count();
    if found >= 18 {
        RigKind::Mixamo
    } else {
        RigKind::Other
    }
}

/// A hand or finger joint in Mixamo naming.
pub fn is_hand_joint(name: &str) -> bool {
    let bare = bare_joint_name(name);
    bare.starts_with("LeftHand") || bare.starts_with("RightHand")
}
