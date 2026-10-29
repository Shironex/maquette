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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
    /// Set by hand in the avatar spec.
    Override,
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
    out
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
