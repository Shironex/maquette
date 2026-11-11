//! How far each clip carries the model over the ground, in the file's own
//! world axes (glTF: Y up), so a walk can be held against the way the model
//! faces. The node that travels is the one nearest the scene root whose
//! translation keys move it: a root bone when the rig has one that walks,
//! else the hips.

use std::collections::HashMap;

use gltf::animation::Interpolation;
use gltf::animation::util::ReadOutputs;
use serde::{Deserialize, Serialize};

use crate::math::{self, IDENTITY, Mat4, Vec3};

/// A node travels when its keys carry it this share of the model's height
/// over the ground; less is a sway in place.
pub const TRAVEL_SHARE: f32 = 0.05;

/// One clip that travels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipTravel {
    pub clip: String,
    /// The node that carries it.
    pub node: String,
    /// From the first key to the last over the ground, along world X and Z,
    /// in file units.
    pub along: [f32; 2],
}

/// Each node's parent, by index.
fn parents(doc: &gltf::Document) -> HashMap<usize, usize> {
    doc.nodes()
        .flat_map(|n| n.children().map(move |c| (c.index(), n.index())))
        .collect()
}

fn depth(parents: &HashMap<usize, usize>, node: usize) -> usize {
    let mut d = 0;
    let mut at = node;
    while let Some(p) = parents.get(&at) {
        d += 1;
        at = *p;
        if d > parents.len() {
            break;
        }
    }
    d
}

/// The first and last translation key of a channel.
fn ends(outputs: ReadOutputs<'_>, cubic: bool) -> Option<(Vec3, Vec3)> {
    let ReadOutputs::Translations(it) = outputs else {
        return None;
    };
    let values: Vec<Vec3> = it.collect();
    // A cubic spline stores an in tangent, the value and an out tangent per key.
    if cubic {
        Some((*values.get(1)?, *values.get(values.len().checked_sub(2)?)?))
    } else {
        Some((*values.first()?, *values.last()?))
    }
}

/// The clips that travel, in file order. Keys in compressed buffers are not
/// read, so a packed file answers none.
pub(super) fn clip_travel(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    worlds: &[Mat4],
    (height, compressed): (f32, bool),
) -> Vec<ClipTravel> {
    if compressed || height <= 0.0 {
        return Vec::new();
    }
    let parents = parents(doc);
    let mut out = Vec::new();
    for (i, anim) in doc.animations().enumerate() {
        let mut moves: Vec<(usize, usize, [f32; 2])> = Vec::new();
        for channel in anim.channels() {
            let node = channel.target().node();
            let reader = channel.reader(|b| buffers.get(b.index()).and_then(Option::as_deref));
            let cubic = channel.sampler().interpolation() == Interpolation::CubicSpline;
            let Some((first, last)) = reader.read_outputs().and_then(|o| ends(o, cubic)) else {
                continue;
            };
            // The keys are in the parent's frame: its rest matrix takes them to the world.
            let parent = parents
                .get(&node.index())
                .and_then(|p| worlds.get(*p))
                .copied()
                .unwrap_or(IDENTITY);
            let (a, b) = (math::point(&parent, first), math::point(&parent, last));
            let along = [b[0] - a[0], b[2] - a[2]];
            if along[0].hypot(along[1]) >= TRAVEL_SHARE * height {
                moves.push((depth(&parents, node.index()), node.index(), along));
            }
        }
        let Some((_, node, along)) = moves.into_iter().min_by_key(|m| (m.0, m.1)) else {
            continue;
        };
        out.push(ClipTravel {
            clip: anim
                .name()
                .map_or_else(|| format!("Animation {}", i + 1), str::to_string),
            node: doc
                .nodes()
                .nth(node)
                .and_then(|n| n.name().map(str::to_string))
                .unwrap_or_else(|| format!("node {node}")),
            along,
        });
    }
    out
}
