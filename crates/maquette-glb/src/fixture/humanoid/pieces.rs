//! What the box humanoid wears beyond its boxes: a cape, a strip or a tail
//! of cloth, a piece the hand holds, hair by the hand, a sleeve fused to the
//! hip, a hand of one piece, a ponytail on the shoulder and a charm.

use super::super::builder::Mesh;
use super::super::skeleton::skeleton;
use super::bridge::sleeve_bridge;
use super::bunched::bunched_hand;
use super::charm::charm;
use super::hands::{hair_strip, hand_piece};
use super::shoulder_tail::shoulder_tail;
use super::{HumanoidSpec, joint_index};

/// The slabs of the tail, 2 cm tall and 4 cm wide, so the finder reads
/// their corners as one strip.
const TAIL_SLABS: usize = 33;

/// Adds every piece `spec` asks for; `world` holds the posed joints.
pub(super) fn pieces(mesh: &mut Mesh, spec: &HumanoidSpec, world: &[[f32; 3]]) {
    let at = |i: usize| world[i];
    if spec.cape
        && let Some(arm) = joint_index(&skeleton(), "LeftArm")
    {
        let a = at(arm);
        mesh.cuboid(
            [a[0] / 2.0, a[1] - 0.5, a[2] - 0.25],
            [0.3, 0.5, 0.02],
            u16::try_from(arm).unwrap_or(0),
        );
    }
    if spec.cape_strip
        && let (Some(arm), Some(fore)) = (
            joint_index(&skeleton(), "LeftArm"),
            joint_index(&skeleton(), "LeftForeArm"),
        )
    {
        let (a, b) = (at(arm), at(fore));
        mesh.cuboid(
            [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0 - 0.1, a[2] - 0.03],
            [0.12, 0.05, 0.01],
            u16::try_from(arm).unwrap_or(0),
        );
    }
    if spec.hand_piece {
        hand_piece(mesh, world);
    }
    if spec.hair_strip {
        hair_strip(mesh, world);
    }
    if spec.shoulder_tail {
        shoulder_tail(mesh, world);
    }
    if spec.charm {
        charm(mesh, world);
    }
    if spec.sleeve_bridge {
        sleeve_bridge(mesh, world, spec.arm_drop);
    }
    if spec.bunched_hand {
        bunched_hand(mesh, world, spec.arm_drop);
    }
    if spec.tail
        && let Some(head) = joint_index(&skeleton(), "Head")
    {
        let h = at(head);
        // Thin slabs stacked edge to edge, so the strip reads as one surface
        // hanging from the back of the head to the waist, clear of the
        // thin box body (whose back it would otherwise pass for).
        for k in 0..TAIL_SLABS {
            #[expect(clippy::cast_precision_loss, reason = "a few slabs")]
            let y = h[1] - 0.02 * k as f32;
            mesh.cuboid(
                [h[0], y, h[2] - 0.3],
                [0.04, 0.02, 0.02],
                u16::try_from(head).unwrap_or(0),
            );
        }
    }
}
