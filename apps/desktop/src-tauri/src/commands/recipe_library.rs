//! The recipe library: recipes kept outside any project with every revision
//! saved to it, next to the clip library. Listing and reading them, using
//! one in a project, saving a project's recipe back as a new revision (the
//! person's confirm only: an agent's propose_recipe_save waits for it),
//! removing a revision or an entry, and moving recipe files in and out
//! through the system's file pickers. The helpers are shared with the MCP
//! tools, so both make the same checks.

use std::path::Path;

use maquette_core::animations::recipes::library::{
    self, ExportSource, LibraryRecipeDoc, LibraryRecipeEntry, LibraryRecipeSummary, RecipeCompare,
    RecipeExported, RecipeImported, RecipeSave, RecipeSavePlan, RecipeSaved, RecipeUsed, UseRecipe,
};
use maquette_core::providers::ledger::Decision;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt as _;
use tauri_specta::Event as _;

use super::CommandResult;
use crate::events::{RecipeChanged, RecipeLibraryChanged};
use crate::mcp::Agents;
use crate::session::Session;

pub(crate) fn list(session: &Session) -> CommandResult<Vec<LibraryRecipeSummary>> {
    let (root, _) = session.workspace()?;
    Ok(library::list(&root)?)
}

pub(crate) fn get(
    session: &Session,
    entry: &str,
    revision: Option<u32>,
) -> CommandResult<LibraryRecipeDoc> {
    let (root, _) = session.workspace()?;
    Ok(library::get(&root, entry, revision)?)
}

pub(crate) fn use_in(session: &Session, request: &UseRecipe) -> CommandResult<RecipeUsed> {
    let (root, config) = session.workspace()?;
    Ok(library::use_recipe(&root, &config.layout, request)?)
}

pub(crate) fn plan_save(
    session: &Session,
    project: &str,
    recipe: &str,
    entry: Option<&str>,
) -> CommandResult<RecipeSavePlan> {
    let (root, config) = session.workspace()?;
    Ok(library::plan_save(
        &root,
        &config.layout,
        project,
        recipe,
        entry,
    )?)
}

pub(crate) fn import_to_project(
    session: &Session,
    project: &str,
    file: &Path,
) -> CommandResult<RecipeUsed> {
    let (root, config) = session.workspace()?;
    Ok(library::import_to_project(
        &root,
        &config.layout,
        project,
        file,
    )?)
}

pub(crate) fn import_to_library(session: &Session, file: &Path) -> CommandResult<RecipeImported> {
    let (root, _) = session.workspace()?;
    Ok(library::import_to_library(&root, file)?)
}

pub(crate) fn export_to(
    session: &Session,
    source: &ExportSource,
    to: &Path,
) -> CommandResult<RecipeExported> {
    let (root, config) = session.workspace()?;
    Ok(library::export(&root, &config.layout, source, to)?)
}

fn changed(app: &AppHandle, project: Option<&str>) {
    if let Err(err) = RecipeLibraryChanged.emit(app) {
        tracing::warn!(error = %err, "could not emit a recipe library event");
    }
    if let Some(project) = project {
        let event = RecipeChanged {
            project: project.to_string(),
        };
        if let Err(err) = event.emit(app) {
            tracing::warn!(error = %err, "could not emit a recipe event");
        }
    }
}

#[tauri::command(async)]
#[specta::specta]
pub fn recipe_library_list(
    session: State<'_, Session>,
) -> CommandResult<Vec<LibraryRecipeSummary>> {
    list(&session)
}

/// A library recipe's revision, the newest when none is given.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_library_get(
    session: State<'_, Session>,
    entry: String,
    revision: Option<u32>,
) -> CommandResult<LibraryRecipeDoc> {
    get(&session, &entry, revision)
}

/// Copies a library recipe's revision into a project as a new recipe.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_library_use(
    app: AppHandle,
    session: State<'_, Session>,
    request: UseRecipe,
) -> CommandResult<RecipeUsed> {
    let used = use_in(&session, &request)?;
    changed(&app, Some(&request.project));
    Ok(used)
}

/// What saving a project's recipe to the library would do, for the
/// confirm sheet: a new library recipe, or the next revision of `entry`.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_library_plan_save(
    session: State<'_, Session>,
    project: String,
    recipe: String,
    entry: Option<String>,
) -> CommandResult<RecipeSavePlan> {
    plan_save(&session, &project, &recipe, entry.as_deref())
}

/// What the library's newest revision changes against a project's recipe
/// that came from an older one.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_library_compare(
    session: State<'_, Session>,
    project: String,
    recipe: String,
) -> CommandResult<RecipeCompare> {
    let (root, config) = session.workspace()?;
    Ok(library::compare(&root, &config.layout, &project, &recipe)?)
}

/// Saves a project's recipe to the library as the person confirmed it in
/// the sheet: always a new revision, never an overwrite. An entry that
/// moved since the sheet read it is refused (`moved`). An agent's proposal
/// for this recipe ends as approved.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_library_save(
    app: AppHandle,
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    save: RecipeSave,
) -> CommandResult<RecipeSaved> {
    let (root, config) = session.workspace()?;
    let saved = library::save_back(&root, &config.layout, &save)?;
    agents
        .hub
        .close_recipe_save(&save.project, &save.recipe, Decision::Approved);
    changed(&app, Some(&save.project));
    Ok(saved)
}

/// Removes one revision of a library recipe (moved aside, never deleted).
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_library_remove_revision(
    app: AppHandle,
    session: State<'_, Session>,
    entry: String,
    revision: u32,
    expected: Option<String>,
) -> CommandResult<LibraryRecipeEntry> {
    let (root, _) = session.workspace()?;
    let left = library::remove_revision(&root, &entry, revision, expected.as_deref())?;
    changed(&app, None);
    Ok(left)
}

/// Removes a library recipe with every revision (moved aside, never deleted).
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_library_remove(
    app: AppHandle,
    session: State<'_, Session>,
    entry: String,
    expected: Option<String>,
) -> CommandResult<String> {
    let (root, _) = session.workspace()?;
    let went = library::remove_entry(&root, &entry, expected.as_deref())?;
    changed(&app, None);
    Ok(went)
}

/// Asks where to save a recipe file and writes it there: a project's recipe
/// or a library revision, as `source` says. Null when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn recipe_export(
    app: AppHandle,
    source: ExportSource,
    name: String,
) -> CommandResult<Option<RecipeExported>> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager as _;
        let stem = maquette_core::project::latin_slug(&name)
            .map_or_else(|| "recipe".into(), maquette_core::project::windows_safe);
        let picked = app
            .dialog()
            .file()
            .set_title("Export the recipe")
            .set_file_name(format!("{stem}.recipe.json"))
            .add_filter("Recipe", &["json"])
            .blocking_save_file();
        let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
            return Ok(None);
        };
        export_to(&app.state::<Session>(), &source, &path).map(Some)
    })
    .await
    .map_err(super::join_failed)?
}

fn pick_recipe(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.dialog()
        .file()
        .set_title("Import a recipe")
        .add_filter("Recipe", &["json"])
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
}

/// Asks for a `.recipe.json` and imports it into the project as a new
/// recipe. Null when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn recipe_import_to_project(
    app: AppHandle,
    project: String,
) -> CommandResult<Option<RecipeUsed>> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager as _;
        let Some(file) = pick_recipe(&app) else {
            return Ok(None);
        };
        let used = import_to_project(&app.state::<Session>(), &project, &file)?;
        changed(&app, Some(&project));
        Ok(Some(used))
    })
    .await
    .map_err(super::join_failed)?
}

/// Asks for a `.recipe.json` and imports it into the library as a new
/// recipe at r1. Null when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn recipe_import_to_library(app: AppHandle) -> CommandResult<Option<RecipeImported>> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager as _;
        let Some(file) = pick_recipe(&app) else {
            return Ok(None);
        };
        let imported = import_to_library(&app.state::<Session>(), &file)?;
        changed(&app, None);
        Ok(Some(imported))
    })
    .await
    .map_err(super::join_failed)?
}
