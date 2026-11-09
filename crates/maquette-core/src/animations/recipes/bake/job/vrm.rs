//! The model's bones by VRM name: the humanoid (Mixamo) name the bone map
//! lists each under, the limb a reach solves, every bone a bake moves, and
//! the left and right pairs a mirror swaps.

use std::collections::BTreeMap;

use crate::animations::retarget::{BonePair, Model};
use crate::error::{CoreError, CoreResult};
use crate::inbox::analyse::bare_joint_name;
use crate::recipes::{LayerKind, RecipeLayer};

/// VRM body bones with the humanoid (Mixamo) name the bone map lists each
/// under; `{side}` reads `left` or `right`, `{Side}` `Left` or `Right`.
const BODY: [(&str, &str); 14] = [
    ("hips", "Hips"),
    ("spine", "Spine"),
    ("chest", "Spine1"),
    ("upperChest", "Spine2"),
    ("neck", "Neck"),
    ("head", "Head"),
    ("{side}UpperLeg", "{Side}UpLeg"),
    ("{side}LowerLeg", "{Side}Leg"),
    ("{side}Foot", "{Side}Foot"),
    ("{side}Toes", "{Side}ToeBase"),
    ("{side}Shoulder", "{Side}Shoulder"),
    ("{side}UpperArm", "{Side}Arm"),
    ("{side}LowerArm", "{Side}ForeArm"),
    ("{side}Hand", "{Side}Hand"),
];

/// VRM fingers, their three segments, and Mixamo's finger name (joints 1 to 3).
const FINGERS: [(&str, [&str; 3], &str); 5] = [
    ("Thumb", ["Metacarpal", "Proximal", "Distal"], "Thumb"),
    ("Index", ["Proximal", "Intermediate", "Distal"], "Index"),
    ("Middle", ["Proximal", "Intermediate", "Distal"], "Middle"),
    ("Ring", ["Proximal", "Intermediate", "Distal"], "Ring"),
    ("Little", ["Proximal", "Intermediate", "Distal"], "Pinky"),
];

/// The bones a model's facing is read from, when it has them.
const FACING: [&str; 4] = [
    "leftUpperArm",
    "rightUpperArm",
    "leftUpperLeg",
    "rightUpperLeg",
];

/// The bones a held pose breathes with, when the model has them.
const BREATHE: [&str; 2] = ["chest", "spine"];

/// The humanoid (Mixamo) name of VRM bone `vrm`: `rightHand` reads
/// `RightHand`, `leftIndexProximal` reads `LeftHandIndex1`.
pub(crate) fn mixamo_name(vrm: &str) -> Option<String> {
    for (side, upper) in [("left", "Left"), ("right", "Right")] {
        for (pattern, mixamo) in BODY {
            if pattern.replace("{side}", side) == vrm {
                return Some(mixamo.replace("{Side}", upper));
            }
        }
        for (finger, segments, mixamo) in FINGERS {
            for (i, segment) in segments.iter().enumerate() {
                if format!("{side}{finger}{segment}") == vrm {
                    return Some(format!("{upper}Hand{mixamo}{}", i + 1));
                }
            }
        }
    }
    None
}

/// The upper and lower bones a reach of `end` solves.
pub(crate) fn limb_of(end: &str) -> Option<(&'static str, &'static str)> {
    match end {
        "leftHand" => Some(("leftUpperArm", "leftLowerArm")),
        "rightHand" => Some(("rightUpperArm", "rightLowerArm")),
        "leftFoot" => Some(("leftUpperLeg", "leftLowerLeg")),
        "rightFoot" => Some(("rightUpperLeg", "rightLowerLeg")),
        _ => None,
    }
}

/// The model bone for VRM bone `vrm`: the bone map's choice, else a joint
/// with the Mixamo name itself (fingers, which the map leaves out).
pub(crate) fn model_bone(vrm: &str, map: &[BonePair], joints: &[String]) -> Option<String> {
    let mixamo = mixamo_name(vrm)?;
    if let Some(pair) = map.iter().find(|p| p.bone == mixamo) {
        return pair.target.clone();
    }
    joints
        .iter()
        .find(|j| bare_joint_name(j).eq_ignore_ascii_case(&mixamo))
        .cloned()
}

/// Every bone the changes move, by VRM name, with the limbs a reach or pin solves
/// and the bones the facing is read from.
pub(super) fn bones_for(
    layers: &[RecipeLayer],
    map: &[BonePair],
    model: &Model,
) -> CoreResult<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    let mut missing = Vec::new();
    for layer in layers {
        let mut wanted = vec![layer.bone.as_str()];
        if matches!(layer.kind, LayerKind::Reach | LayerKind::Pin)
            && let Some((upper, lower)) = limb_of(&layer.bone)
        {
            wanted.extend([upper, lower]);
        }
        for vrm in wanted {
            match model_bone(vrm, map, &model.joints) {
                Some(bone) => {
                    out.insert(vrm.to_string(), bone);
                }
                None => missing.push(format!("{vrm} ({})", layer.id)),
            }
        }
    }
    if !missing.is_empty() {
        missing.dedup();
        return Err(CoreError::Invalid(format!(
            "{} has no bone for {}. Map it in Use on model's bone map, or take that change out.",
            model.name,
            missing.join(", ")
        )));
    }
    for vrm in FACING.iter().chain(&BREATHE) {
        if let Some(bone) = model_bone(vrm, map, &model.joints) {
            out.insert(vrm.to_string(), bone);
        }
    }
    Ok(out)
}

/// Every left and right pair of model bones the bone map (and the fingers'
/// own names) gives, left first, for a mirror to swap.
pub(crate) fn mirror_pairs(map: &[BonePair], joints: &[String]) -> Vec<[String; 2]> {
    let mut out = Vec::new();
    let mut add = |left: String, right: String| {
        if let (Some(l), Some(r)) = (
            model_bone(&left, map, joints),
            model_bone(&right, map, joints),
        ) && l != r
        {
            out.push([l, r]);
        }
    };
    for (pattern, _) in BODY {
        if pattern.contains("{side}") {
            add(
                pattern.replace("{side}", "left"),
                pattern.replace("{side}", "right"),
            );
        }
    }
    for (finger, segments, _) in FINGERS {
        for segment in segments {
            add(
                format!("left{finger}{segment}"),
                format!("right{finger}{segment}"),
            );
        }
    }
    out
}
