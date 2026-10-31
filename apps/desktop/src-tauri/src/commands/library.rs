//! The library: models kept outside any project. Add to library copies a
//! Board asset in, Use in project copies an entry out as a new asset, and
//! Update brings a newer library version into an asset that came from it.
//! None of it spends credits.

use maquette_core::inbox;
use maquette_core::library::{
    self, AddToLibrary, LibraryAdded, LibraryUpdated, LibraryUsed, LibraryView, UpdatePlan,
};
use maquette_core::project::{self, ProjectManifest};
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
    let added = library::add(&root, &config.layout, request, &sends)?;
    inbox::note_library(&root, &config, &request.project, &request.asset);
    Ok(added)
}

/// Closes the By hand sends a library model just stood in for, and reads
/// the project again when that moved a card.
fn settle_handed(
    session: &Session,
    jobs: &ProviderJobs,
    project: &str,
    manifest: &mut ProjectManifest,
) -> CommandResult<()> {
    if jobs.settle_handed(project) > 0 {
        let (root, config) = session.workspace()?;
        *manifest = project::load(&root, &config.layout, project)?;
    }
    Ok(())
}

pub(crate) fn use_entry(
    session: &Session,
    jobs: &ProviderJobs,
    entry: &str,
    version: Option<u32>,
    project: &str,
) -> CommandResult<LibraryUsed> {
    let (root, config) = session.workspace()?;
    let mut used = library::use_in_project(&root, &config.layout, entry, version, project)?;
    // An asset of the same name already here takes the model as a version.
    settle_handed(session, jobs, project, &mut used.project)?;
    Ok(used)
}

pub(crate) fn plan(session: &Session, project: &str, asset: &str) -> CommandResult<UpdatePlan> {
    let (root, config) = session.workspace()?;
    Ok(library::plan_update(&root, &config.layout, project, asset)?)
}

pub(crate) fn update(
    session: &Session,
    jobs: &ProviderJobs,
    project: &str,
    asset: &str,
) -> CommandResult<LibraryUpdated> {
    let (root, config) = session.workspace()?;
    let mut done = library::update(&root, &config.layout, project, asset)?;
    settle_handed(session, jobs, project, &mut done.project)?;
    Ok(done)
}

/// Every library entry with the projects that use it.
#[tauri::command(async)]
#[specta::specta]
pub fn library_get(session: State<'_, Session>) -> CommandResult<LibraryView> {
    get(&session)
}

/// Copies an asset's newest model, picture, refs and settings into the
/// library, as a new entry or the next version of its entry.
#[tauri::command(async)]
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
#[tauri::command(async)]
#[specta::specta]
pub fn library_use(
    session: State<'_, Session>,
    studio: State<'_, Studio>,
    entry: String,
    version: Option<u32>,
    project: String,
) -> CommandResult<LibraryUsed> {
    use_entry(&session, &studio.0, &entry, version, &project)
}

/// What updating the asset from the library would keep and drop.
#[tauri::command(async)]
#[specta::specta]
pub fn library_update_plan(
    session: State<'_, Session>,
    project: String,
    asset: String,
) -> CommandResult<UpdatePlan> {
    plan(&session, &project, &asset)
}

/// Brings the library's newest version in as the asset's next version.
#[tauri::command(async)]
#[specta::specta]
pub fn library_update(
    session: State<'_, Session>,
    studio: State<'_, Studio>,
    project: String,
    asset: String,
) -> CommandResult<LibraryUpdated> {
    update(&session, &studio.0, &project, &asset)
}
