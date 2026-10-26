//! The library: models kept outside any project. Add to library copies a
//! Board asset in, Use in project copies an entry out as a new asset, and
//! Update brings a newer library version into an asset that came from it.
//! None of it spends credits.

use maquette_core::library::{
    self, AddToLibrary, LibraryAdded, LibraryUpdated, LibraryUsed, LibraryView, UpdatePlan,
};
use maquette_core::providers::ProviderJobs;
use tauri::State;

use super::CommandResult;
use super::providers::Studio;
use crate::session::Session;

pub(crate) fn get(session: &Session) -> CommandResult<LibraryView> {
    let (root, config) = session.workspace()?;
    Ok(library::view(&root, &config.layout)?)
}

/// Adds the asset's newest model; its project's sends say what made it.
pub(crate) fn add(
    session: &Session,
    jobs: &ProviderJobs,
    request: &AddToLibrary,
) -> CommandResult<LibraryAdded> {
    let (root, config) = session.workspace()?;
    let sends = jobs.list(Some(&request.project));
    Ok(library::add(&root, &config.layout, request, &sends)?)
}

pub(crate) fn use_entry(
    session: &Session,
    entry: &str,
    version: Option<u32>,
    project: &str,
) -> CommandResult<LibraryUsed> {
    let (root, config) = session.workspace()?;
    Ok(library::use_in_project(
        &root,
        &config.layout,
        entry,
        version,
        project,
    )?)
}

pub(crate) fn plan(session: &Session, project: &str, asset: &str) -> CommandResult<UpdatePlan> {
    let (root, config) = session.workspace()?;
    Ok(library::plan_update(&root, &config.layout, project, asset)?)
}

pub(crate) fn update(
    session: &Session,
    project: &str,
    asset: &str,
) -> CommandResult<LibraryUpdated> {
    let (root, config) = session.workspace()?;
    Ok(library::update(&root, &config.layout, project, asset)?)
}

/// Every library entry with the projects that use it.
#[tauri::command]
#[specta::specta]
pub fn library_get(session: State<'_, Session>) -> CommandResult<LibraryView> {
    get(&session)
}

/// Copies an asset's newest model, picture, refs and settings into the
/// library, as a new entry or the next version of its entry.
#[tauri::command]
#[specta::specta]
pub fn library_add(
    session: State<'_, Session>,
    studio: State<'_, Studio>,
    request: AddToLibrary,
) -> CommandResult<LibraryAdded> {
    add(&session, &studio.0, &request)
}

/// Copies a library version (the newest without `version`) into a project
/// as a new asset that remembers where it came from.
#[tauri::command]
#[specta::specta]
pub fn library_use(
    session: State<'_, Session>,
    entry: String,
    version: Option<u32>,
    project: String,
) -> CommandResult<LibraryUsed> {
    use_entry(&session, &entry, version, &project)
}

/// What updating the asset from the library would keep and drop.
#[tauri::command]
#[specta::specta]
pub fn library_update_plan(
    session: State<'_, Session>,
    project: String,
    asset: String,
) -> CommandResult<UpdatePlan> {
    plan(&session, &project, &asset)
}

/// Brings the library's newest version in as the asset's next version.
#[tauri::command]
#[specta::specta]
pub fn library_update(
    session: State<'_, Session>,
    project: String,
    asset: String,
) -> CommandResult<LibraryUpdated> {
    update(&session, &project, &asset)
}
