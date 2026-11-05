//! A project's recipe files: listing, making, reading, renaming, changing
//! and removing them. Reading checks prop blanks against the project, so a
//! prop that is no longer an asset reads as an open blank with a warning. Writes in this process go one at a time, and each
//! reads the file on disk under the same lock first, so a recipe removed
//! meanwhile (here or by a sync from another computer) is not written back.

use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use super::{
    ID_MAX_CHARS, RECIPE_SUFFIX, RECIPES_DIR, RecipeBase, RecipeBlank, RecipeFile, RecipeLayer,
    check_name, check_props, dir, is_recipe_id, load_file, normalized, path, settle_props,
    validate,
};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::plans::trimmed;
use crate::project::{ProjectManifest, latin_slug, now_rfc3339, windows_safe};

/// A line in the list of a project's recipes.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSummary {
    pub id: String,
    /// `recipes/<id>.recipe.json`.
    pub file: String,
    /// The recipe name, or the file's id when it could not be read.
    pub name: String,
    /// The base clip's name; empty when the file could not be read.
    pub base: String,
    pub frames: u32,
    /// The base clip came from a Mixamo rig.
    pub mixamo: bool,
    pub layers: u32,
    pub blanks: u32,
    /// Blanks nothing fills yet, a prop the project no longer has included.
    pub open: u32,
    /// What to know before baking: a prop blank that opened again.
    pub warnings: Vec<String>,
    /// RFC 3339; empty when the file could not be read.
    pub edited: String,
    /// Why the file could not be read, for a recipe that only lists.
    pub problem: Option<String>,
}

/// What a change to a recipe replaces: its name, the changes, the blanks,
/// or any of them, each whole. A change with an empty id gets a free one
/// (`l-1`, `l-2`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeChange {
    /// A new name; the file and id stay.
    #[serde(default)]
    pub name: Option<String>,
    pub layers: Option<Vec<RecipeLayer>>,
    pub blanks: Option<Vec<RecipeBlank>>,
}

/// A recipe as read for its project, prop blanks the project no longer
/// has opened, with a warning for each.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeRead {
    pub recipe: RecipeFile,
    pub warnings: Vec<String>,
}

/// Opens the prop blanks `manifest`'s project no longer has.
fn settled(mut recipe: RecipeFile, manifest: &ProjectManifest) -> RecipeRead {
    let warnings = settle_props(&mut recipe, manifest);
    RecipeRead { recipe, warnings }
}

/// The longest slug a free id starts from, leaving room for `-9999`.
const SLUG_MAX_CHARS: usize = ID_MAX_CHARS - 5;

static WRITES: Mutex<()> = Mutex::new(());

fn writing() -> MutexGuard<'static, ()> {
    WRITES.lock().unwrap_or_else(PoisonError::into_inner)
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn file_of(id: &str) -> String {
    format!("{RECIPES_DIR}/{id}{RECIPE_SUFFIX}")
}

fn summary(id: &str, read: &RecipeRead) -> RecipeSummary {
    let recipe = &read.recipe;
    RecipeSummary {
        id: id.to_string(),
        file: file_of(id),
        name: recipe.name.clone(),
        base: recipe.base.name.clone(),
        frames: recipe.base.frames,
        mixamo: recipe.base.mixamo,
        layers: count(recipe.layers.len()),
        blanks: count(recipe.blanks.len()),
        open: count(recipe.blanks.iter().filter(|b| !b.filled()).count()),
        edited: recipe.edited.clone(),
        warnings: read.warnings.clone(),
        problem: None,
    }
}

fn broken(id: &str, problem: String) -> RecipeSummary {
    RecipeSummary {
        id: id.to_string(),
        file: file_of(id),
        name: id.to_string(),
        base: String::new(),
        frames: 0,
        mixamo: false,
        layers: 0,
        blanks: 0,
        open: 0,
        edited: String::new(),
        warnings: Vec::new(),
        problem: Some(problem),
    }
}

/// Every recipe in the project of `manifest` at `project_dir`, sorted by
/// id. A file that cannot be read still lists, with its problem, so it can
/// be removed.
///
/// # Errors
///
/// Fails only when the `recipes` folder exists but cannot be read.
pub fn list(project_dir: &Path, manifest: &ProjectManifest) -> CoreResult<Vec<RecipeSummary>> {
    let folder = dir(project_dir);
    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&folder, err)),
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(id) = name.strip_suffix(RECIPE_SUFFIX) else {
            continue;
        };
        if !is_recipe_id(id) || !entry.path().is_file() {
            continue;
        }
        found.push(match load_file(&entry.path()) {
            Ok(recipe) => summary(id, &settled(recipe, manifest)),
            Err(err) => broken(id, err.to_string()),
        });
    }
    found.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(found)
}

/// The recipe `id` of the project of `manifest` at `project_dir`.
///
/// # Errors
///
/// Fails when it is missing or [`load_file`] refuses it.
pub fn get(project_dir: &Path, manifest: &ProjectManifest, id: &str) -> CoreResult<RecipeRead> {
    Ok(settled(load_file(&path(project_dir, id)?)?, manifest))
}

/// A free id for a recipe called `name`: its slug, then `-2`, `-3` and on.
fn free_id(project_dir: &Path, name: &str) -> CoreResult<String> {
    let base = latin_slug(name).map_or_else(
        || "recipe".to_string(),
        |slug| {
            let short: String = slug.chars().take(SLUG_MAX_CHARS).collect();
            windows_safe(short.trim_end_matches('-').to_string())
        },
    );
    for n in 1..10_000 {
        let id = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        if !path(project_dir, &id)?.exists() {
            return Ok(id);
        }
    }
    Err(CoreError::Invalid(format!(
        "There are too many recipes called {name}."
    )))
}

/// Makes a recipe called `name` on `base`, with no changes yet, under a
/// free id made from its name. Answers with the id and the recipe.
///
/// # Errors
///
/// Fails on a name [`check_name`] refuses, a base [`validate`] refuses or
/// when the file cannot be written.
pub fn create(
    project_dir: &Path,
    name: &str,
    base: RecipeBase,
) -> CoreResult<(String, RecipeFile)> {
    check_name(name)?;
    let recipe = normalized(&RecipeFile::new(name, base));
    validate(&recipe)?;
    let _lock = writing();
    let id = free_id(project_dir, &recipe.name)?;
    let folder = dir(project_dir);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    write_json(&path(project_dir, &id)?, &recipe)?;
    Ok((id, recipe))
}

/// Renames the recipe `id`. The file and its id stay.
///
/// # Errors
///
/// Fails on a name [`check_name`] refuses, when the recipe is gone or the
/// file cannot be written.
pub fn rename(project_dir: &Path, id: &str, name: &str) -> CoreResult<RecipeSummary> {
    check_name(name)?;
    let _lock = writing();
    let file = path(project_dir, id)?;
    let mut recipe = load_file(&file)?;
    let name = trimmed(name);
    if recipe.name != name {
        recipe.name = name.to_string();
        recipe.edited = now_rfc3339();
        write_json(&file, &recipe)?;
    }
    let read = RecipeRead {
        recipe,
        warnings: Vec::new(),
    };
    Ok(summary(id, &read))
}

/// Free ids for the changes that came without one.
fn with_ids(layers: Vec<RecipeLayer>) -> Vec<RecipeLayer> {
    let mut taken: Vec<String> = layers
        .iter()
        .map(|l| trimmed(&l.id).to_string())
        .filter(|id| !id.is_empty())
        .collect();
    let mut next = 1u32;
    layers
        .into_iter()
        .map(|mut layer| {
            if trimmed(&layer.id).is_empty() {
                while taken.contains(&format!("l-{next}")) {
                    next += 1;
                }
                layer.id = format!("l-{next}");
                taken.push(layer.id.clone());
            }
            layer
        })
        .collect()
}

/// Replaces the name, the changes, the blanks or any of them of the recipe
/// `id`, as the file holds it now, in one write after checking the whole
/// result. Blanks sent must name assets of `manifest`'s project. When the
/// blanks are kept, they stay in the file as they are: a prop blank naming
/// an asset the project no longer has keeps its prop, so restoring the
/// asset fills it again, and only the answer reads it as open, with a
/// warning, as [`get`] does.
/// `edited` moves only when something changed; the revision never does.
///
/// # Errors
///
/// Fails on a name [`check_name`] refuses, when the recipe is gone, the
/// result does not validate or the file cannot be written.
pub fn update(
    project_dir: &Path,
    manifest: &ProjectManifest,
    id: &str,
    change: RecipeChange,
) -> CoreResult<RecipeRead> {
    if let Some(name) = &change.name {
        check_name(name)?;
    }
    let _lock = writing();
    let file = path(project_dir, id)?;
    let current = load_file(&file)?;
    let mut next = current.clone();
    if let Some(name) = change.name {
        next.name = trimmed(&name).to_string();
    }
    if let Some(layers) = change.layers {
        next.layers = with_ids(layers);
    }
    let sent_blanks = change.blanks.is_some();
    if let Some(blanks) = change.blanks {
        next.blanks = blanks;
    }
    let mut next = normalized(&next);
    validate(&next)?;
    if sent_blanks {
        check_props(&next, manifest)?;
    }
    if next != current {
        next.edited = now_rfc3339();
        write_json(&file, &next)?;
    }
    Ok(settled(next, manifest))
}

/// Moves the recipe file into `trash_dir` (the workspace's trash for this
/// project) as `recipes/<id>-<time>.recipe.json`, so it can still be got
/// back by hand. Answers with where it went.
///
/// # Errors
///
/// Fails when the recipe is missing or the file cannot be moved.
pub fn remove(project_dir: &Path, trash_dir: &Path, id: &str) -> CoreResult<String> {
    let _lock = writing();
    let file = path(project_dir, id)?;
    if !file.is_file() {
        return Err(CoreError::NotFound(format!("the recipe {id}")));
    }
    let to_dir = trash_dir.join(RECIPES_DIR);
    fs::create_dir_all(&to_dir).map_err(|e| CoreError::io(&to_dir, e))?;
    let stamp: String = now_rfc3339().chars().filter(char::is_ascii_digit).collect();
    let to = to_dir.join(format!("{id}-{stamp}{RECIPE_SUFFIX}"));
    crate::plans::move_file(&file, &to)?;
    Ok(to.display().to_string())
}
