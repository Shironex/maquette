//! Recipe files in and out: exporting a project's recipe or a library
//! revision to one `.recipe.json`, and importing one into a project or the
//! library. An exported file is a version 1 recipe with no `from`, so it
//! names no library entry and no path. An imported file is untrusted: its
//! size is checked before it is parsed, it passes every v1 check and cap
//! (32 changes, 16 blanks, 256 KB), and nothing in it becomes a path: the
//! new id is made from a slug of the file's name or the recipe's, fresh
//! when it is taken. Its base clip is matched by name and frame count; a
//! hash that differs warns.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::use_in::landed;
use super::{
    ENTRY_VERSION, LibraryRecipeEntry, LibraryRecipeRevision, RecipeUsed, free_entry_id,
    load_entry, open_missing_props, read_revision, revision_file, turn, write_revision,
};
use crate::animations::recipes::match_base;
use crate::error::{CoreError, CoreResult};
use crate::project::{self, latin_slug, now_rfc3339, windows_safe};
use crate::recipes::{self, RECIPE_SUFFIX, RecipeFile};
use crate::workspace::FolderLayout;

/// The folder under the workspace root agents export recipes to.
pub const AGENT_EXPORTS: [&str; 2] = ["exports", "recipes"];

/// What to export: a project's recipe (`project` and `recipe`) or a
/// library recipe's revision (`entry`, and `revision`, the newest when
/// left out).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportSource {
    pub project: Option<String>,
    pub recipe: Option<String>,
    pub entry: Option<String>,
    pub revision: Option<u32>,
}

/// Where an export went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeExported {
    /// The file written, absolute.
    pub path: String,
    /// Its name.
    pub file: String,
}

/// A recipe file imported into the library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeImported {
    pub entry: String,
    pub name: String,
    /// The base clip missing or with other keys, an id that was taken.
    pub notes: Vec<String>,
}

/// The recipe `source` names, with no `from`.
///
/// # Errors
///
/// Fails when the source names neither a recipe nor an entry, or what it
/// names cannot be read.
pub fn source_recipe(
    root: &Path,
    layout: &FolderLayout,
    source: &ExportSource,
) -> CoreResult<RecipeFile> {
    let mut recipe = match (&source.project, &source.recipe, &source.entry) {
        (Some(project), Some(recipe), None) => {
            let dir = project::dir(root, layout, project)?;
            recipes::load_file(&recipes::path(&dir, recipe)?)?
        }
        (None, None, Some(entry)) => {
            let (found, _) = load_entry(root, entry)?;
            read_revision(root, &found, source.revision)?
        }
        _ => {
            return Err(CoreError::Invalid(
                "Export a project's recipe (project and recipe) or a library recipe (entry)."
                    .into(),
            ));
        }
    };
    recipe.from = None;
    Ok(recipe)
}

/// The text of `recipe` as it is exported: normalised, version 1, no `from`.
///
/// # Errors
///
/// Fails when the recipe does not validate.
pub fn export_text(recipe: &RecipeFile) -> CoreResult<String> {
    let mut out = recipes::normalized(recipe);
    out.from = None;
    recipes::validate(&out)?;
    let mut value = serde_json::to_value(&out)
        .map_err(|e| CoreError::Invalid(format!("The recipe could not be written: {e}")))?;
    if let Some(fields) = value.as_object_mut() {
        fields.remove("from");
    }
    let mut text = serde_json::to_string_pretty(&value)
        .map_err(|e| CoreError::Invalid(format!("The recipe could not be written: {e}")))?;
    text.push('\n');
    Ok(text)
}

/// Writes the recipe `source` names to `to`, replacing a file there (the
/// person picked it in a save dialog, which asks first).
///
/// # Errors
///
/// Fails when the source cannot be read or the file cannot be written.
pub fn export(
    root: &Path,
    layout: &FolderLayout,
    source: &ExportSource,
    to: &Path,
) -> CoreResult<RecipeExported> {
    let text = export_text(&source_recipe(root, layout, source)?)?;
    crate::fs::write_atomic(to, text.as_bytes()).map_err(|e| CoreError::io(to, e))?;
    Ok(RecipeExported {
        path: to.display().to_string(),
        file: to
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    })
}

/// Writes an agent's export of the recipe `source` names as a new file
/// called `name` in the workspace's `exports/recipes` folder: its slug with
/// `.recipe.json`, then `-2`, `-3`. Each name is claimed by creating the
/// file only when it is not there (`create_new`), so two exports at once
/// never replace each other or a file already there.
///
/// # Errors
///
/// Fails when the source cannot be read or the file cannot be written.
pub fn agent_export(
    root: &Path,
    layout: &FolderLayout,
    source: &ExportSource,
    name: &str,
) -> CoreResult<RecipeExported> {
    use std::io::Write as _;
    let text = export_text(&source_recipe(root, layout, source)?)?;
    for path in agent_export_names(root, name)? {
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(CoreError::io(&path, e)),
        };
        if let Err(e) = file
            .write_all(text.as_bytes())
            .and_then(|()| file.sync_all())
        {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err(CoreError::io(&path, e));
        }
        return Ok(RecipeExported {
            file: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.display().to_string(),
        });
    }
    Err(CoreError::Invalid(format!(
        "There are too many exports called {name}."
    )))
}

/// The names an agent's export called `name` may take, in order, in the
/// workspace's `exports/recipes` folder (made when missing).
fn agent_export_names(root: &Path, name: &str) -> CoreResult<impl Iterator<Item = PathBuf>> {
    let folder = AGENT_EXPORTS
        .iter()
        .fold(root.to_path_buf(), |p, s| p.join(s));
    std::fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    let stem = name
        .trim()
        .trim_end_matches(RECIPE_SUFFIX)
        .trim_end_matches(".json");
    let base = latin_slug(stem).map_or_else(
        || "recipe".to_string(),
        |slug| {
            let short: String = slug.chars().take(recipes::ID_MAX_CHARS).collect();
            windows_safe(short.trim_end_matches('-').to_string())
        },
    );
    Ok((1..10_000).map(move |n| {
        let stem = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        folder.join(format!("{stem}{RECIPE_SUFFIX}"))
    }))
}

/// The file an agent names for an import, which has to stay inside the
/// workspace at `root` (relative to it, or absolute under it, links
/// followed) and end in `.recipe.json`.
///
/// # Errors
///
/// Fails for a path outside the workspace, a missing file or another kind.
pub fn agent_import_path(root: &Path, given: &str) -> CoreResult<PathBuf> {
    let given = Path::new(given.trim());
    let joined = if given.is_absolute() {
        given.to_path_buf()
    } else {
        root.join(given)
    };
    let outside = || {
        CoreError::Invalid(
            "Import a recipe file inside the workspace: a path relative to it, or absolute under it."
                .into(),
        )
    };
    let real = joined.canonicalize().map_err(|_| outside())?;
    let real_root = root.canonicalize().map_err(|e| CoreError::io(root, e))?;
    if !real.starts_with(&real_root) {
        return Err(outside());
    }
    let recipe_file = real
        .file_name()
        .is_some_and(|n| n.to_string_lossy().ends_with(RECIPE_SUFFIX));
    if !real.is_file() || !recipe_file {
        return Err(CoreError::Invalid("Import a .recipe.json file.".into()));
    }
    Ok(real)
}

/// Reads a recipe file to import: at most 256 KB, checked before parsing,
/// then every v1 check. Its `from` is dropped.
///
/// # Errors
///
/// Fails for a missing, too big, broken, newer or invalid file.
pub fn read_import(file: &Path) -> CoreResult<RecipeFile> {
    if file.extension().is_none_or(|e| e != "json") {
        return Err(CoreError::Invalid("Import a .recipe.json file.".into()));
    }
    let mut recipe = recipes::load_file(file)?;
    recipe.from = None;
    Ok(recipe)
}

/// The file's name without `.recipe.json` (or `.json`), as a seed for an id.
fn stem_of(file: &Path) -> Option<String> {
    let name = file.file_name()?.to_string_lossy().into_owned();
    let stem = name
        .strip_suffix(RECIPE_SUFFIX)
        .or_else(|| name.strip_suffix(".json"))
        .unwrap_or(&name);
    latin_slug(stem)
}

/// Imports the recipe file `file` into a project as a new recipe. Prop
/// blanks whose asset the project does not have open, with a note.
///
/// # Errors
///
/// Fails for an unknown project or a file [`read_import`] refuses.
pub fn import_to_project(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    file: &Path,
) -> CoreResult<RecipeUsed> {
    let manifest = project::load(root, layout, project_id)?;
    let project_dir = project::dir(root, layout, project_id)?;
    let mut recipe = read_import(file)?;
    let now = now_rfc3339();
    recipe.created.clone_from(&now);
    recipe.edited = now;
    let notes = open_missing_props(&mut recipe, &manifest);
    landed(
        root,
        &project_dir,
        &manifest,
        &recipe,
        stem_of(file).as_deref(),
        notes,
    )
}

/// Imports the recipe file `file` into the library as a new entry with
/// revision 1, which records the file's name.
///
/// # Errors
///
/// Fails for a file [`read_import`] refuses or when it cannot be written.
pub fn import_to_library(root: &Path, file: &Path) -> CoreResult<RecipeImported> {
    let recipe = read_import(file)?;
    let seed = stem_of(file).unwrap_or_else(|| recipe.name.clone());
    let imported = file.file_name().map(|n| n.to_string_lossy().into_owned());
    add_entry(root, &seed, recipe, imported)
}

/// Writes `recipe` as revision 1 of a new library recipe whose id is made
/// from `seed`, noting `imported` as where it came from.
pub(super) fn add_entry(
    root: &Path,
    seed: &str,
    mut recipe: RecipeFile,
    imported: Option<String>,
) -> CoreResult<RecipeImported> {
    recipe.revision = 1;
    recipe.from = None;
    let now = now_rfc3339();
    let mut notes = Vec::new();
    let entry = {
        let _turn = turn();
        let named = recipes::unique_name(&recipe.name, &super::names(root));
        if named != recipe.name {
            notes.push(format!(
                "The library has a recipe called {} already, so this one is {named}.",
                recipe.name
            ));
            recipe.name = named;
        }
        let id = free_entry_id(root, seed)?;
        if latin_slug(seed).is_some_and(|s| s != id && id.starts_with(&s)) {
            notes.push(format!(
                "The library has a recipe {seed} already, so this one is {id}."
            ));
        }
        let entry = LibraryRecipeEntry {
            version: ENTRY_VERSION,
            uid: super::link::new_uid(&id),
            id,
            name: recipe.name.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            last_revision: 1,
            revisions: vec![LibraryRecipeRevision {
                revision: 1,
                file: revision_file(1),
                saved_at: now,
                project: None,
                recipe: None,
                imported,
            }],
        };
        write_revision(root, &entry, &recipe)?;
        entry
    };
    let clips = crate::animations::list(root)?;
    if let Some(warning) = match_base(&clips, &recipe.base).0.warning {
        notes.push(warning);
    }
    Ok(RecipeImported {
        entry: entry.id,
        name: entry.name,
        notes,
    })
}
