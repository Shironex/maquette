//! The library at scale: tags on many entries, renames, collections,
//! removing and restoring, look-alikes, merges and the plan for adding a
//! whole project. None of it spends credits; a merge is undone by
//! restoring the merged entry.

use maquette_core::library::{
    self, AddManyPlan, CollectionChange, LibraryCollection, LibraryEntry, LibraryMerged,
    LibraryRestored, LookAlikeSettings, LookAlikeVerdict, LookAlikes, MergeEntries, TagEntries,
};
use tauri::{AppHandle, Manager, State};

use super::CommandResult;
use crate::session::Session;

pub(crate) fn tag(session: &Session, request: &TagEntries) -> CommandResult<Vec<LibraryEntry>> {
    let (root, _) = session.workspace()?;
    Ok(library::tag(&root, request)?)
}

pub(crate) fn rename(session: &Session, entry: &str, name: &str) -> CommandResult<LibraryEntry> {
    let (root, _) = session.workspace()?;
    Ok(library::rename(&root, entry, name)?)
}

pub(crate) fn collections(session: &Session) -> CommandResult<Vec<LibraryCollection>> {
    let (root, _) = session.workspace()?;
    Ok(library::collections(&root)?)
}

pub(crate) fn collection(
    session: &Session,
    change: &CollectionChange,
) -> CommandResult<Vec<LibraryCollection>> {
    let (root, _) = session.workspace()?;
    Ok(library::change_collection(&root, change)?)
}

pub(crate) fn remove(session: &Session, entries: &[String]) -> CommandResult<Vec<String>> {
    let (root, _) = session.workspace()?;
    Ok(library::remove(&root, entries)?)
}

pub(crate) fn restore(session: &Session, entries: &[String]) -> CommandResult<LibraryRestored> {
    let (root, config) = session.workspace()?;
    Ok(library::restore(&root, &config.layout, entries)?)
}

pub(crate) fn look_alikes(session: &Session) -> CommandResult<LookAlikes> {
    let (root, _) = session.workspace()?;
    Ok(library::look_alikes(&root)?)
}

pub(crate) fn look_settings(session: &Session) -> CommandResult<LookAlikeSettings> {
    let (root, _) = session.workspace()?;
    Ok(library::look_alike_settings(&root)?)
}

pub(crate) fn merge(session: &Session, request: &MergeEntries) -> CommandResult<LibraryMerged> {
    let (root, config) = session.workspace()?;
    Ok(library::merge(&root, &config.layout, request)?)
}

pub(crate) fn plan_many(session: &Session, project: &str) -> CommandResult<AddManyPlan> {
    let (root, config) = session.workspace()?;
    Ok(library::plan_add_many(&root, &config.layout, project)?)
}

/// Puts tags on and takes tags off every entry named.
#[tauri::command(async)]
#[specta::specta]
pub fn library_tag(
    session: State<'_, Session>,
    request: TagEntries,
) -> CommandResult<Vec<LibraryEntry>> {
    tag(&session, &request)
}

/// Renames an entry; its id and files stay.
#[tauri::command(async)]
#[specta::specta]
pub fn library_rename(
    session: State<'_, Session>,
    entry: String,
    name: String,
) -> CommandResult<LibraryEntry> {
    rename(&session, &entry, &name)
}

/// Creates, fills, empties, renames or deletes a collection. Answers every
/// collection after the change.
#[tauri::command(async)]
#[specta::specta]
pub fn library_collection(
    session: State<'_, Session>,
    change: CollectionChange,
) -> CommandResult<Vec<LibraryCollection>> {
    collection(&session, &change)
}

/// Moves entries out of the library; projects keep their copies.
#[tauri::command(async)]
#[specta::specta]
pub fn library_remove(
    session: State<'_, Session>,
    entries: Vec<String>,
) -> CommandResult<Vec<String>> {
    remove(&session, &entries)
}

/// Puts removed entries back, undoing a merge for a merged one.
#[tauri::command(async)]
#[specta::specta]
pub fn library_restore(
    session: State<'_, Session>,
    entries: Vec<String>,
) -> CommandResult<LibraryRestored> {
    restore(&session, &entries)
}

/// The entries that may be one model twice, with the recent merges.
///
/// Opening the library asks for this, and it decodes every entry's picture and
/// scores every pair: seconds at a few hundred models, so never the main thread.
#[tauri::command]
#[specta::specta]
pub async fn library_look_alikes(app: AppHandle) -> CommandResult<LookAlikes> {
    tauri::async_runtime::spawn_blocking(move || look_alikes(app.state::<Session>().inner()))
        .await
        .map_err(super::join_failed)?
}

/// Records a verdict on a look-alike pair so it is no longer listed.
#[tauri::command(async)]
#[specta::specta]
pub fn library_decide(
    session: State<'_, Session>,
    a: String,
    b: String,
    verdict: LookAlikeVerdict,
) -> CommandResult<()> {
    let (root, _) = session.workspace()?;
    Ok(library::decide(&root, &a, &b, verdict)?)
}

/// Saves the look-alike settings; answers them as saved.
#[tauri::command(async)]
#[specta::specta]
pub fn library_look_settings(
    session: State<'_, Session>,
    settings: LookAlikeSettings,
) -> CommandResult<LookAlikeSettings> {
    let (root, _) = session.workspace()?;
    Ok(library::save_settings(&root, &settings)?)
}

/// Merges one entry into another; restoring the merged entry undoes it.
#[tauri::command(async)]
#[specta::specta]
pub fn library_merge(
    session: State<'_, Session>,
    request: MergeEntries,
) -> CommandResult<LibraryMerged> {
    merge(&session, &request)
}

/// What adding every asset of a project would do. Reads only.
#[tauri::command(async)]
#[specta::specta]
pub fn library_add_many_plan(
    session: State<'_, Session>,
    project: String,
) -> CommandResult<AddManyPlan> {
    plan_many(&session, &project)
}
