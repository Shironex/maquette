//! Editing one asset's tweaks field by field, for agents: only what the
//! patch names changes, and the result is checked whole before it is kept.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{MaterialTweak, ModelMaterials, RoughnessTweak, SceneMaterials, normalized, validate};
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
