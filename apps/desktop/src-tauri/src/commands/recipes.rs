//! Animation recipes: a project's recipes, one file each in its `recipes/`
//! folder, each a base clip from the animation library with changes laid
//! over it. Listing, making one from a library clip, reading, renaming,
//! changing and removing them. Nothing here bakes or runs Blender. The
//! helpers are shared with the MCP tools, so both make the same checks.

use maquette_core::CoreError;
use maquette_core::animations::recipes::{self as library_recipes, ClipPick, RecipeDoc};
use maquette_core::assets::remove::project_trash;
use maquette_core::error::ErrorPayload;
use maquette_core::project::now_rfc3339;
use maquette_core::project::{self, ProjectManifest};
use maquette_core::providers::ledger::Decision;
use maquette_core::recipes::drafts;
use maquette_core::recipes::{
    self, RecipeBase, RecipeChange, RecipeFile, RecipeSummary, RemovedRecipe,
};
use tauri::State;

use super::CommandResult;
use super::plans::project_dir;
use super::recipe_bakes::RecipeBakeJobs;
use crate::session::Session;

/// The code of a recipe refused because its file would be too big to open
/// again.
pub(crate) const TOO_BIG: &str = "recipe-too-big";
/// The code of clips refused because they take away a clip keys or time
/// edits still sit on.
pub(crate) const CLIP_IN_USE: &str = "clip-in-use";

/// `err` as the view and agents get it, a recipe too big to write or clips
/// that strand keys or time edits under their own codes.
pub(crate) fn coded(err: CoreError) -> ErrorPayload {
    let mut payload = err.payload();
    if let CoreError::Invalid(message) = &err {
        if message.starts_with(recipes::TOO_BIG) {
            payload.code = TOO_BIG.to_string();
        } else if message.starts_with(recipes::CLIP_IN_USE) {
            payload.code = CLIP_IN_USE.to_string();
        }
    }
    payload
}

/// The workspace root, the project's folder and its manifest, which prop
/// blanks are read against.
fn opened(
    session: &Session,
    project: &str,
) -> CommandResult<(std::path::PathBuf, std::path::PathBuf, ProjectManifest)> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project)?;
    let dir = project::dir(&root, &config.layout, project)?;
    Ok((root, dir, manifest))
}

pub(crate) fn list(session: &Session, project: &str) -> CommandResult<Vec<RecipeSummary>> {
    let (root, dir, manifest) = opened(session, project)?;
    Ok(library_recipes::summaries(&root, &dir, &manifest)?)
}

pub(crate) fn get(session: &Session, project: &str, id: &str) -> CommandResult<RecipeDoc> {
    let (root, dir, manifest) = opened(session, project)?;
    Ok(library_recipes::get(&root, &dir, &manifest, id)?)
}

pub(crate) fn create(
    session: &Session,
    project: &str,
    clip: Option<&str>,
    name: Option<&str>,
) -> CommandResult<RecipeDoc> {
    let (root, _) = session.workspace()?;
    let dir = project_dir(session, project)?;
    Ok(library_recipes::create(&root, &dir, clip, name)?)
}

/// Makes a recipe on the library clips `picks` names, joined in order.
pub(crate) fn create_joined(
    session: &Session,
    project: &str,
    picks: &[ClipPick],
    mirror: bool,
    name: Option<&str>,
) -> CommandResult<RecipeDoc> {
    let (root, _) = session.workspace()?;
    let dir = project_dir(session, project)?;
    library_recipes::create_joined(&root, &dir, picks, mirror, name).map_err(coded)
}

/// The clips `picks` names for a change to the recipe `id`: library clips
/// read from the library, kept ones from the recipe as its file holds it.
pub(crate) fn picked_clips(
    session: &Session,
    project: &str,
    id: &str,
    picks: &[ClipPick],
) -> CommandResult<Vec<RecipeBase>> {
    let (root, dir, manifest) = opened(session, project)?;
    let current = library_recipes::current(&dir, &manifest, id)?;
    Ok(library_recipes::clips_of(&root, Some(&current), picks)?)
}

pub(crate) fn update(
    session: &Session,
    project: &str,
    id: &str,
    change: RecipeChange,
) -> CommandResult<RecipeDoc> {
    let (root, dir, manifest) = opened(session, project)?;
    library_recipes::update(&root, &dir, &manifest, id, change).map_err(coded)
}

/// Marks what the agent `by` changed in `doc` (from `before`, none when it
/// made it) as drafts, and puts the marks on `doc`. The write itself has
/// landed; a mark that cannot be written only loses the editor's hint. The
/// mark checks, under the recipes' write lock, that the file is still at
/// `doc.version`, so a save that landed after the write is never drafted.
pub(crate) fn drafted(
    session: &Session,
    project: &str,
    before: Option<&RecipeFile>,
    doc: &mut RecipeDoc,
    by: &str,
) {
    let Ok(dir) = project_dir(session, project) else {
        return;
    };
    let at = now_rfc3339();
    match drafts::mark_written(&dir, &doc.id, &doc.version, before, &doc.recipe, by, &at) {
        Ok(marks) => doc.drafts = marks,
        Err(err) => tracing::warn!("recipe draft marks for {}: {err}", doc.id),
    }
}

/// Moves the recipe and its draft marks into the workspace trash, refused
/// while it bakes or its bake is being added. A staged bake stays in the
/// cache and reads as out of date until the recipe is back; an agent's
/// approval for it ends as cancelled.
pub(crate) fn remove(
    session: &Session,
    jobs: &RecipeBakeJobs,
    project: &str,
    id: &str,
) -> CommandResult<RemovedRecipe> {
    let (root, config) = session.workspace()?;
    let dir = project_dir(session, project)?;
    let trash = project_trash(&root, &config.layout, project)?;
    let removed = jobs.0.while_idle(&root, &config, project, id, || {
        recipes::remove(&dir, &trash, id)
    })?;
    jobs.1.close_recipe_bake(project, id, Decision::Cancelled);
    Ok(removed)
}

/// Puts a removed recipe back with its draft marks; answers its id.
pub(crate) fn restore(session: &Session, project: &str, trash: &str) -> CommandResult<String> {
    let (root, config) = session.workspace()?;
    let dir = project_dir(session, project)?;
    let from = project_trash(&root, &config.layout, project)?;
    Ok(recipes::restore(&dir, &from, trash)?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn recipe_list(
    session: State<'_, Session>,
    project: String,
) -> CommandResult<Vec<RecipeSummary>> {
    list(&session, &project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn recipe_get(
    session: State<'_, Session>,
    project: String,
    id: String,
) -> CommandResult<RecipeDoc> {
    get(&session, &project, &id)
}

/// Makes a recipe on the library clip `clip`, or on the model's rest pose
/// when it is null, called `name` or after the clip.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_create(
    session: State<'_, Session>,
    project: String,
    clip: Option<String>,
    name: Option<String>,
) -> CommandResult<RecipeDoc> {
    create(&session, &project, clip.as_deref(), name.as_deref())
}

/// Renames a recipe; its file and id stay.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_rename(
    session: State<'_, Session>,
    project: String,
    id: String,
    name: String,
) -> CommandResult<RecipeSummary> {
    let (root, dir, _) = opened(&session, &project)?;
    library_recipes::rename(&root, &dir, &id, &name).map_err(coded)
}

/// Replaces a recipe's name, changes, blanks or any of them in one write,
/// checked whole first. A person saves here ([`save`]).
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_update(
    session: State<'_, Session>,
    project: String,
    id: String,
    change: RecipeChange,
) -> CommandResult<RecipeDoc> {
    save(&session, &project, &id, change)
}

/// A person's save of the recipe `id` in the editor: the change, and the
/// draft marks cleared in the same write lock once it is written, since what
/// agents drafted is theirs now.
///
/// # Errors
///
/// As the change is refused; marks that cannot be cleared never fail it.
pub fn save(
    session: &Session,
    project: &str,
    id: &str,
    change: RecipeChange,
) -> CommandResult<RecipeDoc> {
    let (root, dir, manifest) = opened(session, project)?;
    library_recipes::save(&root, &dir, &manifest, id, change).map_err(coded)
}

/// Moves the recipe and its draft marks into the workspace trash, refused
/// while it bakes; a person only, no MCP tool removes a recipe.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_remove(
    session: State<'_, Session>,
    jobs: State<'_, RecipeBakeJobs>,
    project: String,
    id: String,
) -> CommandResult<RemovedRecipe> {
    remove(&session, &jobs, &project, &id)
}

/// Puts a removed recipe back from the workspace trash (Undo); answers its
/// id. Refused when a recipe with that id is in the project again.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_restore(
    session: State<'_, Session>,
    project: String,
    trash: String,
) -> CommandResult<String> {
    restore(&session, &project, &trash)
}
