//! The finding for a bone map the names leave unclear.

use super::{finding, plural};
use crate::vrm::bones::{HUMANOID_JOINTS_MIN, NamingReport};
use crate::vrm::{FindingLevel, VrmFinding};

/// When the names leave the bone map unclear, or bones were read from
/// descriptive names: the bones are for a person to assign or check.
pub(in crate::vrm) fn naming_finding(naming: &NamingReport) -> Option<VrmFinding> {
    if naming.confident && naming.descriptive == 0 {
        return None;
    }
    let mut parts = Vec::new();
    if naming.missing_required > 0 && naming.humanoid_joints >= HUMANOID_JOINTS_MIN {
        let matched = match naming.convention {
            Some(c) => format!(
                "only {} of the 15 required bones match {} names",
                naming.convention_required,
                c.label()
            ),
            None => "none of the 15 required bones match by name".to_string(),
        };
        parts.push(format!(
            "{} look like humanoid bones, but {matched}.",
            plural(naming.humanoid_joints as usize, "joint", "joints")
        ));
    }
    parts.extend(naming.ambiguous.iter().cloned());
    if naming.descriptive > 0 {
        parts.push(format!(
            "{} read from descriptive names such as Left elbow, which is a guess.",
            plural(naming.descriptive as usize, "bone was", "bones were")
        ));
    }
    let (title, ask) = if naming.confident {
        (
            format!(
                "{} matched by descriptive names",
                plural(naming.descriptive as usize, "bone", "bones")
            ),
            "Check them in the Bone map and set any that are wrong.",
        )
    } else {
        (
            "Bone names are unclear".to_string(),
            "Assign the bones in the Bone map, or have an agent set them with set_avatar. Nothing past this is guessed.",
        )
    };
    parts.push(ask.to_string());
    Some(finding(
        "bone-naming",
        FindingLevel::Warning,
        title,
        Some(parts.join(" ")),
    ))
}
