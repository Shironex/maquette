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
    BlankKind, GoneProp, ID_MAX_CHARS, RECIPE_SUFFIX, RECIPES_DIR, RecipeBase, RecipeBlank,
    RecipeFile, RecipeFrom, RecipeLayer, check_name, check_props, dir, gone_props, is_recipe_id,
    load_versioned, motion_version, normalized, path, settle_props, validate,
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
    /// A hash of the file's bytes, as get answers it, which a write or a
    /// bake may send back as `expected`; empty when it could not be read.
    pub version: String,
    /// A hash of what the recipe does, leaving out its library link and
    /// other bookkeeping ([`super::motion_version`]); what a bake's
    /// staleness is read by. Empty when it could not be read.
    pub motion_version: String,
    /// Why the file could not be read, for a recipe that only lists.
    pub problem: Option<String>,
    /// The library recipe and revision it came from or was last saved as.
    pub from: Option<RecipeFrom>,
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
    /// The `version` of the recipe as it was read (a hash of the file's
    /// bytes). Given, a file that changed since in any way is refused (code
    /// `moved`) and nothing is written.
    #[serde(default)]
    pub expected: Option<String>,
    /// Prop blanks to empty for good, even when the prop they name is no
    /// longer an asset (which a blank sent open otherwise keeps). A blank
    /// sent with a prop keeps that prop.
    #[serde(default)]
    pub clear: Vec<String>,
}

/// A recipe as read for its project, prop blanks the project no longer
/// has opened, with a warning for each.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeRead {
    pub recipe: RecipeFile,
    pub warnings: Vec<String>,
    /// The prop blanks that read open because their asset is gone; the
    /// file keeps each prop.
    pub gone: Vec<GoneProp>,
    /// A hash of the file's bytes, which a write may send back as
    /// `expected`.
    pub version: String,
    /// [`super::motion_version`] of the recipe as the file holds it.
    pub motion_version: String,
}

/// Opens the prop blanks `manifest`'s project no longer has.
fn settled(mut recipe: RecipeFile, version: String, manifest: &ProjectManifest) -> RecipeRead {
    let gone = gone_props(&recipe, manifest);
    let motion = motion_version(&recipe);
    let warnings = settle_props(&mut recipe, manifest);
    RecipeRead {
        recipe,
        warnings,
        gone,
        version,
        motion_version: motion,
    }
}

/// The longest slug a free id starts from, leaving room for `-9999`.
const SLUG_MAX_CHARS: usize = ID_MAX_CHARS - 5;

static WRITES: Mutex<()> = Mutex::new(());

pub(super) fn writing() -> MutexGuard<'static, ()> {
    WRITES.lock().unwrap_or_else(PoisonError::into_inner)
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn file_of(id: &str) -> String {
    format!("{RECIPES_DIR}/{id}{RECIPE_SUFFIX}")
}

fn summary(
    id: &str,
    read: &RecipeRead,
    clip_mixamo: &dyn Fn(&RecipeBase) -> bool,
) -> RecipeSummary {
    let recipe = &read.recipe;
    RecipeSummary {
        id: id.to_string(),
        file: file_of(id),
        name: recipe.name.clone(),
        base: recipe.base.name.clone(),
        frames: recipe.base.frames,
        mixamo: recipe.base.mixamo || clip_mixamo(&recipe.base),
        layers: count(recipe.layers.len()),
        blanks: count(recipe.blanks.len()),
        open: count(recipe.blanks.iter().filter(|b| !b.filled()).count()),
        edited: recipe.edited.clone(),
        version: read.version.clone(),
        motion_version: read.motion_version.clone(),
        warnings: read.warnings.clone(),
        problem: None,
        from: recipe.from.clone(),
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
        version: String::new(),
        motion_version: String::new(),
        warnings: Vec::new(),
        problem: Some(problem),
        from: None,
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
    list_with(project_dir, manifest, &|_| false)
}

/// As [`list`], each recipe also marked Mixamo when `clip_mixamo` says its
/// base matches a clip read from a Mixamo rig, as reading it one by one
/// does.
///
/// # Errors
///
/// As [`list`].
pub fn list_with(
    project_dir: &Path,
    manifest: &ProjectManifest,
    clip_mixamo: &dyn Fn(&RecipeBase) -> bool,
) -> CoreResult<Vec<RecipeSummary>> {
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
        found.push(match load_versioned(&entry.path()) {
            Ok((recipe, version)) => summary(id, &settled(recipe, version, manifest), clip_mixamo),
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
/// Fails when it is missing or [`super::load_file`] refuses it.
pub fn get(project_dir: &Path, manifest: &ProjectManifest, id: &str) -> CoreResult<RecipeRead> {
    let (recipe, version) = load_versioned(&path(project_dir, id)?)?;
    Ok(settled(recipe, version, manifest))
}

/// The version ([`super::version_of`]) of the recipe `id` as it is on disk.
///
/// # Errors
///
/// Fails when it is missing or [`super::load_file`] refuses it.
pub fn version(project_dir: &Path, id: &str) -> CoreResult<String> {
    Ok(load_versioned(&path(project_dir, id)?)?.1)
}

/// The [`super::motion_version`] of the recipe `id` as it is on disk.
///
/// # Errors
///
/// Fails when it is missing or [`super::load_file`] refuses it.
pub fn motion_version_of(project_dir: &Path, id: &str) -> CoreResult<String> {
    Ok(motion_version(&super::load_file(&path(project_dir, id)?)?))
}

/// The names of the recipes of the project at `project_dir`; a file that
/// cannot be read has none.
pub fn names(project_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir(project_dir)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(RECIPE_SUFFIX))
        .filter_map(|e| super::load_file(&e.path()).ok())
        .map(|r| r.name)
        .collect()
}

/// A free id for a recipe called `name`: its slug, then `-2`, `-3` and on.
pub(super) fn free_id(project_dir: &Path, name: &str) -> CoreResult<String> {
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

/// Renames the recipe `id`. The file and its id stay. Answers with the
/// recipe as it lists, its version that of the file as written.
///
/// # Errors
///
/// Fails on a name [`check_name`] refuses, when the recipe is gone or the
/// file cannot be written.
pub fn rename(project_dir: &Path, id: &str, name: &str) -> CoreResult<RecipeSummary> {
    rename_with(project_dir, id, name, &|_| false)
}

/// As [`rename`], its Mixamo mark read as [`list_with`] reads it.
///
/// # Errors
///
/// As [`rename`].
pub fn rename_with(
    project_dir: &Path,
    id: &str,
    name: &str,
    clip_mixamo: &dyn Fn(&RecipeBase) -> bool,
) -> CoreResult<RecipeSummary> {
    check_name(name)?;
    let _lock = writing();
    let file = path(project_dir, id)?;
    let (mut recipe, mut version) = load_versioned(&file)?;
    let name = trimmed(name);
    if recipe.name != name {
        recipe.name = name.to_string();
        recipe.edited = now_rfc3339();
        write_json(&file, &recipe)?;
        version = load_versioned(&file)?.1;
    }
    let read = RecipeRead {
        motion_version: motion_version(&recipe),
        recipe,
        warnings: Vec::new(),
        gone: Vec::new(),
        version,
    };
    Ok(summary(id, &read, clip_mixamo))
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

/// Puts back the stored prop of each prop blank sent open whose prop the
/// project no longer has: it read as open, so sending it open again is not
/// a choice to empty it, and restoring the asset fills it again. A blank
/// named in `clear` is left empty.
fn keep_stale_props(
    next: &mut RecipeFile,
    current: &RecipeFile,
    manifest: &ProjectManifest,
    clear: &[String],
) {
    for blank in &mut next.blanks {
        if blank.kind != BlankKind::Prop || blank.prop.is_some() || clear.contains(&blank.id) {
            continue;
        }
        let stale = current
            .blanks
            .iter()
            .find(|b| b.id == blank.id && b.kind == BlankKind::Prop)
            .and_then(|b| b.prop.as_ref())
            .filter(|p| !manifest.assets.iter().any(|a| &a.id == *p));
        if let Some(prop) = stale {
            blank.prop = Some(prop.clone());
        }
    }
}

/// Refuses a `clear` naming something that is not a prop blank of `next`.
fn check_clear(next: &RecipeFile, clear: &[String]) -> CoreResult<()> {
    match clear.iter().find(|id| {
        !next
            .blanks
            .iter()
            .any(|b| &&b.id == id && b.kind == BlankKind::Prop)
    }) {
        Some(id) => Err(CoreError::Invalid(format!(
            "The recipe has no prop blank {id} to clear."
        ))),
        None => Ok(()),
    }
}

/// Refuses a write that read the recipe at another version.
fn check_expected(id: &str, current: &str, expected: Option<&str>) -> CoreResult<()> {
    match expected {
        Some(read) if read != current => Err(CoreError::Moved(format!(
            "The recipe {id} changed since it was read (its version is now {current}). Read it again and send the change once more."
        ))),
        _ => Ok(()),
    }
}

/// Replaces the name, the changes, the blanks or any of them of the recipe
/// `id`, as the file holds it now, in one write after checking the whole
/// result. Blanks sent must name assets of `manifest`'s project. When the
/// blanks are kept, they stay in the file as they are: a prop blank naming
/// an asset the project no longer has keeps its prop, so restoring the
/// asset fills it again, and only the answer reads it as open, with a
/// warning, as [`get`] does. A prop blank sent open whose stored prop is
/// such a gone asset keeps it too, since it only read as open, unless
/// `clear` names it. With `expected`, a file whose version (a hash of its
/// bytes) moved since it was read is refused under the write lock, so no
/// write landing in between, nor a hand edit or a sync, is overwritten.
/// `edited` moves only when something changed; the revision never does.
///
/// # Errors
///
/// Fails on a name [`check_name`] refuses, when the recipe is gone or
/// changed since `expected`, `clear` names no prop blank, the result does
/// not validate or the file cannot be written.
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
    let (current, current_version) = load_versioned(&file)?;
    check_expected(id, &current_version, change.expected.as_deref())?;
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
    check_clear(&next, &change.clear)?;
    if sent_blanks {
        check_props(&next, manifest)?;
        keep_stale_props(&mut next, &current, manifest, &change.clear);
    } else {
        for blank in &mut next.blanks {
            if change.clear.contains(&blank.id) {
                blank.prop = None;
            }
        }
    }
    let version = if next == current {
        current_version
    } else {
        next.edited = now_rfc3339();
        write_json(&file, &next)?;
        version(project_dir, id)?
    };
    Ok(settled(next, version, manifest))
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
