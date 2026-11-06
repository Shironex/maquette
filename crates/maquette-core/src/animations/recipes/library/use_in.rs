//! Using a library recipe in a project: a revision is copied in as a new
//! project recipe that remembers the entry and revision it came from, so
//! the project can say when the library has a newer one. Point blanks keep
//! their points. A prop blank whose asset the project does not have opens,
//! with a note. The base clip is matched in the clip library by name and
//! frame count: missing, the recipe is still made and says so; another
//! hash warns.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{load_entry, read_revision};
use crate::animations::recipes::{RecipeDoc, doc_of};
use crate::error::CoreResult;
use crate::project::{self, ProjectManifest, now_rfc3339};
use crate::recipes::{self, BlankKind, RecipeFile, RecipeRead};
use crate::workspace::FolderLayout;

/// What use_recipe copies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UseRecipe {
    /// The project it lands in.
    pub project: String,
    /// The library recipe, or a starter (`starter:...`).
    pub entry: String,
    /// Its revision; the newest when left out.
    pub revision: Option<u32>,
    /// The project recipe's name; the library recipe's when left out.
    pub name: Option<String>,
}

/// A recipe brought into a project, with what to know about it.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeUsed {
    pub doc: RecipeDoc,
    /// Plain lines: blanks that opened, the base clip missing or with other
    /// keys, an id that was taken.
    pub notes: Vec<String>,
}

/// Opens each prop blank of `recipe` whose asset `manifest`'s project does
/// not have, so a copy never names another project's asset. Answers with a
/// line per blank it opened.
pub fn open_missing_props(recipe: &mut RecipeFile, manifest: &ProjectManifest) -> Vec<String> {
    let mut notes = Vec::new();
    for blank in &mut recipe.blanks {
        if blank.kind != BlankKind::Prop {
            continue;
        }
        if let Some(prop) = &blank.prop
            && !manifest.assets.iter().any(|a| &a.id == prop)
        {
            notes.push(format!(
                "The blank {} named {prop}, which {} does not have, so it is open. Fill it with a prop of this project.",
                blank.id, manifest.name
            ));
            blank.prop = None;
        }
    }
    notes
}

/// The project recipe made from `recipe`, read back as the view reads it,
/// with the notes of the base match and of the id it took.
pub(super) fn landed(
    root: &Path,
    project_dir: &Path,
    manifest: &ProjectManifest,
    recipe: &RecipeFile,
    wanted: Option<&str>,
    mut notes: Vec<String>,
) -> CoreResult<RecipeUsed> {
    let (id, written, version) = recipes::create_whole(project_dir, recipe, wanted)?;
    let wanted_id = wanted
        .and_then(project::latin_slug)
        .or_else(|| project::latin_slug(&written.name))
        .map(|w| {
            let short: String = w.chars().take(recipes::ID_MAX_CHARS - 5).collect();
            short.trim_end_matches('-').to_string()
        });
    if let Some(wanted_id) = wanted_id.filter(|w| *w != id && id.starts_with(w.as_str())) {
        notes.push(format!(
            "The project has a recipe {wanted_id} already, so this one is {id}."
        ));
    }
    let gone = recipes::gone_props(&written, manifest);
    let motion_version = recipes::motion_version(&written);
    let mut settled = written;
    let warnings = recipes::settle_props(&mut settled, manifest);
    let read = RecipeRead {
        recipe: settled,
        warnings,
        gone,
        version,
        motion_version,
    };
    let doc = doc_of(&id, read, &crate::animations::list(root)?);
    if let Some(warning) = &doc.base.warning {
        notes.push(warning.clone());
    }
    Ok(RecipeUsed { doc, notes })
}

/// Copies revision `request.revision` (the newest by default) of a library
/// recipe into a project as a new recipe. A starter id (`starter:...`)
/// copies that starter instead ([`super::starters::use_starter`]).
///
/// # Errors
///
/// Fails for an unknown project, entry or revision, a name the recipe file
/// refuses, or when the file cannot be written.
pub fn use_recipe(
    root: &Path,
    layout: &FolderLayout,
    request: &UseRecipe,
) -> CoreResult<RecipeUsed> {
    if super::starters::starter_key(&request.entry).is_some() {
        return super::starters::use_starter(root, layout, request);
    }
    let manifest = project::load(root, layout, &request.project)?;
    let project_dir = project::dir(root, layout, &request.project)?;
    let (entry, _) = load_entry(root, &request.entry)?;
    let mut recipe = read_revision(root, &entry, request.revision)?;
    if let Some(name) = request
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        name.clone_into(&mut recipe.name);
    }
    let now = now_rfc3339();
    recipe.created.clone_from(&now);
    recipe.edited = now;
    recipe.from = Some(super::link_to(&entry, recipe.revision));
    let notes = open_missing_props(&mut recipe, &manifest);
    landed(root, &project_dir, &manifest, &recipe, None, notes)
}
