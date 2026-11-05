//! The mark a staged bake carries in its own GLB (`asset.extras`): which
//! recipe and version it was baked from, the new clip and whether it counts
//! as Mixamo's. It travels with the file, so a pack and Save clips to the
//! library honour it wherever the file ends up, and adding the bake as a
//! version can turn it into a durable record.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::RecipeBakeResult;
use crate::error::{CoreError, CoreResult};
use maquette_glb::extras;

/// The key under the GLB's `asset.extras`.
pub const MARK_KEY: &str = "maquetteRecipeBake";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBakeMark {
    pub recipe: String,
    pub recipe_version: String,
    /// The library clip the bake started from.
    pub base_clip: String,
    /// The clip the bake added.
    pub clip: String,
    pub mixamo: bool,
}

impl RecipeBakeMark {
    pub fn of(result: &RecipeBakeResult) -> Self {
        Self {
            recipe: result.recipe.clone(),
            recipe_version: result.recipe_version.clone(),
            base_clip: result.base_clip.clone(),
            clip: result.clip.clone(),
            mixamo: result.mixamo,
        }
    }
}

/// Writes `mark` into the GLB at `file`.
///
/// # Errors
///
/// Fails when the file cannot be read or written, or is no GLB.
pub fn write_mark(file: &Path, mark: &RecipeBakeMark) -> CoreResult<()> {
    let bytes = fs::read(file).map_err(|e| CoreError::io(file, e))?;
    let value = serde_json::to_value(mark).map_err(|e| CoreError::json(file, e))?;
    let marked = extras::with_asset_extra(&bytes, MARK_KEY, value)?;
    fs::write(file, marked).map_err(|e| CoreError::io(file, e))
}

/// The mark of the model at `file`; none for a model no recipe baked.
pub fn mark_of(file: &Path) -> Option<RecipeBakeMark> {
    serde_json::from_value(extras::asset_extra(file, MARK_KEY)?).ok()
}

/// Whether the model at `file` holds a recipe bake that counts as Mixamo's.
pub fn marked_mixamo(file: &Path) -> bool {
    mark_of(file).is_some_and(|mark| mark.mixamo)
}
