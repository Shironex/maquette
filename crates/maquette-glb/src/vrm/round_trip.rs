//! Reading a written VRM back: the bytes open again with the `gltf` crate
//! (validation on), and `VRMC_vrm` parses into a struct that is checked
//! the way a runtime would trip on it. The spec version is 1.0, every
//! human bone points at a node that exists and no node twice, the 15
//! required bones are there, each bone's nearest humanoid ancestor is the
//! one VRM's tree puts above it, the meta has a name, an author and the
//! licence URL, `extensionsUsed` lists every extension the file uses,
//! there are no animations, and every texture an MToon material points at
//! is there with an image behind it.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::bones::{self, Mapped};
use super::mtoon::VRMC_MATERIALS_MTOON;
use super::write::VRMC_VRM;
use super::{BoneNaming, container, parents};

/// What reading a written VRM back found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RoundTrip {
    /// No problem: the file reads back as the VRM it should be.
    pub passed: bool,
    /// What failed, as sentences.
    pub problems: Vec<String>,
    /// Human bones the file names.
    pub bones: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VrmRead {
    spec_version: Option<String>,
    meta: Option<MetaRead>,
    humanoid: Option<HumanoidRead>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HumanoidRead {
    #[serde(default)]
    human_bones: BTreeMap<String, HumanBoneRead>,
}

#[derive(Debug, Deserialize)]
struct HumanBoneRead {
    node: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaRead {
    name: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    license_url: Option<String>,
}

/// Reads `bytes` back as VRM 1.0 and lists what does not hold.
pub fn round_trip(bytes: &[u8]) -> RoundTrip {
    let mut problems = Vec::new();
    let bones = check(bytes, &mut problems);
    RoundTrip {
        passed: problems.is_empty(),
        problems,
        bones,
    }
}

/// Runs every check, pushing what fails; returns the human bones named.
fn check(bytes: &[u8], problems: &mut Vec<String>) -> u32 {
    let opened = std::panic::catch_unwind(|| gltf::Gltf::from_slice(bytes));
    let doc = match opened {
        Ok(Ok(gltf)) => gltf.document,
        Ok(Err(e)) => {
            problems.push(format!("The file does not open as valid glTF: {e}."));
            return 0;
        }
        Err(_) => {
            problems.push("The file does not open as valid glTF.".to_string());
            return 0;
        }
    };
    let json = match container::split(bytes) {
        Ok((json, _)) => json,
        Err(e) => {
            problems.push(e.to_string());
            return 0;
        }
    };
    if doc.animations().len() > 0 {
        problems.push(format!(
            "The file still has {} animations; a VRM carries none.",
            doc.animations().len()
        ));
    }
    check_extensions(&json, problems);
    check_mtoon(&json, problems);
    let Some(raw) = json.get("extensions").and_then(|e| e.get(VRMC_VRM)) else {
        problems.push(format!("The file has no {VRMC_VRM} extension."));
        return 0;
    };
    let vrm: VrmRead = match serde_json::from_value(raw.clone()) {
        Ok(vrm) => vrm,
        Err(e) => {
            problems.push(format!("{VRMC_VRM} does not read: {e}."));
            return 0;
        }
    };
    if vrm.spec_version.as_deref() != Some("1.0") {
        problems.push(format!(
            "{VRMC_VRM} says spec version {}, not 1.0.",
            vrm.spec_version.as_deref().unwrap_or("none")
        ));
    }
    check_meta(vrm.meta.as_ref(), problems);
    let named = vrm.humanoid.map(|h| h.human_bones).unwrap_or_default();
    check_bones(&doc, &named, problems);
    u32::try_from(named.len()).unwrap_or(u32::MAX)
}

fn check_meta(meta: Option<&MetaRead>, problems: &mut Vec<String>) {
    let Some(meta) = meta else {
        problems.push("The VRM has no meta.".to_string());
        return;
    };
    if meta.name.as_deref().is_none_or(|n| n.trim().is_empty()) {
        problems.push("The meta has no name.".to_string());
    }
    if !meta.authors.iter().any(|a| !a.trim().is_empty()) {
        problems.push("The meta has no author.".to_string());
    }
    if meta
        .license_url
        .as_deref()
        .is_none_or(|u| u.trim().is_empty())
    {
        problems.push("The meta has no licence URL.".to_string());
    }
}

fn check_bones(
    doc: &gltf::Document,
    named: &BTreeMap<String, HumanBoneRead>,
    problems: &mut Vec<String>,
) {
    let nodes = doc.nodes().len();
    let human = bones::human_bones();
    let mut mapped: Vec<Mapped> = Vec::new();
    let mut seen: BTreeMap<usize, &str> = BTreeMap::new();
    for (bone, read) in named {
        if !human.iter().any(|b| b.name == *bone) {
            problems.push(format!("{bone} is not a VRM 1.0 humanoid bone."));
            continue;
        }
        let Some(node) = read
            .node
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n < nodes)
        else {
            problems.push(format!(
                "{bone} points at node {}, which the file does not have.",
                read.node
                    .map_or_else(|| "none".to_string(), |n| n.to_string())
            ));
            continue;
        };
        if let Some(first) = seen.insert(node, bone) {
            problems.push(format!("{first} and {bone} both point at node {node}."));
            continue;
        }
        mapped.push(Mapped {
            bone: bone.clone(),
            node,
            naming: BoneNaming::Vrm,
        });
    }
    let missing: Vec<&str> = human
        .iter()
        .filter(|b| b.required && !named.contains_key(&b.name))
        .map(|b| b.name.as_str())
        .collect();
    if !missing.is_empty() {
        problems.push(format!("Required bones missing: {}.", missing.join(", ")));
    }
    for wrong in bones::wrong_parents(&human, &mapped, &parents(doc)) {
        problems.push(format!(
            "{} sits under {} in the node tree; VRM puts it under {}.",
            wrong.bone,
            wrong.found.as_deref().unwrap_or("no humanoid bone"),
            wrong.expected.as_deref().unwrap_or("no humanoid bone"),
        ));
    }
}

/// Every extension name used anywhere in the JSON.
fn used_extensions(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if key == "extensions"
                    && let Value::Object(found) = inner
                {
                    out.extend(found.keys().cloned());
                }
                used_extensions(inner, out);
            }
        }
        Value::Array(list) => {
            for inner in list {
                used_extensions(inner, out);
            }
        }
        _ => {}
    }
}

fn listed(json: &Value, key: &str) -> BTreeSet<String> {
    json.get(key)
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn check_extensions(json: &Value, problems: &mut Vec<String>) {
    let mut used = BTreeSet::new();
    used_extensions(json, &mut used);
    let declared = listed(json, "extensionsUsed");
    let unlisted: Vec<&str> = used
        .iter()
        .filter(|e| !declared.contains(*e))
        .map(String::as_str)
        .collect();
    if !unlisted.is_empty() {
        problems.push(format!(
            "extensionsUsed does not list {}.",
            unlisted.join(", ")
        ));
    }
    let required: Vec<String> = listed(json, "extensionsRequired")
        .into_iter()
        .filter(|e| !declared.contains(e))
        .collect();
    if !required.is_empty() {
        problems.push(format!(
            "extensionsRequired names {}, which extensionsUsed does not.",
            required.join(", ")
        ));
    }
}

/// The image a texture shows, from its own source or an extension's.
fn texture_has_image(texture: &Value, images: usize) -> bool {
    let valid = |v: Option<&Value>| {
        v.and_then(Value::as_u64)
            .and_then(|i| usize::try_from(i).ok())
            .is_some_and(|i| i < images)
    };
    valid(texture.get("source"))
        || texture
            .get("extensions")
            .and_then(Value::as_object)
            .is_some_and(|exts| exts.values().any(|e| valid(e.get("source"))))
}

/// Every `*Texture` reference under `value`, as (key, index).
fn texture_refs<'a>(value: &'a Value, out: &mut Vec<(&'a str, Option<u64>)>) {
    if let Value::Object(map) = value {
        for (key, inner) in map {
            if key.ends_with("Texture") && inner.is_object() {
                out.push((key.as_str(), inner.get("index").and_then(Value::as_u64)));
            } else {
                texture_refs(inner, out);
            }
        }
    }
}

fn check_mtoon(json: &Value, problems: &mut Vec<String>) {
    let count = |key: &str| json.get(key).and_then(Value::as_array).map_or(0, Vec::len);
    let textures = json.get("textures").and_then(Value::as_array);
    let images = count("images");
    let Some(materials) = json.get("materials").and_then(Value::as_array) else {
        return;
    };
    for (i, material) in materials.iter().enumerate() {
        let Some(mtoon) = material
            .get("extensions")
            .and_then(|e| e.get(VRMC_MATERIALS_MTOON))
        else {
            continue;
        };
        let name = material
            .get("name")
            .and_then(Value::as_str)
            .map_or_else(|| format!("Material {i}"), |n| format!("Material {n}"));
        let mut refs = Vec::new();
        texture_refs(mtoon, &mut refs);
        if let Some(base) = material
            .get("pbrMetallicRoughness")
            .and_then(|p| p.get("baseColorTexture"))
        {
            refs.push((
                "baseColorTexture",
                base.get("index").and_then(Value::as_u64),
            ));
        }
        for (key, index) in refs {
            let texture = index
                .and_then(|i| usize::try_from(i).ok())
                .and_then(|i| textures.and_then(|t| t.get(i)));
            match texture {
                None => problems.push(format!(
                    "{name}'s {key} points at a texture the file does not have."
                )),
                Some(t) if !texture_has_image(t, images) => problems.push(format!(
                    "{name}'s {key} uses a texture with no image behind it."
                )),
                Some(_) => {}
            }
        }
    }
}
