//! Which rig a clip was made on, read from its bone names: Mixamo, Tripo's
//! own naming, or anything else (custom).

use serde::{Deserialize, Serialize};

use crate::inbox::analyse::bare_joint_name;
use crate::motion::{BoneMatch, humanoid_map};

/// The rig a library clip was made on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SourceRig {
    Mixamo,
    Tripo,
    Custom,
}

/// Mixamo names a rig must use for its clips to count as Mixamo's, as the
/// Inbox counts them.
const MIXAMO_MIN: usize = 18;

/// Unverified fallback: bare bone names a Tripo rig might use instead of
/// the `mixamorig:` names of Tripo's Mixamo rig spec, with the side letter
/// as `{s}`: `L_Upperarm`, `R_Thigh`. No real rig with these names has been
/// checked; a Tripo job's rig is told apart by `from_tripo` instead.
pub(crate) const TRIPO_BONES: [&str; 14] = [
    "Hip",
    "Waist",
    "Spine01",
    "Spine02",
    "NeckTwist01",
    "Head",
    "{s}_Clavicle",
    "{s}_Upperarm",
    "{s}_Forearm",
    "{s}_Hand",
    "{s}_Thigh",
    "{s}_Calf",
    "{s}_Foot",
    "{s}_ToeBase",
];

/// Tripo names a rig must use, either side counted, to count as Tripo's.
const TRIPO_MIN: usize = 16;

/// How many of Tripo's own bone names `bones` use, both sides counted.
fn tripo_names(bones: &[String]) -> usize {
    let bare: Vec<String> = bones
        .iter()
        .map(|b| bare_joint_name(b).to_ascii_lowercase())
        .collect();
    TRIPO_BONES
        .iter()
        .flat_map(|name| {
            if name.contains("{s}") {
                vec![name.replace("{s}", "L"), name.replace("{s}", "R")]
            } else {
                vec![(*name).to_string()]
            }
        })
        .filter(|name| bare.contains(&name.to_ascii_lowercase()))
        .count()
}

/// The rig `bones` belong to. A rig that came from a Tripo job is Tripo's
/// whatever it names its bones, so callers that know pass `from_tripo`.
pub fn detect(bones: &[String], from_tripo: bool) -> SourceRig {
    if from_tripo || tripo_names(bones) >= TRIPO_MIN {
        return SourceRig::Tripo;
    }
    let mixamo = humanoid_map(bones)
        .iter()
        .filter(|m| m.how == BoneMatch::Exact)
        .count();
    if mixamo >= MIXAMO_MIN {
        SourceRig::Mixamo
    } else {
        SourceRig::Custom
    }
}

/// The prefix a rig puts on every bone, like `mixamorig:`, when it has one.
pub fn prefix(bones: &[String]) -> Option<String> {
    let first = bones.first()?;
    let (head, _) = first.split_once(':')?;
    let prefix = format!("{head}:");
    bones
        .iter()
        .all(|b| b.starts_with(&prefix))
        .then_some(prefix)
}
