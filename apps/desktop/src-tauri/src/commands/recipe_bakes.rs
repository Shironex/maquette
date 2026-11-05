//! Recipe bakes: the shell side of `maquette_core::animations::recipes::bake`.
//! Apply in the Motion view, or an agent's bake_recipe, starts one headless
//! Blender run per recipe; each change reaches the web view as a
//! `RecipeBakeChanged` event. The result is staged in the workspace cache
//! until a person reviews it and adds it as the model's next version, on the
//! same route the rig doctor's Add as vN takes.

use std::sync::Arc;

use maquette_core::animations::recipes::bake::{
    RecipeBakeRecord, RecipeBakeRequest, RecipeBakeStarter, RecipeBakes, ready_to_add, record_of,
};
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::providers::ledger::Decision;
use tauri::{AppHandle, State};
use tauri_specta::Event as _;

use super::inbox::{NextVersionFrom, add_as_next_version};
use super::providers::Studio;
use super::thumbs::Thumbs;
use super::{CommandResult, toolchain};
use crate::events::RecipeBakeChanged;
use crate::mcp::hub::Hub;
use crate::session::Session;

/// The recipe bake service the app shares between commands and MCP tools.
pub struct RecipeBakeJobs(pub RecipeBakes, pub Arc<Hub>);

impl RecipeBakeJobs {
    /// A service whose every change tells the web view. A new bake of a
    /// recipe ends any approval the one before it waited for.
    pub fn new(handle: AppHandle, hub: Arc<Hub>) -> Self {
        let runner = Arc::new(PipelineRunner::new(Arc::new(NullSink)));
        let asks = Arc::clone(&hub);
        let bakes = RecipeBakes::new(runner, move |record: &RecipeBakeRecord| {
            if record.running() {
                asks.close_recipe_bake(&record.project, &record.recipe, Decision::Cancelled);
            }
            let event = RecipeBakeChanged {
                record: record.clone(),
            };
            if let Err(err) = event.emit(&handle) {
                tracing::warn!(error = %err, "could not emit a recipe bake event");
            }
        });
        Self(bakes, hub)
    }
}

/// The bake of `recipe` in `project`, read through the service when there
/// is one (a bake the app closed on then reads as failed), else from disk.
pub(crate) fn get(
    session: &Session,
    bakes: Option<&RecipeBakes>,
    project: &str,
    recipe: &str,
) -> CommandResult<Option<RecipeBakeRecord>> {
    let (root, config) = session.workspace()?;
    Ok(match bakes {
        Some(bakes) => bakes.get(&root, &config, project, recipe)?,
        None => record_of(&root, &config, project, recipe)?,
    })
}

/// Starts a bake as `starter`; answers at once with its record.
pub(crate) fn start(
    session: &Session,
    bakes: &RecipeBakes,
    tools: maquette_core::pipeline::tools::Toolbox,
    request: &RecipeBakeRequest,
    starter: RecipeBakeStarter,
) -> CommandResult<RecipeBakeRecord> {
    let (root, config) = session.workspace()?;
    Ok(bakes.start(&root, &config, tools, request, starter)?)
}

/// Bakes a recipe onto a rigged model of its project, headless: the base
/// clip through the retarget with the recipe's changes solved over it. The
/// model goes to the workspace cache, staged for a review.
#[tauri::command]
#[specta::specta]
pub async fn recipe_bake_start(
    session: State<'_, Session>,
    bakes: State<'_, RecipeBakeJobs>,
    request: RecipeBakeRequest,
) -> CommandResult<RecipeBakeRecord> {
    let tools = toolchain::toolbox(&session).await?;
    start(
        &session,
        &bakes.0,
        tools,
        &request,
        RecipeBakeStarter::Person,
    )
}

/// The recipe's last bake, or none when it was never baked.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_bake_get(
    session: State<'_, Session>,
    bakes: State<'_, RecipeBakeJobs>,
    project: String,
    recipe: String,
) -> CommandResult<Option<RecipeBakeRecord>> {
    get(&session, Some(&bakes.0), &project, &recipe)
}

/// Stops a running bake: Blender is killed and nothing is staged.
#[tauri::command(async)]
#[specta::specta]
pub fn recipe_bake_cancel(
    bakes: State<'_, RecipeBakeJobs>,
    project: String,
    recipe: String,
) -> CommandResult<()> {
    Ok(bakes.0.cancel(&project, &recipe)?)
}

/// Adds a reviewed bake as its model's next version, on the route the rig
/// doctor's Add as vN takes: into the Inbox for the asset, then accepted.
/// A bake that is out of date is refused. The new version is recorded as
/// the recipe's (and Mixamo's, for a Mixamo base), the staged model is
/// cleared and an agent's approval for it ends as approved.
#[tauri::command]
#[specta::specta]
pub async fn recipe_bake_add(
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    studio: State<'_, Studio>,
    pipelines: State<'_, super::pipeline::Pipelines>,
    bakes: State<'_, RecipeBakeJobs>,
    project: String,
    recipe: String,
) -> CommandResult<RecipeBakeRecord> {
    let (root, config) = session.workspace()?;
    let (ready, staged) = ready_to_add(&root, &config, &project, &recipe)?;
    let done = add_as_next_version(
        session,
        service,
        studio,
        pipelines,
        (project.clone(), ready.asset.clone()),
        &staged,
        NextVersionFrom::StagedBake,
    )
    .await?;
    let added = bakes
        .0
        .added(&root, &config, &project, &recipe, done.version.version)?;
    bakes
        .1
        .close_recipe_bake(&project, &recipe, Decision::Approved);
    Ok(added)
}
