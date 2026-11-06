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
use crate::animations::recipes::{BaseMatch, base_of, clip_named, match_base};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, now_rfc3339};
use crate::recipes::{
    BlankKind, LayerKind, LayerSettings, RecipeBase, RecipeBlank, RecipeFile, RecipeLayer,
};
use crate::workspace::FolderLayout;

/// What every starter id starts with.
pub const STARTER_PREFIX: &str = "starter:";

/// The clip names an idle goes by, tried in order.
const IDLE: &[&str] = &["Idle", "Breathing Idle", "Standing Idle", "Happy Idle"];
/// The clip names a walk goes by, tried in order.
const WALK: &[&str] = &["Walking", "Walk"];

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
}

/// One starter as it is built in.
struct Spec {
    key: &'static str,
    name: &'static str,
    about: &'static str,
    /// Empty for the rest pose.
    clips: &'static [&'static str],
    /// How long a rest pose starter runs, frames.
    frames: u32,
    blanks: fn() -> Vec<RecipeBlank>,
    layers: fn() -> Vec<RecipeLayer>,
}

fn blank(id: &str, label: &str, point: Option<[f32; 3]>) -> RecipeBlank {
    RecipeBlank {
        id: id.into(),
        label: label.into(),
        kind: if point.is_some() {
            BlankKind::Point
        } else {
            BlankKind::Prop
        },
        prop: None,
        point,
    }
}

fn layer(id: &str, kind: LayerKind, bone: &str, settings: LayerSettings) -> RecipeLayer {
    RecipeLayer {
        id: id.into(),
        kind,
        bone: bone.into(),
        settings,
    }
}

fn span(start: f32, end: f32) -> LayerSettings {
    LayerSettings {
        start,
        end,
        ..LayerSettings::default()
    }
}

fn aimed(start: f32, end: f32, target: &str) -> LayerSettings {
    LayerSettings {
        target: Some(target.into()),
        ..span(start, end)
    }
}

fn turned(start: f32, end: f32, rotation: [f32; 3]) -> LayerSettings {
    LayerSettings {
        rotation: Some(rotation),
        ..span(start, end)
    }
}

const SPECS: [Spec; 5] = [
    Spec {
        key: "wave-hello",
        name: "Wave hello",
        about: "Raises the right arm and waves the forearm, on the rest pose.",
        clips: &[],
        frames: 60,
        blanks: Vec::new,
        layers: || {
            vec![
                layer(
                    "l-1",
                    LayerKind::Nudge,
                    "rightUpperArm",
                    turned(0.0, 1.0, [0.0, 0.0, -60.0]),
                ),
                layer(
                    "l-2",
                    LayerKind::Wave,
                    "rightLowerArm",
                    LayerSettings {
                        cycles: Some(3.0),
                        amount: Some(30.0),
                        ..span(0.15, 0.95)
                    },
                ),
            ]
        },
    },
    Spec {
        key: "look-at-point",
        name: "Look at a point",
        about: "Turns the head toward a point while the idle plays.",
        clips: IDLE,
        frames: 0,
        blanks: || vec![blank("spot", "Where the head looks", Some([0.6, 1.6, 1.2]))],
        layers: || {
            vec![layer(
                "l-1",
                LayerKind::LookAt,
                "head",
                aimed(0.1, 0.9, "spot"),
            )]
        },
    },
    Spec {
        key: "reach-for-prop",
        name: "Reach for a prop",
        about: "The right hand reaches a prop of the project while the idle plays.",
        clips: IDLE,
        frames: 0,
        blanks: || vec![blank("prop", "The prop the hand reaches", None)],
        layers: || {
            vec![layer(
                "l-1",
                LayerKind::Reach,
                "rightHand",
                aimed(0.2, 0.8, "prop"),
            )]
        },
    },
    Spec {
        key: "lean-in",
        name: "Lean in",
        about: "Leans the upper body forward over the idle, as if listening.",
        clips: IDLE,
        frames: 0,
        blanks: Vec::new,
        layers: || {
            vec![
                layer(
                    "l-1",
                    LayerKind::Nudge,
                    "spine",
                    turned(0.1, 0.9, [12.0, 0.0, 0.0]),
                ),
                layer(
                    "l-2",
                    LayerKind::Nudge,
                    "head",
                    turned(0.1, 0.9, [-6.0, 0.0, 0.0]),
                ),
            ]
        },
    },
    Spec {
        key: "point-while-walking",
        name: "Point while walking",
        about: "The right hand points ahead and to the right during a walk.",
        clips: WALK,
        frames: 0,
        blanks: || {
            vec![blank(
                "spot",
                "Where the hand points",
                Some([-0.4, 1.4, 0.8]),
            )]
        },
        layers: || {
            vec![layer(
                "l-1",
                LayerKind::Reach,
                "rightHand",
                aimed(0.25, 0.75, "spot"),
            )]
        },
    },
];

/// The starter key in `id`, when it is a starter id.
pub fn starter_key(id: &str) -> Option<&str> {
    id.strip_prefix(STARTER_PREFIX)
}

fn spec(id: &str) -> CoreResult<&'static Spec> {
    starter_key(id)
        .and_then(|key| SPECS.iter().find(|s| s.key == key))
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

fn view(spec: &Spec, clips: &[AnimationClip]) -> RecipeStarter {
    let recipe = recipe_of(spec, clips);
    let (base, clip) = match_base(clips, &recipe.base);
    RecipeStarter {
        id: format!("{STARTER_PREFIX}{}", spec.key),
        name: spec.name.into(),
        about: spec.about.into(),
        clips: spec.clips.iter().map(|c| (*c).to_string()).collect(),
        rest: spec.clips.is_empty(),
        mixamo: recipe.base.mixamo,
        base_name: clip.map_or(recipe.base.name.clone(), |c| c.name.clone()),
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
    Ok(SPECS.iter().map(|s| view(s, &clips)).collect())
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
    let spec = spec(id)?;
    Ok(recipe_of(spec, &crate::animations::list(root)?))
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
    let spec = spec(id)?;
    let recipe = recipe(root, id)?;
    add_entry(
        root,
        spec.name,
        recipe,
        Some(format!("Starter: {}", spec.name)),
    )
}

#[cfg(test)]
mod tests;
