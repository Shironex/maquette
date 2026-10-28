//! The animation library: clips kept once outside any project, skeleton
//! only. Import reads FBX files and pack zips headless and adds the clips
//! the person keeps; Save clips copies a rigged model's clips in. Blender
//! always runs headless, one job at a time, and none of it spends credits.

use std::sync::{Arc, LazyLock};

use maquette_core::animations::{
    self, AnimationsView, ClipsFrom, ImportPlan, ImportRequest, Imported, SaveClips, SavePlan,
    Saved,
};
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{Provider, ProviderJobs};
use tauri::{AppHandle, Manager as _, State};
use tauri_specta::Event as _;

use super::CommandResult;
use super::providers::Studio;
use crate::events::LibraryChanged;
use crate::session::Session;

/// The library's Blender jobs run one at a time, app and agents alike.
static RUNNER: LazyLock<PipelineRunner> = LazyLock::new(|| PipelineRunner::new(Arc::new(NullSink)));

pub(crate) fn get(session: &Session) -> CommandResult<AnimationsView> {
    let (root, _) = session.workspace()?;
    Ok(animations::view(&root)?)
}

pub(crate) fn read_import(
    session: &Session,
    tools: &Toolbox,
    paths: &[String],
) -> CommandResult<ImportPlan> {
    let (root, config) = session.workspace()?;
    let paths: Vec<String> = paths
        .iter()
        .map(|p| session.expand(p.trim()).display().to_string())
        .collect();
    Ok(animations::import::read(
        &root, &config, &RUNNER, tools, &paths,
    )?)
}

pub(crate) fn import(session: &Session, request: &ImportRequest) -> CommandResult<Imported> {
    let (root, config) = session.workspace()?;
    Ok(animations::import::commit(&root, &config, request)?)
}

/// Whether the model came from a Tripo job: the send that made the asset's
/// newest version, or the one an Inbox file is linked to.
fn from_tripo(session: &Session, jobs: &ProviderJobs, from: &ClipsFrom) -> bool {
    let tripo = |p: Provider| p == Provider::Tripo;
    if let Some(file) = &from.inbox {
        let Ok((root, config)) = session.workspace() else {
            return false;
        };
        return maquette_core::inbox::list(&root, &config)
            .unwrap_or_default()
            .iter()
            .find(|i| &i.id == file)
            .and_then(|i| i.link.as_ref())
            .is_some_and(|l| tripo(l.provider));
    }
    let (Some(project), Some(asset)) = (&from.project, &from.asset) else {
        return false;
    };
    let mut sends = jobs.list(Some(project));
    sends.retain(|j| &j.asset == asset);
    sends.sort_by_key(|j| j.version);
    sends.last().is_some_and(|j| tripo(j.provider))
}

pub(crate) fn save_plan(
    session: &Session,
    jobs: &ProviderJobs,
    from: &ClipsFrom,
) -> CommandResult<SavePlan> {
    let (root, config) = session.workspace()?;
    let tripo = from_tripo(session, jobs, from);
    Ok(animations::save::plan(&root, &config, from, tripo, &[])?)
}

pub(crate) fn save(
    session: &Session,
    jobs: &ProviderJobs,
    tools: &Toolbox,
    request: &SaveClips,
) -> CommandResult<Saved> {
    let (root, config) = session.workspace()?;
    let tripo = from_tripo(session, jobs, &request.from);
    let runner: &PipelineRunner = &RUNNER;
    Ok(animations::save::save(
        &root,
        &config,
        (runner, tools),
        request,
        tripo,
        &[],
    )?)
}

fn changed(app: &AppHandle) {
    if let Err(err) = LibraryChanged.emit(app) {
        tracing::warn!(error = %err, "could not emit a library event");
    }
}

/// Every clip in the animation library.
#[tauri::command]
#[specta::specta]
pub fn animations_get(session: State<'_, Session>) -> CommandResult<AnimationsView> {
    get(&session)
}

/// Reads FBX files and pack zips headless and says what an import would
/// add. Nothing is added yet.
#[tauri::command]
#[specta::specta]
pub async fn animations_read_import(
    app: AppHandle,
    session: State<'_, Session>,
    paths: Vec<String>,
) -> CommandResult<ImportPlan> {
    let tools = super::toolchain::toolbox(&session).await?;
    tauri::async_runtime::spawn_blocking(move || {
        read_import(app.state::<Session>().inner(), &tools, &paths)
    })
    .await
    .map_err(super::join_failed)?
}

/// Adds the clips of a read import as the person chose.
#[tauri::command]
#[specta::specta]
pub fn animations_import(
    app: AppHandle,
    session: State<'_, Session>,
    request: ImportRequest,
) -> CommandResult<Imported> {
    let done = import(&session, &request)?;
    changed(&app);
    Ok(done)
}

/// The clips a rigged model offers to the library.
#[tauri::command]
#[specta::specta]
pub fn animations_save_plan(
    session: State<'_, Session>,
    studio: State<'_, Studio>,
    from: ClipsFrom,
) -> CommandResult<SavePlan> {
    save_plan(&session, &studio.0, &from)
}

/// Saves the chosen clips of a rigged model as a pack, skeleton only.
#[tauri::command]
#[specta::specta]
pub async fn animations_save(
    app: AppHandle,
    session: State<'_, Session>,
    request: SaveClips,
) -> CommandResult<Saved> {
    let tools = super::toolchain::toolbox(&session).await?;
    tauri::async_runtime::spawn_blocking(move || {
        let studio = app.state::<Studio>();
        let saved = save(app.state::<Session>().inner(), &studio.0, &tools, &request)?;
        changed(&app);
        Ok(saved)
    })
    .await
    .map_err(super::join_failed)?
}
