//! The grip library: hand poses kept outside any project with every
//! revision saved to it, next to the recipe library. Listing and reading
//! them, saving a grip as a new library grip or the next revision of one
//! (the person's confirm only: an agent's propose_grip_save waits for it),
//! and removing a revision or a grip. Laying a grip on a recipe and reading
//! one from a recipe's pose keys are here for the MCP tools; the app's own
//! editor does both on the recipe as the person is editing it. The helpers
//! are shared with the MCP tools, so both make the same checks.

use maquette_core::animations::grips::{
    self, ApplyGrip, GripApplied, GripFromKeys, GripKeyed, GripLink, GripSave, GripSavePlan,
    GripSaved, LibraryGripDoc, LibraryGripEntry, LibraryGripSummary,
};
use maquette_core::recipes::RecipeFile;
use maquette_core::recipes::grips::GripFile;
use tauri::{AppHandle, State};
use tauri_specta::Event as _;

use super::CommandResult;
use crate::events::GripLibraryChanged;
use crate::mcp::Agents;
use crate::session::Session;

pub(crate) fn list(session: &Session) -> CommandResult<Vec<LibraryGripSummary>> {
    let (root, _) = session.workspace()?;
    Ok(grips::list(&root)?)
}

pub(crate) fn get(
    session: &Session,
    entry: &str,
    revision: Option<u32>,
) -> CommandResult<LibraryGripDoc> {
    let (root, _) = session.workspace()?;
    Ok(grips::get(&root, entry, revision)?)
}

pub(crate) fn plan_save(
    session: &Session,
    grip: &GripFile,
    entry: Option<&str>,
) -> CommandResult<GripSavePlan> {
    let (root, _) = session.workspace()?;
    Ok(grips::plan_save(&root, grip, entry)?)
}

/// The grip a pose key change of a project's recipe holds, as its file has it.
pub(crate) fn keyed(session: &Session, request: &GripFromKeys) -> CommandResult<GripKeyed> {
    let (root, config) = session.workspace()?;
    Ok(grips::from_keys(&root, &config.layout, request)?)
}

/// Lays a library grip on a project's recipe in one checked write.
pub(crate) fn apply(session: &Session, request: &ApplyGrip) -> CommandResult<GripApplied> {
    let (root, config) = session.workspace()?;
    grips::apply(&root, &config.layout, request).map_err(super::recipes::coded)
}

/// How each grip change of `recipe` stands against the library now.
pub(crate) fn links(session: &Session, recipe: &RecipeFile) -> CommandResult<Vec<GripLink>> {
    let (root, _) = session.workspace()?;
    Ok(grips::links(&root, recipe))
}

fn changed(app: &AppHandle) {
    if let Err(err) = GripLibraryChanged.emit(app) {
        tracing::warn!(error = %err, "could not emit a grip library event");
    }
}

/// Every library grip with its newest revision's weapon, hands and pose.
#[tauri::command(async)]
#[specta::specta]
pub fn grip_library_list(session: State<'_, Session>) -> CommandResult<Vec<LibraryGripSummary>> {
    list(&session)
}

/// A library grip's revision, the newest when none is given.
#[tauri::command(async)]
#[specta::specta]
pub fn grip_library_get(
    session: State<'_, Session>,
    entry: String,
    revision: Option<u32>,
) -> CommandResult<LibraryGripDoc> {
    get(&session, &entry, revision)
}

/// What saving `grip` to the library would do, for the save sheet: a new
/// library grip, or the next revision of `entry`.
#[tauri::command(async)]
#[specta::specta]
pub fn grip_library_plan_save(
    session: State<'_, Session>,
    grip: GripFile,
    entry: Option<String>,
) -> CommandResult<GripSavePlan> {
    plan_save(&session, &grip, entry.as_deref())
}

/// Saves a grip to the library as the person confirmed it in the sheet:
/// always a new revision, never an overwrite. A grip that moved since the
/// sheet read it is refused (`moved`). An agent's proposal for the pose key
/// change it was read from ends as approved when it went where the agent
/// asked, and as cancelled when the person saved it elsewhere.
#[tauri::command(async)]
#[specta::specta]
pub fn grip_library_save(
    app: AppHandle,
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    save: GripSave,
) -> CommandResult<GripSaved> {
    let (root, _) = session.workspace()?;
    let saved = grips::save_grip(&root, &save)?;
    if let Some(source) = &save.source {
        agents.hub.close_grip_save(
            &source.project,
            &source.recipe,
            &source.layer,
            save.entry.as_deref(),
        );
    }
    changed(&app);
    Ok(saved)
}

/// Removes one revision of a library grip (moved aside, never deleted).
#[tauri::command(async)]
#[specta::specta]
pub fn grip_library_remove_revision(
    app: AppHandle,
    session: State<'_, Session>,
    entry: String,
    revision: u32,
    expected: Option<String>,
) -> CommandResult<LibraryGripEntry> {
    let (root, _) = session.workspace()?;
    let left = grips::remove_revision(&root, &entry, revision, expected.as_deref())?;
    changed(&app);
    Ok(left)
}

/// Removes a library grip with every revision (moved aside, never deleted).
#[tauri::command(async)]
#[specta::specta]
pub fn grip_library_remove(
    app: AppHandle,
    session: State<'_, Session>,
    entry: String,
    expected: Option<String>,
) -> CommandResult<String> {
    let (root, _) = session.workspace()?;
    let went = grips::remove_entry(&root, &entry, expected.as_deref())?;
    changed(&app);
    Ok(went)
}
