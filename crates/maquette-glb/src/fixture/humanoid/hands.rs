//! The box humanoid's hands: curled fingers, a piece weighted to the hand
//! that sits well off it, and a strip of hair that hangs beside it.

use super::super::builder::Mesh;
use super::super::skeleton::skeleton;
use super::{below, joint_index, turn_xy};

/// Bends the four fingers of both hands `deg` degrees at each of their
/// first three joints, towards the palm (-Y in the T-pose).
pub(super) fn curl_fingers(
    joints: &[(String, Option<usize>, [f32; 3])],
    world: &mut [[f32; 3]],
    deg: f32,
) {
    if deg == 0.0 {
        return;
    }
    for (side, sign) in [("Left", 1.0f32), ("Right", -1.0f32)] {
        for finger in ["Index", "Middle", "Ring", "Pinky"] {
            for n in 1..=3 {
                let Some(joint) = joint_index(joints, &format!("{side}Hand{finger}{n}")) else {
                    continue;
                };
                let pivot = world[joint];
                for (i, p) in world.iter_mut().enumerate() {
                    if below(joints, i, joint) {
                        *p = turn_xy(*p, pivot, -sign * deg);
                    }
                }
            }
        }
    }
}

/// A small box of its own in front of the left hand, weighted wholly to it.
pub(super) fn hand_piece(mesh: &mut Mesh, world: &[[f32; 3]]) {
    let Some(hand) = joint_index(&skeleton(), "LeftHand") else {
        return;
    };
    let h = world[hand];
    mesh.cuboid(
        [h[0], h[1], h[2] + 0.35],
        [0.03, 0.03, 0.03],
        u16::try_from(hand).unwrap_or(0),
    );
}

/// A ribbon of hair: its top edge is the left edge of the head box's
/// bottom face, so it joins the head and nothing else; it runs out behind
/// the left shoulder, then hangs straight down just behind the left hand to
/// below the fingers. Rows level with the hand and below are weighted 0.6
/// to the left index finger's end joint (which no box uses) and 0.4 to the
/// head, as an auto rig weights hair that hangs by a hand.
pub(super) fn hair_strip(mesh: &mut Mesh, world: &[[f32; 3]]) {
    let joints = skeleton();
    let (Some(head), Some(top), Some(hand), Some(finger)) = (
        joint_index(&joints, "Head"),
        joint_index(&joints, "HeadTop_End"),
        joint_index(&joints, "LeftHand"),
        joint_index(&joints, "LeftHandIndex4"),
    ) else {
        return;
    };
    // The head box's bottom left edge, worked out as `Mesh::cuboid` does.
    let (a, b) = (world[head], world[top]);
    let half = |k: usize| (a[k] - b[k]).abs().max(0.04) / 2.0;
    let centre = |k: usize| (a[k] + b[k]) / 2.0;
    let start = [centre(0) + half(0), centre(1) + -half(1), centre(2)];
    let width = half(2);
    let h = world[hand];
    let (behind, out, bottom) = (-0.11, h[0] + 0.04, h[1] - 0.2);
    let mut path = vec![];
    for i in 0..=8u8 {
        let t = f32::from(i) / 8.0;
        let z = start[2] + (behind - start[2]) * t;
        path.push([start[0] + (out - start[0]) * t, start[1], z]);
    }
    for i in 1..=16u8 {
        let t = f32::from(i) / 16.0;
        path.push([out, start[1] + (bottom - start[1]) * t, behind]);
    }
    let (head, finger) = (
        u16::try_from(head).unwrap_or(0),
        u16::try_from(finger).unwrap_or(0),
    );
    let first = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX);
    for c in &path {
        let held = c[1] <= h[1] + 0.12;
        for dz in [-width, width] {
            mesh.positions.push([c[0], c[1], c[2] + dz]);
            mesh.normals.push([1.0, 0.0, 0.0]);
            mesh.uvs.push([0.5, 0.5]);
            if held {
                mesh.joints.push([finger, head, 0, 0]);
                mesh.weights.push([0.6, 0.4, 0.0, 0.0]);
            } else {
                mesh.joints.push([head, 0, 0, 0]);
                mesh.weights.push([1.0, 0.0, 0.0, 0.0]);
            }
        }
    }
    let rows = u32::try_from(path.len()).unwrap_or(0);
    for r in 0..rows.saturating_sub(1) {
        let (a, b) = (first + 2 * r, first + 2 * r + 2);
        mesh.indices
            .extend_from_slice(&[a, a + 1, b + 1, a, b + 1, b]);
    }
}
