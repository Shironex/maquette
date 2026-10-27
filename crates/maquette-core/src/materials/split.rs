//! Marking the images of one map group by name, so gltf-transform's
//! `resize --pattern` can shrink that group alone (it cannot pick slots),
//! and taking the mark off again once the passes ran.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::glb;
use crate::fs::write_atomic;
use crate::pipeline::presets::SMALL_MARK;

const COLOR_SLOTS: [&str; 2] = ["baseColorTexture", "emissiveTexture"];

/// Every `(slot, texture index)` a material uses, extensions included.
fn texture_refs(value: &Value, slot: Option<&str>, out: &mut Vec<(String, u64)>) {
    match value {
        Value::Object(map) => {
            if let (Some(slot), Some(index)) = (slot, map.get("index").and_then(Value::as_u64)) {
                out.push((slot.to_string(), index));
            }
            for (key, child) in map {
                let next = key.ends_with("Texture").then_some(key.as_str());
                texture_refs(child, next, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|i| texture_refs(i, None, out)),
        _ => {}
    }
}

/// The image a texture shows: its own source, or an extension's.
fn image_of(texture: &Value) -> Option<u64> {
    texture.get("source").and_then(Value::as_u64).or_else(|| {
        texture
            .get("extensions")
            .and_then(Value::as_object)
            .and_then(|e| {
                e.values()
                    .find_map(|x| x.get("source").and_then(Value::as_u64))
            })
    })
}

fn images_of(json: &Value, color: bool) -> BTreeSet<u64> {
    let mut refs = Vec::new();
    for material in json
        .get("materials")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        texture_refs(material, None, &mut refs);
    }
    let textures = json.get("textures").and_then(Value::as_array);
    refs.into_iter()
        .filter(|(slot, _)| COLOR_SLOTS.contains(&slot.as_str()) == color)
        .filter_map(|(_, t)| textures.and_then(|ts| ts.get(usize::try_from(t).ok()?)))
        .filter_map(image_of)
        .collect()
}

fn rewrite(from: &Path, to: &Path, edit: impl Fn(&mut Value) -> usize) -> Result<usize, String> {
    let bytes = fs::read(from).map_err(|e| format!("Could not read {}. {e}", from.display()))?;
    let mut model = glb::read(&bytes)?;
    let changed = edit(&mut model.json);
    let out = glb::write(&model)?;
    write_atomic(to, &out).map_err(|e| format!("Could not write {}. {e}", to.display()))?;
    Ok(changed)
}

/// Copies `from` to `to` with [`SMALL_MARK`] on the name of every image
/// only the smaller group uses (the color maps when `color_smaller`, else
/// the data and normal maps). An image both groups share stays unmarked, so
/// it keeps the larger size. Returns how many were marked.
///
/// # Errors
///
/// Fails when `from` is no GLB or `to` cannot be written.
pub fn mark_small_maps(from: &Path, to: &Path, color_smaller: bool) -> Result<usize, String> {
    rewrite(from, to, |json| {
        let small = images_of(json, color_smaller);
        let large = images_of(json, !color_smaller);
        let Some(images) = json.get_mut("images").and_then(Value::as_array_mut) else {
            return 0;
        };
        let mut marked = 0;
        for (i, image) in images.iter_mut().enumerate() {
            let index = u64::try_from(i).unwrap_or(u64::MAX);
            if !small.contains(&index) || large.contains(&index) {
                continue;
            }
            let Some(object) = image.as_object_mut() else {
                continue;
            };
            let name = object
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            object.insert("name".into(), Value::String(format!("{name}{SMALL_MARK}")));
            marked += 1;
        }
        marked
    })
}

/// Takes [`SMALL_MARK`] off every image name in `path`, dropping a name the
/// image did not have before.
///
/// # Errors
///
/// Fails when `path` is no GLB or cannot be written.
pub fn unmark_small_maps(path: &Path) -> Result<usize, String> {
    rewrite(path, path, |json| {
        let Some(images) = json.get_mut("images").and_then(Value::as_array_mut) else {
            return 0;
        };
        let mut restored = 0;
        for object in images.iter_mut().filter_map(Value::as_object_mut) {
            let Some(name) = object.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(original) = name.strip_suffix(SMALL_MARK) else {
                continue;
            };
            if original.is_empty() {
                object.remove("name");
            } else {
                object.insert("name".into(), Value::String(original.to_string()));
            }
            restored += 1;
        }
        restored
    })
}
