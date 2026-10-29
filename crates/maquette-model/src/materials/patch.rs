//! Changing the saved tweaks without losing anyone else's: [`update`]
//! reads `materials.json` on disk, applies one change and writes the result,
//! so the app and agents only ever replace the assets they edit. Agents
//! edit one asset field by field ([`apply_patch`]); the Material panel
//! replaces the looks it edited whole ([`save_looks`]).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use serde::Deserialize;

use super::{
    MATERIALS_FILE, MaterialTweak, ModelMaterials, RoughnessTweak, SceneMaterials, load,
    normalized, save, validate,
};
use crate::error::{CoreError, CoreResult};
use crate::json::explicit;

/// A change to one tweak. A field left out keeps its value; null clears it.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TweakPatch {
    #[serde(default, deserialize_with = "explicit")]
    pub tint: Option<Option<String>>,
    #[serde(default, deserialize_with = "explicit")]
    pub roughness: Option<Option<RoughnessTweak>>,
    #[serde(default, deserialize_with = "explicit")]
    pub metalness: Option<Option<f32>>,
    #[serde(default, deserialize_with = "explicit")]
    pub emissive_strength: Option<Option<f32>>,
}

impl TweakPatch {
    fn apply(&self, to: &MaterialTweak) -> MaterialTweak {
        MaterialTweak {
            tint: self.tint.clone().unwrap_or_else(|| to.tint.clone()),
            roughness: self.roughness.unwrap_or(to.roughness),
            metalness: self.metalness.unwrap_or(to.metalness),
            emissive_strength: self.emissive_strength.unwrap_or(to.emissive_strength),
        }
    }
}

/// A partial update of one asset's tweaks.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialsPatch {
    /// Drops every tweak of the asset before the rest applies.
    #[serde(default)]
    pub reset: bool,
    /// The change for every material.
    pub all: Option<TweakPatch>,
    /// Material name to its change; `None` removes that material's entry.
    #[serde(default)]
    pub materials: BTreeMap<String, Option<TweakPatch>>,
}

/// Applies `patch` to `asset`'s tweaks in `current`. `names` are the
/// model's material names when known; a name outside them is refused with
/// the list. Nothing changes when any part is refused.
///
/// # Errors
///
/// Fails on an unknown material name or a value [`validate`] refuses.
pub fn apply_patch(
    current: &SceneMaterials,
    asset: &str,
    patch: &MaterialsPatch,
    names: &[String],
) -> CoreResult<SceneMaterials> {
    let mut next = current.clone();
    let mut look = if patch.reset {
        ModelMaterials::default()
    } else {
        next.models.get(asset).cloned().unwrap_or_default()
    };
    if let Some(all) = &patch.all {
        look.all = all.apply(&look.all);
    }
    for (name, change) in &patch.materials {
        let known = names.is_empty() || names.iter().any(|n| n == name);
        match change {
            None => {
                look.materials.remove(name);
            }
            Some(_) if !known => {
                return Err(CoreError::Invalid(format!(
                    "{asset} has no material {name:?}. Its materials: {}.",
                    names.join(", ")
                )));
            }
            Some(change) => {
                let was = look.materials.get(name).cloned().unwrap_or_default();
                look.materials.insert(name.clone(), change.apply(&was));
            }
        }
    }
    next.models.insert(asset.to_string(), look);
    validate(&next)?;
    Ok(normalized(&next))
}

/// One read, change and write at a time, so the app and an agent saving
/// together both land.
static WRITES: Mutex<()> = Mutex::new(());

/// Reads `materials.json` from disk, applies `change` to it and writes what
/// comes back, checked and tidied. Returns what was written.
///
/// # Errors
///
/// Refuses, writing nothing, when the file on disk comes from a newer
/// Maquette or cannot be read (so a broken file is never overwritten), and
/// fails when `change` or [`validate`] refuses or the write fails.
pub fn update(
    project_dir: &Path,
    change: impl FnOnce(&SceneMaterials) -> CoreResult<SceneMaterials>,
) -> CoreResult<SceneMaterials> {
    let _one = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let current = load(project_dir).map_err(|err| match err {
        CoreError::UnsupportedVersion { .. } => err,
        other => CoreError::Invalid(format!(
            "{MATERIALS_FILE} could not be read, so nothing was saved over it. Fix or remove the file, then save again. {other}"
        )),
    })?;
    save(project_dir, &change(&current)?)
}

/// Replaces the tweaks of each asset in `looks` in the file on disk; an
/// empty look removes the asset's entry. Every other asset keeps what is
/// saved. Returns the whole file as written.
///
/// # Errors
///
/// As [`update`].
pub fn save_looks(
    project_dir: &Path,
    looks: &BTreeMap<String, ModelMaterials>,
) -> CoreResult<SceneMaterials> {
    update(project_dir, |current| {
        let mut next = current.clone();
        for (asset, look) in looks {
            if look.is_empty() {
                next.models.remove(asset);
            } else {
                next.models.insert(asset.clone(), look.clone());
            }
        }
        Ok(next)
    })
}
