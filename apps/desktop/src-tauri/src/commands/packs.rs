//! Export packs: the shell side of `maquette_core::packs`. The Library's
//! Export pack and the Board's project pack plan and start a pack here;
//! each change to a pack reaches the web view as a `PackChanged` event. A
//! person may write a pack to any folder they picked; an agent only inside
//! the workspace (`mcp/tools/packs.rs` passes `from_anywhere` false).

use std::path::Path;
use std::sync::Arc;

use maquette_core::inbox::checks::texture_px;
use maquette_core::library::{self, LibrarySource};
use maquette_core::packs::{
    PackDecision, PackPlan, PackPreset, PackRecord, PackRequest, PackSettings, PackSource, Packs,
    ZipChoice,
};
use maquette_core::pipeline::CompressDefaults;
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::ProviderJobs;
use maquette_core::{Asset, CoreError, WorkspaceConfig};
use tauri::{AppHandle, Manager as _, State};
use tauri_specta::Event as _;

use super::CommandResult;
use super::providers::Studio;
use crate::events::PackChanged;
use crate::session::Session;

/// The pack service the app shares between commands and the MCP tools.
pub struct PackJobs(pub Packs);

impl PackJobs {
    /// A service whose every change tells the web view.
    pub fn new(handle: AppHandle) -> Self {
        let runner = PipelineRunner::new(Arc::new(NullSink));
        Self(Packs::new(runner, move |record| {
            let event = PackChanged {
                record: record.clone(),
            };
            if let Err(err) = event.emit(&handle) {
                tracing::warn!(error = %err, "could not emit a pack event");
            }
        }))
    }
}

/// The workspace's compress settings, for a pack's web compression.
pub(crate) fn defaults(config: &WorkspaceConfig) -> CompressDefaults {
    CompressDefaults {
        texture_size: texture_px(config.compression.texture_size),
        codec: config.compression.texture_codec,
        meshopt: config.compression.meshopt,
    }
}

/// What `source` would hold; a project's sends say what made its models.
pub(crate) fn plan(
    session: &Session,
    jobs: &ProviderJobs,
    source: &PackSource,
    settings: &PackSettings,
) -> CommandResult<PackPlan> {
    let (root, config) = session.workspace()?;
    let sends = match source {
        PackSource::Project { project } => jobs.list(Some(project)),
        _ => Vec::new(),
    };
    let source_of = |dir: &Path, asset: &Asset, version: u32| -> (LibrarySource, Option<f64>) {
        library::detect_source(dir, asset, version, &sends)
    };
    Ok(maquette_core::packs::plan(
        &root,
        &config.layout,
        source,
        settings,
        &source_of,
    )?)
}

/// Plans and starts a pack. `from_anywhere` lets it write to a folder
/// outside the workspace (a folder a person picked).
pub(crate) fn start(
    session: &Session,
    jobs: &ProviderJobs,
    packs: &Packs,
    tools: Toolbox,
    request: &PackRequest,
    from_anywhere: bool,
) -> CommandResult<PackRecord> {
    let (root, config) = session.workspace()?;
    let planned = plan(session, jobs, &request.source, &request.settings)?;
    Ok(packs.start(
        &root,
        tools,
        defaults(&config),
        request,
        planned,
        from_anywhere,
    )?)
}

pub(crate) fn decide(
    session: &Session,
    packs: &Packs,
    tools: Toolbox,
    id: &str,
    decision: &PackDecision,
) -> CommandResult<PackRecord> {
    let (root, config) = session.workspace()?;
    Ok(packs.decide(&root, tools, defaults(&config), id, decision)?)
}

pub(crate) fn resume(
    session: &Session,
    packs: &Packs,
    tools: Toolbox,
    id: &str,
) -> CommandResult<PackRecord> {
    let (root, config) = session.workspace()?;
    Ok(packs.resume(&root, tools, defaults(&config), id)?)
}

/// What a pack made of `source` would hold, with the settings' budgets and
/// a size estimate. Reads only.
#[tauri::command]
#[specta::specta]
pub fn pack_plan(
    session: State<'_, Session>,
    studio: State<'_, Studio>,
    source: PackSource,
    settings: PackSettings,
) -> CommandResult<PackPlan> {
    plan(&session, &studio.0, &source, &settings)
}

/// The pack presets, the built in one first.
#[tauri::command]
#[specta::specta]
pub fn pack_presets(session: State<'_, Session>) -> CommandResult<Vec<PackPreset>> {
    let (root, _) = session.workspace()?;
    Ok(maquette_core::packs::presets(&root)?)
}

/// Saves the settings as a preset; answers every preset.
#[tauri::command]
#[specta::specta]
pub fn pack_save_preset(
    session: State<'_, Session>,
    name: String,
    settings: PackSettings,
) -> CommandResult<Vec<PackPreset>> {
    let (root, _) = session.workspace()?;
    Ok(maquette_core::packs::save_preset(&root, &name, &settings)?)
}

/// Starts a pack; its progress comes as `PackChanged` events.
#[tauri::command]
#[specta::specta]
pub async fn pack_start(
    app: AppHandle,
    session: State<'_, Session>,
    request: PackRequest,
) -> CommandResult<PackRecord> {
    let tools = super::toolchain::toolbox(&session).await?;
    tauri::async_runtime::spawn_blocking(move || {
        start(
            app.state::<Session>().inner(),
            &app.state::<Studio>().0,
            &app.state::<PackJobs>().0,
            tools,
            &request,
            true,
        )
    })
    .await
    .map_err(super::join_failed)?
}

/// One pack's record.
#[tauri::command(async)]
#[specta::specta]
pub fn pack_get(
    session: State<'_, Session>,
    packs: State<'_, PackJobs>,
    id: String,
) -> CommandResult<PackRecord> {
    let (root, _) = session.workspace()?;
    Ok(packs.0.get(&root, &id)?)
}

/// The newest packs first.
#[tauri::command(async)]
#[specta::specta]
pub fn pack_recent(
    session: State<'_, Session>,
    packs: State<'_, PackJobs>,
) -> CommandResult<Vec<PackRecord>> {
    let (root, _) = session.workspace()?;
    Ok(packs.0.recent(&root))
}

/// Raises a model's budget, keeps its warning or exports it again.
#[tauri::command]
#[specta::specta]
pub async fn pack_decide(
    session: State<'_, Session>,
    packs: State<'_, PackJobs>,
    id: String,
    decision: PackDecision,
) -> CommandResult<PackRecord> {
    let tools = super::toolchain::toolbox(&session).await?;
    decide(&session, &packs.0, tools, &id, &decision)
}

/// Writes the README, manifest and contact sheet and zips the pack, with
/// the models that pass or every one with files.
#[tauri::command]
#[specta::specta]
pub async fn pack_zip(app: AppHandle, id: String, choice: ZipChoice) -> CommandResult<PackRecord> {
    tauri::async_runtime::spawn_blocking(move || {
        let (root, _) = app.state::<Session>().workspace()?;
        Ok(app.state::<PackJobs>().0.zip(&root, &id, choice)?)
    })
    .await
    .map_err(super::join_failed)?
}

/// Stops a pack once the model running now is done.
#[tauri::command]
#[specta::specta]
pub fn pack_stop(packs: State<'_, PackJobs>, id: String) -> CommandResult<PackRecord> {
    Ok(packs.0.stop(&id)?)
}

/// Goes on with a stopped pack from the model it reached.
#[tauri::command]
#[specta::specta]
pub async fn pack_resume(
    session: State<'_, Session>,
    packs: State<'_, PackJobs>,
    id: String,
) -> CommandResult<PackRecord> {
    let tools = super::toolchain::toolbox(&session).await?;
    resume(&session, &packs.0, tools, &id)
}

/// Shows a pack's zip (or its folder, without one) in Finder or Explorer.
/// Only a pack's own files can be shown this way.
#[tauri::command]
#[specta::specta]
pub fn pack_reveal(
    session: State<'_, Session>,
    packs: State<'_, PackJobs>,
    id: String,
) -> CommandResult<()> {
    let (root, _) = session.workspace()?;
    let record = packs.0.get(&root, &id)?;
    let zip = record.zip.as_deref().map(Path::new).filter(|z| z.is_file());
    let target = zip.unwrap_or_else(|| Path::new(&record.dir));
    if !target.exists() {
        return Err(CoreError::NotFound(format!("the pack {}", record.name)).into());
    }
    crate::opener::reveal_file(target).map_err(|err| CoreError::io(target, err).payload())
}
