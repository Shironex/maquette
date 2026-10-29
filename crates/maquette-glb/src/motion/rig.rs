//! The rig check: how many bones, whether they map onto a humanoid
//! skeleton (the Mixamo names most clip libraries use), which bones such a
//! clip would miss, and how many bones weigh on one vertex.

use serde::{Deserialize, Serialize};

use crate::analyse::{RigKind, bare_joint_name};

/// The bones a humanoid clip drives, in Mixamo names, with the names other
/// rigs give them (Unreal and Rigify style; `{s}` is the side letter).
const HUMANOID: [(&str, &[&str]); 22] = [
    ("Hips", &["hips", "pelvis", "hip"]),
    ("Spine", &["spine_01", "spine", "spine_001"]),
    ("Spine1", &["spine_02", "chest", "spine_002"]),
    (
        "Spine2",
        &["spine_03", "upper_chest", "upperchest", "spine_003"],
    ),
    ("Neck", &["neck", "neck_01"]),
    ("Head", &["head"]),
    ("LeftShoulder", &["clavicle_{s}", "shoulder_{s}"]),
    ("LeftArm", &["upperarm_{s}", "upper_arm_{s}", "arm_{s}"]),
    (
        "LeftForeArm",
        &["lowerarm_{s}", "forearm_{s}", "lower_arm_{s}"],
    ),
    ("LeftHand", &["hand_{s}"]),
    ("RightShoulder", &["clavicle_{s}", "shoulder_{s}"]),
    ("RightArm", &["upperarm_{s}", "upper_arm_{s}", "arm_{s}"]),
    (
        "RightForeArm",
        &["lowerarm_{s}", "forearm_{s}", "lower_arm_{s}"],
    ),
    ("RightHand", &["hand_{s}"]),
    (
        "LeftUpLeg",
        &["thigh_{s}", "upperleg_{s}", "upper_leg_{s}", "upleg_{s}"],
    ),
    (
        "LeftLeg",
        &[
            "calf_{s}",
            "shin_{s}",
            "lowerleg_{s}",
            "lower_leg_{s}",
            "leg_{s}",
        ],
    ),
    ("LeftFoot", &["foot_{s}"]),
    (
        "LeftToeBase",
        &["ball_{s}", "toe_{s}", "toes_{s}", "toebase_{s}"],
    ),
    (
        "RightUpLeg",
        &["thigh_{s}", "upperleg_{s}", "upper_leg_{s}", "upleg_{s}"],
    ),
    (
        "RightLeg",
        &[
            "calf_{s}",
            "shin_{s}",
            "lowerleg_{s}",
            "lower_leg_{s}",
            "leg_{s}",
        ],
    ),
    ("RightFoot", &["foot_{s}"]),
    (
        "RightToeBase",
        &["ball_{s}", "toe_{s}", "toes_{s}", "toebase_{s}"],
    ),
];

/// A rig is humanoid when this many of the bones above (toes aside) map.
const HUMANOID_MIN: usize = 16;

/// Bones a vertex may weigh on before a web page has to drop some: three.js
/// and most engines skin with four.
pub const WEB_INFLUENCES: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum BoneMatch {
    /// The rig uses the humanoid name itself.
    Exact,
    /// A name other rigs use for that bone.
    ByName,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BoneMap {
    /// The humanoid (Mixamo) name.
    pub bone: String,
    /// The rig's bone it maps to.
    pub joint: Option<String>,
    pub how: BoneMatch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Rig {
    pub joints: u32,
    pub joint_names: Vec<String>,
    pub kind: RigKind,
    /// Enough bones map onto the humanoid skeleton for its clips to play.
    pub humanoid: bool,
    /// Every humanoid bone and what it maps to, for the bone map.
    pub map: Vec<BoneMap>,
    /// Most bones one vertex weighs on; `None` when the weights cannot be
    /// read (compressed geometry).
    pub max_influences: Option<u32>,
}

/// `upper_arm.L` and `Upper-Arm_L` both read `upper_arm_l`.
fn normal(name: &str) -> String {
    bare_joint_name(name)
        .chars()
        .map(|c| match c {
            '.' | '-' | ' ' => '_',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// The spellings of an alias on one side: `hand_l`, `l_hand`, `left_hand`.
fn spellings(alias: &str, side: Option<char>) -> Vec<String> {
    let Some(s) = side else {
        return vec![alias.to_string()];
    };
    let word = alias.replace("_{s}", "");
    let long = if s == 'l' { "left" } else { "right" };
    vec![
        alias.replace("{s}", &s.to_string()),
        format!("{s}_{word}"),
        format!("{long}_{word}"),
        format!("{word}_{long}"),
        format!("{long}{word}"),
    ]
}

/// Maps the rig's joints onto the humanoid bones.
pub fn humanoid_map(joints: &[String]) -> Vec<BoneMap> {
    let normals: Vec<String> = joints.iter().map(|j| normal(j)).collect();
    HUMANOID
        .iter()
        .map(|(bone, aliases)| {
            if let Some(j) = joints.iter().find(|j| bare_joint_name(j) == *bone) {
                return BoneMap {
                    bone: (*bone).to_string(),
                    joint: Some(j.clone()),
                    how: BoneMatch::Exact,
                };
            }
            let side = if bone.starts_with("Left") {
                Some('l')
            } else if bone.starts_with("Right") {
                Some('r')
            } else {
                None
            };
            let found = aliases
                .iter()
                .flat_map(|a| spellings(a, side))
                .find_map(|spelling| normals.iter().position(|n| *n == spelling));
            BoneMap {
                bone: (*bone).to_string(),
                joint: found.map(|i| joints[i].clone()),
                how: if found.is_some() {
                    BoneMatch::ByName
                } else {
                    BoneMatch::Missing
                },
            }
        })
        .collect()
}

pub(super) fn is_humanoid(map: &[BoneMap]) -> bool {
    map.iter()
        .filter(|m| !m.bone.ends_with("ToeBase") && m.how != BoneMatch::Missing)
        .count()
        >= HUMANOID_MIN
}

/// The most joints any vertex weighs on, over every skinned primitive.
pub(super) fn max_influences(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    readable: bool,
) -> Option<u32> {
    if !readable {
        return None;
    }
    let mut most = 0u32;
    for node in doc.nodes().filter(|n| n.skin().is_some()) {
        let Some(mesh) = node.mesh() else { continue };
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| buffers.get(b.index()).and_then(Option::as_deref));
            let sets: Vec<Vec<[f32; 4]>> = (0..)
                .map_while(|set| reader.read_weights(set).map(|w| w.into_f32().collect()))
                .collect();
            let Some(first) = sets.first() else { continue };
            for v in 0..first.len() {
                let n = sets
                    .iter()
                    .filter_map(|s| s.get(v))
                    .flat_map(|w| w.iter())
                    .filter(|w| **w > 1e-4)
                    .count();
                most = most.max(u32::try_from(n).unwrap_or(u32::MAX));
            }
        }
    }
    Some(most)
}
