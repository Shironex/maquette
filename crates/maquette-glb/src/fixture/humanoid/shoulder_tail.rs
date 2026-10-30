//! A ponytail an auto rig hangs from the head and the arm: it leaves the
//! back of the head and falls behind the left shoulder.

use super::super::builder::Mesh;
use super::super::skeleton::skeleton;
use super::joint_index;

/// The rows of the ribbon, top to bottom.
pub const SHOULDER_TAIL_ROWS: u8 = 20;

/// A ribbon of hair: its top edge is the back edge of the head box's
/// bottom face, so it joins the head and nothing else; it falls behind the
/// left shoulder to the waist. Its top rows are weighted 0.7 to the head
/// and 0.3 to the left upper arm, and the head's share fades to 0.1 over
/// the upper part, as an auto rig weights a ponytail resting on the
/// shoulder. Built after every box, so it does not go with `split` or a
/// `flaw`.
pub(super) fn shoulder_tail(mesh: &mut Mesh, world: &[[f32; 3]]) {
    let joints = skeleton();
    let (Some(head), Some(top), Some(arm), Some(hips)) = (
        joint_index(&joints, "Head"),
        joint_index(&joints, "HeadTop_End"),
        joint_index(&joints, "LeftArm"),
        joint_index(&joints, "Hips"),
    ) else {
        return;
    };
    // The head box's bottom back edge, worked out as `Mesh::cuboid` does.
    let (a, b) = (world[head], world[top]);
    let half = |k: usize| (a[k] - b[k]).abs().max(0.04) / 2.0;
    let centre = |k: usize| (a[k] + b[k]) / 2.0;
    let start = [centre(0), centre(1) - half(1), centre(2) - half(2)];
    let width = half(0) / 2.0;
    let (out, behind, bottom) = (world[arm][0] * 0.6, start[2] - 0.12, world[hips][1]);
    let (head, arm) = (
        u16::try_from(head).unwrap_or(0),
        u16::try_from(arm).unwrap_or(0),
    );
    let first = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX);
    for i in 0..SHOULDER_TAIL_ROWS {
        let t = f32::from(i) / f32::from(SHOULDER_TAIL_ROWS - 1);
        let bend = (t * 3.0).min(1.0);
        let c = [
            start[0] + (out - start[0]) * bend,
            start[1] + (bottom - start[1]) * t,
            start[2] + (behind - start[2]) * bend,
        ];
        let on_head = 0.7 - 0.6 * (t / 0.4).min(1.0);
        for dx in [-width, width] {
            mesh.positions.push([c[0] + dx, c[1], c[2]]);
            mesh.normals.push([0.0, 0.0, -1.0]);
            mesh.uvs.push([0.5, 0.5]);
            mesh.joints.push([head, arm, 0, 0]);
            mesh.weights.push([on_head, 1.0 - on_head, 0.0, 0.0]);
        }
    }
    let rows = u32::from(SHOULDER_TAIL_ROWS);
    for r in 0..rows - 1 {
        let (a, b) = (first + 2 * r, first + 2 * r + 2);
        mesh.indices
            .extend_from_slice(&[a, a + 1, b + 1, a, b + 1, b]);
    }
}
