//! A right hand of one piece whose finger joints sit bunched at the wrist,
//! as a broken auto rig leaves them: the mesh has a palm, four fingers and
//! a thumb, all weighted to the hand, and no finger bone lies in a finger.

use std::collections::BTreeMap;

use super::super::builder::Mesh;
use super::super::skeleton::skeleton;
use super::{below, joint_index, turn_xy};

/// The share of each finger joint's reach from the wrist it keeps.
const BUNCH: f32 = 0.08;
/// The grid the hand is built on, metres.
const CELL: f32 = 0.01;
/// Half the hand's thickness, metres.
const HALF: f32 = 0.01;

/// Pulls the right hand's finger joints towards the wrist, to [`BUNCH`] of
/// where they were.
pub(super) fn bunch_fingers(joints: &[(String, Option<usize>, [f32; 3])], world: &mut [[f32; 3]]) {
    let Some(hand) = joint_index(joints, "RightHand") else {
        return;
    };
    let h = world[hand];
    for (i, at) in world.iter_mut().enumerate() {
        if below(joints, i, hand) {
            for (v, c) in at.iter_mut().zip(h) {
                *v = c + (*v - c) * BUNCH;
            }
        }
    }
}

/// True when joint `p` is the right hand or one of its fingers, so the box
/// hanging from it is left out in favour of the hand of one piece.
pub(super) fn in_hand(joints: &[(String, Option<usize>, [f32; 3])], p: usize) -> bool {
    joint_index(&skeleton(), "RightHand").is_some_and(|hand| p == hand || below(joints, p, hand))
}

/// The cells the hand covers: `(along, across)`, along the hand from the
/// wrist (towards -X in the T-pose) and across it (+Z is the thumb's side).
fn cells() -> Vec<(i32, i32)> {
    let mut out = vec![];
    // The palm, 8 cm long and 11 cm wide.
    for u in 0..8 {
        out.extend((-5..6).map(|v| (u, v)));
    }
    // The fingers, 2 cm wide with 1 cm between them: index, middle, ring,
    // little, then the thumb out of the palm's side near the wrist.
    for (across, length) in [(4, 7), (1, 8), (-2, 7), (-5, 6)] {
        for u in 8..8 + length {
            out.extend([(u, across), (u, across + 1)]);
        }
    }
    for v in 6..12 {
        out.extend([(2, v), (3, v)]);
    }
    out
}

/// Builds the right hand as one closed slab over [`cells`], every vertex
/// weighted wholly to the hand, turned down with the arm by `arm_drop`.
pub(super) fn bunched_hand(mesh: &mut Mesh, world: &[[f32; 3]], arm_drop: f32) {
    let joints = skeleton();
    let (Some(arm), Some(hand)) = (
        joint_index(&joints, "RightArm"),
        joint_index(&joints, "RightHand"),
    ) else {
        return;
    };
    let pivot = world[arm];
    // The wrist in the T-pose; the right arm dropped by `arm_drop`.
    let wrist = turn_xy(world[hand], pivot, -arm_drop);
    let joint = u16::try_from(hand).unwrap_or(0);
    let filled = cells();
    let has = |c: (i32, i32)| filled.contains(&c);
    let mut index: BTreeMap<(i32, i32, bool), u32> = BTreeMap::new();
    let mut corner = |mesh: &mut Mesh, u: i32, v: i32, top: bool| -> u32 {
        *index.entry((u, v, top)).or_insert_with(|| {
            #[expect(clippy::cast_precision_loss, reason = "a small grid")]
            let p = [
                wrist[0] - CELL * u as f32,
                wrist[1] + if top { HALF } else { -HALF },
                wrist[2] + CELL * v as f32,
            ];
            mesh.positions.push(turn_xy(p, pivot, arm_drop));
            mesh.normals.push(turn_xy(
                [0.0, if top { 1.0 } else { -1.0 }, 0.0],
                [0.0; 3],
                arm_drop,
            ));
            mesh.uvs.push([0.5, 0.5]);
            mesh.joints.push([joint, 0, 0, 0]);
            mesh.weights.push([1.0, 0.0, 0.0, 0.0]);
            u32::try_from(mesh.positions.len() - 1).unwrap_or(u32::MAX)
        })
    };
    let quad = |mesh: &mut Mesh, q: [u32; 4]| {
        mesh.indices
            .extend_from_slice(&[q[0], q[1], q[2], q[0], q[2], q[3]]);
    };
    for &(u, v) in &filled {
        let top =
            [(u, v), (u, v + 1), (u + 1, v + 1), (u + 1, v)].map(|(a, b)| corner(mesh, a, b, true));
        let bottom = [(u, v), (u + 1, v), (u + 1, v + 1), (u, v + 1)]
            .map(|(a, b)| corner(mesh, a, b, false));
        quad(mesh, top);
        quad(mesh, bottom);
        // A wall wherever the next cell is empty: its edge, bottom to top.
        for (next, a, b) in [
            ((u - 1, v), (u, v + 1), (u, v)),
            ((u + 1, v), (u + 1, v), (u + 1, v + 1)),
            ((u, v - 1), (u, v), (u + 1, v)),
            ((u, v + 1), (u + 1, v + 1), (u, v + 1)),
        ] {
            if !has(next) {
                let wall = [
                    corner(mesh, a.0, a.1, false),
                    corner(mesh, b.0, b.1, false),
                    corner(mesh, b.0, b.1, true),
                    corner(mesh, a.0, a.1, true),
                ];
                quad(mesh, wall);
            }
        }
    }
}
