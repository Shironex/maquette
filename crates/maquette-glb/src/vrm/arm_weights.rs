//! Cloth or hair weighted to the arms: vertices the arm chain moves at rest
//! that sit far from the arm, such as a cape, a sleeve's long hem or a
//! braid. Lifting the arms to a T-pose drags them along.
//!
//! Everything is read in each skinned mesh's own (bind) space: the vertex
//! positions as stored, and each joint where its inverse bind matrix puts
//! it. Distances are measured against the arm's segments (upper arm to
//! lower arm to hand, hand to each finger) and compared with the height of
//! the skinned meshes, so the scale of the export does not matter.

use std::collections::BTreeMap;

use super::bones::{HumanBone, Mapped};
use crate::math::{self, Aabb, IDENTITY, Mat4, Vec3};

/// A vertex counts when the arm chain holds at least this much of its
/// weight.
pub(super) const ARM_WEIGHT_MIN: f32 = 0.25;

/// ...and it sits further than this share of the height from the arm.
/// Sleeves on a real rigged character stay within 0.05.
pub(super) const ARM_REACH: f32 = 0.08;

/// The finding needs at least this share of the skinned vertices...
pub(super) const ARM_SHARE: f32 = 0.005;

/// ...and at least this many vertices.
pub(super) const ARM_MIN_VERTICES: u32 = 16;

const ARM_PARTS: [&str; 8] = [
    "UpperArm", "LowerArm", "Hand", "Thumb", "Index", "Middle", "Ring", "Little",
];

/// True for the upper arm, lower arm, hand and finger bones of either side.
fn is_arm_bone(bone: &str) -> bool {
    let Some(rest) = bone
        .strip_prefix("left")
        .or_else(|| bone.strip_prefix("right"))
    else {
        return false;
    };
    ARM_PARTS.iter().any(|p| rest.starts_with(p))
}

fn side_of(bone: &str) -> &'static str {
    if bone.starts_with("left") {
        "left"
    } else {
        "right"
    }
}

/// What the check found over every skinned mesh.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ArmDrag {
    /// Vertices with arm weight that sit far from the arm.
    pub(crate) vertices: u32,
    /// Every skinned vertex read.
    pub(crate) skinned: u32,
    /// The arm bones that hold them most, with how many each, most first.
    pub(crate) bones: Vec<(String, u32)>,
}

impl ArmDrag {
    /// Enough of them to warn about.
    pub(crate) fn matters(&self) -> bool {
        #[expect(clippy::cast_precision_loss, reason = "vertex counts are small")]
        let share = self.skinned as f32 * ARM_SHARE;
        self.vertices >= ARM_MIN_VERTICES && self.vertices as f32 >= share
    }
}

/// One skinned primitive's vertices, ready to measure.
struct Skinned {
    positions: Vec<Vec3>,
    joints: Vec<[u16; 4]>,
    weights: Vec<[f32; 4]>,
    /// Skin joint index to its arm bone.
    arm: BTreeMap<usize, String>,
    /// Arm bone to where it sits in this mesh's bind space.
    at: BTreeMap<String, Vec3>,
}

/// Where each skin joint sits in its mesh's bind space.
fn bind_positions(
    skin: &gltf::Skin<'_>,
    buffers: &[Option<Vec<u8>>],
    worlds: &[Mat4],
) -> Vec<Vec3> {
    let ibms: Vec<Mat4> = skin
        .reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()))
        .read_inverse_bind_matrices()
        .map(Iterator::collect)
        .unwrap_or_default();
    skin.joints()
        .enumerate()
        .map(|(i, joint)| {
            ibms.get(i)
                .and_then(math::invert_affine)
                .map(|m| math::translation(&m))
                .unwrap_or_else(|| {
                    math::translation(&worlds.get(joint.index()).copied().unwrap_or(IDENTITY))
                })
        })
        .collect()
}

fn read_skinned(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    worlds: &[Mat4],
    mapped: &[Mapped],
) -> Vec<Skinned> {
    let by_node: BTreeMap<usize, &str> = mapped
        .iter()
        .filter(|m| is_arm_bone(&m.bone))
        .map(|m| (m.node, m.bone.as_str()))
        .collect();
    let mut out = Vec::new();
    for node in doc.nodes() {
        let (Some(mesh), Some(skin)) = (node.mesh(), node.skin()) else {
            continue;
        };
        let bind = bind_positions(&skin, buffers, worlds);
        let mut arm = BTreeMap::new();
        let mut at = BTreeMap::new();
        for (i, joint) in skin.joints().enumerate() {
            if let (Some(bone), Some(p)) = (by_node.get(&joint.index()), bind.get(i)) {
                arm.insert(i, (*bone).to_string());
                at.insert((*bone).to_string(), *p);
            }
        }
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()));
            let (Some(positions), Some(joints), Some(weights)) = (
                reader.read_positions(),
                reader.read_joints(0),
                reader.read_weights(0),
            ) else {
                continue;
            };
            out.push(Skinned {
                positions: positions.collect(),
                joints: joints.into_u16().collect(),
                weights: weights.into_f32().collect(),
                arm: arm.clone(),
                at: at.clone(),
            });
        }
    }
    out
}

/// The arm's segments on one side: each mapped arm bone to its mapped
/// parent in the humanoid tree, and every lone bone as a point.
fn segments(human: &[HumanBone], at: &BTreeMap<String, Vec3>, side: &str) -> Vec<(Vec3, Vec3)> {
    let mut out = Vec::new();
    for (bone, p) in at.iter().filter(|(b, _)| side_of(b) == side) {
        let parent = human
            .iter()
            .find(|h| &h.name == bone)
            .and_then(|h| h.parent.as_ref())
            .and_then(|parent| at.get(parent));
        out.push((parent.copied().unwrap_or(*p), *p));
    }
    out
}

/// Counts the vertices the arm chain moves that sit far from the arm.
/// `None` when no skinned mesh could be read or nothing maps to an arm
/// (a meshopt packed file, or an unrigged model).
pub(crate) fn measure(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    worlds: &[Mat4],
    mapped: &[Mapped],
    human: &[HumanBone],
) -> Option<ArmDrag> {
    if doc.extensions_used().any(|e| e.contains("meshopt")) {
        return None;
    }
    let meshes = read_skinned(doc, buffers, worlds, mapped);
    if meshes.iter().all(|m| m.arm.is_empty()) {
        return None;
    }
    let mut bounds = Aabb::EMPTY;
    for p in meshes.iter().flat_map(|m| &m.positions) {
        bounds.add(*p);
    }
    let reach = bounds.size()[1] * ARM_REACH;
    let mut skinned = 0u32;
    let mut vertices = 0u32;
    let mut per_bone: BTreeMap<String, u32> = BTreeMap::new();
    for mesh in &meshes {
        let sides: BTreeMap<&str, Vec<(Vec3, Vec3)>> = ["left", "right"]
            .into_iter()
            .map(|s| (s, segments(human, &mesh.at, s)))
            .collect();
        for ((p, j), w) in mesh.positions.iter().zip(&mesh.joints).zip(&mesh.weights) {
            skinned += 1;
            let mut held = 0.0f32;
            let mut top: Option<(&str, f32)> = None;
            for (joint, weight) in j.iter().zip(w) {
                let Some(bone) = mesh.arm.get(&usize::from(*joint)) else {
                    continue;
                };
                held += weight;
                if top.is_none_or(|(_, w)| *weight > w) {
                    top = Some((bone.as_str(), *weight));
                }
            }
            let Some((bone, _)) = top.filter(|_| held >= ARM_WEIGHT_MIN) else {
                continue;
            };
            let near = sides
                .get(side_of(bone))
                .into_iter()
                .flatten()
                .map(|(a, b)| math::segment_distance(*p, *a, *b))
                .fold(f32::INFINITY, f32::min);
            if near > reach {
                vertices += 1;
                *per_bone.entry(bone.to_string()).or_default() += 1;
            }
        }
    }
    let mut bones: Vec<(String, u32)> = per_bone.into_iter().collect();
    bones.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Some(ArmDrag {
        vertices,
        skinned,
        bones,
    })
}
