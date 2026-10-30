//! The VRM 1.0 humanoid bones and how a rig's joints map onto them.
//!
//! Every bone has its VRM name, its parent in the humanoid tree, whether
//! VRM requires it, and the names four namings give it: VRM's own (and the
//! `J_Bip_` names of VRoid exports), Mixamo, Unreal and Tripo's own rig.
//! Tripo's names are an unverified guess, as in the animations' rig table:
//! no Tripo rig with them has been checked.
//!
//! Joints are compared by their bare name (no `mixamorig:` prefix), in
//! lower case with `.`, `-` and spaces read as `_`. The naming that matches
//! the most bones is tried first, so a mixed rig does not pull one bone from
//! another naming when its own has it. A joint maps to one bone at most.
//!
//! Bones no naming fills may then be read from descriptive names such as
//! `Left elbow` ([`descriptive`]), one joint per bone or none. What the
//! matching makes of the rig (the naming that won, the bones it left, two
//! joints for one bone or one joint for two) is the [`NamingReport`]: a
//! rig the names leave unclear is for a person to map, never a guess.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

mod descriptive;
mod table;

pub use table::{HUMAN_BONES, REQUIRED_BONES};
pub(crate) use table::{HumanBone, human_bones, needs, normal};

/// The naming a joint matched its humanoid bone by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum BoneNaming {
    /// VRM's own names (`leftUpperArm`) or VRoid's (`J_Bip_L_UpperArm`).
    Vrm,
    Mixamo,
    Unreal,
    /// Tripo's own rig names, unverified.
    Tripo,
    /// Read from a descriptive name (`Left elbow`, `ThumbProximal_L`): a
    /// guess, ranked below every exact naming.
    Descriptive,
    /// Set by hand in the avatar spec.
    Override,
}

impl BoneNaming {
    /// "Mixamo", the way the findings write it.
    pub fn label(self) -> &'static str {
        match self {
            BoneNaming::Vrm => "VRM",
            BoneNaming::Mixamo => "Mixamo",
            BoneNaming::Unreal => "Unreal",
            BoneNaming::Tripo => "Tripo",
            BoneNaming::Descriptive => "descriptive",
            BoneNaming::Override => "set by hand",
        }
    }
}

/// With fewer joints that look like humanoid bones than this, a rig that
/// misses required bones is just not a humanoid rig, not an unclear one.
pub const HUMANOID_JOINTS_MIN: u32 = 10;

/// What the name matching made of a rig.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NamingReport {
    /// The naming most bones matched by, overrides aside; `None` when no
    /// bone matched by name.
    pub convention: Option<BoneNaming>,
    /// Required bones that naming matched, of the 15.
    pub convention_required: u32,
    /// Required bones nothing maps that no override leaves out on purpose.
    pub missing_required: u32,
    /// Skin joints whose names read as a humanoid bone in any naming, or
    /// hold a body word such as arm or knee.
    pub humanoid_joints: u32,
    /// Bones mapped from descriptive names.
    pub descriptive: u32,
    /// Two joints that match one bone, or one joint that matches two, as
    /// sentences.
    pub ambiguous: Vec<String>,
    /// False when the names leave the map unclear: required bones missing
    /// from a rig with enough humanoid joints, or any ambiguity. The
    /// bones are then for a person to assign.
    pub confident: bool,
}

const NAMINGS: [BoneNaming; 4] = [
    BoneNaming::Vrm,
    BoneNaming::Mixamo,
    BoneNaming::Unreal,
    BoneNaming::Tripo,
];

/// A humanoid bone and the node it maps to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mapped {
    pub(crate) bone: String,
    pub(crate) node: usize,
    pub(crate) naming: BoneNaming,
}

/// Bone overrides from an avatar spec: a VRM bone name to the joint (node
/// name) it should map to, or `None` to leave it unmapped.
pub type BoneOverrides = BTreeMap<String, Option<String>>;

/// Maps `joints` (node index and name) onto the humanoid bones, in
/// [`human_bones`] order. An override wins over every naming, and may name
/// any node in `nodes`.
pub(crate) fn map_joints(
    bones: &[HumanBone],
    joints: &[(usize, String)],
    nodes: &[(usize, String)],
    overrides: &BoneOverrides,
) -> Vec<Mapped> {
    let normals: Vec<(usize, String)> = joints.iter().map(|(i, n)| (*i, normal(n))).collect();
    let order = naming_order(bones, &normals);
    let mut taken: Vec<usize> = Vec::new();
    let mut out = Vec::new();
    for bone in bones {
        if let Some(choice) = overrides.get(&bone.name) {
            let found = choice
                .as_ref()
                .and_then(|joint| nodes.iter().find(|(_, n)| n == joint));
            if let Some((node, _)) = found {
                taken.push(*node);
                out.push(Mapped {
                    bone: bone.name.clone(),
                    node: *node,
                    naming: BoneNaming::Override,
                });
            }
            continue;
        }
        let found = order.iter().find_map(|naming| {
            bone.names
                .iter()
                .filter(|(n, _)| n == naming)
                .find_map(|(_, want)| {
                    normals
                        .iter()
                        .find(|(i, n)| n == want && !taken.contains(i))
                        .map(|(i, _)| (*i, *naming))
                })
        });
        if let Some((node, naming)) = found {
            taken.push(node);
            out.push(Mapped {
                bone: bone.name.clone(),
                node,
                naming,
            });
        }
    }
    let guesses = descriptive::guesses(joints);
    for bone in bones {
        if overrides.contains_key(&bone.name) || out.iter().any(|m| m.bone == bone.name) {
            continue;
        }
        let found: Vec<usize> = guesses
            .iter()
            .filter(|(i, b)| *b == bone.name && !taken.contains(i))
            .map(|(i, _)| *i)
            .collect();
        if let [node] = found[..] {
            taken.push(node);
            out.push(Mapped {
                bone: bone.name.clone(),
                node,
                naming: BoneNaming::Descriptive,
            });
        }
    }
    let rank = |bone: &str| bones.iter().position(|b| b.name == bone);
    out.sort_by_key(|m| rank(&m.bone));
    out
}

/// "a, b and c".
fn and_list(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Reads what the matching made of a rig: the winning naming, what it
/// left, and every ambiguity among the exact names and, for bones no exact
/// name filled, the descriptive ones.
pub(crate) fn naming_report(
    bones: &[HumanBone],
    joints: &[(usize, String)],
    mapped: &[Mapped],
    overrides: &BoneOverrides,
) -> NamingReport {
    let normals: Vec<(usize, String)> = joints.iter().map(|(i, n)| (*i, normal(n))).collect();
    let name_of = |node: usize| {
        joints
            .iter()
            .find(|(i, _)| *i == node)
            .map_or("", |(_, n)| n.as_str())
    };
    let guesses = descriptive::guesses(joints);
    let exact = |bone: &HumanBone| -> Vec<usize> {
        normals
            .iter()
            .filter(|(_, n)| bone.names.iter().any(|(_, want)| want == n))
            .map(|(i, _)| *i)
            .collect()
    };
    let mut ambiguous = Vec::new();
    for bone in bones.iter().filter(|b| !overrides.contains_key(&b.name)) {
        let mut found = exact(bone);
        if found.is_empty() {
            found = guesses
                .iter()
                .filter(|(_, b)| *b == bone.name)
                .map(|(i, _)| *i)
                .collect();
        }
        if found.len() > 1 {
            let names: Vec<&str> = found.iter().map(|i| name_of(*i)).collect();
            ambiguous.push(format!(
                "{} joints match {}: {}.",
                found.len(),
                bone.name,
                and_list(&names)
            ));
        }
    }
    for (node, joint) in &normals {
        let matched: Vec<&str> = bones
            .iter()
            .filter(|b| !overrides.contains_key(&b.name))
            .filter(|b| b.names.iter().any(|(_, want)| want == joint))
            .map(|b| b.name.as_str())
            .collect();
        if matched.len() > 1 {
            ambiguous.push(format!(
                "{} matches {}.",
                name_of(*node),
                and_list(&matched)
            ));
        }
    }
    let mut by_naming: Vec<(usize, BoneNaming)> = NAMINGS
        .iter()
        .chain(std::iter::once(&BoneNaming::Descriptive))
        .map(|naming| {
            (
                mapped.iter().filter(|m| m.naming == *naming).count(),
                *naming,
            )
        })
        .filter(|(n, _)| *n > 0)
        .collect();
    // Stable: ties keep the exact namings first.
    by_naming.sort_by(|a, b| b.0.cmp(&a.0));
    let convention = by_naming.first().map(|(_, n)| *n);
    let required = |bone: &str| bones.iter().any(|b| b.required && b.name == bone);
    let convention_required = mapped
        .iter()
        .filter(|m| Some(m.naming) == convention && required(&m.bone))
        .count();
    let missing_required = bones
        .iter()
        .filter(|b| b.required && !overrides.contains_key(&b.name))
        .filter(|b| !mapped.iter().any(|m| m.bone == b.name))
        .count();
    let humanoid_joints = joints
        .iter()
        .zip(&normals)
        .filter(|((_, name), (_, n))| {
            bones
                .iter()
                .any(|b| b.names.iter().any(|(_, want)| want == n))
                || descriptive::looks_humanoid(name)
        })
        .count();
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let unclear = (missing_required > 0 && count(humanoid_joints) >= HUMANOID_JOINTS_MIN)
        || !ambiguous.is_empty();
    NamingReport {
        convention,
        convention_required: count(convention_required),
        missing_required: count(missing_required),
        humanoid_joints: count(humanoid_joints),
        descriptive: count(
            mapped
                .iter()
                .filter(|m| m.naming == BoneNaming::Descriptive)
                .count(),
        ),
        ambiguous,
        confident: !unclear,
    }
}

/// The namings by how many bones each matches, most first; ties keep
/// [`NAMINGS`] order.
fn naming_order(bones: &[HumanBone], normals: &[(usize, String)]) -> Vec<BoneNaming> {
    let mut counted: Vec<(usize, BoneNaming)> = NAMINGS
        .iter()
        .map(|naming| {
            let hits = bones
                .iter()
                .filter(|b| {
                    b.names
                        .iter()
                        .any(|(n, want)| n == naming && normals.iter().any(|(_, j)| j == want))
                })
                .count();
            (hits, *naming)
        })
        .collect();
    counted.sort_by(|a, b| b.0.cmp(&a.0));
    counted.into_iter().map(|(_, n)| n).collect()
}

/// A bone whose nearest humanoid ancestor in the node tree is not the one
/// VRM's tree asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WrongParent {
    pub(crate) bone: String,
    /// The bone VRM expects above it, `None` for none.
    pub(crate) expected: Option<String>,
    /// The bone the node tree has above it, `None` for none.
    pub(crate) found: Option<String>,
}

/// Checks each mapped bone's nearest mapped ancestor in the node tree
/// against the nearest mapped bone up VRM's tree. Nodes that are not
/// humanoid bones may sit between.
pub(crate) fn wrong_parents(
    bones: &[HumanBone],
    mapped: &[Mapped],
    parents: &[Option<usize>],
) -> Vec<WrongParent> {
    let by_bone: BTreeMap<&str, usize> = mapped.iter().map(|m| (m.bone.as_str(), m.node)).collect();
    let by_node: BTreeMap<usize, &str> = mapped.iter().map(|m| (m.node, m.bone.as_str())).collect();
    let parent_of: BTreeMap<&str, Option<&str>> = bones
        .iter()
        .map(|b| (b.name.as_str(), b.parent.as_deref()))
        .collect();
    let mut out = Vec::new();
    for m in mapped {
        let mut expected = parent_of.get(m.bone.as_str()).copied().flatten();
        while let Some(bone) = expected {
            if by_bone.contains_key(bone) {
                break;
            }
            expected = parent_of.get(bone).copied().flatten();
        }
        let mut found = None;
        let mut at = parents.get(m.node).copied().flatten();
        let mut steps = 0;
        while let Some(node) = at {
            if let Some(bone) = by_node.get(&node) {
                found = Some(*bone);
                break;
            }
            steps += 1;
            if steps > parents.len() {
                break;
            }
            at = parents.get(node).copied().flatten();
        }
        if expected != found {
            out.push(WrongParent {
                bone: m.bone.clone(),
                expected: expected.map(str::to_string),
                found: found.map(str::to_string),
            });
        }
    }
    out
}
