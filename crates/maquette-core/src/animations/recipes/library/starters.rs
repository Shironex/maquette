//! Starter recipes: a few recipes built into the app that show the four
//! kinds of change. They live in the binary, never in the recipe library:
//! listing them writes nothing, and only a person's Save to library (or a
//! save an agent proposed and the person approved) puts a copy there.
//!
//! A starter names its base clip by name only, a few names tried in order
//! (an idle is `Idle` in one pack and `Breathing Idle` in another), or runs
//! on the rest pose. It never carries a clip hash, keys, frames or any clip
//! data. Used in a project or copied to the library, it takes the first
//! clip of the clip library with one of those names, whatever its frame
//! count, and stores that clip's base. With none of them, the recipe keeps
//! the first name only and reads as missing its base, as any recipe whose
//! clip is not in the library does.
//!
//! Starter ids start with `starter:`, which no library recipe id can hold,
//! so a starter is never mistaken for a library recipe.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::use_in::{RecipeUsed, UseRecipe, landed, open_missing_props};
use super::{RecipeImported, transfer::add_entry};
use crate::animations::AnimationClip;
use crate::animations::recipes::{BaseMatch, BaseStatus, base_of, clip_named, match_base};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, now_rfc3339};
use crate::recipes::{RecipeBase, RecipeBlank, RecipeFile, RecipeLayer};
use crate::workspace::FolderLayout;

mod combat;
mod specs;

use combat::{COMBOS, Combo};
use specs::{SPECS, Spec};

/// What every starter id starts with.
pub const STARTER_PREFIX: &str = "starter:";

/// A starter recipe as the app and agents read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeStarter {
    /// `starter:<key>`.
    pub id: String,
    pub name: String,
    /// One line on what it does.
    pub about: String,
    /// The clip names it runs on, tried in order; empty on the rest pose.
    pub clips: Vec<String>,
    /// Runs on the model's rest pose, with no base clip.
    pub rest: bool,
    pub blanks: Vec<RecipeBlank>,
    pub layers: Vec<RecipeLayer>,
    /// The clip it would take from this computer's clip library, or the
    /// first name it tries, or "Rest pose".
    pub base_name: String,
    /// How its base stands in the clip library: found, missing or rest.
    pub base: BaseMatch,
    pub mixamo: bool,
    /// The clips joined after the base, each as it would take it from the
    /// clip library or its first name; empty on one clip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[specta(optional)]
    pub joined: Vec<String>,
    /// Left and right swap across the whole recipe: a left handed copy.
    #[serde(default)]
    #[specta(optional)]
    pub mirror: bool,
    /// The joined clips no clip of the clip library stands for yet, by the
    /// first name each tries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[specta(optional)]
    pub missing: Vec<String>,
    /// Its holds and other time edits, as on the clips it would take.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[specta(optional)]
    pub time: Vec<crate::recipes::TimeEdit>,
}

/// A built in starter: a plain one, or a combat combo of joined clips.
#[derive(Clone, Copy)]
enum Starter {
    Plain(&'static Spec),
    Combo(&'static Combo),
}

impl Starter {
    fn key(self) -> &'static str {
        match self {
            Self::Plain(s) => s.key,
            Self::Combo(c) => c.key,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Plain(s) => s.name,
            Self::Combo(c) => c.name,
        }
    }

    fn about(self) -> &'static str {
        match self {
            Self::Plain(s) => s.about,
            Self::Combo(c) => c.about,
        }
    }

    /// The names its base goes by, tried in order; empty on the rest pose.
    fn clips(self) -> &'static [&'static str] {
        match self {
            Self::Plain(s) => s.clips,
            Self::Combo(c) => c.clips[0].names,
        }
    }

    fn recipe(self, clips: &[AnimationClip]) -> RecipeFile {
        match self {
            Self::Plain(s) => recipe_of(s, clips),
            Self::Combo(c) => combat::recipe_of(c, clips),
        }
    }
}

/// Every starter, the plain ones first.
fn all() -> impl Iterator<Item = Starter> {
    SPECS
        .iter()
        .map(Starter::Plain)
        .chain(COMBOS.iter().map(Starter::Combo))
}

/// The starter key in `id`, when it is a starter id.
pub fn starter_key(id: &str) -> Option<&str> {
    id.strip_prefix(STARTER_PREFIX)
}

fn spec(id: &str) -> CoreResult<Starter> {
    starter_key(id)
        .and_then(|key| all().find(|s| s.key() == key))
        .ok_or_else(|| CoreError::NotFound(format!("the starter recipe {id}")))
}

/// The base `spec` takes from `clips`: the rest pose, the first clip with
/// one of its names, or its first name only.
fn base_from(spec: &Spec, clips: &[AnimationClip]) -> RecipeBase {
    if spec.clips.is_empty() {
        return RecipeBase::rest_pose(spec.frames);
    }
    spec.clips
        .iter()
        .find_map(|name| clip_named(clips, name))
        .map_or_else(|| RecipeBase::named(spec.clips[0]), base_of)
}

/// `spec` as a recipe file on what `clips` holds, made now.
fn recipe_of(spec: &Spec, clips: &[AnimationClip]) -> RecipeFile {
    let mut recipe = RecipeFile::new(spec.name, base_from(spec, clips));
    recipe.blanks = (spec.blanks)();
    recipe.layers = (spec.layers)();
    recipe
}

fn view(starter: Starter, clips: &[AnimationClip]) -> RecipeStarter {
    let recipe = starter.recipe(clips);
    let (base, clip) = match_base(clips, &recipe.base);
    RecipeStarter {
        id: format!("{STARTER_PREFIX}{}", starter.key()),
        name: starter.name().into(),
        about: starter.about().into(),
        clips: starter.clips().iter().map(|c| (*c).to_string()).collect(),
        rest: starter.clips().is_empty(),
        mixamo: recipe.base.mixamo,
        base_name: clip.map_or(recipe.base.name.clone(), |c| c.name.clone()),
        joined: recipe.joined.iter().map(|c| c.name.clone()).collect(),
        missing: recipe
            .joined
            .iter()
            .filter(|c| match_base(clips, c).0.status == BaseStatus::Missing)
            .map(|c| c.name.clone())
            .collect(),
        mirror: recipe.mirror,
        time: recipe.time,
        blanks: recipe.blanks,
        layers: recipe.layers,
        base,
    }
}

/// Every starter, matched against the clip library of the workspace at
/// `root`. Writes nothing.
///
/// # Errors
///
/// Fails only when the clip library cannot be read.
pub fn list(root: &Path) -> CoreResult<Vec<RecipeStarter>> {
    let clips = crate::animations::list(root)?;
    Ok(all().map(|s| view(s, &clips)).collect())
}

/// The starter `id`, matched against the clip library.
///
/// # Errors
///
/// Fails for an unknown starter or when the clip library cannot be read.
pub fn get(root: &Path, id: &str) -> CoreResult<RecipeStarter> {
    let spec = spec(id)?;
    Ok(view(spec, &crate::animations::list(root)?))
}

/// The starter `id` as the recipe file it makes on this computer.
///
/// # Errors
///
/// As [`get`].
pub fn recipe(root: &Path, id: &str) -> CoreResult<RecipeFile> {
    Ok(spec(id)?.recipe(&crate::animations::list(root)?))
}

/// Copies starter `request.entry` into a project as a new recipe, as
/// use_recipe does with a library recipe, but with no library link.
///
/// # Errors
///
/// Fails for an unknown project or starter, a revision (a starter has
/// none), or when the file cannot be written.
pub fn use_starter(
    root: &Path,
    layout: &FolderLayout,
    request: &UseRecipe,
) -> CoreResult<RecipeUsed> {
    if request.revision.is_some() {
        return Err(CoreError::Invalid(
            "A starter recipe has no revisions. Leave revision out.".into(),
        ));
    }
    let manifest = project::load(root, layout, &request.project)?;
    let project_dir = project::dir(root, layout, &request.project)?;
    let mut recipe = recipe(root, &request.entry)?;
    if let Some(name) = request
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        name.clone_into(&mut recipe.name);
    }
    recipe.edited = now_rfc3339();
    let notes = open_missing_props(&mut recipe, &manifest);
    landed(root, &project_dir, &manifest, &recipe, None, notes)
}

/// Copies starter `id` into the recipe library as a new recipe at r1, which
/// the person can then edit like any other. Only the person's Save to
/// library calls this; no agent tool reaches it.
///
/// # Errors
///
/// Fails for an unknown starter or when the library cannot be written.
pub fn copy_to_library(root: &Path, id: &str) -> CoreResult<RecipeImported> {
    let name = spec(id)?.name();
    let recipe = recipe(root, id)?;
    add_entry(root, name, recipe, Some(format!("Starter: {name}")))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_combat;
