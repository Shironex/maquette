//! The compression preset edits `set_compression_presets` takes.

use maquette_core::pipeline::presets::{self, CompressPreset};
use maquette_core::{MeshoptLevel, TextureCodec};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;

use crate::commands::pipeline;

use super::super::explicit;

/// The texture sizes the Pipeline view offers, in pixels.
const PRESET_SIZES: [u32; 4] = [512, 1024, 2048, 4096];

const PRESET_CODECS: &str = "etc1s, uastc";

const PRESET_MESHOPT: &str = "low, medium, high";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PresetEdit {
    /// The model as get_pipeline lists it in `models` (the GLB file stem, for example
    /// "clay_v1"), or the asset's name when it has one model.
    pub model: String,
    /// The longest texture edge in pixels: 512, 1024, 2048 or 4096.
    pub texture_size: Option<u32>,
    /// A smaller edge for the metal, roughness, occlusion and normal maps: 512, 1024,
    /// 2048 or 4096. null clears the split, so every map uses textureSize.
    #[serde(default, deserialize_with = "explicit")]
    pub data_texture_size: Option<Option<u32>>,
    /// The texture codec: "etc1s" or "uastc".
    pub codec: Option<String>,
    /// The meshopt level: "low", "medium" or "high".
    pub meshopt: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetPresetsArgs {
    /// The project id.
    pub project: String,
    /// The rows to change. Only the fields you give change; other models stay as they are.
    pub presets: Vec<PresetEdit>,
}

fn check_size(what: &str, size: u32) -> Result<u32, String> {
    if PRESET_SIZES.contains(&size) {
        Ok(size)
    } else {
        Err(format!(
            "{what} must be one of 512, 1024, 2048 or 4096, not {size}."
        ))
    }
}

fn parse_codec(word: &str) -> Result<TextureCodec, String> {
    serde_json::from_value(json!(word.to_lowercase()))
        .map_err(|_| format!("codec must be one of {PRESET_CODECS}, not {word:?}."))
}

fn parse_meshopt(word: &str) -> Result<MeshoptLevel, String> {
    serde_json::from_value(json!(word.to_lowercase()))
        .map_err(|_| format!("meshopt must be one of {PRESET_MESHOPT}, not {word:?}."))
}

/// The model an edit names: a stem, or an asset name with one model.
fn preset_target<'a>(models: &'a [pipeline::ModelFile], name: &str) -> Result<&'a str, String> {
    let wanted = name.to_lowercase();
    let stem = models.iter().find(|m| m.model.to_lowercase() == wanted);
    let named: Vec<_> = models
        .iter()
        .filter(|m| {
            m.asset_name
                .as_ref()
                .is_some_and(|n| n.to_lowercase() == wanted)
        })
        .collect();
    match (stem, named.as_slice()) {
        (Some(m), _) => Ok(&m.model),
        (None, [m]) => Ok(&m.model),
        (None, [_, ..]) => Err(format!(
            "{name:?} has several models ({}). Use the model name.",
            named
                .iter()
                .map(|m| m.model.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        (None, []) => Err(format!(
            "Unknown model {name:?}. Models: {}.",
            models
                .iter()
                .map(|m| m.model.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The stored table with the edits applied. A model without a row starts from
/// the workspace defaults, the way the Pipeline view's table does.
pub(super) fn edited_presets(
    view: &pipeline::PipelineView,
    edits: &[PresetEdit],
) -> Result<Vec<CompressPreset>, String> {
    let mut table = view.def.presets.clone();
    let mut touched = Vec::new();
    for edit in edits {
        let model = preset_target(&view.models, &edit.model)
            .or_else(|err| {
                // A stored row can name a model that has no build yet.
                let stored = table
                    .iter()
                    .find(|p| p.model.eq_ignore_ascii_case(&edit.model));
                stored.map(|p| p.model.as_str()).ok_or(err)
            })?
            .to_string();
        if touched.contains(&model) {
            return Err(format!("{model} appears twice. Give each model one entry."));
        }
        let index = table
            .iter()
            .position(|p| p.model == model)
            .unwrap_or_else(|| {
                let label = view
                    .models
                    .iter()
                    .find(|m| m.model == model)
                    .and_then(|m| m.asset_name.clone())
                    .unwrap_or_else(|| presets::model_label(&model));
                table.push(CompressPreset {
                    label,
                    model: model.clone(),
                    texture_size: view.defaults.texture_size,
                    data_texture_size: None,
                    codec: view.defaults.codec,
                    map_codecs: None,
                    meshopt: view.defaults.meshopt,
                    lock: None,
                });
                table.len() - 1
            });
        let row = &mut table[index];
        if let Some(reason) = &row.lock {
            return Err(format!("{} is locked: {reason}", row.label));
        }
        let name = |what: &str| format!("{}: {what}", row.label);
        if let Some(size) = edit.texture_size {
            row.texture_size = check_size(&name("textureSize"), size)?;
        }
        if let Some(data) = edit.data_texture_size {
            row.data_texture_size = data
                .map(|size| check_size(&name("dataTextureSize"), size))
                .transpose()?;
        }
        if let Some(word) = &edit.codec {
            row.codec = parse_codec(word).map_err(|e| name(&e))?;
        }
        if let Some(word) = &edit.meshopt {
            row.meshopt = parse_meshopt(word).map_err(|e| name(&e))?;
        }
        touched.push(model);
    }
    Ok(table)
}
