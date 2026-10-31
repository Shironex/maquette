//! The two skeletons at rest: what stance each stands in (a T pose, an A
//! pose), how far each limb turns to line up, the hips heights, and the
//! bones only on the model with what they move with. All read from the
//! files, so the bone map can show them before anything bakes.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::bones::BonePair;
use crate::animations::{AnimationClip, RestBone};
use crate::motion::rest_joints;

/// Arms within this of level stand in a T pose, degrees.
const T_POSE_DROP: f32 = 15.0;
/// Arms lower than this hang down rather than stand in an A pose, degrees.
const A_POSE_DROP: f32 = 75.0;

/// The humanoid bones with the one they point at.
const POINTS_AT: [(&str, &str); 16] = [
    ("Spine", "Spine1"),
    ("Spine1", "Spine2"),
    ("Spine2", "Neck"),
    ("Neck", "Head"),
    ("LeftShoulder", "LeftArm"),
    ("LeftArm", "LeftForeArm"),
    ("LeftForeArm", "LeftHand"),
    ("RightShoulder", "RightArm"),
    ("RightArm", "RightForeArm"),
    ("RightForeArm", "RightHand"),
    ("LeftUpLeg", "LeftLeg"),
    ("LeftLeg", "LeftFoot"),
    ("LeftFoot", "LeftToeBase"),
    ("RightUpLeg", "RightLeg"),
    ("RightLeg", "RightFoot"),
    ("RightFoot", "RightToeBase"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RestShape {
    TPose,
    APose,
    Other,
}

/// How far a limb turns to line the model up with the clip, degrees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BoneTurn {
    /// The humanoid name.
    pub bone: String,
    pub deg: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RestCompare {
    pub clip: Vec<RestBone>,
    pub model: Vec<RestBone>,
    pub clip_shape: RestShape,
    pub model_shape: RestShape,
    /// How far each one's arms drop from level, degrees.
    pub clip_arms_deg: f32,
    pub model_arms_deg: f32,
    /// Hips heights, metres, and what the hips' moves are scaled by.
    pub clip_hips: f32,
    pub model_hips: f32,
    pub hips_scale: f32,
    pub turns: Vec<BoneTurn>,
}

/// A bone only on the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OnlyBone {
    pub bone: String,
    /// The nearest driven bone above it, which it moves with on its own.
    pub carried_by: Option<String>,
    /// The driven bone chosen for it instead.
    pub follows: Option<String>,
    /// Whether it can be given another bone: not with a driven bone under it.
    pub free: bool,
}

/// What one bone only on the model follows; `None` goes back to its parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FollowChoice {
    pub bone: String,
    pub follows: Option<String>,
}

/// The model file's skeleton, empty when it cannot be read.
pub(crate) fn model_bones(path: &Path) -> Vec<RestBone> {
    rest_joints(path)
        .unwrap_or_default()
        .into_iter()
        .map(|j| RestBone {
            name: j.name,
            parent: j.parent,
            head: j.head,
            tail: j.tail,
        })
        .collect()
}

fn head_of(bones: &[RestBone], name: Option<&String>) -> Option<[f32; 3]> {
    let name = name?;
    bones.iter().find(|b| b.name == *name).map(|b| b.head)
}

fn direction(from: [f32; 3], to: [f32; 3]) -> Option<[f32; 3]> {
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    (len > 1e-5).then(|| [d[0] / len, d[1] / len, d[2] / len])
}

/// How far one upper arm drops from level, degrees.
fn drop_of(upper: Option<[f32; 3]>, lower: Option<[f32; 3]>) -> Option<f32> {
    let d = direction(upper?, lower?)?;
    Some((-d[1]).clamp(-1.0, 1.0).asin().to_degrees())
}

/// How far the arms drop, the mean of the sides that can be read.
fn arms_drop(sides: [Option<f32>; 2]) -> f32 {
    let read: Vec<f32> = sides.into_iter().flatten().collect();
    if read.is_empty() {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let count = read.len() as f32;
    read.iter().sum::<f32>() / count
}

fn shape(drop: f32) -> RestShape {
    if drop.abs() <= T_POSE_DROP {
        RestShape::TPose
    } else if drop > T_POSE_DROP && drop <= A_POSE_DROP {
        RestShape::APose
    } else {
        RestShape::Other
    }
}

fn pair_of<'a>(map: &'a [BonePair], bone: &str) -> Option<&'a BonePair> {
    map.iter().find(|p| p.bone == bone)
}

/// The clip's rig against the model's, or `None` without both to read.
pub(crate) fn compare(
    clip: &AnimationClip,
    model: &[RestBone],
    map: &[BonePair],
) -> Option<RestCompare> {
    if model.is_empty() || clip.rig.bones.is_empty() {
        return None;
    }
    let source = &clip.rig.bones;
    let clip_head = |b: &str| head_of(source, pair_of(map, b)?.source.as_ref());
    let model_head = |b: &str| head_of(model, pair_of(map, b)?.target.as_ref());
    let drops = |head: &dyn Fn(&str) -> Option<[f32; 3]>| {
        arms_drop([
            drop_of(head("LeftArm"), head("LeftForeArm")),
            drop_of(head("RightArm"), head("RightForeArm")),
        ])
    };
    let clip_arms_deg = drops(&clip_head);
    let model_arms_deg = drops(&model_head);
    let turns = POINTS_AT
        .iter()
        .filter_map(|(bone, next)| {
            let from = direction(clip_head(bone)?, clip_head(next)?)?;
            let to = direction(model_head(bone)?, model_head(next)?)?;
            let dot = from[0] * to[0] + from[1] * to[1] + from[2] * to[2];
            Some(BoneTurn {
                bone: (*bone).to_string(),
                deg: dot.clamp(-1.0, 1.0).acos().to_degrees(),
            })
        })
        .collect();
    let model_hips = model_head("Hips").map_or(0.0, |h| h[1]);
    let clip_hips = clip.rig.hips_height;
    Some(RestCompare {
        clip: source.clone(),
        model: model.to_vec(),
        clip_shape: shape(clip_arms_deg),
        model_shape: shape(model_arms_deg),
        clip_arms_deg,
        model_arms_deg,
        clip_hips,
        model_hips,
        hips_scale: if clip_hips > 1e-4 {
            model_hips / clip_hips
        } else {
            1.0
        },
        turns,
    })
}

fn parent_of<'a>(bones: &'a [RestBone], name: &str) -> Option<&'a String> {
    bones
        .iter()
        .find(|b| b.name == name)
        .and_then(|b| b.parent.as_ref())
}

fn is_under(bones: &[RestBone], bone: &str, above: &str) -> bool {
    let mut at = parent_of(bones, bone);
    while let Some(up) = at {
        if up == above {
            return true;
        }
        at = parent_of(bones, up);
    }
    false
}

fn driven(map: &[BonePair]) -> Vec<&String> {
    map.iter().filter_map(|p| p.target.as_ref()).collect()
}

/// The bones no humanoid bone maps to, with what each moves with.
pub(crate) fn only_bones(
    bones: &[RestBone],
    map: &[BonePair],
    saved: &[FollowChoice],
) -> Vec<OnlyBone> {
    let driven = driven(map);
    bones
        .iter()
        .filter(|b| !driven.contains(&&b.name))
        .map(|b| {
            let mut carried_by = None;
            let mut at = b.parent.as_ref();
            while let Some(up) = at {
                if driven.contains(&up) {
                    carried_by = Some(up.clone());
                    break;
                }
                at = parent_of(bones, up);
            }
            OnlyBone {
                bone: b.name.clone(),
                carried_by,
                follows: saved
                    .iter()
                    .find(|c| c.bone == b.name)
                    .and_then(|c| c.follows.clone()),
                free: !driven.iter().any(|d| is_under(bones, d, &b.name)),
            }
        })
        .collect()
}

/// The saved choices with this retarget's over them.
pub(crate) fn merged(saved: &[FollowChoice], picked: &[FollowChoice]) -> Vec<FollowChoice> {
    let mut out: Vec<FollowChoice> = saved
        .iter()
        .filter(|s| !picked.iter().any(|p| p.bone == s.bone))
        .cloned()
        .collect();
    out.extend(picked.iter().cloned());
    out.retain(|c| c.follows.is_some());
    out
}

/// The choices the bake can keep, as bone and lead. A driven bone cannot
/// follow, nor can a bone with a driven bone under it or one above its own
/// lead, and following the bone that carries it already is nothing to do.
pub(crate) fn follow_pairs(
    choices: &[FollowChoice],
    bones: &[RestBone],
    map: &[BonePair],
) -> Vec<(String, String)> {
    let only = only_bones(bones, map, choices);
    let driven = driven(map);
    only.into_iter()
        .filter_map(|o| {
            let lead = o.follows?;
            let keeps = o.free
                && driven.contains(&&lead)
                && o.carried_by.as_ref() != Some(&lead)
                && !is_under(bones, &lead, &o.bone);
            keeps.then_some((o.bone, lead))
        })
        .collect()
}

#[cfg(test)]
mod tests;
