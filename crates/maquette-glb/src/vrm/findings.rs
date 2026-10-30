//! The readiness findings: what stands between a rigged model and a VRM 1.0
//! file, in plain words, ranked blockers first and never left out.

use super::arm_weights::ArmDrag;
use super::bones::{HumanBone, Mapped, WrongParent, needs};
use super::pose::RestPose;
use super::{FindingLevel, VrmFinding};

mod naming;
mod skin;

pub(super) use naming::naming_finding;
pub(super) use skin::skin_findings;

/// The front may turn this far from +Z and still count as facing it.
pub const FACING_SLACK_DEG: f32 = 5.0;

/// An arm may sit this far from horizontal and still count as a T-pose.
pub const ARM_SLACK_DEG: f32 = 3.0;

/// The soles may sit this far from Y = 0, metres.
pub const GROUND_SLACK: f32 = 0.01;

/// The hips may sit this far from the Y axis, metres.
pub(super) const CENTRE_SLACK: f32 = 0.05;

/// Mirrored bones may differ by this share of the height.
pub(super) const SYMMETRY_SLACK: f32 = 0.02;

pub(super) fn finding(
    id: &str,
    level: FindingLevel,
    title: String,
    detail: Option<String>,
) -> VrmFinding {
    VrmFinding {
        id: id.to_string(),
        level,
        title,
        detail,
    }
}

pub(super) fn list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

pub(super) fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// What the rig has and lacks.
pub(super) struct RigFacts<'a> {
    pub(super) bones: &'a [HumanBone],
    pub(super) mapped: &'a [Mapped],
    pub(super) wrong: &'a [WrongParent],
    pub(super) skinned: bool,
    pub(super) animations: usize,
}

pub(super) fn bone_findings(rig: &RigFacts<'_>) -> Vec<VrmFinding> {
    let mut out = Vec::new();
    if !rig.skinned {
        out.push(finding(
            "skin",
            FindingLevel::Blocker,
            "No skinned mesh".to_string(),
            Some("A VRM is a rigged character. Rig it first.".to_string()),
        ));
    }
    let has = |bone: &str| rig.mapped.iter().any(|m| m.bone == bone);
    let missing: Vec<String> = rig
        .bones
        .iter()
        .filter(|b| b.required && !has(&b.name))
        .map(|b| b.name.clone())
        .collect();
    if !missing.is_empty() {
        out.push(finding(
            "required-bones",
            FindingLevel::Blocker,
            format!(
                "{} VRM needs not found",
                plural(missing.len(), "bone", "bones")
            ),
            Some(format!(
                "No joint maps to {}. Rename the joints, or pick them in the bone map.",
                list(&missing)
            )),
        ));
    }
    let gaps: Vec<String> = rig
        .bones
        .iter()
        .filter(|b| has(&b.name))
        .filter_map(|b| {
            needs(b)
                .filter(|p| !has(p))
                .map(|p| format!("{} without {p}", b.name))
        })
        .collect();
    if !gaps.is_empty() {
        out.push(finding(
            "bone-gaps",
            FindingLevel::Blocker,
            format!(
                "{} skip the bone above",
                plural(gaps.len(), "bone", "bones")
            ),
            Some(format!(
                "VRM needs each one's parent set too: {}.",
                list(&gaps)
            )),
        ));
    }
    if !rig.wrong.is_empty() {
        let lines: Vec<String> = rig
            .wrong
            .iter()
            .map(|w| {
                format!(
                    "{} sits under {}, not {}",
                    w.bone,
                    w.found.as_deref().unwrap_or("no humanoid bone"),
                    w.expected.as_deref().unwrap_or("nothing")
                )
            })
            .collect();
        out.push(finding(
            "parents",
            FindingLevel::Blocker,
            format!(
                "{} out of the humanoid order",
                plural(rig.wrong.len(), "bone is", "bones are")
            ),
            Some(format!("{}.", list(&lines))),
        ));
    }
    let mapped = rig.mapped.len();
    let fingers = rig
        .mapped
        .iter()
        .filter(|m| {
            ["Thumb", "Index", "Middle", "Ring", "Little"]
                .iter()
                .any(|f| m.bone.contains(f))
        })
        .count();
    let mut absent = Vec::new();
    if !has("leftEye") || !has("rightEye") {
        absent.push("eyes");
    }
    if !has("jaw") {
        absent.push("jaw");
    }
    if fingers == 0 {
        absent.push("fingers");
    }
    let mut detail = if fingers > 0 {
        format!("{fingers} finger bones among them.")
    } else {
        String::new()
    };
    if !absent.is_empty() {
        detail.push_str(&format!(" Not found, all optional: {}.", absent.join(", ")));
    }
    if absent.contains(&"eyes") {
        detail.push_str(" Without eye bones the look at stays off.");
    }
    out.push(finding(
        "bones",
        FindingLevel::Note,
        format!("{mapped} of {} humanoid bones mapped", rig.bones.len()),
        Some(detail.trim_start().to_string()),
    ));
    if rig.animations > 0 {
        out.push(finding(
            "animations",
            FindingLevel::Note,
            format!(
                "{} dropped on export",
                plural(rig.animations, "animation is", "animations are")
            ),
            Some("VRM carries no clips. Motion ships as VRMA files instead.".to_string()),
        ));
    }
    out
}

pub(super) fn pose_findings(pose: &RestPose) -> Vec<VrmFinding> {
    let mut out = Vec::new();
    match (pose.facing, pose.facing_deg) {
        (Some(axis), Some(deg)) if deg.abs() > FACING_SLACK_DEG => out.push(finding(
            "facing",
            FindingLevel::Blocker,
            format!("Faces {}, VRM wants +Z", axis.label()),
            Some(format!(
                "The front is turned {deg:.0} degrees from +Z. Turning it is part of normalising the rest pose."
            )),
        )),
        (None, _) => out.push(finding(
            "facing",
            FindingLevel::Blocker,
            "Facing unknown".to_string(),
            Some("Without both legs or both arms mapped the front cannot be read.".to_string()),
        )),
        _ => {}
    }
    if pose.sides_swapped {
        out.push(finding(
            "sides",
            FindingLevel::Blocker,
            "Left and right look swapped".to_string(),
            Some("The toes point backwards from the front the limbs give. Check the left and right names.".to_string()),
        ));
    }
    arm_findings(pose, &mut out);
    scale_findings(pose, &mut out);
    if let Some(y) = pose.ground_y.filter(|y| y.abs() > GROUND_SLACK) {
        out.push(finding(
            "ground",
            FindingLevel::Warning,
            format!(
                "Soles sit {:.0} cm {} the ground",
                y.abs() * 100.0,
                if y > 0.0 { "above" } else { "below" }
            ),
            Some("VRM stands the model on Y = 0.".to_string()),
        ));
    }
    if let Some(off) = pose.hips_offset.filter(|o| *o > CENTRE_SLACK) {
        let mut detail = "VRM centres the model on the Y axis.".to_string();
        if let (Some(side), Some(forward)) = (pose.hips_side, pose.hips_forward) {
            let parts: Vec<String> = [
                (forward, "forward", "back"),
                (side, "to its left", "to its right"),
            ]
            .iter()
            .filter(|(m, _, _)| m.abs() >= 0.005)
            .map(|(m, plus, minus)| offset_cm(*m, plus, minus))
            .collect();
            if !parts.is_empty() {
                detail.push_str(&format!(" Seen from the character: {}.", list(&parts)));
            }
        }
        out.push(finding(
            "centre",
            FindingLevel::Warning,
            format!("Hips sit {:.0} cm off the centre", off * 100.0),
            Some(detail),
        ));
    }
    if let (Some(gap), Some(height)) = (pose.asymmetry, pose.height)
        && gap > SYMMETRY_SLACK * height
    {
        let at = pose
            .asymmetry_bone
            .as_deref()
            .map(|b| format!(" The widest gap is at {b} and its right twin."))
            .unwrap_or_default();
        out.push(finding(
            "symmetry",
            FindingLevel::Warning,
            format!("Sides differ by up to {:.0} cm", gap * 100.0),
            Some(format!("VRM wants the rest pose mirrored across X. A mirrored joint sits that far from its pair, in the character's own frame.{at}")),
        ));
    }
    out
}

/// "26 cm forward", or "4 cm back" for a negative offset.
fn offset_cm(metres: f32, plus: &str, minus: &str) -> String {
    format!(
        "{:.0} cm {}",
        metres.abs() * 100.0,
        if metres < 0.0 { minus } else { plus }
    )
}

fn arm_findings(pose: &RestPose, out: &mut Vec<VrmFinding>) {
    let arms = [("left", pose.arm_left_deg), ("right", pose.arm_right_deg)];
    let off: Vec<String> = arms
        .iter()
        .filter_map(|(side, deg)| {
            deg.filter(|d| d.abs() > ARM_SLACK_DEG).map(|d| {
                format!(
                    "{side} {:.0} degrees {}",
                    d.abs(),
                    if d > 0.0 { "below" } else { "above" }
                )
            })
        })
        .collect();
    if !off.is_empty() {
        let down = arms
            .iter()
            .any(|(_, d)| d.is_some_and(|d| d > ARM_SLACK_DEG));
        out.push(finding(
            "arms",
            FindingLevel::Blocker,
            if down {
                "Arms hang below horizontal, VRM wants a T-pose".to_string()
            } else {
                "Arms are not horizontal, VRM wants a T-pose".to_string()
            },
            Some(format!(
                "Upper arm to hand: {}. Normalising the rest pose lifts them.",
                list(&off)
            )),
        ));
    }
}

fn scale_findings(pose: &RestPose, out: &mut Vec<VrmFinding>) {
    if !pose.negative_scale.is_empty() {
        out.push(finding(
            "negative-scale",
            FindingLevel::Blocker,
            format!(
                "{} mirrored by a negative scale",
                plural(pose.negative_scale.len(), "node is", "nodes are")
            ),
            Some(format!(
                "{}. Apply the transforms in Blender.",
                list(&pose.negative_scale)
            )),
        ));
    }
    if !pose.uneven_scale.is_empty() {
        out.push(finding(
            "uneven-scale",
            FindingLevel::Blocker,
            format!(
                "{} scaled unevenly",
                plural(pose.uneven_scale.len(), "node is", "nodes are")
            ),
            Some(format!(
                "{}. VRM needs the same scale on every axis and no shear. Apply the transforms in Blender.",
                list(&pose.uneven_scale)
            )),
        ));
    }
    if !pose.scaled.is_empty() {
        out.push(finding(
            "scaled",
            FindingLevel::Warning,
            format!(
                "{} scaled",
                plural(pose.scaled.len(), "node is", "nodes are")
            ),
            Some(format!(
                "{}. Allowed, but spring bones behave better with the scale applied.",
                list(&pose.scaled)
            )),
        ));
    }
    if pose.scale_noise > 0 {
        out.push(finding(
            "scale-noise",
            FindingLevel::Note,
            format!(
                "{} off 1 by float noise",
                plural(
                    pose.scale_noise as usize,
                    "node scale is",
                    "node scales are"
                )
            ),
            Some("The VRM writer snaps them to 1.".to_string()),
        ));
    }
}

/// Cloth or hair the arm chain moves at rest, far from the arm: lifting
/// the arms drags it along. `height` is the model's, metres, for the reach.
pub(super) fn arm_weight_finding(drag: &ArmDrag) -> VrmFinding {
    let bones: Vec<String> = drag
        .bones
        .iter()
        .take(3)
        .map(|(bone, n)| format!("{bone} ({n})"))
        .collect();
    finding(
        "arm-weights",
        FindingLevel::Warning,
        format!(
            "{} of cloth or hair weighted to the arms",
            plural(drag.vertices as usize, "vertex", "vertices")
        ),
        Some(format!(
            "They move with {} but sit more than twice the arm's own radius from it, like a cape or hair beside it. Lifting the arms to a T-pose drags them along. Free them first: a reweight keeps the arms, sleeves and gloves and moves the rest to the trunk, for a person to approve on before and after sheets.",
            list(&bones)
        )),
    )
}
