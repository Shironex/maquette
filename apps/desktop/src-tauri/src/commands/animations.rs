//! The animation library: clips kept once outside any project, skeleton
//! only. Import reads FBX files and pack zips headless and adds the clips
//! the person keeps; Save clips copies a rigged model's clips in; Use on
//! model bakes clips onto a rigged model as its next version. Blender
//! always runs headless, one job at a time, and none of it spends credits;
//! the review's Open in Blender is the one window, on a click.

use std::sync::{Arc, LazyLock};

use maquette_core::animations::retarget::{
    self, Baked, MapChoice, RetargetDraft, RetargetPlan, UseClips,
};
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
use crate::events::{LibraryChanged, ProjectChanged};
use crate::session::Session;

/// The library's Blender jobs run one at a time, app and agents alike.
static RUNNER: LazyLock<PipelineRunner> = LazyLock::new(|| PipelineRunner::new(Arc::new(NullSink)));

pub(crate) fn get(session: &Session) -> CommandResult<AnimationsView> {
    let (root, config) = session.workspace()?;
    Ok(animations::view(&root, &config.layout)?)
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

/// The model's clips that came from the library, as (name in the model,
/// library clip name), so Save clips never saves them back.
fn baked(session: &Session, from: &ClipsFrom) -> Vec<(String, String)> {
    let (Some(project), Some(asset)) = (&from.project, &from.asset) else {
        return Vec::new();
    };
    let Ok((root, config)) = session.workspace() else {
        return Vec::new();
    };
    maquette_core::project::dir(&root, &config.layout, project)
        .and_then(|dir| retarget::uses::of_asset(&dir, asset))
        .map(|entry| {
            entry
                .baked
                .into_iter()
                .map(|b| (b.setting.name, b.library_name))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn save_plan(
    session: &Session,
    jobs: &ProviderJobs,
    from: &ClipsFrom,
) -> CommandResult<SavePlan> {
    let (root, config) = session.workspace()?;
    let tripo = from_tripo(session, jobs, from);
    let baked = baked(session, from);
    Ok(animations::save::plan(&root, &config, from, tripo, &baked)?)
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
        &baked(session, &request.from),
    )?)
}

pub(crate) fn retarget_plan(
    session: &Session,
    (project, asset): (&str, &str),
    clips: &[String],
    picked: &[MapChoice],
) -> CommandResult<RetargetPlan> {
    let (root, config) = session.workspace()?;
    Ok(retarget::plan(
        &root,
        &config,
        (project, asset),
        clips,
        picked,
    )?)
}

pub(crate) fn retarget(
    session: &Session,
    tools: &Toolbox,
    request: &UseClips,
) -> CommandResult<RetargetDraft> {
    let (root, config) = session.workspace()?;
    let runner: &PipelineRunner = &RUNNER;
    Ok(retarget::draft(&root, &config, (runner, tools), request)?)
}

pub(crate) fn bake(session: &Session, token: &str) -> CommandResult<Baked> {
    let (root, config) = session.workspace()?;
    Ok(retarget::bake(&root, &config, token)?)
}

/// Tells the windows a bake made a new version.
pub(crate) fn baked_into(app: &AppHandle, baked: &Baked) {
    let event = ProjectChanged {
        project: baked.project.clone(),
    };
    if let Err(err) = event.emit(app) {
        tracing::warn!(error = %err, "could not emit a project event");
    }
    changed(app);
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

/// Where library clips would go on a rigged model: the suggested bone map
/// and each clip's settings. Reads, never writes.
#[tauri::command]
#[specta::specta]
pub fn animations_retarget_plan(
    session: State<'_, Session>,
    project: String,
    asset: String,
    clips: Vec<String>,
    picked: Vec<MapChoice>,
) -> CommandResult<RetargetPlan> {
    retarget_plan(&session, (&project, &asset), &clips, &picked)
}

/// Bakes library clips onto a copy of the model's newest version, headless,
/// and reviews them. The project is not changed yet.
#[tauri::command]
#[specta::specta]
pub async fn animations_retarget(
    app: AppHandle,
    session: State<'_, Session>,
    request: UseClips,
) -> CommandResult<RetargetDraft> {
    let tools = super::toolchain::toolbox(&session).await?;
    tauri::async_runtime::spawn_blocking(move || {
        retarget(app.state::<Session>().inner(), &tools, &request)
    })
    .await
    .map_err(super::join_failed)?
}

/// Adds a reviewed bake as the model's next version, the failed clips
/// left out and the older version kept.
#[tauri::command]
#[specta::specta]
pub fn animations_bake(
    app: AppHandle,
    session: State<'_, Session>,
    token: String,
) -> CommandResult<Baked> {
    let baked = bake(&session, &token)?;
    baked_into(&app, &baked);
    Ok(baked)
}

/// Opens a retarget's baked copy in Blender's window. Only ever from a
/// click on Open in Blender in the review.
#[tauri::command]
#[specta::specta]
pub async fn animations_open_draft_in_blender(
    session: State<'_, Session>,
    token: String,
) -> CommandResult<()> {
    let (root, config) = session.workspace()?;
    let file = retarget::draft_file(&root, &config, &token)?;
    super::blender::open_gui(&session, &file).await
}
