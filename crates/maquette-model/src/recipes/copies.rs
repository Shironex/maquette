//! Whole recipes written into a project: one copied from the recipe library
//! or imported from a file, under a free id, and the link a project's recipe
//! keeps to the library revision it was saved as.

use std::fs;
use std::path::Path;

use super::files::{free_id, writing};
use super::{RecipeFile, RecipeFrom, check_name, dir, load_versioned, normalized, path, validate};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::latin_slug;

/// Writes `recipe` whole into the project at `project_dir` as a new recipe,
/// under a free id made from `wanted` (a file name's stem, say) or else its
/// name, normalised and checked first. Prop blanks are not checked against
/// the project: the caller opens the ones it does not have. Answers with
/// the id, the recipe as written and its version.
///
/// # Errors
///
/// Fails on a name [`check_name`] refuses, a recipe [`validate`] refuses or
/// when the file cannot be written.
pub fn create_whole(
    project_dir: &Path,
    recipe: &RecipeFile,
    wanted: Option<&str>,
) -> CoreResult<(String, RecipeFile, String)> {
    check_name(&recipe.name)?;
    let recipe = normalized(recipe);
    validate(&recipe)?;
    let _lock = writing();
    let seed = wanted
        .and_then(latin_slug)
        .unwrap_or_else(|| recipe.name.clone());
    let id = free_id(project_dir, &seed)?;
    let folder = dir(project_dir);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    let file = path(project_dir, &id)?;
    write_json(&file, &recipe)?;
    let version = load_versioned(&file)?.1;
    Ok((id, recipe, version))
}

/// Marks the recipe `id` as saved to the library as `from`: its `from` and
/// its revision follow, nothing else changes. With `expected`, a file whose
/// version moved since it was read is refused (code `moved`). Answers with
/// the new version.
///
/// # Errors
///
/// Fails when the recipe is gone, moved since `expected` or cannot be
/// written.
pub fn link(
    project_dir: &Path,
    id: &str,
    from: RecipeFrom,
    expected: Option<&str>,
) -> CoreResult<String> {
    let _lock = writing();
    let file = path(project_dir, id)?;
    let (mut recipe, version) = load_versioned(&file)?;
    if let Some(read) = expected
        && read != version
    {
        return Err(CoreError::Moved(format!(
            "The recipe {id} changed since it was read (its version is now {version}). Read it again."
        )));
    }
    recipe.revision = from.revision;
    recipe.from = Some(from);
    validate(&recipe)?;
    write_json(&file, &recipe)?;
    Ok(load_versioned(&file)?.1)
}
