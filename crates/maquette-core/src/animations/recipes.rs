//! Recipes over library clips: making a project's recipe from a clip of the
//! animation library, and finding a recipe's base clip in the library again
//! by its name and frame count, as on another computer. A hash that differs
//! is a warning, never a refusal: the clip may have been imported again. A
//! recipe that joins clips is matched clip by clip, and its bake counts as
//! Mixamo's when any of them does.
//! The recipe file itself is model data ([`crate::recipes`]).

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{AnimationClip, SourceRig, list, load};
use crate::error::CoreError;
use crate::error::CoreResult;
use crate::project::ProjectManifest;
use crate::recipes::drafts::{self, RecipeDrafts};
use crate::recipes::{
    self, BlendWindow, GoneProp, RecipeBase, RecipeChange, RecipeFile, RecipeRead,
};

pub mod bake;
pub mod library;

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
    /// The recipe runs on the model's rest pose and needs no clip.
    Rest,
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
    /// Each joined clip in the library, in order (clip 2 first). Empty for
    /// a recipe on one clip.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[specta(optional)]
    pub joined: Vec<BaseMatch>,
    /// A bake counts as Mixamo's: the recipe says so of any clip, or any of
    /// its clips in the library came from a Mixamo rig.
    pub mixamo: bool,
    /// Prop blanks that read open because the project no longer has their
    /// asset, a line each.
    pub warnings: Vec<String>,
    /// The same blanks, each with the asset id the file still holds.
    pub gone: Vec<GoneProp>,
    /// A hash of the file's bytes; a write sends it back as `expected`.
    pub version: String,
    /// A hash of what the recipe does, without its library link and other
    /// bookkeeping; a bake is out of date when this changes, not `version`.
    pub motion_version: String,
    /// What agents changed since a person last saved it, which the editor
    /// shows as drafted; none when a person has saved since.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub drafts: Option<RecipeDrafts>,
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
        rest: false,
        by_name: false,
        trim: None,
        blend: None,
    }
}

/// A clip a recipe is made or changed on: a library clip by id, or one the
/// recipe already has by its index (kept as it is, its hash, trim and blend
/// included unless the pick sends a trim or blend), with its trim and,
/// after the first, its blend window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClipPick {
    /// A library clip's id, as list_animations gives it.
    pub clip: Option<String>,
    /// The index of a clip the recipe has: 0 the base.
    pub keep: Option<u32>,
    /// None keeps a kept clip's trim, or takes the whole of a library clip.
    pub trim: Option<[u32; 2]>,
    /// None keeps a kept clip's blend, or takes the default: 8 frames,
    /// smooth, hips and facing lined up. The first clip never has one.
    pub blend: Option<BlendWindow>,
}

/// For each of `picks`, the index of the recipe's clip it keeps, or none
/// for a library clip: what [`RecipeChange::kept`] takes, so keys and time
/// edits follow their clip.
pub fn kept_of(picks: &[ClipPick]) -> Vec<Option<u32>> {
    picks.iter().map(|p| p.keep).collect()
}

/// The clips `picks` name, in order, each library clip read from the
/// library of the workspace at `root` and each kept one from `current`.
///
/// # Errors
///
/// `invalid` for a pick that names both or neither, or a clip index
/// `current` does not have; as [`load`] for an unknown library clip.
pub fn clips_of(
    root: &Path,
    current: Option<&RecipeFile>,
    picks: &[ClipPick],
) -> CoreResult<Vec<RecipeBase>> {
    picks
        .iter()
        .enumerate()
        .map(|(i, pick)| {
            let mut clip = match (&pick.clip, pick.keep) {
                (Some(id), None) => base_of(&load(root, id)?),
                (None, Some(index)) => current
                    .and_then(|r| r.clip(index))
                    .cloned()
                    .ok_or_else(|| {
                        CoreError::Invalid(format!(
                            "The recipe has no clip {} to keep.",
                            u64::from(index) + 1
                        ))
                    })?,
                _ => {
                    return Err(CoreError::Invalid(format!(
                        "Give clip {} as a library clip id or the index of a clip the recipe has, one of the two.",
                        i + 1
                    )));
                }
            };
            // A kept clip keeps its trim and blend unless the pick sends them.
            if pick.trim.is_some() || pick.keep.is_none() {
                clip.trim = pick.trim;
            }
            clip.blend = if i == 0 {
                None
            } else {
                Some(pick.blend.clone().or(clip.blend.take()).unwrap_or_default())
            };
            Ok(clip)
        })
        .collect()
}

/// The first clip of `clips` called `name`, case aside, whatever its
/// frame count.
pub fn clip_named<'a>(clips: &'a [AnimationClip], name: &str) -> Option<&'a AnimationClip> {
    let name = name.trim().to_lowercase();
    clips.iter().find(|c| c.name.trim().to_lowercase() == name)
}

/// Finds `base` among `clips` by name (case aside) and frame count,
/// preferring the clip with the same keys. A rest pose base needs no clip;
/// a base known by name only takes the first clip of that name.
pub fn match_base<'a>(
    clips: &'a [AnimationClip],
    base: &RecipeBase,
) -> (BaseMatch, Option<&'a AnimationClip>) {
    if base.rest {
        let rest = BaseMatch {
            status: BaseStatus::Rest,
            clip: None,
            warning: None,
        };
        return (rest, None);
    }
    if base.name_only() {
        if let Some(clip) = clip_named(clips, &base.name) {
            let found = BaseMatch {
                status: BaseStatus::Found,
                clip: Some(clip.id.clone()),
                warning: None,
            };
            return (found, Some(clip));
        }
        let missing = BaseMatch {
            status: BaseStatus::Missing,
            clip: None,
            warning: Some(format!(
                "The animation library has no clip called {}. Import one before baking.",
                base.name
            )),
        };
        return (missing, None);
    }
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

/// Whether a recipe on `base` bakes as Mixamo's: marked so in its file, or
/// its base matches a clip of `clips` read from a Mixamo rig.
pub fn base_mixamo(clips: &[AnimationClip], base: &RecipeBase) -> bool {
    base.mixamo
        || match_base(clips, base)
            .1
            .is_some_and(|c| c.rig.kind == SourceRig::Mixamo)
}

/// Whether `recipe` bakes as Mixamo's: any of its clips does
/// ([`base_mixamo`]).
pub fn recipe_mixamo(clips: &[AnimationClip], recipe: &RecipeFile) -> bool {
    recipe.clips().any(|c| base_mixamo(clips, c))
}

/// Every recipe of the project at `project_dir`, each Mixamo mark read
/// against the clip library of the workspace at `root` as [`get`] reads it.
///
/// # Errors
///
/// Fails when the recipes folder or the clip library cannot be read.
pub fn summaries(
    root: &Path,
    project_dir: &Path,
    manifest: &ProjectManifest,
) -> CoreResult<Vec<recipes::RecipeSummary>> {
    let clips = list(root)?;
    recipes::list_with(project_dir, manifest, &|b| base_mixamo(&clips, b))
}

/// The recipe `id` as its file holds it, for a change that keeps some of
/// its clips ([`clips_of`]).
///
/// # Errors
///
/// As [`recipes::get`].
pub fn current(project_dir: &Path, manifest: &ProjectManifest, id: &str) -> CoreResult<RecipeFile> {
    Ok(recipes::get(project_dir, manifest, id)?.recipe)
}

/// Renames the recipe `id` as [`recipes::rename`] does, its Mixamo mark read
/// as [`summaries`] reads it.
///
/// # Errors
///
/// As [`recipes::rename`], or when the clip library cannot be read.
pub fn rename(
    root: &Path,
    project_dir: &Path,
    id: &str,
    name: &str,
) -> CoreResult<recipes::RecipeSummary> {
    let clips = list(root)?;
    recipes::rename_with(project_dir, id, name, &|b| base_mixamo(&clips, b))
}

/// `read` as the view gets it, its base matched against `clips`.
pub fn doc_of(id: &str, read: RecipeRead, clips: &[AnimationClip]) -> RecipeDoc {
    let RecipeRead {
        recipe,
        warnings,
        gone,
        version,
        motion_version,
    } = read;
    let (base, _) = match_base(clips, &recipe.base);
    let joined = recipe
        .joined
        .iter()
        .map(|c| match_base(clips, c).0)
        .collect();
    let mixamo = recipe_mixamo(clips, &recipe);
    RecipeDoc {
        id: id.to_string(),
        file: format!("{}/{id}{}", recipes::RECIPES_DIR, recipes::RECIPE_SUFFIX),
        recipe,
        base,
        joined,
        mixamo,
        warnings,
        gone,
        version,
        motion_version,
        drafts: None,
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
    Ok(with_drafts(project_dir, doc_of(id, read, &list(root)?)))
}

/// `doc` with the draft marks the project keeps on it.
fn with_drafts(project_dir: &Path, mut doc: RecipeDoc) -> RecipeDoc {
    doc.drafts = drafts::read(project_dir, &doc.id, &doc.recipe);
    doc
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
    Ok(with_drafts(project_dir, doc_of(id, read, &list(root)?)))
}

/// A person's save of the recipe `id`: [`update`], with the draft marks
/// cleared under the same write lock once it is written ([`recipes::save`]).
///
/// # Errors
///
/// As [`update`]; marks that cannot be cleared never fail it.
pub fn save(
    root: &Path,
    project_dir: &Path,
    manifest: &ProjectManifest,
    id: &str,
    change: RecipeChange,
) -> CoreResult<RecipeDoc> {
    let read = recipes::save(project_dir, manifest, id, change)?;
    Ok(with_drafts(project_dir, doc_of(id, read, &list(root)?)))
}

/// Makes a recipe in the project at `project_dir` on the library clip
/// `clip_id`, or on the model's rest pose when there is none, called
/// `name` or else after the clip (or "Rest pose").
///
/// # Errors
///
/// Fails for an unknown clip, a name the recipe file refuses or when the
/// file cannot be written.
pub fn create(
    root: &Path,
    project_dir: &Path,
    clip_id: Option<&str>,
    name: Option<&str>,
) -> CoreResult<RecipeDoc> {
    let clip = clip_id.map(|id| load(root, id)).transpose()?;
    let base = clip
        .as_ref()
        .map_or_else(|| RecipeBase::rest_pose(recipes::REST_FRAMES), base_of);
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(&base.name)
        .to_string();
    let (id, recipe) = recipes::create(project_dir, &name, base)?;
    created(project_dir, &id, recipe, clip.as_slice())
}

/// The recipe `id` just written, as [`get`] reads it against `clips`.
fn created(
    project_dir: &Path,
    id: &str,
    recipe: RecipeFile,
    clips: &[AnimationClip],
) -> CoreResult<RecipeDoc> {
    let version = recipes::version(project_dir, id)?;
    let read = RecipeRead {
        motion_version: recipes::motion_version(&recipe),
        recipe,
        warnings: Vec::new(),
        gone: Vec::new(),
        version,
    };
    Ok(doc_of(id, read, clips))
}

/// Makes a recipe in the project at `project_dir` on the library clips
/// `picks` names, joined in order, mirrored or not, called `name` or else
/// after its first clip. One clip that is not trimmed or mirrored writes a
/// version 1 file, as [`create`] does.
///
/// # Errors
///
/// As [`clips_of`] (where a pick may not keep a clip, there being no
/// recipe yet) and [`create`].
pub fn create_joined(
    root: &Path,
    project_dir: &Path,
    picks: &[ClipPick],
    mirror: bool,
    name: Option<&str>,
) -> CoreResult<RecipeDoc> {
    let joined = clips_of(root, None, picks)?;
    let first = joined.first().map(|c| c.name.clone()).unwrap_or_default();
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(&first)
        .to_string();
    let (id, recipe) = recipes::create_joined(project_dir, &name, joined, mirror)?;
    created(project_dir, &id, recipe, &list(root)?)
}

#[cfg(test)]
mod tests;
