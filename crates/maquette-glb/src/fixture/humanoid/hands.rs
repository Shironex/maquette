//! The box humanoid's hands: curled fingers, and a piece weighted to the
//! hand that sits well off it.

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
