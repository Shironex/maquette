//! The Avatar view: one character asset's VRM export. The spec lives in the
//! project's `avatar.json`; a build normalises the rest pose headless when
//! it is not VRM's yet, waits for a person's yes to the new pose, then
//! writes `exports/<asset>.vrm`. When cloth or hair hangs on the arms, it
//! first frees them and waits for a yes to the new weights. The yes or no
//! is only ever given here, in the app: agents build, and wait.

use maquette_core::avatar::export::{self, AvatarBuild, AvatarPlan, AvatarRequest, AvatarView};
use maquette_core::avatar::normalize::{Elbows, Normalized};
use maquette_core::avatar::reweight::{ReweightMode, Reweighted};
use maquette_core::avatar::{self, AvatarSpec};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::project;
use tauri::{AppHandle, Manager as _, State};

use super::CommandResult;
use super::pipeline::Pipelines;
use crate::mcp::Agents;
use crate::session::Session;

/// Which asset (and version) a call is about.
pub(crate) struct Target<'a> {
    pub project: &'a str,
    pub asset: &'a str,
    pub version: Option<u32>,
    pub elbows: Elbows,
    pub reweight: ReweightMode,
}

/// Runs `then` on the request a target names in the open workspace.
fn with_request<T>(
    session: &Session,
    target: &Target<'_>,
    then: impl FnOnce(&AvatarRequest<'_>) -> maquette_core::CoreResult<T>,
) -> CommandResult<T> {
    let (root, config) = session.workspace()?;
    let req = AvatarRequest {
        workspace_root: &root,
        config: &config,
        project: target.project,
        asset: target.asset,
        version: target.version,
        elbows: target.elbows,
        reweight: target.reweight,
    };
    Ok(then(&req)?)
}

pub(crate) fn view(session: &Session, target: &Target<'_>) -> CommandResult<AvatarView> {
    with_request(session, target, export::view)
}

pub(crate) fn plan(session: &Session, target: &Target<'_>) -> CommandResult<AvatarPlan> {
    with_request(session, target, export::plan)
}

/// Builds the avatar. Blocks while Blender runs.
pub(crate) fn build(
    session: &Session,
    runner: &dyn maquette_core::base::ports::ScriptRunner,
    tools: &Toolbox,
    target: &Target<'_>,
) -> CommandResult<AvatarBuild> {
    with_request(session, target, |req| export::build(runner, tools, req))
}

/// Stores `spec` as the asset's avatar, checked, and reads the view again.
pub(crate) fn save(
    session: &Session,
    target: &Target<'_>,
    spec: AvatarSpec,
) -> CommandResult<AvatarView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, target.project)?;
    let asset = maquette_core::assets::find(&manifest, target.asset)?;
    let dir = project::dir(&root, &config.layout, target.project)?;
    avatar::set(&dir, &asset.id, Some(spec))?;
    view(session, target)
}

/// One character asset's avatar: the spec (or the workspace defaults), the
/// readiness, the last normalised rest pose and the last export.
#[tauri::command]
#[specta::specta]
pub fn avatar_get(
    session: State<'_, Session>,
    project: String,
    asset: String,
    version: Option<u32>,
) -> CommandResult<AvatarView> {
    view(
        &session,
        &Target {
            project: &project,
            asset: &asset,
            version,
            elbows: Elbows::default(),
            reweight: ReweightMode::default(),
        },
    )
}

/// Saves the asset's avatar spec (meta, bone overrides, material mode).
#[tauri::command]
#[specta::specta]
pub fn avatar_save(
    session: State<'_, Session>,
    project: String,
    asset: String,
    spec: AvatarSpec,
) -> CommandResult<AvatarView> {
    save(
        &session,
        &Target {
            project: &project,
            asset: &asset,
            version: None,
            elbows: Elbows::default(),
            reweight: ReweightMode::default(),
        },
        spec,
    )
}

/// Builds the avatar's `.vrm`: frees cloth and hair from the arms when they
/// hang there (unless `reweight` is off) and waits for that yes, normalises
/// the rest pose headless when it is not VRM's yet and waits for the yes,
/// or writes the file once both are given.
#[tauri::command]
#[specta::specta]
pub async fn avatar_build(
    app: AppHandle,
    session: State<'_, Session>,
    runner: State<'_, Pipelines>,
    project: String,
    asset: String,
    version: Option<u32>,
    elbows: Option<Elbows>,
    reweight: Option<ReweightMode>,
) -> CommandResult<AvatarBuild> {
    let tools = super::toolchain::toolbox(&session).await?;
    let runner = runner.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        build(
            app.state::<Session>().inner(),
            &runner,
            &tools,
            &Target {
                project: &project,
                asset: &asset,
                version,
                elbows: elbows.unwrap_or_default(),
                reweight: reweight.unwrap_or_default(),
            },
        )
    })
    .await
    .map_err(super::join_failed)?
}

/// A person's yes or no to a version's new rest pose. Not an agent tool:
/// only the app's approval card calls it. An agent's waiting request for
/// that asset is settled with it.
#[tauri::command]
#[specta::specta]
pub fn avatar_decide(
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    project: String,
    asset: String,
    version: Option<u32>,
    approve: bool,
) -> CommandResult<Normalized> {
    let target = Target {
        project: &project,
        asset: &asset,
        version,
        elbows: Elbows::default(),
        reweight: ReweightMode::default(),
    };
    let decided = with_request(&session, &target, |req| export::decide(req, approve))?;
    if let Some(waiting) = agents.hub.rest_pose_for(&decided.project, &decided.asset) {
        agents.hub.decide(&waiting.id, approve);
    }
    Ok(decided)
}

/// A person's yes or no to a version's new weights, which free its cloth
/// and hair from the arms. Not an agent tool: only the app's approval card
/// calls it. An agent's waiting request for that asset is settled with it.
#[tauri::command]
#[specta::specta]
pub fn avatar_decide_reweight(
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    project: String,
    asset: String,
    version: Option<u32>,
    approve: bool,
) -> CommandResult<Reweighted> {
    let target = Target {
        project: &project,
        asset: &asset,
        version,
        elbows: Elbows::default(),
        reweight: ReweightMode::default(),
    };
    let decided = with_request(&session, &target, |req| {
        export::decide_reweight(req, approve)
    })?;
    if let Some(waiting) = agents.hub.reweight_for(&decided.project, &decided.asset) {
        agents.hub.decide(&waiting.id, approve);
    }
    Ok(decided)
}

#[cfg(test)]
mod tests;
