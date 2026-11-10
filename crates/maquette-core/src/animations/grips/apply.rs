//! Laying a library grip on a project's recipe: a new grip change that
//! holds a copy of the revision's pose and a link back, or a grip change
//! that is there already taking another revision. Also what the library
//! says about the links a recipe holds, which is where the offer of a
//! newer revision comes from. Nothing here reads the library to play or
//! bake a recipe: the copy in the recipe is all a bake uses.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use super::{LibraryGripEntry, link_to, linked_entry, load_entry, read_revision};
use crate::animations::recipes::{self as library_recipes, RecipeDoc};
use crate::error::{CoreError, CoreResult};
use crate::project;
use crate::recipes::grips::{self, GripHands};
use crate::recipes::{self, LayerKind, LayerMode, RecipeChange, RecipeFile, RecipeLayer};
use crate::workspace::FolderLayout;

/// A grip to lay on a recipe.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ApplyGrip {
    pub project: String,
    pub recipe: String,
    /// The library grip's id.
    pub grip: String,
    /// Its revision; the newest when left out.
    pub revision: Option<u32>,
    /// The hands to take: both, or one (the other hand's as its mirror
    /// when the grip does not pose it). Left out, a new change takes the
    /// grip's own hands and one that is there keeps the hands it posed.
    pub hands: Option<GripHands>,
    /// The grip change to take it into. None, or an id the recipe has not
    /// got, adds a change (with that id, or the lowest free `l-N`).
    pub layer: Option<String>,
    /// The recipe's version as it was read; a recipe that changed since is
    /// refused (code `moved`).
    pub expected: Option<String>,
    /// Where it sits and how it is laid on; what is left out stays (a new
    /// change covers the whole recipe at full weight).
    pub start: Option<f32>,
    pub end: Option<f32>,
    pub weight: Option<f32>,
    pub ease_in: Option<u32>,
    pub ease_out: Option<u32>,
    pub mode: Option<LayerMode>,
    /// The blank that holds what the hand holds.
    pub target: Option<String>,
}

/// A grip laid on a recipe.
#[derive(Debug, Clone, PartialEq)]
pub struct GripApplied {
    pub doc: RecipeDoc,
    /// The grip change's id.
    pub layer: String,
    /// The library grip, its name and the revision copied.
    pub entry: String,
    pub name: String,
    pub revision: u32,
    /// The library's newest revision.
    pub latest: u32,
    /// A change that was there took the grip; false for a new change.
    pub updated: bool,
}

fn settle(layer: &mut RecipeLayer, request: &ApplyGrip) {
    let s = &mut layer.settings;
    if let Some(start) = request.start {
        s.start = start;
    }
    if let Some(end) = request.end {
        s.end = end;
    }
    if let Some(weight) = request.weight {
        s.weight = weight;
    }
    if let Some(ease) = request.ease_in {
        s.ease_in = Some(ease);
    }
    if let Some(ease) = request.ease_out {
        s.ease_out = Some(ease);
    }
    if let Some(target) = &request.target {
        s.target = Some(target.clone());
    }
    if let Some(mode) = request.mode {
        layer.mode = Some(mode);
    }
}

/// Lays revision `request.revision` (the newest by default) of a library
/// grip on a project's recipe, as a new grip change or into the one
/// `request.layer` names, in one checked write.
///
/// # Errors
///
/// Fails for an unknown project, recipe, grip or revision, a `layer` that
/// is another kind of change, a recipe that moved since `expected` (code
/// `moved`), or a result the recipe checks refuse (32 changes, 1024 bone
/// turns, a target that is no blank).
pub fn apply(root: &Path, layout: &FolderLayout, request: &ApplyGrip) -> CoreResult<GripApplied> {
    let manifest = project::load(root, layout, &request.project)?;
    let dir = project::dir(root, layout, &request.project)?;
    let read = recipes::get(&dir, &manifest, &request.recipe)?;
    let (entry, _) = load_entry(root, &request.grip)?;
    let grip = read_revision(root, &entry, request.revision)?;
    let latest = entry.latest().map_or(grip.revision, |r| r.revision);
    let from = link_to(&entry, grip.revision);
    let mut layers = read.recipe.layers.clone();
    let wanted = request
        .layer
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty());
    let at = wanted.and_then(|id| layers.iter().position(|l| l.id == id));
    let index = match at {
        Some(i) => {
            if layers[i].kind != LayerKind::Grip {
                return Err(CoreError::Invalid(format!(
                    "{} is not a grip change. Send the id of one, or leave layer out to add one.",
                    layers[i].id
                )));
            }
            layers[i] = grips::updated(&layers[i], &grip, from, request.hands);
            i
        }
        None => {
            layers.push(grips::layer_of(
                wanted.unwrap_or(""),
                &grip,
                from,
                request.hands,
            ));
            layers.len() - 1
        }
    };
    settle(&mut layers[index], request);
    let change = RecipeChange {
        layers: Some(layers),
        expected: Some(request.expected.clone().unwrap_or(read.version)),
        ..RecipeChange::default()
    };
    let doc = library_recipes::update(root, &dir, &manifest, &request.recipe, change)?;
    let layer = doc
        .recipe
        .layers
        .get(index)
        .map(|l| l.id.clone())
        .unwrap_or_default();
    Ok(GripApplied {
        doc,
        layer,
        entry: entry.id,
        name: entry.name,
        revision: grip.revision,
        latest,
        updated: at.is_some(),
    })
}

/// What the library says about one grip change's link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GripLink {
    /// The grip change's id.
    pub layer: String,
    /// The library grip it names and the revision it copied.
    pub entry: String,
    pub revision: u32,
    /// The library grip's name; none when it is gone or cannot be read.
    pub name: Option<String>,
    /// The library's newest revision; none when it is gone or cannot be read.
    pub latest: Option<u32>,
    /// The library has a newer revision than the one copied.
    pub newer: bool,
    /// The grip was removed, or its id now holds another grip.
    pub gone: bool,
}

/// Each grip change of `recipe` that came from a library grip, with how
/// that grip stands in the library of the workspace at `root` now.
pub fn links(root: &Path, recipe: &RecipeFile) -> Vec<GripLink> {
    let mut read: BTreeMap<(String, Option<String>), Option<Option<LibraryGripEntry>>> =
        BTreeMap::new();
    recipe
        .layers
        .iter()
        .filter_map(|l| l.from.as_ref().map(|from| (l, from)))
        .map(|(layer, from)| {
            // Some(Some) is there, Some(None) is gone, None cannot be read.
            let found = read
                .entry((from.entry.clone(), from.uid.clone()))
                .or_insert_with(|| linked_entry(root, from).ok().map(|e| e.map(|(e, _)| e)));
            let entry = found.as_ref().and_then(Option::as_ref);
            let latest = entry.and_then(|e| e.latest().map(|r| r.revision));
            GripLink {
                layer: layer.id.clone(),
                entry: from.entry.clone(),
                revision: from.revision,
                name: entry.map(|e| e.name.clone()),
                latest,
                newer: latest.is_some_and(|n| n > from.revision),
                gone: matches!(found, Some(None)),
            }
        })
        .collect()
}
