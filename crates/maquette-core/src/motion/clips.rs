//! Animation clips read from a model file: their length, key rate, size,
//! how far the root travels and whether they loop.

use std::collections::{BTreeMap, BTreeSet};

use gltf::animation::util::ReadOutputs;
use gltf::animation::{Interpolation, Property};
use serde_json::Value;

use super::Clip;

/// Key rates a sampled clip snaps to when it is within 2% of one.
const RATES: [f32; 9] = [12.0, 15.0, 24.0, 25.0, 30.0, 48.0, 50.0, 60.0, 120.0];
/// How close the first and last key must be for a clip to loop.
const LOOP_TOLERANCE: f32 = 1e-3;

/// The JSON of a GLB or a `.gltf` file.
pub(super) fn json_of(bytes: &[u8]) -> Option<Value> {
    if bytes.get(..4) == Some(b"glTF".as_slice()) {
        let len = u32::from_le_bytes(bytes.get(12..16)?.try_into().ok()?) as usize;
        return serde_json::from_slice(bytes.get(20..20 + len)?).ok();
    }
    serde_json::from_slice(bytes).ok()
}

fn component_bytes(kind: u64) -> f64 {
    match kind {
        5120 | 5121 => 1.0,
        5122 | 5123 => 2.0,
        _ => 4.0,
    }
}

fn components(kind: &str) -> f64 {
    match kind {
        "VEC2" => 2.0,
        "VEC3" => 3.0,
        "VEC4" | "MAT2" => 4.0,
        "MAT3" => 9.0,
        "MAT4" => 16.0,
        _ => 1.0,
    }
}

/// What one accessor costs in the file: its share of a meshopt compressed
/// buffer view when the view is compressed, else its own bytes.
fn accessor_bytes(json: &Value, index: u64) -> f64 {
    let accessor = &json["accessors"][usize::try_from(index).unwrap_or(usize::MAX)];
    let count = accessor["count"].as_f64().unwrap_or(0.0);
    let raw = count
        * component_bytes(accessor["componentType"].as_u64().unwrap_or(5126))
        * components(accessor["type"].as_str().unwrap_or("SCALAR"));
    let Some(view) = accessor["bufferView"]
        .as_u64()
        .and_then(|v| json["bufferViews"].get(usize::try_from(v).ok()?))
    else {
        return raw;
    };
    let compressed = ["EXT_meshopt_compression", "KHR_meshopt_compression"]
        .iter()
        .find_map(|ext| view["extensions"][ext]["byteLength"].as_f64());
    match (compressed, view["byteLength"].as_f64()) {
        (Some(packed), Some(whole)) if whole > 0.0 => packed * (raw / whole).min(1.0),
        _ => raw,
    }
}

/// The accessors each animation reads, by animation index.
fn accessors_of(json: &Value) -> Vec<BTreeSet<u64>> {
    json["animations"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|a| {
                    a["samplers"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .flat_map(|s| [s["input"].as_u64(), s["output"].as_u64()])
                        .flatten()
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Keyframe bytes per clip name, and every clip together (an accessor two
/// clips share counts once there).
pub(super) fn clip_bytes(json: &Value) -> (BTreeMap<String, f64>, f64) {
    let sets = accessors_of(json);
    let names = json["animations"].as_array().cloned().unwrap_or_default();
    let mut per = BTreeMap::new();
    let mut all = BTreeSet::new();
    for (i, set) in sets.iter().enumerate() {
        let name = names
            .get(i)
            .and_then(|a| a["name"].as_str())
            .map_or_else(|| default_name(i), str::to_string);
        per.insert(name, set.iter().map(|a| accessor_bytes(json, *a)).sum());
        all.extend(set.iter().copied());
    }
    let total = all.iter().map(|a| accessor_bytes(json, *a)).sum();
    (per, total)
}

pub(super) fn default_name(index: usize) -> String {
    format!("Animation {}", index + 1)
}

/// The rate keys were sampled at: a common rate when within 2%, else the
/// nearest whole number.
pub(super) fn key_rate(keys: u32, duration: f32) -> f32 {
    if keys < 2 || duration <= 0.0 {
        return 0.0;
    }
    #[expect(clippy::cast_precision_loss, reason = "key counts stay small")]
    let raw = (keys - 1) as f32 / duration;
    RATES
        .iter()
        .copied()
        .find(|r| (raw - r).abs() <= r * 0.02)
        .unwrap_or_else(|| raw.round())
}

/// Each node's depth below the scene root, for finding a clip's root.
fn depths(doc: &gltf::Document) -> Vec<usize> {
    let mut depth = vec![usize::MAX; doc.nodes().len()];
    let mut stack: Vec<(gltf::Node<'_>, usize)> = doc
        .scenes()
        .flat_map(|s| s.nodes().map(|n| (n, 0)))
        .collect();
    while let Some((node, d)) = stack.pop() {
        if depth[node.index()] <= d {
            continue;
        }
        depth[node.index()] = d;
        stack.extend(node.children().map(|c| (c, d + 1)));
    }
    depth
}

/// The values of a channel at its first and last key.
fn ends(outputs: ReadOutputs<'_>, cubic: bool) -> Option<(Vec<f32>, Vec<f32>)> {
    let values: Vec<Vec<f32>> = match outputs {
        ReadOutputs::Translations(it) | ReadOutputs::Scales(it) => it.map(|v| v.to_vec()).collect(),
        ReadOutputs::Rotations(r) => r.into_f32().map(|v| v.to_vec()).collect(),
        ReadOutputs::MorphTargetWeights(w) => w.into_f32().map(|v| vec![v]).collect(),
    };
    // A cubic spline stores an in tangent, the value and an out tangent per key.
    let (first, last) = if cubic {
        (values.get(1)?, values.get(values.len().checked_sub(2)?)?)
    } else {
        (values.first()?, values.last()?)
    };
    Some((first.clone(), last.clone()))
}

fn same(a: &[f32], b: &[f32], rotation: bool) -> bool {
    let close = |sign: f32| {
        a.iter()
            .zip(b)
            .all(|(x, y)| (x - sign * y).abs() < LOOP_TOLERANCE)
    };
    // q and -q are the same rotation.
    close(1.0) || (rotation && close(-1.0))
}

/// The clips of a parsed model; `buffers` are its buffers in order.
pub(super) fn read(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    bytes: &BTreeMap<String, f64>,
) -> Vec<Clip> {
    let depth = depths(doc);
    doc.animations()
        .enumerate()
        .map(|(i, anim)| {
            let name = anim.name().map_or_else(|| default_name(i), str::to_string);
            let mut keys = 0u32;
            let mut duration = 0.0f32;
            let mut targets = BTreeSet::new();
            // The translated node nearest the scene root carries the root motion.
            let mut root: Option<(usize, f32)> = None;
            let mut loops = true;
            let mut readable = false;
            let channels = anim.channels().collect::<Vec<_>>();
            for channel in &channels {
                let input = channel.sampler().input();
                keys = keys.max(u32::try_from(input.count()).unwrap_or(u32::MAX));
                if let Some(max) = input.max().and_then(|m| m.get(0).and_then(Value::as_f64)) {
                    #[expect(clippy::cast_possible_truncation, reason = "seconds fit f32")]
                    let max = max as f32;
                    duration = duration.max(max);
                }
                let node = channel.target().node().index();
                targets.insert(node);
                if channel.target().property() == Property::Translation
                    && root.is_none_or(|(n, _)| depth[node] < depth[n])
                {
                    root = Some((node, 0.0));
                }
            }
            let mut travel = None;
            for channel in &channels {
                let reader = channel.reader(|b| buffers.get(b.index()).and_then(Option::as_deref));
                let cubic = channel.sampler().interpolation() == Interpolation::CubicSpline;
                let Some((first, last)) = reader.read_outputs().and_then(|o| ends(o, cubic)) else {
                    continue;
                };
                readable = true;
                let property = channel.target().property();
                let node = channel.target().node().index();
                if property == Property::Translation && root.is_some_and(|(n, _)| n == node) {
                    let dx = last[0] - first[0];
                    let dz =
                        last.get(2).copied().unwrap_or(0.0) - first.get(2).copied().unwrap_or(0.0);
                    let dy =
                        last.get(1).copied().unwrap_or(0.0) - first.get(1).copied().unwrap_or(0.0);
                    travel = Some(dx.hypot(dz));
                    // A root that only travels still loops; one that climbs does not.
                    if dy.abs() >= LOOP_TOLERANCE {
                        loops = false;
                    }
                    continue;
                }
                if !same(&first, &last, property == Property::Rotation) {
                    loops = false;
                }
            }
            let fps = key_rate(keys, duration);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "frames of a clip are few and positive"
            )]
            let frames = (duration * fps).round() as u32;
            Clip {
                bytes: bytes.get(&name).copied().unwrap_or(0.0),
                shipped_bytes: None,
                name,
                index: u32::try_from(i).unwrap_or(u32::MAX),
                duration,
                keys,
                frames,
                fps,
                channels: u32::try_from(channels.len()).unwrap_or(u32::MAX),
                targets: u32::try_from(targets.len()).unwrap_or(u32::MAX),
                root_travel: travel.filter(|t| *t >= 0.01),
                loops: readable && loops,
            }
        })
        .collect()
}
