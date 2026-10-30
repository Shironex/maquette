//! A sleeve fused to the hip: a ribbon whose ends are the wrist end of the
//! left forearm's box and the top of the hip's box, so lifting the arm can
//! only stretch it somewhere.

use super::super::builder::Mesh;
use super::super::skeleton::skeleton;
use super::{joint_index, turn_xy};

/// The ribbon's rows, wrist to hip.
const ROWS: u8 = 16;

/// The ribbon: its first row is the bottom edge of the left forearm box's
/// wrist end, its last the top outer edge of the box from the hips to the
/// left thigh (worked out as `Mesh::cuboid` builds them), so welding joins
/// it to both. Row `t` (0 at the wrist, 1 at the hip) is weighted `1 - t`
/// to the forearm and `t` to the hips, as an auto rig blends a sheet it
/// cannot tell from the sleeve. `arm_drop` is how far the arms dropped.
pub(super) fn sleeve_bridge(mesh: &mut Mesh, world: &[[f32; 3]], arm_drop: f32) {
    let joints = skeleton();
    let (Some(arm), Some(fore), Some(hand), Some(hips), Some(leg)) = (
        joint_index(&joints, "LeftArm"),
        joint_index(&joints, "LeftForeArm"),
        joint_index(&joints, "LeftHand"),
        joint_index(&joints, "Hips"),
        joint_index(&joints, "LeftUpLeg"),
    ) else {
        return;
    };
    // The forearm box is built in the T-pose and turned down with the arm.
    let pivot = world[arm];
    let (a, b) = (
        turn_xy(world[fore], pivot, arm_drop),
        turn_xy(world[hand], pivot, arm_drop),
    );
    let wrist = |dz: f32| {
        turn_xy(
            [
                a[0].max(b[0]),
                (a[1] + b[1]) / 2.0 - 0.02,
                (a[2] + b[2]) / 2.0 + dz,
            ],
            pivot,
            -arm_drop,
        )
    };
    let (h, l) = (world[hips], world[leg]);
    let hip = |dz: f32| [h[0].max(l[0]), h[1].max(l[1]), (h[2] + l[2]) / 2.0 + dz];
    let (forearm, hips) = (
        u16::try_from(fore).unwrap_or(0),
        u16::try_from(hips).unwrap_or(0),
    );
    let first = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX);
    for r in 0..=ROWS {
        let t = f32::from(r) / f32::from(ROWS);
        for dz in [-0.02, 0.02] {
            let (p, q) = (wrist(dz), hip(dz));
            mesh.positions
                .push([0, 1, 2].map(|k| p[k] + (q[k] - p[k]) * t));
            mesh.normals.push([0.0, 0.0, 1.0]);
            mesh.uvs.push([0.5, 0.5]);
            mesh.joints.push([forearm, hips, 0, 0]);
            mesh.weights.push([1.0 - t, t, 0.0, 0.0]);
        }
    }
    for r in 0..u32::from(ROWS) {
        let (a, b) = (first + 2 * r, first + 2 * r + 2);
        mesh.indices
            .extend_from_slice(&[a, a + 1, b + 1, a, b + 1, b]);
    }
}
