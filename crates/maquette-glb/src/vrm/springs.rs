//! VRM 1.0 secondary motion: the `VRMC_springBone` extension a written
//! VRM gains when chains of joints were added for its hanging parts (hair,
//! cloth, accessories). Each spring is a chain of joint nodes, parent to
//! child, with how it swings (stiffness, gravity, drag and the radius it
//! keeps from colliders); colliders are spheres and capsules on bones,
//! gathered into named groups that springs list.
//!
//! [`add_springs`] only adds: the JSON gains the extension (nodes are named
//! and looked up by name) and `extensionsUsed` lists it. The joints must
//! already be nodes of the file, each the child of the one before.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::container::{self, invalid, set_extension, use_extension};
use super::parents;
use maquette_base::error::CoreResult;

/// The spring bone extension.
pub const VRMC_SPRING_BONE: &str = "VRMC_springBone";

/// What a hanging part is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SpringKind {
    /// Hair hanging from the head or neck.
    Hair,
    /// A long part hanging from the trunk: a cape, a coat's tails.
    Cloth,
    /// A short part hanging from the trunk: a charm, a strap.
    Accessory,
}

impl SpringKind {
    /// The preset a part of this kind starts with.
    pub fn preset(self) -> SpringPreset {
        match self {
            SpringKind::Hair => SpringPreset::Soft,
            SpringKind::Cloth => SpringPreset::Heavy,
            SpringKind::Accessory => SpringPreset::Stiff,
        }
    }
}

/// How a spring swings, as a starting point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SpringPreset {
    /// Soft with light drag: hair.
    Soft,
    /// More drag and gravity: cloth.
    Heavy,
    /// Stiff and small: charms and straps.
    Stiff,
}

/// One joint's swing, as VRM writes it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpringSettings {
    /// How hard the joint springs back to its rest direction.
    pub stiffness: f64,
    pub gravity_power: f64,
    pub gravity_dir: [f64; 3],
    /// How much of its speed the joint loses each step, 0 to 1.
    pub drag_force: f64,
    /// The radius, in metres, the joint keeps from colliders.
    pub hit_radius: f64,
}

impl SpringPreset {
    pub fn settings(self) -> SpringSettings {
        let (stiffness, gravity_power, drag_force, hit_radius) = match self {
            SpringPreset::Soft => (0.5, 0.2, 0.25, 0.02),
            SpringPreset::Heavy => (0.8, 0.6, 0.55, 0.03),
            SpringPreset::Stiff => (1.6, 0.3, 0.6, 0.01),
        };
        SpringSettings {
            stiffness,
            gravity_power,
            gravity_dir: [0.0, -1.0, 0.0],
            drag_force,
            hit_radius,
        }
    }
}

/// A collider's shape, in its bone's space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ColliderShape {
    Sphere {
        offset: [f64; 3],
        radius: f64,
    },
    Capsule {
        offset: [f64; 3],
        radius: f64,
        tail: [f64; 3],
    },
}

/// A collider on a bone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpringCollider {
    /// The group it belongs to: Head, Neck, Chest, Arms, Hips or Legs.
    pub group: String,
    /// The VRM humanoid bone it sits on.
    pub bone: String,
    /// That bone's joint (node name).
    pub joint: String,
    pub shape: ColliderShape,
}

/// One spring: a chain of joints and how they swing.
#[derive(Debug, Clone, PartialEq)]
pub struct SpringChain {
    pub name: String,
    /// The joint nodes by name, root to tip; the last only gives the chain
    /// its length.
    pub joints: Vec<String>,
    pub settings: SpringSettings,
    /// The collider groups it touches, by name.
    pub collider_groups: Vec<String>,
}

/// Everything `VRMC_springBone` holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpringBones {
    pub springs: Vec<SpringChain>,
    pub colliders: Vec<SpringCollider>,
}

fn round(v: f64) -> f64 {
    (v * 1e5).round() / 1e5
}

fn vec3(v: [f64; 3]) -> Value {
    json!([round(v[0]), round(v[1]), round(v[2])])
}

/// Every node's index by name; the first wins.
fn nodes_by_name(json: &Value) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    if let Some(nodes) = json.get("nodes").and_then(Value::as_array) {
        for (i, node) in nodes.iter().enumerate() {
            if let Some(name) = node.get("name").and_then(Value::as_str) {
                out.entry(name.to_string()).or_insert(i);
            }
        }
    }
    out
}

/// Each node's parent, read from the JSON's children lists.
fn json_parents(json: &Value) -> Vec<Option<usize>> {
    let nodes = json.get("nodes").and_then(Value::as_array);
    let mut out = vec![None; nodes.map_or(0, Vec::len)];
    for (i, node) in nodes.into_iter().flatten().enumerate() {
        for child in node
            .get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(slot) = child
                .as_u64()
                .and_then(|c| usize::try_from(c).ok())
                .and_then(|c| out.get_mut(c))
            {
                *slot = Some(i);
            }
        }
    }
    out
}

fn shape_json(shape: &ColliderShape) -> Value {
    match shape {
        ColliderShape::Sphere { offset, radius } => {
            json!({ "sphere": { "offset": vec3(*offset), "radius": round(*radius) } })
        }
        ColliderShape::Capsule {
            offset,
            radius,
            tail,
        } => json!({ "capsule": {
            "offset": vec3(*offset), "radius": round(*radius), "tail": vec3(*tail)
        } }),
    }
}

/// Adds `VRMC_springBone` to written VRM bytes.
///
/// # Errors
///
/// Refuses a file that is not a GLB, a joint or collider bone the file does
/// not have, a spring with fewer than two joints or whose joints are not
/// each the child of the one before, and settings out of range.
pub fn add_springs(vrm: &[u8], bones: &SpringBones) -> CoreResult<Vec<u8>> {
    let (mut json, bin) = container::split(vrm)?;
    let by_name = nodes_by_name(&json);
    let parent = json_parents(&json);
    let node = |name: &str| {
        by_name
            .get(name)
            .copied()
            .ok_or_else(|| invalid(format!("The file has no node named {name}.")))
    };
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    let mut colliders = Vec::new();
    for (i, c) in bones.colliders.iter().enumerate() {
        colliders.push(json!({ "node": node(&c.joint)?, "shape": shape_json(&c.shape) }));
        match groups.iter_mut().find(|(g, _)| *g == c.group) {
            Some((_, list)) => list.push(i),
            None => groups.push((c.group.clone(), vec![i])),
        }
    }
    let mut springs = Vec::new();
    for spring in &bones.springs {
        let s = &spring.settings;
        if spring.joints.len() < 2 {
            return Err(invalid(format!(
                "The spring {} needs at least two joints.",
                spring.name
            )));
        }
        let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
        if !(ok(s.stiffness, 0.0, 4.0)
            && ok(s.gravity_power, 0.0, 2.0)
            && ok(s.drag_force, 0.0, 1.0)
            && ok(s.hit_radius, 0.0, 0.5))
        {
            return Err(invalid(format!(
                "The spring {} has a setting out of range.",
                spring.name
            )));
        }
        let mut joints = Vec::new();
        let mut last: Option<usize> = None;
        for name in &spring.joints {
            let n = node(name)?;
            if last.is_some() && parent.get(n).copied().flatten() != last {
                return Err(invalid(format!(
                    "In the spring {}, {name} is not the child of the joint before it.",
                    spring.name
                )));
            }
            last = Some(n);
            joints.push(json!({
                "node": n,
                "hitRadius": round(s.hit_radius),
                "stiffness": round(s.stiffness),
                "gravityPower": round(s.gravity_power),
                "gravityDir": vec3(s.gravity_dir),
                "dragForce": round(s.drag_force),
            }));
        }
        let touched: Vec<usize> = spring
            .collider_groups
            .iter()
            .filter_map(|g| groups.iter().position(|(name, _)| name == g))
            .collect();
        springs.push(json!({
            "name": spring.name,
            "joints": joints,
            "colliderGroups": touched,
        }));
    }
    let groups: Vec<Value> = groups
        .into_iter()
        .map(|(name, list)| json!({ "name": name, "colliders": list }))
        .collect();
    let ext = json!({
        "specVersion": "1.0",
        "colliders": colliders,
        "colliderGroups": groups,
        "springs": springs,
    });
    set_extension(&mut json, VRMC_SPRING_BONE, ext);
    use_extension(&mut json, VRMC_SPRING_BONE);
    container::join(&json, bin)
}

/// Reads `VRMC_springBone` back as a runtime would and lists what does not
/// hold; returns how many springs the file has.
pub(super) fn check(json: &Value, doc: &gltf::Document, problems: &mut Vec<String>) -> u32 {
    let Some(ext) = json.get("extensions").and_then(|e| e.get(VRMC_SPRING_BONE)) else {
        return 0;
    };
    if ext.get("specVersion").and_then(Value::as_str) != Some("1.0") {
        problems.push(format!("{VRMC_SPRING_BONE} is not spec version 1.0."));
    }
    let nodes = doc.nodes().len();
    let parent = parents(doc);
    let index = |v: &Value| v.as_u64().and_then(|n| usize::try_from(n).ok());
    let list = |key: &str| {
        ext.get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let colliders = list("colliders");
    for (i, c) in colliders.iter().enumerate() {
        if c.get("node").and_then(index).is_none_or(|n| n >= nodes) {
            problems.push(format!(
                "Spring collider {i} points at a node the file does not have."
            ));
        }
        let shape = c.get("shape");
        let has = |k: &str| shape.and_then(|s| s.get(k)).is_some();
        if !has("sphere") && !has("capsule") {
            problems.push(format!(
                "Spring collider {i} is neither a sphere nor a capsule."
            ));
        }
    }
    let groups = list("colliderGroups");
    for (i, g) in groups.iter().enumerate() {
        let bad = g
            .get("colliders")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(|c| index(c).is_none_or(|c| c >= colliders.len()));
        if bad {
            problems.push(format!(
                "Collider group {i} lists a collider the file does not have."
            ));
        }
    }
    let springs = list("springs");
    for (i, s) in springs.iter().enumerate() {
        let joints: Vec<Option<usize>> = s
            .get("joints")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|j| j.get("node").and_then(index).filter(|n| *n < nodes))
            .collect();
        if joints.len() < 2 || joints.iter().any(Option::is_none) {
            problems.push(format!(
                "Spring {i} needs two or more joints on nodes the file has."
            ));
            continue;
        }
        let chained = joints
            .windows(2)
            .all(|w| parent.get(w[1].unwrap_or(0)).copied().flatten() == w[0]);
        if !chained {
            problems.push(format!(
                "Spring {i}'s joints are not each the child of the one before."
            ));
        }
        let bad_group = s
            .get("colliderGroups")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(|g| index(g).is_none_or(|g| g >= groups.len()));
        if bad_group {
            problems.push(format!(
                "Spring {i} lists a collider group the file does not have."
            ));
        }
    }
    u32::try_from(springs.len()).unwrap_or(u32::MAX)
}
