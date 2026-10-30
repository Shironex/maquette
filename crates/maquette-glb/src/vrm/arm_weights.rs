//! Cloth or hair weighted to the arms: vertices the arm chain moves at rest
//! that sit well outside the arm, such as a cape, a sleeve's long hem or a
//! braid. Lifting the arms to a T-pose drags them along.
//!
//! Everything is read in each skinned mesh's own (bind) space: the vertex
//! positions as stored, and each joint where its inverse bind matrix puts
//! it. Distances are measured against the arm's segments (upper arm to
//! lower arm to hand, hand to each finger) and compared with the arm's own
//! radius, read from the mesh around the upper and lower arm: in a few
//! slices along each, the nearest surface in each direction around the
//! bone, and the median of those. Cloth beside the arm is further out than
//! the sleeve, so it does not widen the limb. The radius is a share of the
//! height of the skinned meshes, so the scale of the export does not matter.

use std::collections::BTreeMap;

use super::bones::{HumanBone, Mapped};
use crate::math::{self, Aabb, IDENTITY, Mat4, Vec3};

/// A vertex counts when the arm chain holds at least this much of its
/// weight.
pub(super) const ARM_WEIGHT_MIN: f32 = 0.25;

/// ...and it sits further than this many limb radii from the arm...
const ARM_RADII: f32 = 2.0;

/// ...and further than this share of the height outside the limb.
const ARM_GAP: f32 = 0.01;

/// The limb radius, as a share of the height, when the mesh around the arm
/// cannot be read.
const LIMB_FALLBACK: f32 = 0.03;

/// Surface further than this share of the height from the bone is never
/// read as the limb.
const LIMB_NEAR: f32 = 0.15;

/// Slices along each arm segment, and directions around the bone in each.
const SLICES: usize = 4;
const SECTORS: usize = 8;

/// A slice reads the limb when at least this many directions found surface.
const SECTORS_MIN: usize = 4;

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

fn median(mut values: Vec<f32>) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f32::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        f32::midpoint(values[mid - 1], values[mid])
    } else {
        values[mid]
    })
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(v: Vec3) -> Option<Vec3> {
    let n = dot(v, v).sqrt();
    (n > 1e-6).then(|| [v[0] / n, v[1] / n, v[2] / n])
}

/// The radius of the limb along the segment `a` to `b`: in each slice
/// across it, the nearest surface in each direction around the bone, and
/// the median of those. `None` when no slice found enough surface.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "slice and sector indices are small and clamped"
)]
fn segment_radius(positions: &[Vec3], a: Vec3, b: Vec3, height: f32) -> Option<f32> {
    let along = sub(b, a);
    let length = dot(along, along).sqrt();
    let axis = unit(along)?;
    let e1 = unit(cross(axis, [0.0, 1.0, 0.0])).or_else(|| unit(cross(axis, [1.0, 0.0, 0.0])))?;
    let e2 = cross(axis, e1);
    let mut nearest = [[f32::INFINITY; SECTORS]; SLICES];
    for p in positions {
        let rel = sub(*p, a);
        let d = dot(rel, axis);
        let t = d / length;
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        let off = [
            rel[0] - axis[0] * d,
            rel[1] - axis[1] * d,
            rel[2] - axis[2] * d,
        ];
        let r = dot(off, off).sqrt();
        if r >= LIMB_NEAR * height {
            continue;
        }
        let slice = ((t * SLICES as f32) as usize).min(SLICES - 1);
        let turn = dot(off, e2).atan2(dot(off, e1)) + std::f32::consts::PI;
        let sector = (turn / std::f32::consts::TAU * SECTORS as f32) as usize % SECTORS;
        nearest[slice][sector] = nearest[slice][sector].min(r);
    }
    let slices = nearest.iter().filter_map(|sectors| {
        let found: Vec<f32> = sectors.iter().copied().filter(|r| r.is_finite()).collect();
        (found.len() >= SECTORS_MIN)
            .then(|| median(found))
            .flatten()
    });
    median(slices.collect())
}

/// The limb radius of one side in one mesh, from its upper and lower arm.
fn limb_radius(mesh: &Skinned, side: &str, height: f32) -> Option<f32> {
    let at = |part: &str| mesh.at.get(&format!("{side}{part}")).copied();
    let radii: Vec<f32> = [("UpperArm", "LowerArm"), ("LowerArm", "Hand")]
        .iter()
        .filter_map(|(from, to)| segment_radius(&mesh.positions, at(from)?, at(to)?, height))
        .collect();
    median(radii)
}

/// How far from its arm a vertex may sit, per mesh and side: a few limb
/// radii. A mesh that does not show the limb (a cape on its own) borrows
/// the radius the other meshes read.
fn reaches(meshes: &[Skinned], height: f32) -> Vec<BTreeMap<&'static str, f32>> {
    let read: Vec<BTreeMap<&'static str, Option<f32>>> = meshes
        .iter()
        .map(|m| {
            ["left", "right"]
                .into_iter()
                .map(|s| (s, limb_radius(m, s, height)))
                .collect()
        })
        .collect();
    let shared: BTreeMap<&str, Option<f32>> = ["left", "right"]
        .into_iter()
        .map(|s| (s, median(read.iter().filter_map(|r| r[s]).collect())))
        .collect();
    read.iter()
        .map(|r| {
            r.iter()
                .map(|(side, own)| {
                    let radius = own
                        .or(shared[side])
                        .unwrap_or(LIMB_FALLBACK * height)
                        .max(1e-6);
                    (*side, (ARM_RADII * radius).max(radius + ARM_GAP * height))
                })
                .collect()
        })
        .collect()
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
    let height = bounds.size()[1];
    let reaches = reaches(&meshes, height);
    let mut skinned = 0u32;
    let mut vertices = 0u32;
    let mut per_bone: BTreeMap<String, u32> = BTreeMap::new();
    for (mesh, reach) in meshes.iter().zip(&reaches) {
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
            if near > reach.get(side_of(bone)).copied().unwrap_or(f32::INFINITY) {
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
