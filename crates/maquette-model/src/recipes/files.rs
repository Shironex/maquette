//! A project's recipe files: listing, making, reading, renaming, changing
//! and removing them. Reading checks prop blanks against the project, so a
//! prop that is no longer an asset reads as an open blank with a warning. Writes in this process go one at a time, and each
//! reads the file on disk under the same lock first, so a recipe removed
//! meanwhile (here or by a sync from another computer) is not written back.

use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;

use super::{
    GoneProp, ID_MAX_CHARS, MAX_FILE_BYTES, RECIPE_SUFFIX, RECIPES_DIR, RecipeBase, RecipeFile,
    RecipeFrom, check_name, dir, gone_props, is_recipe_id, load_versioned, motion_version,
    normalized, path, settle_props, validate,
};
use crate::error::{CoreError, CoreResult};
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
    /// The file's version: 1, or 2 once it uses something of version 2.
    /// 0 when it could not be read.
    #[specta(optional)]
    pub format: u32,
    /// Each grip change that came from a library grip, so a list can say
    /// when the library has a newer revision without reading every recipe.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[specta(optional)]
    pub grips: Vec<RecipeGripUse>,
}

/// A grip change of a recipe and the library grip it was copied from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeGripUse {
    /// The change's id.
    pub layer: String,
    pub from: super::GripFrom,
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
pub(super) fn settled(
    mut recipe: RecipeFile,
    version: String,
    manifest: &ProjectManifest,
) -> RecipeRead {
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

/// How a refusal of a recipe too big to write starts, so an answer can
/// carry its own code.
pub const TOO_BIG: &str = "The recipe would be over";

/// Writes a recipe file as [`write_json`] would, refused before anything is
/// written when it would be over [`MAX_FILE_BYTES`], since such a file would
/// not open again.
pub(super) fn write_recipe(file: &Path, recipe: &RecipeFile) -> CoreResult<()> {
    let mut text = serde_json::to_string_pretty(recipe).map_err(|e| CoreError::json(file, e))?;
    text.push('\n');
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        return Err(CoreError::Invalid(format!(
            "{TOO_BIG} {} KB, too big to save. Take out some pose keys, grips or changes.",
            MAX_FILE_BYTES / 1024
        )));
    }
    maquette_base::fs::write_atomic(file, text.as_bytes()).map_err(|e| CoreError::io(file, e))
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn file_of(id: &str) -> String {
    format!("{RECIPES_DIR}/{id}{RECIPE_SUFFIX}")
}

pub(super) fn summary(
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
        mixamo: recipe.clips().any(|c| c.mixamo || clip_mixamo(c)),
        layers: count(recipe.layers.len()),
        blanks: count(recipe.blanks.len()),
        open: count(recipe.blanks.iter().filter(|b| !b.filled()).count()),
        edited: recipe.edited.clone(),
        version: read.version.clone(),
        motion_version: read.motion_version.clone(),
        warnings: read.warnings.clone(),
        problem: None,
        from: recipe.from.clone(),
        format: recipe.version,
        grips: recipe
            .layers
            .iter()
            .filter_map(|l| {
                l.from.as_ref().map(|from| RecipeGripUse {
                    layer: l.id.clone(),
                    from: from.clone(),
                })
            })
            .collect(),
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
        format: 0,
        grips: Vec::new(),
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
    create_joined(project_dir, name, vec![base], false)
}

/// As [`create`], on `clips` joined in order (the first the base), mirrored
/// or not. One clip that is not mirrored and has no trim writes version 1.
///
/// # Errors
///
/// As [`create`], and for no clips at all.
pub fn create_joined(
    project_dir: &Path,
    name: &str,
    clips: Vec<RecipeBase>,
    mirror: bool,
) -> CoreResult<(String, RecipeFile)> {
    check_name(name)?;
    let mut clips = clips.into_iter();
    let Some(base) = clips.next() else {
        return Err(CoreError::Invalid(
            "A recipe needs a clip to start from, or the rest pose.".into(),
        ));
    };
    let mut recipe = RecipeFile::new(name, base);
    recipe.joined = clips.collect();
    recipe.mirror = mirror;
    let recipe = normalized(&recipe);
    validate(&recipe)?;
    let _lock = writing();
    let id = free_id(project_dir, &recipe.name)?;
    let folder = dir(project_dir);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    write_recipe(&path(project_dir, &id)?, &recipe)?;
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
        write_recipe(&file, &recipe)?;
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
