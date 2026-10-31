//! Render styles: the shell side of `maquette_core::renders`. The Inspector's
//! Render tab and the batch sheet from a Board or Library selection plan and
//! start renders here; each change reaches the web view as a `RenderChanged`
//! event. Every file lands in a project's or the workspace's exports folder.

use std::path::Path;
use std::sync::Arc;

use maquette_core::CoreError;
use maquette_core::packs::PackSource;
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::renders::{
    self, RenderPlan, RenderPreview, RenderRecord, RenderRequest, RenderSettings, RenderShot,
    Renders,
};
use tauri::{AppHandle, Manager as _, State};
use tauri_specta::Event as _;

use super::CommandResult;
use crate::events::RenderChanged;
use crate::session::Session;

/// The render service the app shares between commands and the MCP tools.
pub struct RenderJobs(pub Renders);

impl RenderJobs {
    /// A service whose every change tells the web view.
    pub fn new(handle: AppHandle) -> Self {
        let runner = PipelineRunner::new(Arc::new(NullSink));
        Self(Renders::new(runner, move |record| {
            let event = RenderChanged {
                record: record.clone(),
            };
            if let Err(err) = event.emit(&handle) {
                tracing::warn!(error = %err, "could not emit a render event");
            }
        }))
    }
}

/// What `source` would render.
pub(crate) fn plan(session: &Session, source: &PackSource) -> CommandResult<RenderPlan> {
    let (root, config) = session.workspace()?;
    Ok(renders::plan(&root, &config.layout, source)?)
}

/// Plans and starts a render.
pub(crate) fn start(
    session: &Session,
    service: &Renders,
    tools: Toolbox,
    request: &RenderRequest,
) -> CommandResult<RenderRecord> {
    let (root, config) = session.workspace()?;
    let planned = renders::plan(&root, &config.layout, &request.source)?;
    Ok(service.start(&root, tools, request, planned)?)
}

/// The models a render of `source` would hold, with where their files go.
/// Reads only.
#[tauri::command(async)]
#[specta::specta]
pub fn render_plan(session: State<'_, Session>, source: PackSource) -> CommandResult<RenderPlan> {
    plan(&session, &source)
}

/// The small pictures of one model the Render tab shows. Blender runs for
/// what is not cached yet.
#[tauri::command]
#[specta::specta]
pub async fn render_preview(
    app: AppHandle,
    source: PackSource,
    key: String,
    settings: RenderSettings,
    shots: Vec<RenderShot>,
) -> CommandResult<RenderPreview> {
    let session = app.state::<Session>();
    let tools = super::toolchain::toolbox(&session).await?;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session = handle.state::<Session>();
        let (root, config) = session.workspace()?;
        Ok(handle
            .state::<RenderJobs>()
            .0
            .preview(&root, &config, &tools, &source, &key, &settings, &shots)?)
    })
    .await
    .map_err(super::join_failed)?
}

/// Starts a render; its progress comes as `RenderChanged` events.
#[tauri::command]
#[specta::specta]
pub async fn render_start(app: AppHandle, request: RenderRequest) -> CommandResult<RenderRecord> {
    let session = app.state::<Session>();
    let tools = super::toolchain::toolbox(&session).await?;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        start(
            handle.state::<Session>().inner(),
            &handle.state::<RenderJobs>().0,
            tools,
            &request,
        )
    })
    .await
    .map_err(super::join_failed)?
}

/// One render's record.
#[tauri::command(async)]
#[specta::specta]
pub fn render_get(
    session: State<'_, Session>,
    renders: State<'_, RenderJobs>,
    id: String,
) -> CommandResult<RenderRecord> {
    let (root, _) = session.workspace()?;
    Ok(renders.0.get(&root, &id)?)
}

/// The newest renders first.
#[tauri::command(async)]
#[specta::specta]
pub fn render_recent(
    session: State<'_, Session>,
    renders: State<'_, RenderJobs>,
) -> CommandResult<Vec<RenderRecord>> {
    let (root, _) = session.workspace()?;
    Ok(renders.0.recent(&root))
}

/// Stops a render once the file running now is done.
#[tauri::command(async)]
#[specta::specta]
pub fn render_stop(renders: State<'_, RenderJobs>, id: String) -> CommandResult<RenderRecord> {
    Ok(renders.0.stop(&id)?)
}

/// Goes on with a stopped render from the next file.
#[tauri::command]
#[specta::specta]
pub async fn render_resume(
    session: State<'_, Session>,
    renders: State<'_, RenderJobs>,
    id: String,
) -> CommandResult<RenderRecord> {
    let tools = super::toolchain::toolbox(&session).await?;
    let (root, _) = session.workspace()?;
    Ok(renders.0.resume(&root, tools, &id)?)
}

/// Shows a render's folder in Finder or Explorer. Only a render's own
/// folder can be shown this way.
#[tauri::command(async)]
#[specta::specta]
pub fn render_reveal(
    session: State<'_, Session>,
    renders: State<'_, RenderJobs>,
    id: String,
) -> CommandResult<()> {
    let (root, _) = session.workspace()?;
    let record = renders.0.get(&root, &id)?;
    let target = Path::new(&record.dir);
    if !target.exists() {
        return Err(CoreError::NotFound(format!("the folder {}", record.dir)).into());
    }
    crate::opener::reveal_file(target).map_err(|err| CoreError::io(target, err).payload())
}
