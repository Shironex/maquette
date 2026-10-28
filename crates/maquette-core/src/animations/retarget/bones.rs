//! The bone map between a clip's skeleton and a model's rig. Both sides are
//! read onto the humanoid bones Motion's rig check uses (Mixamo names), so
//! a map is one model bone per humanoid bone and covers clips of every
//! source rig at once. Tripo rigs follow the Mixamo rig spec and use
//! `mixamorig:` names, which match exactly.

use serde::{Deserialize, Serialize};

use crate::inbox::analyse::bare_joint_name;
use crate::motion::{BoneMap, BoneMatch, humanoid_map};

/// Unverified fallback: bare names a Tripo rig might use instead of the
/// `mixamorig:` ones, for the humanoid bones Motion's map leaves out; the
/// rest (`Hip`, `L_Thigh`, `R_Upperarm`) already read by name. No real rig
/// with these names has been checked, so they only fill bones the Mixamo
/// names left missing.
const TRIPO: [(&str, &str); 4] = [
    ("Spine", "Waist"),
    ("Spine1", "Spine01"),
    ("Spine2", "Spine02"),
    ("Neck", "NeckTwist01"),
];

/// Humanoid bones a retarget leaves in their rest direction when it lines
/// the model up with the clip's rest pose: only the limbs are turned.
const KEEP_REST: [&str; 13] = [
    "Spine",
    "Spine1",
    "Spine2",
    "Neck",
    "Head",
    "LeftShoulder",
    "RightShoulder",
    "LeftHand",
    "RightHand",
    "LeftFoot",
    "RightFoot",
    "LeftToeBase",
    "RightToeBase",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PairMatch {
    /// The map saved for this model.
    Saved,
    /// The model uses the humanoid name itself.
    Exact,
    /// A name other rigs use for that bone.
    ByName,
    /// Picked by hand in this retarget.
    Picked,
    /// No model bone; the bone stays at rest.
    Missing,
}

/// One humanoid bone: the clip's bone and the model's bone it drives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BonePair {
    /// The humanoid (Mixamo) name.
    pub bone: String,
    /// The clip's bone, for the first clip's rig.
    pub source: Option<String>,
    /// The model's bone.
    pub target: Option<String>,
    pub how: PairMatch,
}

/// The model bone chosen for one humanoid bone; `None` leaves it at rest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MapChoice {
    pub bone: String,
    pub target: Option<String>,
}

/// The humanoid map of `bones`, Tripo's names included.
pub(crate) fn canonical(bones: &[String]) -> Vec<BoneMap> {
    let mut map = humanoid_map(bones);
    for entry in map.iter_mut().filter(|m| m.how == BoneMatch::Missing) {
        let Some((_, tripo)) = TRIPO.iter().find(|(b, _)| *b == entry.bone) else {
            continue;
        };
        if let Some(found) = bones
            .iter()
            .find(|b| bare_joint_name(b).eq_ignore_ascii_case(tripo))
        {
            entry.joint = Some(found.clone());
            entry.how = BoneMatch::ByName;
        }
    }
    map
}

fn bone_of(map: &[BoneMap], bone: &str) -> Option<String> {
    map.iter()
        .find(|m| m.bone == bone)
        .and_then(|m| m.joint.clone())
}

/// The suggested map from a clip's bones onto a model's: the saved map
/// where there is one, then the names. `picked` are this retarget's own
/// choices and win over both; a choice of a bone the model lacks is
/// dropped.
pub fn suggest(
    source: &[String],
    target: &[String],
    saved: &[MapChoice],
    picked: &[MapChoice],
) -> Vec<BonePair> {
    let from = canonical(source);
    let onto = canonical(target);
    let known = |b: &Option<String>| b.as_ref().is_none_or(|b| target.contains(b));
    onto.iter()
        .map(|m| {
            let chosen = |list: &[MapChoice]| {
                list.iter()
                    .find(|c| c.bone == m.bone && known(&c.target))
                    .map(|c| c.target.clone())
            };
            let (target, how) = if let Some(t) = chosen(picked) {
                (t, PairMatch::Picked)
            } else if let Some(t) = chosen(saved) {
                (t, PairMatch::Saved)
            } else {
                let how = match m.how {
                    BoneMatch::Exact => PairMatch::Exact,
                    BoneMatch::ByName => PairMatch::ByName,
                    BoneMatch::Missing => PairMatch::Missing,
                };
                (m.joint.clone(), how)
            };
            let how = if target.is_none() {
                PairMatch::Missing
            } else {
                how
            };
            BonePair {
                bone: m.bone.clone(),
                source: bone_of(&from, &m.bone),
                target,
                how,
            }
        })
        .collect()
}

/// Whether the map wants a look before baking: nothing is saved for the
/// model and a bone (toes aside) has no clip bone or no model bone.
pub fn needs_look(pairs: &[BonePair], saved: bool) -> bool {
    !saved
        && pairs
            .iter()
            .any(|p| !p.bone.ends_with("ToeBase") && (p.source.is_none() || p.target.is_none()))
}

/// The map as choices, to save for the model.
pub fn choices(pairs: &[BonePair]) -> Vec<MapChoice> {
    pairs
        .iter()
        .map(|p| MapChoice {
            bone: p.bone.clone(),
            target: p.target.clone(),
        })
        .collect()
}

/// Every clip bone onto its model bone, for one clip's skeleton.
pub(crate) fn pairs_for(source: &[String], map: &[BonePair]) -> Vec<(String, String)> {
    let from = canonical(source);
    map.iter()
        .filter_map(|p| Some((bone_of(&from, &p.bone)?, p.target.clone()?)))
        .collect()
}

/// The model bones a retarget keeps in their rest direction.
pub(crate) fn keep_rest(map: &[BonePair]) -> Vec<String> {
    map.iter()
        .filter(|p| KEEP_REST.contains(&p.bone.as_str()))
        .filter_map(|p| p.target.clone())
        .collect()
}
