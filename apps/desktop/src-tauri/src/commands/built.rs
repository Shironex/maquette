//! Built assets: models a Blender script in the project makes. The Board
//! lists them with how they stand against their files, adds one and builds
//! it again; a full pipeline run rebuilds the stale ones first. Builds run
//! headless (`blender -b`), one at a time for the whole app.

use std::path::PathBuf;
use std::sync::LazyLock;

use maquette_core::built::{
    self, BuildContext, BuildOutcome, BuildResult, Builder, BuiltAsset, BuiltState, NewBuilt,
};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::thumbs::{ThumbTarget, Thumbnails};
use maquette_core::{Asset, ProjectManifest, project};
use serde::Serialize;
use tauri::{AppHandle, Manager as _, State};
use tauri_specta::Event as _;

use super::CommandResult;
use super::thumbs::{self, Thumbs};
use crate::events::{BuiltChanged, ProjectChanged};
use crate::session::Session;

/// Builds run one at a time for the whole app, app and agents alike.
static BUILDER: LazyLock<Builder> = LazyLock::new(Builder::default);

/// What Add built asset made.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BuiltAdded {
    pub asset: Asset,
    pub project: ProjectManifest,
}

/// Who hears about a build: the windows, or an agent's host.
pub(crate) trait Notify: Sync {
    fn project(&self, project: &ProjectManifest);
    fn built(&self, project: &str);
}

/// What a build needs besides the asset.
pub(crate) struct BuildEnv<'a> {
    pub session: &'a Session,
    pub thumbs: &'a Thumbnails,
    pub tools: &'a Toolbox,
    pub notify: &'a dyn Notify,
}

fn project_dir(session: &Session, project_id: &str) -> CommandResult<(PathBuf, ProjectManifest)> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    Ok((project::dir(&root, &config.layout, project_id)?, manifest))
}

pub(crate) fn list(session: &Session, project_id: &str) -> CommandResult<Vec<BuiltAsset>> {
    let (dir, manifest) = project_dir(session, project_id)?;
    Ok(built::list(&dir, &manifest, &BUILDER.busy(project_id))?)
}

/// Adds the asset and its files; it builds in [`rebuild`].
pub(crate) fn add(
    session: &Session,
    project_id: &str,
    request: &NewBuilt,
) -> CommandResult<BuiltAdded> {
    let (root, config) = session.workspace()?;
    let mut request = request.clone();
    // `~/...` works for the files, like the folder fields do.
    request.script = expand(session, &request.script);
    request.helpers = request.helpers.iter().map(|h| expand(session, h)).collect();
    let (asset, project) = built::add(&root, &config.layout, project_id, &request)?;
    Ok(BuiltAdded { asset, project })
}

fn expand(session: &Session, path: &str) -> String {
    if path.trim_start().starts_with('~') {
        session.expand(path.trim()).display().to_string()
    } else {
        path.to_string()
    }
}

/// Builds one asset (again with `force`), then asks for the new version's
/// thumbnail. Blocks until Blender is done.
pub(crate) fn rebuild(
    env: &BuildEnv<'_>,
    project_id: &str,
    asset_id: &str,
    force: bool,
) -> CommandResult<BuildOutcome> {
    let (root, config) = env.session.workspace()?;
    let ctx = BuildContext {
        workspace_root: &root,
        config: &config,
        project: project_id,
        tools: env.tools,
    };
    let notify = |m: &ProjectManifest| {
        env.notify.project(m);
        env.notify.built(project_id);
    };
    let mut outcome = BUILDER.build(&ctx, asset_id, force, &notify)?;
    if let Some(version) = &outcome.version {
        let file = project::dir(&root, &config.layout, project_id)?.join(&version.file);
        let target = ThumbTarget {
            project: project_id.to_string(),
            asset: outcome.asset.id.clone(),
            version: version.version,
        };
        if let Some(project) =
            thumbs::for_version(env.thumbs, &root, &config, env.tools, &file, target)
        {
            if let Some(asset) = project.assets.iter().find(|a| a.id == outcome.asset.id) {
                outcome.asset = asset.clone();
            }
            env.notify.project(&project);
            outcome.project = project;
        }
    }
    Ok(outcome)
}

/// Before a full run: rebuilds every new or stale built asset, so the run
/// builds the scene from what the scripts say. Refused when one is missing
/// a file or fails to build, naming it, since the scene would otherwise
/// ship an old model without a word.
pub(crate) fn before_run(env: &BuildEnv<'_>, project_id: &str) -> CommandResult<Vec<BuildOutcome>> {
    let all = list(env.session, project_id)?;
    let stuck = |b: &BuiltAsset| match b.state {
        BuiltState::Missing => Some(format!("{} is missing {}", b.name, b.missing.join(", "))),
        BuiltState::Failed => Some(format!(
            "{} did not build: {}",
            b.name,
            b.source
                .last
                .as_ref()
                .and_then(|l| l.error.clone())
                .unwrap_or_default()
        )),
        _ => None,
    };
    let mut problems: Vec<String> = all.iter().filter_map(stuck).collect();
    let mut done = Vec::new();
    for item in all
        .iter()
        .filter(|b| matches!(b.state, BuiltState::New | BuiltState::Stale))
    {
        let outcome = rebuild(env, project_id, &item.asset, false)?;
        if outcome.result == BuildResult::Failed {
            let error = outcome.record.as_ref().and_then(|r| r.error.clone());
            problems.push(format!(
                "{} did not build: {}",
                item.name,
                error.unwrap_or_default()
            ));
        }
        done.push(outcome);
    }
    if !problems.is_empty() {
        return Err(super::invalid(&format!(
            "The run did not start. {}. Fix the script in built/ and run again.",
            problems.join("; ")
        )));
    }
    Ok(done)
}

/// Tells the windows.
pub(crate) struct Windows(pub AppHandle);

impl Notify for Windows {
    fn project(&self, project: &ProjectManifest) {
        let event = ProjectChanged {
            project: project.clone(),
        };
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a project event");
        }
    }

    fn built(&self, project: &str) {
        let event = BuiltChanged {
            project: project.to_string(),
        };
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a built event");
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn built_list(session: State<'_, Session>, project: String) -> CommandResult<Vec<BuiltAsset>> {
    list(&session, &project)
}

/// Copies the script and helpers into the project's `built/` folder and
/// puts the asset on the Board at Blender. Nothing is built yet.
#[tauri::command]
#[specta::specta]
pub fn built_add(
    session: State<'_, Session>,
    project: String,
    request: NewBuilt,
) -> CommandResult<BuiltAdded> {
    add(&session, &project, &request)
}

/// Builds the asset headless. Without `force`, files that match the last
/// good build answer at once.
#[tauri::command]
#[specta::specta]
pub async fn built_rebuild(
    app: AppHandle,
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    project: String,
    asset: String,
    force: bool,
) -> CommandResult<BuildOutcome> {
    let tools = super::toolchain::toolbox(&session).await?;
    let thumbs = service.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session = app.state::<Session>();
        let env = BuildEnv {
            session: session.inner(),
            thumbs: &thumbs,
            tools: &tools,
            notify: &Windows(app.clone()),
        };
        rebuild(&env, &project, &asset, force)
    })
    .await
    .map_err(super::join_failed)?
}

#[cfg(test)]
mod tests;
