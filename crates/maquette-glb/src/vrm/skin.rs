//! Skin checks: what a VRM runtime needs from the skins and the node tree
//! before it can deform the model at all, read over every skinned mesh and
//! primitive.
//!
//! Each vertex's joints and weights are read from every `JOINTS_n` and
//! `WEIGHTS_n` set the primitive has, in any form glTF allows (8 or 16 bit
//! joints; float, or 8 or 16 bit normalised weights). A vertex must use
//! only the skin's joints, carry some weight, and have its weights add up
//! to 1 within what a normaliser would fix. More than four influences is
//! allowed but runtimes keep four. Each skin names a joint once, and the
//! node tree has no loop.
//!
//! The limb chains (arms, legs, fingers) should have length, and the two
//! sides the same bones. A meshopt packed or Draco compressed primitive
//! cannot be read here and is said so, never passed silently.

use std::collections::{BTreeMap, BTreeSet};

use super::bones::Mapped;
use crate::math::{self, Mat4, Vec3};

/// Weights may add up to this far from 1: a normaliser's rounding, such as
/// four 8 bit weights, stays well inside.
pub const WEIGHT_SUM_SLACK: f32 = 0.01;

/// VRM runtimes skin with this many joints per vertex.
pub const MAX_INFLUENCES: usize = 4;

/// An arm or leg bone shorter than this share of the height is too short
/// to bend by.
pub(super) const SHORT_LIMB: f32 = 0.01;

/// A finger segment shorter than this share of the height has no length.
pub(super) const SHORT_FINGER: f32 = 0.001;

/// Vertex counts per mesh, in the order the meshes come.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PerMesh(pub(crate) Vec<(String, u32)>);

impl PerMesh {
    fn add(&mut self, mesh: &str, n: u32) {
        if n == 0 {
            return;
        }
        match self.0.iter_mut().find(|(m, _)| m == mesh) {
            Some((_, count)) => *count += n,
            None => self.0.push((mesh.to_string(), n)),
        }
    }

    pub(crate) fn total(&self) -> u32 {
        self.0.iter().map(|(_, n)| n).sum()
    }
}

/// What the skin checks found.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SkinFacts {
    /// Vertices with a joint index past the skin's joints.
    pub(crate) past_joints: PerMesh,
    /// Vertices whose weights are all zero.
    pub(crate) unweighted: PerMesh,
    /// Vertices whose weights do not add up to 1.
    pub(crate) off_sum: PerMesh,
    /// Vertices with more than four joints weighted.
    pub(crate) over_four: PerMesh,
    /// Skins that name a joint twice, with those joints.
    pub(crate) twice: Vec<(String, Vec<String>)>,
    /// Nodes on a loop in the node tree, in loop order.
    pub(crate) cycle: Vec<String>,
    /// Skinned meshes whose vertices could not be read.
    pub(crate) unread: Vec<String>,
    /// Limb bones without length: bone, its length in metres.
    pub(crate) short: Vec<(String, f32)>,
    /// Left and right differ: sentences.
    pub(crate) sides: Vec<String>,
}

fn node_label(node: &gltf::Node<'_>) -> String {
    node.mesh()
        .and_then(|m| m.name().map(str::to_string))
        .or_else(|| node.name().map(str::to_string))
        .unwrap_or_else(|| format!("mesh {}", node.index()))
}

fn joint_label(node: &gltf::Node<'_>) -> String {
    node.name()
        .map_or_else(|| format!("node {}", node.index()), str::to_string)
}

/// One primitive's per vertex joints and weights, every set.
struct Influences {
    joints: Vec<Vec<[u16; 4]>>,
    weights: Vec<Vec<[f32; 4]>>,
}

/// Reads every `JOINTS_n` and `WEIGHTS_n` pair; `None` when the primitive
/// names them but they cannot be read (compressed, or no data).
fn influences(prim: &gltf::Primitive<'_>, buffers: &[Option<Vec<u8>>]) -> Option<Influences> {
    let reader = prim.reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()));
    let mut out = Influences {
        joints: Vec::new(),
        weights: Vec::new(),
    };
    for set in 0..8 {
        let named = prim.get(&gltf::Semantic::Joints(set)).is_some()
            || prim.get(&gltf::Semantic::Weights(set)).is_some();
        if !named {
            break;
        }
        let (Some(joints), Some(weights)) = (reader.read_joints(set), reader.read_weights(set))
        else {
            return None;
        };
        out.joints.push(joints.into_u16().collect());
        out.weights.push(weights.into_f32().collect());
    }
    Some(out)
}

/// The vertex checks over one skinned node's mesh.
fn check_vertices(
    node: &gltf::Node<'_>,
    joint_count: usize,
    buffers: &[Option<Vec<u8>>],
    facts: &mut SkinFacts,
) {
    let Some(mesh) = node.mesh() else { return };
    let label = node_label(node);
    for prim in mesh.primitives() {
        let vertices = prim
            .get(&gltf::Semantic::Positions)
            .map_or(0, |a| a.count());
        let Some(inf) = influences(&prim, buffers) else {
            if !facts.unread.contains(&label) {
                facts.unread.push(label.clone());
            }
            continue;
        };
        if inf.joints.is_empty() {
            // A skinned node's primitive with no weights at all.
            facts
                .unweighted
                .add(&label, u32::try_from(vertices).unwrap_or(u32::MAX));
            continue;
        }
        let n = inf
            .joints
            .iter()
            .map(Vec::len)
            .chain(inf.weights.iter().map(Vec::len))
            .min()
            .unwrap_or(0);
        let (mut past, mut zero, mut off, mut many) = (0u32, 0u32, 0u32, 0u32);
        for v in 0..n {
            let mut sum = 0.0f32;
            let mut weighted = 0usize;
            let mut out_of_range = false;
            for (js, ws) in inf.joints.iter().zip(&inf.weights) {
                for (j, w) in js[v].iter().zip(&ws[v]) {
                    out_of_range |= usize::from(*j) >= joint_count;
                    if *w > 0.0 {
                        weighted += 1;
                        sum += w;
                    }
                }
            }
            past += u32::from(out_of_range);
            if weighted == 0 {
                zero += 1;
            } else if (sum - 1.0).abs() > WEIGHT_SUM_SLACK {
                off += 1;
            }
            many += u32::from(weighted > MAX_INFLUENCES);
        }
        facts.past_joints.add(&label, past);
        facts.unweighted.add(&label, zero);
        facts.off_sum.add(&label, off);
        facts.over_four.add(&label, many);
    }
}

/// A loop in the node tree, as the nodes on it; empty without one.
fn find_cycle(doc: &gltf::Document) -> Vec<String> {
    let nodes: Vec<gltf::Node<'_>> = doc.nodes().collect();
    let children: Vec<Vec<usize>> = nodes
        .iter()
        .map(|n| n.children().map(|c| c.index()).collect())
        .collect();
    // 0 not seen, 1 on the current path, 2 done.
    let mut state = vec![0u8; nodes.len()];
    for start in 0..nodes.len() {
        if state[start] != 0 {
            continue;
        }
        let mut path: Vec<(usize, usize)> = vec![(start, 0)];
        state[start] = 1;
        while let Some((node, next)) = path.last().copied() {
            let Some(child) = children.get(node).and_then(|c| c.get(next)).copied() else {
                state[node] = 2;
                path.pop();
                continue;
            };
            if let Some(top) = path.last_mut() {
                top.1 += 1;
            }
            match state.get(child) {
                Some(1) => {
                    let from = path.iter().position(|(n, _)| *n == child).unwrap_or(0);
                    return path[from..]
                        .iter()
                        .filter_map(|(n, _)| nodes.get(*n).map(joint_label))
                        .collect();
                }
                Some(0) => {
                    state[child] = 1;
                    path.push((child, 0));
                }
                _ => {}
            }
        }
    }
    Vec::new()
}

/// The arm, leg and finger links whose length is checked: bone, the next
/// one down, and whether it is a finger.
fn chains() -> Vec<(String, String, bool)> {
    let mut out = Vec::new();
    for side in ["left", "right"] {
        for (a, b) in [
            ("UpperArm", "LowerArm"),
            ("LowerArm", "Hand"),
            ("UpperLeg", "LowerLeg"),
            ("LowerLeg", "Foot"),
        ] {
            out.push((format!("{side}{a}"), format!("{side}{b}"), false));
        }
        for (finger, segments) in [
            ("Thumb", ["Metacarpal", "Proximal", "Distal"]),
            ("Index", ["Proximal", "Intermediate", "Distal"]),
            ("Middle", ["Proximal", "Intermediate", "Distal"]),
            ("Ring", ["Proximal", "Intermediate", "Distal"]),
            ("Little", ["Proximal", "Intermediate", "Distal"]),
        ] {
            for pair in segments.windows(2) {
                out.push((
                    format!("{side}{finger}{}", pair[0]),
                    format!("{side}{finger}{}", pair[1]),
                    true,
                ));
            }
        }
    }
    out
}

/// Bones in a limb chain shorter than they can bend by. `height` is the
/// model's, metres.
fn short_bones(mapped: &[Mapped], worlds: &[Mat4], height: f32) -> Vec<(String, f32)> {
    let at: BTreeMap<&str, Vec3> = mapped
        .iter()
        .filter_map(|m| {
            worlds
                .get(m.node)
                .map(|w| (m.bone.as_str(), math::translation(w)))
        })
        .collect();
    chains()
        .into_iter()
        .filter_map(|(bone, next, finger)| {
            let length = math::distance(*at.get(bone.as_str())?, *at.get(next.as_str())?);
            let least = height * if finger { SHORT_FINGER } else { SHORT_LIMB };
            (length < least).then_some((bone, length))
        })
        .collect()
}

/// Where the two sides differ: bones mapped on one side only, and the
/// joints under each hand.
fn side_differences(
    doc: &gltf::Document,
    mapped: &[Mapped],
    skin_joints: &BTreeSet<usize>,
) -> Vec<String> {
    let has = |bone: &str| mapped.iter().any(|m| m.bone == bone);
    let fingers = |side: &str| {
        mapped
            .iter()
            .filter(|m| {
                m.bone.starts_with(side)
                    && ["Thumb", "Index", "Middle", "Ring", "Little"]
                        .iter()
                        .any(|f| m.bone.contains(f))
            })
            .count()
    };
    let mut out = Vec::new();
    let (left, right) = (fingers("left"), fingers("right"));
    if left != right {
        out.push(format!(
            "The left hand has {left} finger bones mapped, the right {right}."
        ));
    }
    let one_sided: Vec<String> = mapped
        .iter()
        .filter_map(|m| {
            let twin = if let Some(rest) = m.bone.strip_prefix("left") {
                format!("right{rest}")
            } else {
                format!("left{}", m.bone.strip_prefix("right")?)
            };
            (!has(&twin)).then(|| m.bone.clone())
        })
        .collect();
    if !one_sided.is_empty() {
        out.push(format!(
            "Mapped on one side only: {}.",
            super::findings::list(&one_sided)
        ));
    }
    let node_of = |bone: &str| mapped.iter().find(|m| m.bone == bone).map(|m| m.node);
    if let (Some(l), Some(r)) = (node_of("leftHand"), node_of("rightHand")) {
        let children: Vec<Vec<usize>> = doc
            .nodes()
            .map(|n| n.children().map(|c| c.index()).collect())
            .collect();
        let under = |hand: usize| {
            let mut count = 0usize;
            let mut stack: Vec<usize> = children.get(hand).cloned().unwrap_or_default();
            let mut seen = BTreeSet::new();
            while let Some(node) = stack.pop() {
                if !seen.insert(node) {
                    continue;
                }
                count += usize::from(skin_joints.contains(&node));
                stack.extend(children.get(node).into_iter().flatten());
            }
            count
        };
        let (a, b) = (under(l), under(r));
        if a != b {
            out.push(format!(
                "Under the hands: {a} joints on the left, {b} on the right."
            ));
        }
    }
    out
}

/// The skin checks over every skinned mesh, the node tree and the limb
/// chains. `height` is the model's, metres, when known.
pub(crate) fn check(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    worlds: &[Mat4],
    mapped: &[Mapped],
    height: Option<f32>,
) -> SkinFacts {
    let mut facts = SkinFacts {
        cycle: find_cycle(doc),
        ..SkinFacts::default()
    };
    let packed = doc.extensions_used().any(|e| e.contains("meshopt"));
    let mut skin_joints = BTreeSet::new();
    for skin in doc.skins() {
        let mut seen = BTreeSet::new();
        let mut twice = Vec::new();
        for joint in skin.joints() {
            skin_joints.insert(joint.index());
            if !seen.insert(joint.index()) {
                let name = joint_label(&joint);
                if !twice.contains(&name) {
                    twice.push(name);
                }
            }
        }
        if !twice.is_empty() {
            let name = skin
                .name()
                .map_or_else(|| format!("skin {}", skin.index()), str::to_string);
            facts.twice.push((name, twice));
        }
    }
    for node in doc.nodes() {
        let Some(skin) = node.skin() else { continue };
        if node.mesh().is_none() {
            continue;
        }
        if packed {
            facts.unread.push(node_label(&node));
            continue;
        }
        check_vertices(&node, skin.joints().count(), buffers, &mut facts);
    }
    let height = height.or_else(|| {
        let ys: Vec<f32> = mapped
            .iter()
            .filter_map(|m| worlds.get(m.node).map(|w| math::translation(w)[1]))
            .collect();
        let top = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let low = ys.iter().copied().fold(f32::INFINITY, f32::min);
        (top > low).then_some(top - low)
    });
    if let Some(height) = height.filter(|h| *h > 0.0) {
        facts.short = short_bones(mapped, worlds, height);
    }
    facts.sides = side_differences(doc, mapped, &skin_joints);
    facts
}
