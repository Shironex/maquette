//! Compression presets: per model texture size, texture codec and meshopt
//! level, and how they become gltf-transform args.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::CompressDefaults;
use crate::error::{CoreError, CoreResult};
use crate::workspace::{MeshoptLevel, TextureCodec, is_single_segment};

pub const MIN_TEXTURE: u32 = 64;
pub const MAX_TEXTURE: u32 = 8192;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompressPreset {
    /// The GLB file stem the row applies to, for example `character`.
    pub model: String,
    /// What the table shows, for example `Character`.
    pub label: String,
    /// The longest texture edge after compression, in pixels.
    pub texture_size: u32,
    /// An optional smaller edge for the metal, roughness, occlusion and
    /// normal maps; base color (and emissive) keep `texture_size`. Without
    /// it every map uses the one size.
    #[serde(default)]
    pub data_texture_size: Option<u32>,
    pub codec: TextureCodec,
    pub meshopt: MeshoptLevel,
    /// Why this row cannot change, for example "High spins her upper body".
    /// Locked rows refuse edits until the lock is removed from the file.
    pub lock: Option<String>,
}

impl CompressPreset {
    pub(crate) fn check(&self) -> Result<(), String> {
        if !is_single_segment(&self.model) {
            return Err(format!(
                "{:?} is not a model name. Use the GLB file name without .glb.",
                self.model
            ));
        }
        if !(MIN_TEXTURE..=MAX_TEXTURE).contains(&self.texture_size) {
            return Err(format!(
                "{}: the texture size must be between {MIN_TEXTURE} and {MAX_TEXTURE} px.",
                self.label
            ));
        }
        if let Some(size) = self.data_texture_size
            && !(MIN_TEXTURE..=MAX_TEXTURE).contains(&size)
        {
            return Err(format!(
                "{}: the data map size must be between {MIN_TEXTURE} and {MAX_TEXTURE} px.",
                self.label
            ));
        }
        Ok(())
    }
}

/// "sakura_large" reads as "Sakura large".
pub fn model_label(stem: &str) -> String {
    let words = stem.replace(['_', '-'], " ");
    let words = words
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let mut chars = words.chars();
    chars.next().map_or_else(
        || stem.to_string(),
        |first| first.to_uppercase().chain(chars).collect(),
    )
}

/// The preset a model gets: its own row, or the workspace defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPreset {
    pub model: String,
    pub texture_size: u32,
    /// Set only when it differs from `texture_size`.
    pub data_texture_size: Option<u32>,
    pub codec: TextureCodec,
    pub meshopt: MeshoptLevel,
    pub locked: bool,
}

pub fn resolve(
    presets: &[CompressPreset],
    model: &str,
    defaults: CompressDefaults,
) -> ResolvedPreset {
    match presets.iter().find(|p| p.model == model) {
        Some(p) => ResolvedPreset {
            model: model.to_string(),
            texture_size: p.texture_size,
            data_texture_size: p.data_texture_size.filter(|d| *d != p.texture_size),
            codec: p.codec,
            meshopt: p.meshopt,
            locked: p.lock.is_some(),
        },
        None => ResolvedPreset {
            model: model.to_string(),
            texture_size: defaults.texture_size,
            data_texture_size: None,
            codec: defaults.codec,
            meshopt: defaults.meshopt,
            locked: false,
        },
    }
}

/// The texture slots that keep the main size when a model splits its sizes.
/// An extglob, because gltf-transform's argument parser splits a brace glob
/// at its comma and then refuses the extra argument.
const COLOR_SLOTS: &str = "@(baseColorTexture|emissiveTexture)";

/// One gltf-transform invocation of the compress chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressPass {
    /// What the log says before the pass runs.
    pub label: String,
    pub args: Vec<String>,
}

fn arg(path: &Path) -> String {
    path.display().to_string()
}

/// The gltf-transform passes for one model, in order:
///
/// 1. `etc1s` or `uastc` with `--resize <size>` (skipped without toktx, so
///    the textures stay PNG or JPEG). With a data map size it is two passes:
///    base color and emissive at the size, then every other map at the
///    data size,
/// 2. `meshopt --level medium|high`, or `quantize` for the low level
///    (gltf-transform's meshopt has no low level, so low keeps the geometry
///    uncompressed and only quantises it).
///
/// The texture pass writes `tmp`, the last pass writes `output`.
pub fn compress_passes(
    input: &Path,
    output: &Path,
    tmp: &Path,
    preset: &ResolvedPreset,
    toktx: bool,
) -> Vec<CompressPass> {
    let mut passes = Vec::new();
    let data_tmp = tmp.with_extension("data.glb");
    let mut source = input;
    if toktx {
        let codec = match preset.codec {
            TextureCodec::Etc1s => "etc1s",
            TextureCodec::Uastc => "uastc",
        };
        let texture_pass = |from: &Path, to: &Path, size: u32, slots: Option<&str>, what: &str| {
            let mut args = vec![
                codec.to_string(),
                arg(from),
                arg(to),
                "--resize".to_string(),
                size.to_string(),
            ];
            if let Some(slots) = slots {
                args.extend(["--slots".to_string(), slots.to_string()]);
            }
            CompressPass {
                label: format!(
                    "{} {what}textures to {} at {size} px",
                    preset.model,
                    codec.to_uppercase()
                ),
                args,
            }
        };
        match preset.data_texture_size {
            None => {
                passes.push(texture_pass(input, tmp, preset.texture_size, None, ""));
                source = tmp;
            }
            Some(data) => {
                // Color maps first. A texture already encoded is skipped by
                // the second pass, which takes the data maps and anything
                // left over at the smaller size.
                passes.push(texture_pass(
                    input,
                    tmp,
                    preset.texture_size,
                    Some(COLOR_SLOTS),
                    "color ",
                ));
                passes.push(texture_pass(tmp, &data_tmp, data, None, "data map "));
                source = &data_tmp;
            }
        }
    }
    let geometry = match preset.meshopt {
        MeshoptLevel::Low => CompressPass {
            label: format!("{} geometry quantized (meshopt low)", preset.model),
            args: vec!["quantize".to_string(), arg(source), arg(output)],
        },
        level => {
            let level = if level == MeshoptLevel::High {
                "high"
            } else {
                "medium"
            };
            CompressPass {
                label: format!("{} geometry with meshopt {level}", preset.model),
                args: vec![
                    "meshopt".to_string(),
                    arg(source),
                    arg(output),
                    "--level".to_string(),
                    level.to_string(),
                ],
            }
        }
    };
    passes.push(geometry);
    passes
}

/// Merges an edited preset table over the stored one. Locked rows must come
/// back unchanged (the lock reason is the error), and removing a locked row
/// is refused too.
pub fn apply(
    stored: &[CompressPreset],
    incoming: Vec<CompressPreset>,
) -> CoreResult<Vec<CompressPreset>> {
    for old in stored.iter().filter(|p| p.lock.is_some()) {
        let reason = old.lock.as_deref().unwrap_or_default();
        match incoming.iter().find(|p| p.model == old.model) {
            Some(new) if new == old => {}
            _ => {
                return Err(CoreError::Invalid(format!(
                    "{} is locked: {reason}",
                    old.label
                )));
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for preset in &incoming {
        preset.check().map_err(CoreError::Invalid)?;
        if !seen.insert(preset.model.as_str()) {
            return Err(CoreError::Invalid(format!(
                "{} has two preset rows.",
                preset.label
            )));
        }
        if preset.lock.is_some() && !stored.iter().any(|p| p == preset) {
            return Err(CoreError::Invalid(format!(
                "{}: locks are set in pipeline.json, not from the table.",
                preset.label
            )));
        }
    }
    Ok(incoming)
}

#[cfg(test)]
mod tests;
