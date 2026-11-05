//! Recipes over library clips: making a project's recipe from a clip of the
//! animation library, and finding a recipe's base clip in the library again
//! by its name and frame count, as on another computer. A hash that differs
//! is a warning, never a refusal: the clip may have been imported again.
//! The recipe file itself is model data ([`crate::recipes`]).

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{AnimationClip, SourceRig, list, load};
use crate::error::CoreResult;
use crate::project::ProjectManifest;
use crate::recipes::{self, RecipeBase, RecipeChange, RecipeFile, RecipeRead};

/// How a recipe's base clip stands in this computer's library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum BaseStatus {
    /// A clip of the same name, frame count and keys.
    Found,
    /// A clip of the same name and frame count, with other keys.
    HashDiffers,
    /// No clip of that name and frame count.
    Missing,
}

/// A recipe's base clip in the library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BaseMatch {
    pub status: BaseStatus,
    /// The library clip's id, when one matches.
    pub clip: Option<String>,
    /// What to know before baking, when the match is not exact.
    pub warning: Option<String>,
}

/// One recipe as the view and agents get it.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeDoc {
    pub id: String,
    /// `recipes/<id>.recipe.json`.
    pub file: String,
    pub recipe: RecipeFile,
    pub base: BaseMatch,
    /// A bake counts as Mixamo's: the recipe says so, or its base clip in
    /// the library came from a Mixamo rig.
    pub mixamo: bool,
    /// Prop blanks that opened again because the project no longer has
    /// their asset, a line each.
    pub warnings: Vec<String>,
}

/// The base a recipe on `clip` stores: its name, frame count, length and
/// hash, and whether it was read from a Mixamo rig.
pub fn base_of(clip: &AnimationClip) -> RecipeBase {
    RecipeBase {
        name: clip.name.clone(),
        frames: clip.frames,
        length: clip.length,
        hash: clip.hash.clone(),
        mixamo: clip.rig.kind == SourceRig::Mixamo,
    }
}

/// Finds `base` among `clips` by name (case aside) and frame count,
/// preferring the clip with the same keys.
pub fn match_base<'a>(
    clips: &'a [AnimationClip],
    base: &RecipeBase,
) -> (BaseMatch, Option<&'a AnimationClip>) {
    let name = base.name.trim().to_lowercase();
    let same: Vec<&AnimationClip> = clips
        .iter()
        .filter(|c| c.frames == base.frames && c.name.trim().to_lowercase() == name)
        .collect();
    if let Some(exact) = same.iter().find(|c| c.hash == base.hash) {
        let found = BaseMatch {
            status: BaseStatus::Found,
            clip: Some(exact.id.clone()),
            warning: None,
        };
        return (found, Some(exact));
    }
    if let Some(other) = same.first() {
        let differs = BaseMatch {
            status: BaseStatus::HashDiffers,
            clip: Some(other.id.clone()),
            warning: Some(format!(
                "The library's {} has other keys than the recipe was made on (its hash differs). A bake will use the library's.",
                other.name
            )),
        };
        return (differs, Some(other));
    }
    let missing = BaseMatch {
        status: BaseStatus::Missing,
        clip: None,
        warning: Some(format!(
            "The animation library has no clip called {} with {} frames. Import it before baking.",
            base.name, base.frames
        )),
    };
    (missing, None)
}

/// `read` as the view gets it, its base matched against `clips`.
pub fn doc_of(id: &str, read: RecipeRead, clips: &[AnimationClip]) -> RecipeDoc {
    let RecipeRead { recipe, warnings } = read;
    let (base, clip) = match_base(clips, &recipe.base);
    let mixamo = recipe.base.mixamo || clip.is_some_and(|c| c.rig.kind == SourceRig::Mixamo);
    RecipeDoc {
        id: id.to_string(),
        file: format!("{}/{id}{}", recipes::RECIPES_DIR, recipes::RECIPE_SUFFIX),
        recipe,
        base,
        mixamo,
        warnings,
    }
}

/// The recipe `id` of the project of `manifest` at `project_dir`, its base
/// matched against the library of the workspace at `root`.
///
/// # Errors
///
/// Fails when the recipe is missing or unreadable, or the library cannot
/// be read.
pub fn get(
    root: &Path,
    project_dir: &Path,
    manifest: &ProjectManifest,
    id: &str,
) -> CoreResult<RecipeDoc> {
    let read = recipes::get(project_dir, manifest, id)?;
    Ok(doc_of(id, read, &list(root)?))
}

/// Changes the recipe `id` as [`recipes::update`] does and answers with it
/// as [`get`] reads it, with the warnings of the change.
///
/// # Errors
///
/// Fails when [`recipes::update`] refuses the change or the library cannot
/// be read.
pub fn update(
    root: &Path,
    project_dir: &Path,
    manifest: &ProjectManifest,
    id: &str,
    change: RecipeChange,
) -> CoreResult<RecipeDoc> {
    let read = recipes::update(project_dir, manifest, id, change)?;
    Ok(doc_of(id, read, &list(root)?))
}

/// Makes a recipe in the project at `project_dir` on the library clip
/// `clip_id`, called `name` or else after the clip.
///
/// # Errors
///
/// Fails for an unknown clip, a name the recipe file refuses or when the
/// file cannot be written.
pub fn create(
    root: &Path,
    project_dir: &Path,
    clip_id: &str,
    name: Option<&str>,
) -> CoreResult<RecipeDoc> {
    let clip = load(root, clip_id)?;
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(&clip.name);
    let (id, recipe) = recipes::create(project_dir, name, base_of(&clip))?;
    let read = RecipeRead {
        recipe,
        warnings: Vec::new(),
    };
    Ok(doc_of(&id, read, std::slice::from_ref(&clip)))
}

#[cfg(test)]
mod tests;
