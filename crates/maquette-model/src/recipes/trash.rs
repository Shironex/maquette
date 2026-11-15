//! Removing a project's recipe to the workspace trash and putting it back.
//! A removed recipe waits in the project's own trash folder as
//! `recipes/<id>-<time>.recipe.json`, its draft marks beside it as
//! `recipes/<id>-<time>.drafts.json`. Nothing is deleted. `<id>-<time>` is
//! the trash name [`restore`] takes; the time is digits only, so the id is
//! everything before the last `-`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::plans::move_file;
use crate::project::now_rfc3339;
use crate::recipes::drafts::DRAFTS_SUFFIX;
use crate::recipes::{RECIPE_SUFFIX, RECIPES_DIR, dir, is_recipe_id, load_versioned, path};

/// A recipe [`remove`] moved to the trash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RemovedRecipe {
    /// The trash name [`restore`] takes.
    pub trash: String,
    pub id: String,
    /// The recipe's name; its id when the file could not be read.
    pub name: String,
    /// Where the file went, relative to the workspace trash folder of the
    /// project, with `/` separators.
    pub path: String,
}

/// A free trash name for `id` in `to_dir`: `<id>-<digits of now>`, with
/// more digits when that one is taken.
fn free_name(to_dir: &Path, id: &str) -> String {
    let stamp: String = now_rfc3339().chars().filter(char::is_ascii_digit).collect();
    let base = format!("{id}-{stamp}");
    let taken = |name: &str| {
        to_dir.join(format!("{name}{RECIPE_SUFFIX}")).exists()
            || to_dir.join(format!("{name}{DRAFTS_SUFFIX}")).exists()
    };
    if !taken(&base) {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}{n}"))
        .find(|name| !taken(name))
        .unwrap_or(base)
}

/// The id in a trash name, or none for a name [`remove`] never makes.
fn id_of(trash: &str) -> Option<&str> {
    let (id, stamp) = trash.rsplit_once('-')?;
    let digits = !stamp.is_empty() && stamp.bytes().all(|b| b.is_ascii_digit());
    (digits && is_recipe_id(id)).then_some(id)
}

/// Moves the recipe `id` and its draft marks into `trash_dir` (the
/// workspace's trash folder for this project). When the marks cannot move,
/// the recipe comes back and the error says why.
///
/// # Errors
///
/// Fails when the recipe is missing or a file cannot be moved.
pub fn remove(project_dir: &Path, trash_dir: &Path, id: &str) -> CoreResult<RemovedRecipe> {
    let _lock = super::writing();
    let file = path(project_dir, id)?;
    if !file.is_file() {
        return Err(CoreError::NotFound(format!("the recipe {id}")));
    }
    let name = load_versioned(&file).map_or_else(|_| id.to_string(), |(r, _)| r.name);
    let to_dir = trash_dir.join(RECIPES_DIR);
    std::fs::create_dir_all(&to_dir).map_err(|e| CoreError::io(&to_dir, e))?;
    let trash = free_name(&to_dir, id);
    let to = to_dir.join(format!("{trash}{RECIPE_SUFFIX}"));
    move_file(&file, &to)?;
    let marks = marks_file(project_dir, id);
    if marks.is_file()
        && let Err(err) = move_file(&marks, &to_dir.join(format!("{trash}{DRAFTS_SUFFIX}")))
    {
        if let Err(back) = move_file(&to, &file) {
            tracing::warn!(error = %back, "a removed recipe could not come back");
        }
        return Err(err);
    }
    Ok(RemovedRecipe {
        path: format!("{RECIPES_DIR}/{trash}{RECIPE_SUFFIX}"),
        trash,
        id: id.to_string(),
        name,
    })
}

/// Puts the removed recipe `trash` back in the project with its draft
/// marks; answers its id. Refused when a recipe with that id is there
/// again: nothing is written over.
///
/// # Errors
///
/// Fails when there is no such removed recipe, its id is taken, or a file
/// cannot move back.
pub fn restore(project_dir: &Path, trash_dir: &Path, trash: &str) -> CoreResult<String> {
    let missing = || CoreError::NotFound(format!("the removed recipe {trash}"));
    let id = id_of(trash).ok_or_else(missing)?;
    let _lock = super::writing();
    let from_dir = trash_dir.join(RECIPES_DIR);
    let from = from_dir.join(format!("{trash}{RECIPE_SUFFIX}"));
    if !from.is_file() {
        return Err(missing());
    }
    let to = path(project_dir, id)?;
    if to.exists() {
        return Err(CoreError::Invalid(format!(
            "A recipe with the id {id} is in the project again, so the removed one stays in the trash. Remove or rename that recipe first."
        )));
    }
    let recipes = dir(project_dir);
    std::fs::create_dir_all(&recipes).map_err(|e| CoreError::io(&recipes, e))?;
    move_file(&from, &to)?;
    let marks = from_dir.join(format!("{trash}{DRAFTS_SUFFIX}"));
    if marks.is_file()
        && let Err(err) = move_file(&marks, &marks_file(project_dir, id))
    {
        // The recipe is back; only the editor's hint of what agents drafted stays behind.
        tracing::warn!(error = %err, "a restored recipe's draft marks stayed in the trash");
    }
    Ok(id.to_string())
}

fn marks_file(project_dir: &Path, id: &str) -> PathBuf {
    dir(project_dir).join(format!("{id}{DRAFTS_SUFFIX}"))
}

#[cfg(test)]
#[path = "tests_trash.rs"]
mod tests;
