//! Material tweaks: per asset changes to the shipped model's materials, so
//! a prop that came back chrome or the wrong colour ships matte or olive
//! without touching its source file. They live in `materials.json` next to
//! `project.json`, keyed by asset id like the scene layout, so a new model
//! version keeps them.
//!
//! ```json
//! {
//!   "version": 1,
//!   "models": {
//!     "laptop": {
//!       "all": {
//!         "tint": "#3a3936",
//!         "roughness": { "mode": "override", "value": 0.62 },
//!         "metalness": 0.25,
//!         "emissiveStrength": null
//!       },
//!       "materials": { "Screen": { "tint": null, "roughness": null, "metalness": null, "emissiveStrength": 2 } }
//!     }
//!   }
//! }
//! ```
//!
//! `all` applies to every material of the model and a named entry to the
//! material of that name, its fields winning over `all`. The Compress step
//! applies them to a copy of each raw export before the texture passes
//! ([`apply_file`]), so the source model never changes and the step's cache
//! key covers them.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::ProjectManifest;

mod apply;
mod glb;
mod patch;
mod split;

pub use apply::{Applied, apply_file, apply_to_json, map_groups, material_names};
pub use patch::{MaterialsPatch, TweakPatch, apply_patch, save_looks, update};
pub use split::{mark_small_maps, unmark_small_maps};

pub const MATERIALS_FILE: &str = "materials.json";
pub const MATERIALS_VERSION: u32 = 1;
/// The most a roughness multiplier may scale by.
pub const MAX_MULTIPLY: f32 = 2.0;
/// The brightest emissive strength a tweak may ask for.
pub const MAX_EMISSIVE: f32 = 50.0;
pub const MAX_NAMED: usize = 64;
pub const MATERIAL_NAME_MAX_CHARS: usize = 120;

/// How a roughness tweak treats the model's own value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum FactorMode {
    /// Replaces it, and the metal and roughness map with it, so the value
    /// applies evenly.
    Override,
    /// Scales the factor; the map stays.
    Multiply,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RoughnessTweak {
    pub mode: FactorMode,
    /// 0 to 1 for an override, 0 to 2 for a multiplier.
    pub value: f32,
}

/// One material's changes. A `None` field leaves the model's value alone.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MaterialTweak {
    /// Multiplies the base color, as `#rrggbb` in sRGB. The color map stays.
    pub tint: Option<String>,
    pub roughness: Option<RoughnessTweak>,
    /// Replaces the metalness, 0 to 1. Like a roughness override it drops
    /// the metal and roughness map, which holds both.
    pub metalness: Option<f32>,
    /// The emissive strength (`KHR_materials_emissive_strength`), 0 to 50.
    pub emissive_strength: Option<f32>,
}

impl MaterialTweak {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    /// `self` with every field `over` sets laid on top.
    pub fn under(&self, over: &MaterialTweak) -> MaterialTweak {
        MaterialTweak {
            tint: over.tint.clone().or_else(|| self.tint.clone()),
            roughness: over.roughness.or(self.roughness),
            metalness: over.metalness.or(self.metalness),
            emissive_strength: over.emissive_strength.or(self.emissive_strength),
        }
    }

    /// True when the tweak replaces a factor the metal and roughness map
    /// scales, so the map has to go for the value to apply.
    pub fn drops_map(&self) -> bool {
        self.metalness.is_some()
            || self
                .roughness
                .is_some_and(|r| r.mode == FactorMode::Override)
    }
}

/// The tweaks of one asset's model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelMaterials {
    /// Applies to every material.
    pub all: MaterialTweak,
    /// By material name. A field set here wins over `all`.
    pub materials: BTreeMap<String, MaterialTweak>,
}

impl ModelMaterials {
    /// What the material called `name` gets.
    pub fn for_material(&self, name: Option<&str>) -> MaterialTweak {
        match name.and_then(|n| self.materials.get(n)) {
            Some(named) => self.all.under(named),
            None => self.all.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty() && self.materials.values().all(MaterialTweak::is_empty)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SceneMaterials {
    pub version: u32,
    /// Asset id to its tweaks.
    pub models: BTreeMap<String, ModelMaterials>,
}

impl Default for SceneMaterials {
    fn default() -> Self {
        Self {
            version: MATERIALS_VERSION,
            models: BTreeMap::new(),
        }
    }
}

pub fn path(project_dir: &Path) -> PathBuf {
    project_dir.join(MATERIALS_FILE)
}

/// Reads `materials.json`, or no tweaks when the project has none.
///
/// # Errors
///
/// Fails when the file exists but cannot be read, comes from a newer
/// Maquette or holds values [`validate`] refuses.
pub fn load(project_dir: &Path) -> CoreResult<SceneMaterials> {
    let file = path(project_dir);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(SceneMaterials::default());
        }
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    // The version first, so a newer file whose shape changed says so.
    let version = value.get("version").and_then(serde_json::Value::as_u64);
    if let Some(found) = version.filter(|v| *v > u64::from(MATERIALS_VERSION)) {
        return Err(CoreError::UnsupportedVersion {
            found: u32::try_from(found).unwrap_or(u32::MAX),
            supported: MATERIALS_VERSION,
        });
    }
    let found: SceneMaterials =
        serde_json::from_value(value).map_err(|e| CoreError::json(&file, e))?;
    validate(&found)?;
    Ok(found)
}

/// Checks the tweaks, drops the empty ones and writes the whole file
/// atomically. Returns what was written. Callers go through [`update`],
/// which reads the file on disk first so no other asset's tweaks are lost.
fn save(project_dir: &Path, materials: &SceneMaterials) -> CoreResult<SceneMaterials> {
    validate(materials)?;
    let stored = normalized(materials);
    write_json(&path(project_dir), &stored)?;
    Ok(stored)
}

/// Lowercase hex, no empty entries and the current version.
pub fn normalized(materials: &SceneMaterials) -> SceneMaterials {
    let clean = |t: &MaterialTweak| MaterialTweak {
        tint: t.tint.as_ref().map(|c| c.to_lowercase()),
        ..t.clone()
    };
    let models = materials
        .models
        .iter()
        .map(|(id, look)| {
            let named = look
                .materials
                .iter()
                .filter(|(_, t)| !t.is_empty())
                .map(|(name, t)| (name.clone(), clean(t)))
                .collect();
            (
                id.clone(),
                ModelMaterials {
                    all: clean(&look.all),
                    materials: named,
                },
            )
        })
        .filter(|(_, look)| !look.is_empty())
        .collect();
    SceneMaterials {
        version: MATERIALS_VERSION,
        models,
    }
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

/// The sRGB channels of `#rrggbb`, 0 to 1.
pub fn parse_hex(text: &str) -> Option<[f32; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize| {
        u8::from_str_radix(hex.get(i..i + 2)?, 16)
            .ok()
            .map(|v| f32::from(v) / 255.0)
    };
    Some([channel(0)?, channel(2)?, channel(4)?])
}

fn check_tweak(what: &str, t: &MaterialTweak) -> CoreResult<()> {
    if let Some(tint) = &t.tint
        && parse_hex(tint).is_none()
    {
        return Err(invalid(format!(
            "{what}: the tint {tint:?} is not a color. Use #rrggbb, for example #6f7446."
        )));
    }
    let within = |v: f32, max: f32| v.is_finite() && (0.0..=max).contains(&v);
    if let Some(r) = t.roughness {
        let max = match r.mode {
            FactorMode::Override => 1.0,
            FactorMode::Multiply => MAX_MULTIPLY,
        };
        if !within(r.value, max) {
            return Err(invalid(format!(
                "{what}: the roughness must be between 0 and {max}, not {}.",
                r.value
            )));
        }
    }
    if let Some(m) = t.metalness
        && !within(m, 1.0)
    {
        return Err(invalid(format!(
            "{what}: the metalness must be between 0 and 1, not {m}."
        )));
    }
    if let Some(e) = t.emissive_strength
        && !within(e, MAX_EMISSIVE)
    {
        return Err(invalid(format!(
            "{what}: the emissive strength must be between 0 and {MAX_EMISSIVE}, not {e}."
        )));
    }
    Ok(())
}

/// Checks every tweak: `#rrggbb` tints, roughness 0 to 1 (a multiplier 0
/// to 2), metalness 0 to 1, emissive strength 0 to 50 and named materials
/// with a name of sensible length.
///
/// # Errors
///
/// Returns the first problem as a sentence for the person or agent.
pub fn validate(materials: &SceneMaterials) -> CoreResult<()> {
    for (asset, look) in &materials.models {
        check_tweak(asset, &look.all)?;
        if look.materials.len() > MAX_NAMED {
            return Err(invalid(format!(
                "{asset}: keep the named materials to {MAX_NAMED} or fewer."
            )));
        }
        for (name, tweak) in &look.materials {
            let count = name.chars().count();
            if name.trim().is_empty() || count > MATERIAL_NAME_MAX_CHARS {
                return Err(invalid(format!(
                    "{asset}: a material name must have 1 to {MATERIAL_NAME_MAX_CHARS} characters."
                )));
            }
            check_tweak(&format!("{asset}, {name}"), tweak)?;
        }
    }
    Ok(())
}

/// The tweaks by model file stem (lowercase), for the Compress step: every
/// version of an asset with tweaks gets them, since its raw export is named
/// after the version file. Assets no longer in the project are skipped.
pub fn by_model_stem(
    manifest: &ProjectManifest,
    materials: &SceneMaterials,
) -> BTreeMap<String, ModelMaterials> {
    let mut out = BTreeMap::new();
    for asset in &manifest.assets {
        let Some(look) = materials.models.get(&asset.id).filter(|l| !l.is_empty()) else {
            continue;
        };
        for version in &asset.versions {
            if let Some(stem) = Path::new(&version.file).file_stem() {
                out.insert(stem.to_string_lossy().to_lowercase(), look.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;

/// Registers this module's IPC types (the scene's materials) with the shell's TypeScript export;
/// `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types.register::<SceneMaterials>()
}
