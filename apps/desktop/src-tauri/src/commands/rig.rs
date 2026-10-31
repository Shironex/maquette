//! The rig doctor: the shell side of `maquette_core::rig`. A person opens a
//! case from the Inbox check or an asset's menu; each change reaches the web
//! view as a `RigChanged` event. Only these commands approve a joint move or
//! land a draft: the MCP tools can start and read the doctor, not approve it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use maquette_core::inbox::{self, AcceptRequest};
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::providers::own_file::{self, OwnFile};
use maquette_core::rig::{
    RigChainEdit, RigDoctors, RigRecord, RigRoute, RigRun, RigSource, RigStarter, RigState,
    got_worse, waits_for_chains,
};
use maquette_core::{CoreError, assets, project};
use tauri::{AppHandle, State};
use tauri_specta::Event as _;

use super::providers::Studio;
use super::thumbs::Thumbs;
use super::{CommandResult, toolchain};
use crate::events::RigChanged;
use crate::mcp::hub::Hub;
use crate::session::Session;

/// The rig doctor service the app shares between commands and MCP tools.
pub struct RigJobs(pub RigDoctors, pub Arc<Hub>);

impl RigJobs {
    /// A service whose every change tells the web view. A fix an agent
    /// started asks a person for the version it would become.
    pub fn new(handle: AppHandle, hub: Arc<Hub>) -> Self {
        let asks = Arc::clone(&hub);
        let runner = Arc::new(PipelineRunner::new(Arc::new(NullSink)));
        let doctors = RigDoctors::new(runner, move |record: &RigRecord| {
            asks.ask_about_rig_draft(record);
            let event = RigChanged {
                record: record.clone(),
            };
            if let Err(err) = event.emit(&handle) {
                tracing::warn!(error = %err, "could not emit a rig doctor event");
            }
        });
        Self(doctors, hub)
    }
}

/// Which model a case is for: an Inbox item, or a version of an asset (its
/// newest without `version`).
#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigTarget {
    pub inbox: Option<String>,
    pub project: Option<String>,
    pub asset: Option<String>,
    pub version: Option<u32>,
}

/// Opens the case of `target`, as `starter`. Reads only: no Blender runs.
pub(crate) fn open(
    session: &Session,
    doctors: &RigDoctors,
    target: &RigTarget,
    starter: RigStarter,
) -> CommandResult<RigRecord> {
    let (root, config) = session.workspace()?;
    if let Some(id) = &target.inbox {
        let item = inbox::item(&root, &config, id)?;
        let source = RigSource {
            inbox: Some(id.clone()),
            project: None,
            asset: None,
            version: None,
        };
        return Ok(doctors.open(
            &root,
            &config,
            &item.name,
            Path::new(&item.file),
            source,
            starter,
        )?);
    }
    let (Some(project_id), Some(asset_id)) = (&target.project, &target.asset) else {
        return Err(super::invalid(
            "Give an inbox item, or a project and an asset.",
        ));
    };
    let manifest = project::load(&root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?;
    let chosen = match target.version {
        Some(v) => asset.versions.iter().find(|x| x.version == v),
        None => asset.latest(),
    }
    .ok_or_else(|| super::invalid(&format!("{} has no such version.", asset.name)))?;
    let file = project::dir(&root, &config.layout, project_id)?.join(&chosen.file);
    let source = RigSource {
        inbox: None,
        project: Some(project_id.clone()),
        asset: Some(asset.id.clone()),
        version: Some(chosen.version),
    };
    Ok(doctors.open(&root, &config, &asset.name, &file, source, starter)?)
}

/// Opens the case of a model. Reads only: no Blender runs.
#[tauri::command]
#[specta::specta]
pub fn rig_open(
    session: State<'_, Session>,
    rig: State<'_, RigJobs>,
    target: RigTarget,
) -> CommandResult<RigRecord> {
    open(&session, &rig.0, &target, RigStarter::Person)
}

/// Case `id`.
#[tauri::command]
#[specta::specta]
pub fn rig_get(
    session: State<'_, Session>,
    rig: State<'_, RigJobs>,
    id: String,
) -> CommandResult<RigRecord> {
    let (root, config) = session.workspace()?;
    Ok(rig.0.get(&root, &config, &id)?)
}

/// The cases on disk, newest first.
#[tauri::command]
#[specta::specta]
pub fn rig_list(session: State<'_, Session>) -> CommandResult<Vec<RigRecord>> {
    let (root, config) = session.workspace()?;
    Ok(maquette_core::rig::recent(&root, &config))
}

/// Accepts the case's Inbox model as v1 and goes on from that version. The
/// original stays as it came.
#[tauri::command]
#[specta::specta]
pub async fn rig_accept(
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    studio: State<'_, Studio>,
    rig: State<'_, RigJobs>,
    id: String,
    project: String,
    asset: Option<String>,
) -> CommandResult<RigRecord> {
    let (root, config) = session.workspace()?;
    let case = rig.0.get(&root, &config, &id)?;
    let item = case
        .source
        .inbox
        .clone()
        .ok_or_else(|| super::invalid("This case is not waiting in the Inbox."))?;
    let request = AcceptRequest {
        item,
        project: project.clone(),
        asset,
        name: None,
        prep: None,
    };
    let done = super::inbox::inbox_accept(session.clone(), service, studio, request).await?;
    let file = PathBuf::from(&done.file);
    Ok(rig.0.accepted(
        &root,
        &config,
        &id,
        &project,
        &done.asset.id,
        (done.version.version, &file),
    )?)
}

/// Starts the pose test, the fix or the second round; answers at once.
#[tauri::command]
#[specta::specta]
pub async fn rig_run(
    session: State<'_, Session>,
    rig: State<'_, RigJobs>,
    id: String,
    what: RigRun,
) -> CommandResult<RigRecord> {
    let (root, config) = session.workspace()?;
    let tools = toolchain::toolbox(&session).await?;
    Ok(rig.0.run(&root, &config, tools, &id, what)?)
}

/// Asks a running case to stop after the script it is in.
#[tauri::command]
#[specta::specta]
pub fn rig_cancel(rig: State<'_, RigJobs>, id: String) -> CommandResult<()> {
    Ok(rig.0.cancel(&id)?)
}

/// A person picks the other route. The case starts over.
#[tauri::command]
#[specta::specta]
pub fn rig_set_route(
    session: State<'_, Session>,
    rig: State<'_, RigJobs>,
    id: String,
    route: RigRoute,
) -> CommandResult<RigRecord> {
    let (root, config) = session.workspace()?;
    Ok(rig.0.set_route(&root, &config, &id, route)?)
}

/// A person's word on a creature's chains. With a correction the case starts
/// over, so an agent's card about the old draft closes; its next fix asks again.
#[tauri::command]
#[specta::specta]
pub fn rig_confirm_chains(
    session: State<'_, Session>,
    rig: State<'_, RigJobs>,
    id: String,
    edits: Vec<RigChainEdit>,
) -> CommandResult<RigRecord> {
    let (root, config) = session.workspace()?;
    let case = rig.0.confirm_chains(&root, &config, &id, edits)?;
    if case.state != RigState::Fixed {
        rig.1.close_rig_doctor(&id);
    }
    Ok(case)
}

/// A person's answer to the joint move.
#[tauri::command]
#[specta::specta]
pub fn rig_decide_joints(
    session: State<'_, Session>,
    rig: State<'_, RigJobs>,
    id: String,
    approve: bool,
) -> CommandResult<RigRecord> {
    let (root, config) = session.workspace()?;
    Ok(rig.0.decide_joints(&root, &config, &id, approve)?)
}

/// Sends the draft through the Inbox as the next version. It stops when a
/// check got worse; the version it came from stays either way.
#[tauri::command]
#[specta::specta]
pub async fn rig_land(
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    studio: State<'_, Studio>,
    pipelines: State<'_, super::pipeline::Pipelines>,
    rig: State<'_, RigJobs>,
    id: String,
) -> CommandResult<RigRecord> {
    let (root, config) = session.workspace()?;
    let case = rig.0.get(&root, &config, &id)?;
    let draft = case
        .draft
        .as_deref()
        .map(PathBuf::from)
        .filter(|_| matches!(case.state, RigState::Fixed))
        .ok_or_else(|| super::invalid("There is no finished draft to add."))?;
    if let Some(why) = waits_for_chains(&case) {
        return Err(super::invalid(why));
    }
    let (Some(project), Some(asset)) = (case.source.project.clone(), case.source.asset.clone())
    else {
        return Err(super::invalid("Accept the model as v1 first."));
    };
    if let Some(why) = got_worse(Path::new(&case.input), &draft)? {
        rig.1.close_rig_doctor(&id);
        return Ok(rig.0.held(&root, &config, &id, why)?);
    }
    let tools = toolchain::toolbox(&session).await?;
    let added = own_file::add(
        &studio.0,
        &pipelines.0,
        &tools,
        &OwnFile {
            root: &root,
            config: &config,
            project: &project,
            asset: &asset,
            file: &draft,
        },
    )?;
    let request = AcceptRequest {
        item: added.inbox_file,
        project,
        asset: Some(asset),
        name: None,
        prep: None,
    };
    let done = super::inbox::inbox_accept(session.clone(), service, studio, request).await?;
    rig.1.close_rig_doctor(&id);
    Ok(rig.0.landed(&root, &config, &id, done.version.version)?)
}

/// Drops the draft. The version it came from is as it was.
#[tauri::command]
#[specta::specta]
pub fn rig_discard(
    session: State<'_, Session>,
    rig: State<'_, RigJobs>,
    id: String,
) -> CommandResult<RigRecord> {
    let (root, config) = session.workspace()?;
    let found = rig.0.discard(&root, &config, &id)?;
    rig.1.close_rig_doctor(&id);
    Ok(found)
}

/// Copies the case's newest sheet to `exports/rig/` and shows it.
#[tauri::command]
#[specta::specta]
pub fn rig_save_sheet(
    session: State<'_, Session>,
    rig: State<'_, RigJobs>,
    id: String,
) -> CommandResult<String> {
    let (root, config) = session.workspace()?;
    let case = rig.0.get(&root, &config, &id)?;
    let sheet = case
        .after
        .as_ref()
        .or(case.before.as_ref())
        .map(|s| PathBuf::from(&s.file))
        .ok_or_else(|| super::invalid("Run the pose test first."))?;
    let dir = root.join("exports").join("rig");
    std::fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    let name = format!("{id}_sheet.png");
    let out = dir.join(name);
    std::fs::copy(&sheet, &out).map_err(|e| CoreError::io(&out, e))?;
    crate::opener::reveal_file(&out).map_err(|e| CoreError::io(&out, e).payload())?;
    Ok(out.display().to_string())
}

/// Opens the case's draft, or else its model, in Blender's window.
#[tauri::command]
#[specta::specta]
pub async fn rig_open_blender(
    session: State<'_, Session>,
    rig: State<'_, RigJobs>,
    id: String,
) -> CommandResult<()> {
    let (root, config) = session.workspace()?;
    let case = rig.0.get(&root, &config, &id)?;
    let file = case.draft.unwrap_or(case.input);
    super::blender::open_gui(&session, Path::new(&file)).await
}
