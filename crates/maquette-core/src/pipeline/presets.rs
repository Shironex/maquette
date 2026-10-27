//! Compression presets: per model texture size, texture codec (one for
//! every map, or one per map group) and meshopt level, and how they become
//! gltf-transform args.

mod image;
mod passes;

use serde::{Deserialize, Serialize};

use super::CompressDefaults;
use crate::error::{CoreError, CoreResult};
use crate::workspace::{MeshoptLevel, TextureCodec, is_single_segment};

pub const MIN_TEXTURE: u32 = 64;
pub const MAX_TEXTURE: u32 = 8192;

pub use image::{SMALL_MARK, color_is_smaller, image_passes, needs_split};
pub use passes::{CompressPass, MapGroups, compress_passes};

/// The codec of each map group, where a model picks one. A group left
/// `None` follows the row's `codec`, except normal maps, which default to
/// UASTC: ETC1S blocks show as stripes and banding in the lighting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MapCodecs {
    /// Base color and emissive.
    pub color: Option<TextureCodec>,
    pub normal: Option<TextureCodec>,
    /// Metal and roughness, occlusion and anything else.
    pub other: Option<TextureCodec>,
}

impl MapCodecs {
    /// The codec every group ends up with under a row's `codec`.
    pub fn resolve(self, codec: TextureCodec) -> CodecSet {
        CodecSet {
            color: self.color.unwrap_or(codec),
            normal: self.normal.unwrap_or(TextureCodec::Uastc),
            other: self.other.unwrap_or(codec),
        }
    }

    pub fn is_empty(self) -> bool {
        self == Self::default()
    }
}

/// What each map group is encoded with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CodecSet {
    pub color: TextureCodec,
    pub normal: TextureCodec,
    pub other: TextureCodec,
}

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
    /// The codec of every map group the row does not set in `map_codecs`.
    pub codec: TextureCodec,
    /// Per map group codecs. Without it color and data maps use `codec` and
    /// normal maps UASTC (a locked row's normal maps keep `codec`).
    #[serde(default)]
    pub map_codecs: Option<MapCodecs>,
    pub meshopt: MeshoptLevel,
    /// Why this row cannot change, for example "High spins her upper body".
    /// Locked rows refuse edits until the lock is removed from the file.
    pub lock: Option<String>,
}

impl CompressPreset {
    /// What each map group of this row is encoded with. A locked row
    /// without `map_codecs` keeps `codec` for its normal maps too, as it did
    /// before normal maps defaulted to UASTC, so its output does not change
    /// with no edit.
    pub fn codecs(&self) -> CodecSet {
        match self.map_codecs {
            None if self.lock.is_some() => CodecSet {
                color: self.codec,
                normal: self.codec,
                other: self.codec,
            },
            map_codecs => map_codecs.unwrap_or_default().resolve(self.codec),
        }
    }

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
    pub codecs: CodecSet,
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
            codecs: p.codecs(),
            meshopt: p.meshopt,
            locked: p.lock.is_some(),
        },
        None => ResolvedPreset {
            model: model.to_string(),
            texture_size: defaults.texture_size,
            data_texture_size: None,
            codecs: MapCodecs::default().resolve(defaults.codec),
            meshopt: defaults.meshopt,
            locked: false,
        },
    }
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
