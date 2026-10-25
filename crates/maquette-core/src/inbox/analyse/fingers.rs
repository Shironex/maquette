//! The hair weighted to finger bones heuristic.

use std::collections::BTreeMap;

use super::rig::{bare_joint_name, is_hand_joint};
use super::{FINGER_REACH, FingerWeights, JointCount};
use crate::inbox::math::{self, Aabb, IDENTITY, Mat4, Vec3};

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
            let reach = bounds.size()[1].max(1e-6) * FINGER_REACH;
            reach_used = reach_used.max(reach);
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
                let far = math::distance(*p, *at) > reach;
                let above_head = head_y.is_some_and(|y| p[1] > y);
                if far || above_head {
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
