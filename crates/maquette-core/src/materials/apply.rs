//! Applying tweaks to a model's glTF JSON: base color, roughness and metal
//! factors, the emissive strength extension, and the metal and roughness
//! maps an override leaves unused. Each changed material keeps what it had
//! under `extras.maquette.original`, so the Inspector can preview new
//! tweaks over the shipped build from the same starting point.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde_json::{Map, Value, json};

use super::glb::{self, Glb};
use super::{FactorMode, MaterialTweak, ModelMaterials, parse_hex};
use crate::error::{CoreError, CoreResult};
use crate::fs::write_atomic;
use crate::pipeline::presets::MapGroups;

const EMISSIVE_STRENGTH: &str = "KHR_materials_emissive_strength";

/// What a model's tweaks changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Applied {
    /// Materials that got at least one change.
    pub materials: usize,
    /// Metal and roughness maps an override replaced.
    pub maps_dropped: usize,
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// A tweak value as JSON: rounded, so 0.62 stays 0.62 and not the nearest
/// `f32` widened.
fn rounded(v: f64) -> Value {
    json!((v * 1e6).round() / 1e6)
}

fn widen(v: f32) -> f64 {
    (f64::from(v) * 1e6).round() / 1e6
}

fn factor(object: &Map<String, Value>, key: &str) -> f64 {
    object.get(key).and_then(Value::as_f64).unwrap_or(1.0)
}

/// Changes one material. Returns whether anything changed and whether its
/// metal and roughness map went.
fn tweak_material(material: &mut Map<String, Value>, tweak: &MaterialTweak) -> (bool, bool) {
    if tweak.is_empty() {
        return (false, false);
    }
    let strength = material
        .get("extensions")
        .and_then(|e| e.get(EMISSIVE_STRENGTH))
        .and_then(|e| e.get("emissiveStrength"))
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let pbr = material
        .entry("pbrMetallicRoughness")
        .or_insert_with(|| json!({}));
    let Some(pbr) = pbr.as_object_mut() else {
        return (false, false);
    };
    let base: Vec<f64> = pbr
        .get("baseColorFactor")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|v| v.as_f64().unwrap_or(1.0)).collect())
        .filter(|a: &Vec<f64>| a.len() == 4)
        .unwrap_or_else(|| vec![1.0; 4]);
    let original = json!({
        "baseColorFactor": base.clone(),
        "roughnessFactor": factor(pbr, "roughnessFactor"),
        "metallicFactor": factor(pbr, "metallicFactor"),
        "emissiveStrength": strength,
    });
    if let Some(tint) = tweak.tint.as_deref().and_then(parse_hex) {
        let tinted: Vec<Value> = base
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let by = tint.get(i).map_or(1.0, |c| srgb_to_linear(f64::from(*c)));
                rounded(v * by)
            })
            .collect();
        pbr.insert("baseColorFactor".into(), Value::Array(tinted));
    }
    if let Some(r) = tweak.roughness {
        let value = match r.mode {
            FactorMode::Override => widen(r.value),
            FactorMode::Multiply => (factor(pbr, "roughnessFactor") * widen(r.value)).min(1.0),
        };
        pbr.insert("roughnessFactor".into(), rounded(value));
    }
    if let Some(m) = tweak.metalness {
        pbr.insert("metallicFactor".into(), rounded(widen(m)));
    }
    let dropped = tweak.drops_map() && pbr.remove("metallicRoughnessTexture").is_some();
    if let Some(e) = tweak.emissive_strength {
        let extensions = material.entry("extensions").or_insert_with(|| json!({}));
        if let Some(extensions) = extensions.as_object_mut() {
            extensions.insert(
                EMISSIVE_STRENGTH.into(),
                json!({ "emissiveStrength": rounded(widen(e)) }),
            );
        }
    }
    let extras = material.entry("extras").or_insert_with(|| json!({}));
    if let Some(extras) = extras.as_object_mut() {
        let ours = extras.entry("maquette").or_insert_with(|| json!({}));
        if let Some(ours) = ours.as_object_mut() {
            ours.entry("original").or_insert(original);
        }
    }
    (true, dropped)
}

/// Every texture index a material refers to: any `...Texture` object with
/// an `index`, core and extension slots alike.
fn texture_refs(value: &Value, out: &mut BTreeSet<usize>) {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                if key.ends_with("Texture")
                    && let Some(index) = v.get("index").and_then(Value::as_u64)
                {
                    out.insert(usize::try_from(index).unwrap_or(usize::MAX));
                }
                texture_refs(v, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|v| texture_refs(v, out)),
        _ => {}
    }
}

fn remap_texture_refs(value: &mut Value, map: &BTreeMap<usize, usize>) {
    match value {
        Value::Object(object) => {
            for (key, v) in object.iter_mut() {
                if key.ends_with("Texture")
                    && let Some(slot) = v.get_mut("index")
                    && let Some(new) = slot
                        .as_u64()
                        .and_then(|i| map.get(&usize::try_from(i).unwrap_or(usize::MAX)))
                {
                    *slot = json!(new);
                }
                remap_texture_refs(v, map);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| remap_texture_refs(v, map)),
        _ => {}
    }
}

/// The image indices a texture reads, its own `source` and any extension's.
fn image_sources(texture: &Value) -> Vec<&Value> {
    let mut out: Vec<&Value> = texture.get("source").into_iter().collect();
    if let Some(ext) = texture.get("extensions").and_then(Value::as_object) {
        out.extend(ext.values().filter_map(|e| e.get("source")));
    }
    out
}

fn as_index(v: &Value) -> Option<usize> {
    v.as_u64().and_then(|i| usize::try_from(i).ok())
}

/// Keeps the entries of `items` at the kept indices; old index to new.
fn keep_indices(items: &mut Vec<Value>, drop: &BTreeSet<usize>) -> BTreeMap<usize, usize> {
    let mut map = BTreeMap::new();
    let mut kept = Vec::new();
    for (old, item) in std::mem::take(items).into_iter().enumerate() {
        if !drop.contains(&old) {
            map.insert(old, kept.len());
            kept.push(item);
        }
    }
    *items = kept;
    map
}

/// Removes the textures in `orphans` (no longer used by any material) and
/// the images only they read. The images' bytes stay in the buffer until
/// the next gltf-transform pass writes the file again without them.
fn drop_textures(root: &mut Value, orphans: &BTreeSet<usize>) {
    let Some(textures) = root.get_mut("textures").and_then(Value::as_array_mut) else {
        return;
    };
    let mut read_by_kept = BTreeSet::new();
    let mut read_by_dropped = BTreeSet::new();
    for (i, texture) in textures.iter().enumerate() {
        let into = if orphans.contains(&i) {
            &mut read_by_dropped
        } else {
            &mut read_by_kept
        };
        into.extend(image_sources(texture).into_iter().filter_map(as_index));
    }
    let texture_map = keep_indices(textures, orphans);
    let images: BTreeSet<usize> = read_by_dropped.difference(&read_by_kept).copied().collect();
    if let Some(list) = root.get_mut("materials") {
        remap_texture_refs(list, &texture_map);
    }
    let image_map = match root.get_mut("images").and_then(Value::as_array_mut) {
        Some(list) => keep_indices(list, &images),
        None => return,
    };
    let Some(textures) = root.get_mut("textures").and_then(Value::as_array_mut) else {
        return;
    };
    for texture in textures {
        let remap = |slot: &mut Value| {
            if let Some(new) = as_index(slot).and_then(|i| image_map.get(&i)) {
                *slot = json!(new);
            }
        };
        if let Some(slot) = texture.get_mut("source") {
            remap(slot);
        }
        if let Some(ext) = texture.get_mut("extensions").and_then(Value::as_object_mut) {
            ext.values_mut()
                .filter_map(|e| e.get_mut("source"))
                .for_each(remap);
        }
    }
}

fn add_extension_used(root: &mut Value, name: &str) {
    let Some(object) = root.as_object_mut() else {
        return;
    };
    let used = object.entry("extensionsUsed").or_insert_with(|| json!([]));
    if let Some(list) = used.as_array_mut()
        && !list.iter().any(|v| v.as_str() == Some(name))
    {
        list.push(json!(name));
    }
}

/// Applies `look` to every material of a glTF document.
pub fn apply_to_json(root: &mut Value, look: &ModelMaterials) -> Applied {
    let mut applied = Applied::default();
    let mut before = BTreeSet::new();
    let mut strength = false;
    if let Some(materials) = root.get_mut("materials").and_then(Value::as_array_mut) {
        texture_refs(&Value::Array(materials.clone()), &mut before);
        for material in materials.iter_mut() {
            let Some(object) = material.as_object_mut() else {
                continue;
            };
            let name = object
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string);
            let tweak = look.for_material(name.as_deref());
            let (changed, dropped) = tweak_material(object, &tweak);
            applied.materials += usize::from(changed);
            applied.maps_dropped += usize::from(dropped);
            strength |= tweak.emissive_strength.is_some();
        }
    }
    if strength {
        add_extension_used(root, EMISSIVE_STRENGTH);
    }
    if applied.maps_dropped > 0 {
        let mut after = BTreeSet::new();
        if let Some(materials) = root.get("materials") {
            texture_refs(materials, &mut after);
        }
        let orphans: BTreeSet<usize> = before.difference(&after).copied().collect();
        drop_textures(root, &orphans);
    }
    applied
}

/// Writes `output`: `input` with `look` applied. `input` is only read.
///
/// # Errors
///
/// Fails when `input` cannot be read or is not a GLB, or `output` cannot be
/// written.
pub fn apply_file(input: &Path, output: &Path, look: &ModelMaterials) -> CoreResult<Applied> {
    let bytes = fs::read(input).map_err(|e| CoreError::io(input, e))?;
    let mut model: Glb = glb::read(&bytes).map_err(|why| {
        CoreError::Invalid(format!(
            "{} could not take the material tweaks. {why}",
            input.display()
        ))
    })?;
    let applied = apply_to_json(&mut model.json, look);
    let out = glb::write(&model).map_err(CoreError::Invalid)?;
    write_atomic(output, &out).map_err(|e| CoreError::io(output, e))?;
    Ok(applied)
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read(path)
        .ok()
        .and_then(|bytes| glb::read(&bytes).ok())
        .map(|g| g.json)
}

/// The names of a model's materials, in file order, each once.
pub fn material_names(path: &Path) -> Vec<String> {
    let Some(json) = read_json(path) else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    json.get("materials")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| m.get("name").and_then(Value::as_str))
        .filter(|name| !name.is_empty() && seen.insert(name.to_string()))
        .map(str::to_string)
        .collect()
}

/// Which map groups a model's materials use; every group when the file
/// cannot be read, so nothing is skipped by mistake.
pub fn map_groups(path: &Path) -> MapGroups {
    let Some(materials) = read_json(path).and_then(|j| j.get("materials").cloned()) else {
        return MapGroups::ALL;
    };
    let uses = |slot: &str| {
        materials.as_array().into_iter().flatten().any(|m| {
            m.get(slot).is_some()
                || m.get("pbrMetallicRoughness")
                    .is_some_and(|p| p.get(slot).is_some())
        })
    };
    MapGroups {
        color: uses("baseColorTexture") || uses("emissiveTexture"),
        normal: uses("normalTexture"),
    }
}
