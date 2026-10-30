//! Rest pose measures: which way the model faces, how far each arm drops
//! from horizontal, where the soles sit, whether node scales are uniform
//! and positive, and how symmetric the two sides are.
//!
//! Everything is read from the nodes' rest transforms, which is the pose a
//! VRM runtime treats as the T-pose. Facing comes from the character's
//! left: the left minus the right upper leg and upper arm, flattened, and
//! the front is that left crossed with up (+X left faces +Z). The toes, when
//! mapped, must point the same way, or left and right look swapped.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::bones::Mapped;
use crate::analyse::primitive_corners;
use crate::math::{self, Aabb, IDENTITY, Mat4, Vec3};

/// Scale components this close to 1 are float noise, which the writer
/// snaps to 1. Exports carry noise up to about 2e-5; a real scale is
/// far bigger.
pub const SCALE_NOISE: f32 = 1e-4;

/// Components of one node's scale may differ this much, relative to the
/// largest, and still count as uniform.
const UNIFORM_SLACK: f32 = 1e-4;

/// The axis a model faces, the nearest of four.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum Axis {
    #[serde(rename = "+z")]
    PlusZ,
    #[serde(rename = "-z")]
    MinusZ,
    #[serde(rename = "+x")]
    PlusX,
    #[serde(rename = "-x")]
    MinusX,
}

impl Axis {
    /// "+Z", the way the findings write it.
    pub fn label(self) -> &'static str {
        match self {
            Axis::PlusZ => "+Z",
            Axis::MinusZ => "-Z",
            Axis::PlusX => "+X",
            Axis::MinusX => "-X",
        }
    }

    fn nearest(yaw: f32) -> Axis {
        match yaw {
            y if y.abs() <= 45.0 => Axis::PlusZ,
            y if y.abs() >= 135.0 => Axis::MinusZ,
            y if y > 0.0 => Axis::PlusX,
            _ => Axis::MinusX,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RestPose {
    /// The axis the front points along; `None` without both legs or arms.
    pub facing: Option<Axis>,
    /// Degrees the front is turned from +Z about Y, towards +X positive.
    pub facing_deg: Option<f32>,
    /// The toes point against the front: left and right look swapped.
    pub sides_swapped: bool,
    /// Degrees each arm drops below horizontal, upper arm to hand; negative
    /// when it is raised.
    pub arm_left_deg: Option<f32>,
    pub arm_right_deg: Option<f32>,
    /// The lowest point of the skinned meshes at rest, metres.
    pub ground_y: Option<f32>,
    /// The lowest foot or toe joint, metres.
    pub feet_y: Option<f32>,
    /// The hips' distance from the Y axis, metres.
    pub hips_offset: Option<f32>,
    /// The same offset in the character's own frame: metres to its left
    /// (negative to its right) and metres forward (negative behind);
    /// `None` when the facing is unknown.
    pub hips_side: Option<f32>,
    pub hips_forward: Option<f32>,
    /// From the soles to the top of the meshes, metres.
    pub height: Option<f32>,
    /// Every node's scale is positive and uniform, float noise aside.
    pub uniform_scale: bool,
    /// Nodes with a negative scale component.
    pub negative_scale: Vec<String>,
    /// Nodes whose scale differs per axis, or whose matrix shears.
    pub uneven_scale: Vec<String>,
    /// Nodes scaled evenly by something other than 1, such as a 0.01
    /// armature, each with its scale.
    pub scaled: Vec<String>,
    /// Nodes off 1 by float noise only, which the writer snaps.
    pub scale_noise: u32,
    /// The largest gap between a left bone and its mirrored right bone,
    /// metres; `None` without pairs.
    pub asymmetry: Option<f32>,
    /// The left bone of that pair, such as `leftHand`.
    pub asymmetry_bone: Option<String>,
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn flat_unit(v: Vec3) -> Option<Vec3> {
    let len = (v[0] * v[0] + v[2] * v[2]).sqrt();
    (len > 1e-6).then(|| [v[0] / len, 0.0, v[2] / len])
}

/// Node positions at rest, by humanoid bone.
pub(crate) struct Joints {
    at: BTreeMap<String, Vec3>,
}

impl Joints {
    pub(crate) fn new(mapped: &[Mapped], worlds: &[Mat4]) -> Self {
        let at = mapped
            .iter()
            .filter_map(|m| {
                worlds
                    .get(m.node)
                    .map(|w| (m.bone.clone(), math::translation(w)))
            })
            .collect();
        Joints { at }
    }

    fn get(&self, bone: &str) -> Option<Vec3> {
        self.at.get(bone).copied()
    }

    fn pair(&self, bone: &str) -> Option<(Vec3, Vec3)> {
        Some((
            self.get(&format!("left{bone}"))?,
            self.get(&format!("right{bone}"))?,
        ))
    }
}

/// The character's left, flattened: from the legs and arms that are there.
fn left_axis(joints: &Joints) -> Option<Vec3> {
    let mut sum = [0.0f32; 3];
    let mut any = false;
    for bone in ["UpperLeg", "UpperArm"] {
        if let Some((l, r)) = joints.pair(bone)
            && let Some(dir) = flat_unit(sub(l, r))
        {
            sum = [sum[0] + dir[0], 0.0, sum[2] + dir[2]];
            any = true;
        }
    }
    if any { flat_unit(sum) } else { None }
}

fn toes_forward(joints: &Joints) -> Option<Vec3> {
    let mut sum = [0.0f32; 3];
    for side in ["left", "right"] {
        let toes = joints.get(&format!("{side}Toes"))?;
        let foot = joints.get(&format!("{side}Foot"))?;
        let d = sub(toes, foot);
        sum = [sum[0] + d[0], 0.0, sum[2] + d[2]];
    }
    flat_unit(sum)
}

/// Degrees from upper arm to hand below horizontal.
fn arm_drop(joints: &Joints, side: &str) -> Option<f32> {
    let upper = joints.get(&format!("{side}UpperArm"))?;
    let hand = joints.get(&format!("{side}Hand"))?;
    let d = sub(hand, upper);
    let flat = (d[0] * d[0] + d[2] * d[2]).sqrt();
    (flat > 1e-6 || d[1].abs() > 1e-6).then(|| (-d[1]).atan2(flat).to_degrees())
}

/// Node scale findings over every node.
struct Scales {
    negative: Vec<String>,
    uneven: Vec<String>,
    scaled: Vec<String>,
    noise: u32,
}

fn scales(doc: &gltf::Document) -> Scales {
    let mut out = Scales {
        negative: Vec::new(),
        uneven: Vec::new(),
        scaled: Vec::new(),
        noise: 0,
    };
    for node in doc.nodes() {
        let (_, _, s) = node.transform().decomposed();
        let name = node
            .name()
            .map_or_else(|| format!("node {}", node.index()), str::to_string);
        let det = det3(&node.transform().matrix());
        if s.iter().any(|v| *v < 0.0) || det < 0.0 {
            out.negative.push(name);
            continue;
        }
        if sheared(&node.transform().matrix()) {
            out.uneven.push(format!("{name} (sheared)"));
            continue;
        }
        let top = s.iter().copied().fold(0.0f32, f32::max);
        let low = s.iter().copied().fold(f32::INFINITY, f32::min);
        if s.iter().all(|v| (v - 1.0).abs() <= SCALE_NOISE) {
            if s.iter().any(|v| *v != 1.0) {
                out.noise += 1;
            }
        } else if top - low > UNIFORM_SLACK * top {
            out.uneven.push(name);
        } else {
            let far = s.iter().copied().fold(1.0f32, |a, v| {
                if (v - 1.0).abs() > (a - 1.0).abs() {
                    v
                } else {
                    a
                }
            });
            out.scaled.push(format!("{name} ({far})"));
        }
    }
    out
}

/// A matrix whose axes are not at right angles: a shear, which a
/// translation, rotation and scale cannot hold.
fn sheared(m: &Mat4) -> bool {
    let axes = [m[0], m[1], m[2]].map(|c| [c[0], c[1], c[2]]);
    [(0, 1), (0, 2), (1, 2)].iter().any(|(a, b)| {
        let (x, y) = (axes[*a], axes[*b]);
        let lengths = dot(x, x).sqrt() * dot(y, y).sqrt();
        dot(x, y).abs() > UNIFORM_SLACK * lengths.max(f32::MIN_POSITIVE)
    })
}

fn det3(m: &Mat4) -> f32 {
    m[0][0] * (m[1][1] * m[2][2] - m[2][1] * m[1][2])
        - m[1][0] * (m[0][1] * m[2][2] - m[2][1] * m[0][2])
        + m[2][0] * (m[0][1] * m[1][2] - m[1][1] * m[0][2])
}

/// The meshes' bounds at rest. For a skinned mesh, its first joint's world
/// matrix times that joint's inverse bind matrix takes the mesh from bind
/// space to the rest pose (the two match when the rest pose is the bind
/// pose, as exports write it). Other meshes use their node. A skinned mesh
/// in a meshopt packed file is left out: its inverse bind matrices cannot
/// be read.
fn mesh_bounds(doc: &gltf::Document, buffers: &[Option<Vec<u8>>], worlds: &[Mat4]) -> Aabb {
    let packed = doc.extensions_used().any(|e| e.contains("meshopt"));
    let mut bounds = Aabb::EMPTY;
    for node in doc.nodes() {
        let Some(mesh) = node.mesh() else { continue };
        let to_world = match node.skin() {
            Some(_) if packed => continue,
            Some(skin) => {
                let joint = skin.joints().next();
                let ibm = skin
                    .reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()))
                    .read_inverse_bind_matrices()
                    .and_then(|mut m| m.next());
                let (Some(joint), Some(ibm)) = (joint, ibm) else {
                    continue;
                };
                math::mul(
                    &worlds.get(joint.index()).copied().unwrap_or(IDENTITY),
                    &ibm,
                )
            }
            None => worlds.get(node.index()).copied().unwrap_or(IDENTITY),
        };
        for prim in mesh.primitives() {
            for corner in primitive_corners(&prim) {
                let p = math::point(&to_world, corner);
                if p.iter().all(|v| v.is_finite()) {
                    bounds.add(p);
                }
            }
        }
    }
    bounds
}

/// The largest gap between left bones and their mirrored right bones, in
/// the character's own frame (left, up, front from the hips), with the
/// left bone of that pair.
fn asymmetry(joints: &Joints, hips: Vec3, left: Vec3) -> Option<(f32, String)> {
    let front = [-left[2], 0.0, left[0]];
    let frame = |p: Vec3| {
        let d = sub(p, hips);
        [dot(d, left), d[1], dot(d, front)]
    };
    joints
        .at
        .iter()
        .filter_map(|(bone, l)| {
            let rest = bone.strip_prefix("left")?;
            let r = joints.get(&format!("right{rest}"))?;
            let (a, b) = (frame(*l), frame(r));
            Some((math::distance(a, [-b[0], b[1], b[2]]), bone.clone()))
        })
        .reduce(|a, b| if b.0 > a.0 { b } else { a })
}

/// Measures the rest pose of the mapped bones.
pub(crate) fn measure(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    worlds: &[Mat4],
    mapped: &[Mapped],
) -> RestPose {
    let joints = Joints::new(mapped, worlds);
    let left = left_axis(&joints);
    // Left crossed with up: +X left faces +Z.
    let front = left.map(|l| [-l[2], 0.0, l[0]]);
    let facing_deg = front.map(|f| f[0].atan2(f[2]).to_degrees());
    let sides_swapped = match (front, toes_forward(&joints)) {
        (Some(f), Some(t)) => dot(f, t) < 0.0,
        _ => false,
    };
    let bounds = mesh_bounds(doc, buffers, worlds);
    let feet_y = ["leftFoot", "rightFoot", "leftToes", "rightToes"]
        .iter()
        .filter_map(|b| joints.get(b))
        .map(|p| p[1])
        .reduce(f32::min);
    let hips = joints.get("hips");
    // The hips against the Y axis, along the character's left and front.
    let along = |axis: Option<Vec3>| match (hips, axis) {
        (Some(h), Some(a)) => Some(dot([h[0], 0.0, h[2]], a)),
        _ => None,
    };
    let s = scales(doc);
    let asymmetry = match (hips, left) {
        (Some(h), Some(l)) => asymmetry(&joints, h, l),
        _ => None,
    };
    RestPose {
        facing: facing_deg.map(Axis::nearest),
        facing_deg,
        sides_swapped,
        arm_left_deg: arm_drop(&joints, "left"),
        arm_right_deg: arm_drop(&joints, "right"),
        ground_y: (!bounds.is_empty()).then_some(bounds.min[1]),
        feet_y,
        hips_offset: hips.map(|h| (h[0] * h[0] + h[2] * h[2]).sqrt()),
        hips_side: along(left),
        hips_forward: along(front),
        height: (!bounds.is_empty()).then(|| bounds.size()[1]),
        uniform_scale: s.negative.is_empty() && s.uneven.is_empty(),
        negative_scale: s.negative,
        uneven_scale: s.uneven,
        scaled: s.scaled,
        scale_noise: s.noise,
        asymmetry: asymmetry.as_ref().map(|a| a.0),
        asymmetry_bone: asymmetry.map(|a| a.1),
    }
}
