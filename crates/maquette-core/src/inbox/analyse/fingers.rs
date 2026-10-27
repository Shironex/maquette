//! The hair weighted to finger bones heuristic.
//!
//! A vertex whose dominant joint is a hand or finger bone belongs to that
//! hand when it sits near the hand's chain: the segments from the forearm
//! joint to the hand joint and from the hand joint to each finger joint, at
//! bind. It is flagged when it sits above the head joint, or farther than
//! the reach from its side's chain, unless it hangs below that wrist: at
//! most [`HANG_REACH`] of the model height under the hand joint and within
//! half the reach of the chain seen from above. Sleeve ends, cuffs and
//! wrist chains hang there; hair and tails do not.

use std::collections::BTreeMap;

use super::rig::{bare_joint_name, is_hand_joint};
use super::{FINGER_REACH, FingerWeights, HANG_REACH, JointCount};
use crate::inbox::math::{self, Aabb, IDENTITY, Mat4, Vec3};

/// One side's hand at bind, in the mesh's space.
#[derive(Default)]
struct Hand {
    joint: Option<Vec3>,
    segments: Vec<(Vec3, Vec3)>,
}

/// How far a vertex may sit from its hand, in model units.
struct Reach {
    near: f32,
    hang: f32,
}

/// 0 for the left side, 1 for the right, by Mixamo name, with the rest of
/// the name.
fn side(bare: &str) -> Option<(usize, &str)> {
    bare.strip_prefix("Left")
        .map(|rest| (0, rest))
        .or_else(|| bare.strip_prefix("Right").map(|rest| (1, rest)))
}

/// The left and right hand chains from the skin's joint names.
fn hands(names: &[String], bind: &[Vec3]) -> [Hand; 2] {
    let mut fore: [Option<Vec3>; 2] = [None, None];
    let mut out: [Hand; 2] = Default::default();
    let mut fingers: [Vec<Vec3>; 2] = Default::default();
    for (name, at) in names.iter().zip(bind) {
        let bare = bare_joint_name(name);
        let Some((s, rest)) = side(bare) else {
            continue;
        };
        match rest {
            "ForeArm" => fore[s] = Some(*at),
            "Hand" => out[s].joint = Some(*at),
            _ if rest.starts_with("Hand") => fingers[s].push(*at),
            _ => {}
        }
    }
    for (s, hand) in out.iter_mut().enumerate() {
        let Some(joint) = hand.joint else {
            continue;
        };
        hand.segments.extend(fore[s].map(|f| (f, joint)));
        hand.segments
            .extend(fingers[s].iter().map(|finger| (joint, *finger)));
    }
    out
}

fn chain_distance(p: Vec3, segments: &[(Vec3, Vec3)]) -> f32 {
    segments
        .iter()
        .map(|(a, b)| math::segment_distance(p, *a, *b))
        .fold(f32::INFINITY, f32::min)
}

/// Whether a vertex weighted to a hand or finger joint at `at` is not part
/// of that hand.
fn stray(p: Vec3, at: Vec3, hand: Option<&Hand>, reach: &Reach) -> bool {
    let Some((joint, segments)) = hand
        .and_then(|h| h.joint.map(|j| (j, h.segments.as_slice())))
        .filter(|(_, s)| !s.is_empty())
    else {
        return math::distance(p, at) > reach.near;
    };
    if chain_distance(p, segments) <= reach.near {
        return false;
    }
    let flat = |v: Vec3| [v[0], 0.0, v[2]];
    let flat_segments: Vec<(Vec3, Vec3)> =
        segments.iter().map(|(a, b)| (flat(*a), flat(*b))).collect();
    let drop = joint[1] - p[1];
    let hangs = (0.0..=reach.hang).contains(&drop)
        && chain_distance(flat(p), &flat_segments) <= reach.near / 2.0;
    !hangs
}

pub(super) fn finger_weights(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    worlds: &[Mat4],
) -> Option<FingerWeights> {
    let mut flagged = 0u32;
    let mut per_joint: BTreeMap<String, u32> = BTreeMap::new();
    let mut checked = false;
    let mut reach_used = 0.0f32;
    for node in doc.nodes() {
        let (Some(mesh), Some(skin)) = (node.mesh(), node.skin()) else {
            continue;
        };
        let joints: Vec<gltf::Node<'_>> = skin.joints().collect();
        let ibms: Vec<Mat4> = skin
            .reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()))
            .read_inverse_bind_matrices()
            .map(Iterator::collect)
            .unwrap_or_default();
        // Bind position of each joint in the mesh's space.
        let bind: Vec<Vec3> = joints
            .iter()
            .enumerate()
            .map(|(i, joint)| {
                ibms.get(i)
                    .and_then(math::invert_affine)
                    .map(|m| math::translation(&m))
                    .unwrap_or_else(|| {
                        math::translation(&worlds.get(joint.index()).copied().unwrap_or(IDENTITY))
                    })
            })
            .collect();
        let names: Vec<String> = joints
            .iter()
            .map(|j| j.name().unwrap_or("joint").to_string())
            .collect();
        let head_y = names
            .iter()
            .position(|n| bare_joint_name(n) == "Head")
            .and_then(|i| bind.get(i))
            .map(|p| p[1]);
        let sides = hands(&names, &bind);
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()));
            let (Some(positions), Some(joint_sets), Some(weights)) = (
                reader.read_positions(),
                reader.read_joints(0),
                reader.read_weights(0),
            ) else {
                continue;
            };
            let positions: Vec<Vec3> = positions.collect();
            let mut bounds = Aabb::EMPTY;
            for p in &positions {
                bounds.add(*p);
            }
            let height = bounds.size()[1].max(1e-6);
            let reach = Reach {
                near: height * FINGER_REACH,
                hang: height * HANG_REACH,
            };
            reach_used = reach_used.max(reach.near);
            checked = true;
            for ((p, j), w) in positions
                .iter()
                .zip(joint_sets.into_u16())
                .zip(weights.into_f32())
            {
                let (k, _) = w.iter().enumerate().fold((0, f32::MIN), |best, (k, &v)| {
                    if v > best.1 { (k, v) } else { best }
                });
                let joint = usize::from(j[k]);
                let (Some(name), Some(at)) = (names.get(joint), bind.get(joint)) else {
                    continue;
                };
                if !is_hand_joint(name) {
                    continue;
                }
                let hand = side(bare_joint_name(name)).and_then(|(s, _)| sides.get(s));
                let above_head = head_y.is_some_and(|y| p[1] > y);
                if above_head || stray(*p, *at, hand, &reach) {
                    flagged += 1;
                    *per_joint.entry(name.clone()).or_default() += 1;
                }
            }
        }
    }
    if !checked {
        return None;
    }
    let mut joints: Vec<JointCount> = per_joint
        .into_iter()
        .map(|(name, vertices)| JointCount { name, vertices })
        .collect();
    joints.sort_by(|a, b| b.vertices.cmp(&a.vertices).then(a.name.cmp(&b.name)));
    Some(FingerWeights {
        vertices: flagged,
        joints,
        reach: reach_used,
    })
}
