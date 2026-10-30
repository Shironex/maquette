//! A charm on a jacket: a fine tube around the spine and a narrow ribbon
//! hanging just in front of it, too near the body to count as hanging off
//! it and too short for a part of its own.

use super::super::builder::Mesh;
use super::super::skeleton::skeleton;
use super::joint_index;

/// Points around the tube.
const AROUND: u8 = 24;
/// Rows of the tube, a centimetre apart.
const TUBE_ROWS: u8 = 21;
/// Rows of the ribbon, a centimetre apart.
const CHARM_ROWS: u8 = 9;

/// Joins a grid of `rows` by `columns` points from `first` with two
/// triangles a cell, the last column back to the first when `closed`.
fn stitch(mesh: &mut Mesh, first: u32, rows: u8, columns: u8, closed: bool) {
    let (rows, columns) = (u32::from(rows), u32::from(columns));
    let span = if closed { columns } else { columns - 1 };
    for r in 0..rows - 1 {
        for c in 0..span {
            let (a, a1) = (
                first + r * columns + c,
                first + r * columns + (c + 1) % columns,
            );
            let (b, b1) = (a + columns, a1 + columns);
            mesh.indices.extend_from_slice(&[a, a1, b1, a, b1, b]);
        }
    }
}

fn point(mesh: &mut Mesh, p: [f32; 3], normal: [f32; 3], bone: u16) {
    mesh.positions.push(p);
    mesh.normals.push(normal);
    mesh.uvs.push([0.5, 0.5]);
    mesh.joints.push([bone, 0, 0, 0]);
    mesh.weights.push([1.0, 0.0, 0.0, 0.0]);
}

/// A tube 6 cm in radius and 20 cm tall around the spine, and a ribbon 2 cm
/// wide and 8 cm tall hanging 2 cm out from it (front left), both weighted wholly to
/// the spine, as an auto rig leaves a charm on a jacket. Built after every
/// box, so it does not go with `split` or a `flaw`.
pub(super) fn charm(mesh: &mut Mesh, world: &[[f32; 3]]) {
    let Some(spine) = joint_index(&skeleton(), "Spine") else {
        return;
    };
    let s = world[spine];
    let bone = u16::try_from(spine).unwrap_or(0);
    let first = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX);
    for r in 0..TUBE_ROWS {
        for k in 0..AROUND {
            let a = std::f32::consts::TAU * f32::from(k) / f32::from(AROUND);
            let (x, z) = (a.sin(), a.cos());
            let y = s[1] + 0.16 - 0.01 * f32::from(r);
            point(
                mesh,
                [s[0] + 0.06 * x, y, s[2] + 0.06 * z],
                [x, 0.0, z],
                bone,
            );
        }
    }
    stitch(mesh, first, TUBE_ROWS, AROUND, true);
    // Front and a little to the left, 22.5 degrees from straight ahead.
    let (out, across) = ([0.383f32, 0.924], [0.924f32, -0.383]);
    let first = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX);
    for r in 0..CHARM_ROWS {
        for c in 0..3u8 {
            let side = 0.01 * (f32::from(c) - 1.0);
            let y = s[1] + 0.1 - 0.01 * f32::from(r);
            let p = [
                s[0] + 0.08 * out[0] + side * across[0],
                y,
                s[2] + 0.08 * out[1] + side * across[1],
            ];
            point(mesh, p, [out[0], 0.0, out[1]], bone);
        }
    }
    stitch(mesh, first, CHARM_ROWS, 3, false);
}
