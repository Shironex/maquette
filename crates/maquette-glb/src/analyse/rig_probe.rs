//! What the rig doctor reads when a rigged model arrives, in a blink and
//! without Blender: whether its one skinned primitive can be patched byte
//! for byte, how much hand and arm weight sits far from that limb (the same
//! count as the character rig skill's `probe.py`: hair and capes hang on arm
//! bones as often as on fingers) and how many connected islands the mesh
//! has. A mesh that is one island must not get the per island fix.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use super::JointCount;
use super::rig::bare_joint_name;
use crate::math::{self, Aabb, IDENTITY, Mat4, Vec3};

/// A hand vertex is far beyond this share of the model's height.
const HAND_FAR: f32 = 0.15;
/// An arm vertex is far beyond this share of the height from the arm chain.
const ARM_FAR: f32 = 0.10;
/// A joint counts for a vertex above this weight.
const MIN_WEIGHT: f32 = 0.05;
/// How close two vertices sit to be welded across a UV seam.
const WELD: f32 = 1e-5;
/// One island holding this share of the vertices makes the mesh one island.
const ONE_ISLAND_SHARE: f32 = 0.9;
/// Stray vertices above the neck less this share of the height are hair.
const HAIR_DROP: f32 = 0.12;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct RigProbe {
    /// Mesh primitives with skin weights.
    pub skinned_primitives: u32,
    /// The names of the meshes they belong to.
    pub skinned_meshes: Vec<String>,
    /// Vertex data shares one buffer view, so weights cannot be patched.
    pub interleaved: bool,
    /// Vertices with hand or finger weight far from that hand.
    pub hand_far: u32,
    /// Vertices with arm weight far from that arm.
    pub arm_far: u32,
    /// The bones holding them, most first.
    pub far_bones: Vec<JointCount>,
    /// How many of them are hair, by height.
    pub far_hair: u32,
    pub islands: u32,
    pub largest_island_share: f32,
    pub one_island: bool,
    /// The model's height in file units.
    pub height: f32,
}

fn bind_of(skin: &gltf::Skin<'_>, buffers: &[Option<Vec<u8>>], worlds: &[Mat4]) -> Vec<Vec3> {
    let ibms: Vec<Mat4> = skin
        .reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()))
        .read_inverse_bind_matrices()
        .map(Iterator::collect)
        .unwrap_or_default();
    skin.joints()
        .enumerate()
        .map(|(i, joint)| {
            ibms.get(i)
                .and_then(math::invert_affine)
                .map(|m| math::translation(&m))
                .unwrap_or_else(|| {
                    math::translation(&worlds.get(joint.index()).copied().unwrap_or(IDENTITY))
                })
        })
        .collect()
}

fn accessor_interleaved(accessor: &gltf::Accessor<'_>) -> bool {
    accessor
        .view()
        .and_then(|v| v.stride())
        .is_some_and(|stride| stride != accessor.size())
}

fn primitive_interleaved(prim: &gltf::Primitive<'_>) -> bool {
    prim.attributes().any(|(_, a)| accessor_interleaved(&a))
}

struct Sets(Vec<u32>);

impl Sets {
    fn find(&mut self, mut x: u32) -> u32 {
        while self.0[x as usize] != x {
            let up = self.0[self.0[x as usize] as usize];
            self.0[x as usize] = up;
            x = up;
        }
        x
    }

    fn join(&mut self, a: u32, b: u32) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0[a as usize] = b;
        }
    }
}

/// How many connected islands the triangles make, welding across seams, and
/// the largest one's share of the vertices.
fn islands(positions: &[Vec3], indices: &[u32]) -> (u32, f32) {
    let mut welded: HashMap<[i64; 3], u32> = HashMap::new();
    let ids: Vec<u32> = positions
        .iter()
        .map(|p| {
            let key = p.map(|c| (f64::from(c) / f64::from(WELD)).round() as i64);
            let next = u32::try_from(welded.len()).unwrap_or(u32::MAX);
            *welded.entry(key).or_insert(next)
        })
        .collect();
    let mut sets = Sets((0..u32::try_from(welded.len()).unwrap_or(0)).collect());
    for tri in indices.chunks_exact(3) {
        let at = |i: u32| ids.get(i as usize).copied();
        if let (Some(a), Some(b), Some(c)) = (at(tri[0]), at(tri[1]), at(tri[2])) {
            sets.join(a, b);
            sets.join(b, c);
        }
    }
    let mut sizes: HashMap<u32, u32> = HashMap::new();
    for id in &ids {
        let root = sets.find(*id);
        *sizes.entry(root).or_default() += 1;
    }
    let largest = sizes.values().copied().max().unwrap_or(0);
    let share = largest as f32 / positions.len().max(1) as f32;
    (u32::try_from(sizes.len()).unwrap_or(u32::MAX), share)
}

fn chain_distance(p: Vec3, chain: &[Vec3]) -> f32 {
    chain
        .windows(2)
        .map(|w| math::segment_distance(p, w[0], w[1]))
        .fold(f32::INFINITY, f32::min)
}

/// The far counts of one skinned primitive.
struct Far<'a> {
    bare: Vec<&'a str>,
    bind: &'a [Vec3],
    positions: &'a [Vec3],
    height: f32,
    neck: f32,
}

impl Far<'_> {
    fn joint(&self, name: &str) -> Option<usize> {
        self.bare.iter().position(|b| *b == name)
    }

    fn count(
        &self,
        joints: &[[u16; 4]],
        weights: &[[f32; 4]],
        out: &mut RigProbe,
        bones: &mut BTreeMap<String, u32>,
    ) {
        for side in ["Left", "Right"] {
            let hand = self
                .joint(&format!("{side}Hand"))
                .and_then(|i| self.bind.get(i));
            let arm_ids: Vec<usize> = ["Arm", "ForeArm"]
                .iter()
                .filter_map(|n| self.joint(&format!("{side}{n}")))
                .collect();
            let chain: Vec<Vec3> = ["Arm", "ForeArm", "Hand"]
                .iter()
                .filter_map(|n| self.joint(&format!("{side}{n}")))
                .filter_map(|i| self.bind.get(i).copied())
                .collect();
            let hand_ids: Vec<usize> = self
                .bare
                .iter()
                .enumerate()
                .filter(|(_, b)| b.starts_with(&format!("{side}Hand")))
                .map(|(i, _)| i)
                .collect();
            for ((p, j), w) in self.positions.iter().zip(joints).zip(weights) {
                let mass = |ids: &[usize]| -> f32 {
                    j.iter()
                        .zip(w)
                        .filter(|(joint, _)| ids.contains(&usize::from(**joint)))
                        .map(|(_, v)| *v)
                        .sum()
                };
                let mut far_on = |ids: &[usize]| {
                    for (joint, v) in j.iter().zip(w) {
                        let id = usize::from(*joint);
                        if ids.contains(&id) && *v > MIN_WEIGHT {
                            let name = self.bare[id].to_string();
                            *bones.entry(name).or_default() += 1;
                        }
                    }
                };
                if let Some(hand) = hand
                    && mass(&hand_ids) > MIN_WEIGHT
                    && math::distance(*p, *hand) > HAND_FAR * self.height
                {
                    out.hand_far += 1;
                    out.far_hair += u32::from(p[1] > self.neck - HAIR_DROP * self.height);
                    far_on(&hand_ids);
                }
                if chain.len() >= 2
                    && mass(&arm_ids) > MIN_WEIGHT
                    && chain_distance(*p, &chain) > ARM_FAR * self.height
                {
                    out.arm_far += 1;
                    out.far_hair += u32::from(p[1] > self.neck - HAIR_DROP * self.height);
                    far_on(&arm_ids);
                }
            }
        }
    }
}

/// Reads a model's rig for the rig doctor; `None` without a skin. Compressed
/// geometry keeps only the counts of what is skinned.
pub(super) fn probe(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    worlds: &[Mat4],
    compressed: bool,
) -> Option<RigProbe> {
    let mut out = RigProbe::default();
    let mut first = None;
    let mut any_skin = false;
    for node in doc.nodes() {
        let Some(mesh) = node.mesh() else { continue };
        if node.skin().is_none() {
            continue;
        }
        any_skin = true;
        let mut counted = false;
        for prim in mesh.primitives() {
            if prim.get(&gltf::Semantic::Joints(0)).is_some() {
                out.skinned_primitives += 1;
                counted = true;
                out.interleaved |= primitive_interleaved(&prim);
                first.get_or_insert((node.clone(), prim));
            }
        }
        if counted {
            out.skinned_meshes
                .push(mesh.name().unwrap_or("mesh").to_string());
        }
    }
    if !any_skin && doc.skins().next().is_none() {
        return None;
    }
    let Some((node, prim)) = first else {
        return Some(out);
    };
    if compressed || out.skinned_primitives != 1 {
        return Some(out);
    }
    let skin = node.skin()?;
    let reader = prim.reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()));
    let positions: Vec<Vec3> = reader.read_positions()?.collect();
    let joints: Vec<[u16; 4]> = reader.read_joints(0)?.into_u16().collect();
    let weights: Vec<[f32; 4]> = reader.read_weights(0)?.into_f32().collect();
    let indices: Vec<u32> = reader
        .read_indices()
        .map(|i| i.into_u32().collect())
        .unwrap_or_else(|| (0..u32::try_from(positions.len()).unwrap_or(0)).collect());
    let mut bounds = Aabb::EMPTY;
    for p in &positions {
        bounds.add(*p);
    }
    out.height = bounds.size()[1].max(1e-6);
    (out.islands, out.largest_island_share) = islands(&positions, &indices);
    out.one_island = out.largest_island_share >= ONE_ISLAND_SHARE;
    let names: Vec<String> = skin
        .joints()
        .map(|j| j.name().unwrap_or("joint").to_string())
        .collect();
    let bind = bind_of(&skin, buffers, worlds);
    let bare: Vec<&str> = names.iter().map(|n| bare_joint_name(n)).collect();
    let neck = bare
        .iter()
        .position(|b| *b == "Neck")
        .and_then(|i| bind.get(i))
        .map_or(bounds.size()[1], |p| p[1]);
    let far = Far {
        bare,
        bind: &bind,
        positions: &positions,
        height: out.height,
        neck,
    };
    let mut bones = BTreeMap::new();
    far.count(&joints, &weights, &mut out, &mut bones);
    let mut listed: Vec<JointCount> = bones
        .into_iter()
        .map(|(name, vertices)| JointCount { name, vertices })
        .collect();
    listed.sort_by(|a, b| b.vertices.cmp(&a.vertices).then(a.name.cmp(&b.name)));
    out.far_bones = listed;
    Some(out)
}
