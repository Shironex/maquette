//! Saving a project's recipe back to the recipe library, as a new entry or
//! as the next revision of an entry. It never overwrites: each save adds a
//! revision. The person asks for the plan first (what changes against the
//! newest revision, in plain words) and confirms it; the save sends the
//! entry version the plan read, so an entry that moved since is refused
//! (code `moved`). Agents only propose a save; the app's confirm writes.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    LibraryRecipeEntry, LibraryRecipeRevision, changes, check_expected, free_entry_id, load_entry,
    read_revision, revision_file, turn, write_revision,
};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, now_rfc3339};
use crate::recipes::{self, RecipeFile, RecipeFrom};
use crate::workspace::FolderLayout;

/// What a save back would do, for the confirm sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSavePlan {
    pub project: String,
    pub recipe: String,
    pub recipe_name: String,
    /// The project recipe's version as read; the save sends it back.
    pub recipe_version: String,
    /// The library recipe it goes to; none for a new one.
    pub entry: Option<String>,
    /// The library recipe's name, or the name a new one gets.
    pub entry_name: String,
    /// The library recipe's version as read; the save sends it back.
    pub entry_version: Option<String>,
    /// Its newest revision now; none for a new one.
    pub latest: Option<u32>,
    /// The revision the save writes.
    pub revision: u32,
    /// What changes against the newest revision, a plain line each.
    pub changes: Vec<String>,
    /// The recipe holds what the newest revision holds: nothing to save.
    pub same: bool,
}

/// What a library recipe's newest revision changes against a project's
/// recipe that came from it, for the "library has a newer revision" note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeCompare {
    pub entry: String,
    pub entry_name: String,
    /// The revision the project's recipe came from or was saved as.
    pub revision: u32,
    /// The library's newest revision.
    pub latest: u32,
    /// What the newest revision changes against the project's recipe.
    pub changes: Vec<String>,
}

/// A confirmed save back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSave {
    pub project: String,
    pub recipe: String,
    /// The library recipe to add a revision to; none makes a new one.
    pub entry: Option<String>,
    /// The entry version the plan read. Needed with `entry`.
    pub expected: Option<String>,
    /// The recipe version the plan read.
    pub recipe_expected: Option<String>,
}

/// What a save back wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSaved {
    pub entry: String,
    pub revision: u32,
    /// The library recipe's version after the save.
    pub entry_version: String,
    /// The project recipe now names the revision it was saved as. False when
    /// it changed between the save and that note (the revision is saved).
    pub linked: bool,
}

/// The same recipe for the library: nothing [`changes`] would list (the
/// prop that fills a blank belongs to each project, so it does not count).
fn same_content(a: &RecipeFile, b: &RecipeFile) -> bool {
    changes(Some(a), b).is_empty()
}

fn on_disk(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    recipe: &str,
) -> CoreResult<(std::path::PathBuf, RecipeFile, String)> {
    let dir = project::dir(root, layout, project_id)?;
    project::load(root, layout, project_id)?;
    let (file, version) = recipes::load_versioned(&recipes::path(&dir, recipe)?)?;
    Ok((dir, file, version))
}

/// What saving the recipe `recipe` of `project_id` to `entry` (a new
/// library recipe when none) would do.
///
/// # Errors
///
/// Fails for an unknown project, recipe or entry, or unreadable files.
pub fn plan_save(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    recipe: &str,
    entry: Option<&str>,
) -> CoreResult<RecipeSavePlan> {
    let (_, file, recipe_version) = on_disk(root, layout, project_id, recipe)?;
    let mut plan = RecipeSavePlan {
        project: project_id.to_string(),
        recipe: recipe.to_string(),
        recipe_name: file.name.clone(),
        recipe_version,
        entry: None,
        entry_name: file.name.clone(),
        entry_version: None,
        latest: None,
        revision: 1,
        changes: changes(None, &file),
        same: false,
    };
    let Some(id) = entry else {
        return Ok(plan);
    };
    let (found, version) = load_entry(root, id)?;
    let newest = read_revision(root, &found, None)?;
    plan.entry = Some(found.id.clone());
    plan.entry_name.clone_from(&found.name);
    plan.entry_version = Some(version);
    plan.latest = Some(newest.revision);
    plan.revision = found.last_revision.max(newest.revision) + 1;
    plan.changes = changes(Some(&newest), &file);
    plan.same = same_content(&newest, &file);
    Ok(plan)
}

/// What the newest revision of the library recipe that the recipe `recipe`
/// of `project_id` came from changes against it.
///
/// # Errors
///
/// Fails for an unknown project or recipe, a recipe that came from no
/// library recipe, or an entry that is gone or unreadable.
pub fn compare(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    recipe: &str,
) -> CoreResult<RecipeCompare> {
    let (_, file, _) = on_disk(root, layout, project_id, recipe)?;
    let Some(from) = &file.from else {
        return Err(CoreError::Invalid(format!(
            "{} came from no library recipe.",
            file.name
        )));
    };
    let (found, _) = load_entry(root, &from.entry)?;
    let newest = read_revision(root, &found, None)?;
    Ok(RecipeCompare {
        entry: found.id.clone(),
        entry_name: found.name.clone(),
        revision: from.revision,
        latest: newest.revision,
        changes: changes(Some(&file), &newest),
    })
}

/// Saves the recipe as the person confirmed: a new library recipe, or the
/// next revision of `save.entry`. Then the project recipe is marked as
/// saved as that revision.
///
/// # Errors
///
/// Fails for an unknown project, recipe or entry, a recipe or entry that
/// moved since the plan (code `moved`), an entry save without `expected`,
/// a recipe that holds what the newest revision holds, or files that
/// cannot be written.
pub fn save_back(root: &Path, layout: &FolderLayout, save: &RecipeSave) -> CoreResult<RecipeSaved> {
    let (dir, file, recipe_version) = on_disk(root, layout, &save.project, &save.recipe)?;
    if let Some(read) = &save.recipe_expected
        && *read != recipe_version
    {
        return Err(CoreError::Moved(format!(
            "The recipe {} changed since the save was planned. Look at it again.",
            save.recipe
        )));
    }
    let now = now_rfc3339();
    let source = |revision: u32| LibraryRecipeRevision {
        revision,
        file: revision_file(revision),
        saved_at: now.clone(),
        project: Some(save.project.clone()),
        recipe: Some(save.recipe.clone()),
        imported: None,
    };
    let (entry, revision) = {
        let _turn = turn();
        let entry = match &save.entry {
            Some(id) => {
                let (mut found, version) = load_entry(root, id)?;
                let Some(expected) = save.expected.as_deref() else {
                    return Err(CoreError::Invalid(format!(
                        "Saving a revision of {} needs the version its confirm read.",
                        found.name
                    )));
                };
                check_expected(id, &version, Some(expected))?;
                let newest = read_revision(root, &found, None)?;
                if same_content(&newest, &file) {
                    return Err(CoreError::Invalid(format!(
                        "{} holds what r{} of {} holds already, so there is nothing to save.",
                        file.name, newest.revision, found.name
                    )));
                }
                let next = found.last_revision.max(newest.revision) + 1;
                found.revisions.push(source(next));
                found.last_revision = next;
                found.updated_at.clone_from(&now);
                found
            }
            None => LibraryRecipeEntry {
                version: super::ENTRY_VERSION,
                id: free_entry_id(root, &file.name)?,
                name: file.name.clone(),
                created_at: now.clone(),
                updated_at: now.clone(),
                last_revision: 1,
                revisions: vec![source(1)],
            },
        };
        let mut stored = file.clone();
        stored.revision = entry.last_revision;
        write_revision(root, &entry, &stored)?;
        (entry.id.clone(), entry.last_revision)
    };
    let from = RecipeFrom {
        entry: entry.clone(),
        revision,
    };
    let linked = recipes::link(&dir, &save.recipe, from, Some(&recipe_version)).is_ok();
    Ok(RecipeSaved {
        entry_version: super::entry_version(root, &entry)?,
        entry,
        revision,
        linked,
    })
}
