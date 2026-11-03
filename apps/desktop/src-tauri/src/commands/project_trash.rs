//! Remove project and Removed projects: a whole project into the workspace
//! trash and back. Only the app does this; no MCP tool removes a project.
//!
//! The rule for work going on in the project: anything that could lose
//! work or money refuses the remove with the reason (a pipeline run, a
//! build, concept images on their way, any send that is not over: waiting
//! for approval, queued, being made or waiting for a By hand model). The
//! person cancels or waits, then removes. Nothing is cancelled for them, and
//! `providers.json` is never touched, so every paid send keeps its record
//! and a restore finds it again. What is safe to stop is stopped: the site's
//! dev server when it serves this project, and the canvas file watchers.

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::project::trash::{self, RemovedProject};
use maquette_core::providers::ProviderJobs;
use maquette_core::site::preview::{ServerState, SitePreview};
use maquette_core::{ProjectSummary, project};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::app::{WorkspaceSnapshot, snapshot};
use super::built::building;
use super::lab::Lab;
use super::pipeline::Pipelines;
use super::plan_watch::PlanWatch;
use super::providers::Studio;
use super::site::Preview;
use super::{CommandResult, invalid};
use crate::session::Session;

/// What Remove project did: the project as the trash keeps it, and the
/// workspace after it, open on another project or on none.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRemoved {
    pub removed: RemovedProject,
    pub workspace: WorkspaceSnapshot,
}

/// What runs in a project, as far as removing it cares.
pub(crate) struct Running<'a> {
    pub runner: &'a PipelineRunner,
    pub lab: &'a Concepts,
    pub studio: &'a ProviderJobs,
    pub building: Vec<String>,
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// How much runs in a project now.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Busy {
    pub run: bool,
    pub building: usize,
    pub concepts: usize,
    pub sends: usize,
}

impl Running<'_> {
    fn busy(&self, id: &str) -> Busy {
        let (active, queued) = self.runner.active(id);
        Busy {
            run: active.is_some() || !queued.is_empty(),
            building: self.building.len(),
            concepts: self
                .lab
                .jobs(Some(id))
                .iter()
                .filter(|j| !j.state.is_finished())
                .count(),
            sends: self
                .studio
                .list(Some(id))
                .iter()
                .filter(|j| j.state.is_active())
                .count(),
        }
    }
}

/// Why project `name` cannot be removed now, or None when nothing runs in it.
pub(crate) fn refusal(busy: Busy, name: &str) -> Option<String> {
    if busy.run {
        return Some(format!(
            "A pipeline run is going in {name}. Wait for it or cancel it in Pipeline, then remove the project."
        ));
    }
    if busy.building > 0 {
        return Some(format!(
            "{} building in {name}. Wait for the build to finish, then remove the project.",
            plural(busy.building, "built asset is", "built assets are")
        ));
    }
    if busy.concepts > 0 {
        return Some(format!(
            "{} queued or generating in {name}. Cancel them in the Lab or let them finish, then remove the project.",
            plural(busy.concepts, "concept image is", "concept images are")
        ));
    }
    if busy.sends > 0 {
        return Some(format!(
            "{} not over in {name}: waiting for approval, queued, being made or waiting for a By hand model. Cancel or stop them in Sends, or wait for the Inbox, then remove the project. Their records stay either way.",
            plural(busy.sends, "send is", "sends are")
        ));
    }
    None
}

/// The project the app opens when `id` goes: the next one in the switcher,
/// else the one before it, else none.
pub(crate) fn next_open(rows: &[ProjectSummary], id: &str) -> Option<String> {
    let at = rows.iter().position(|r| r.id == id)?;
    rows.get(at + 1)
        .or_else(|| at.checked_sub(1).and_then(|i| rows.get(i)))
        .map(|r| r.id.clone())
}

/// Stops the site's dev server when it serves `id`; the repo is not touched.
fn stop_preview(preview: &SitePreview, id: &str) {
    let status = preview.status();
    if status.project.as_deref() == Some(id) && status.state != ServerState::Idle {
        preview.stop();
    }
}

/// Removes `id` into the workspace trash once nothing runs in it, and
/// opens another project when it was the open one.
pub(crate) fn remove(
    session: &Session,
    running: &Running<'_>,
    preview: Option<&SitePreview>,
    watching: Option<&PlanWatch>,
    id: &str,
) -> CommandResult<ProjectRemoved> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, id)?;
    if let Some(why) = refusal(running.busy(id), &manifest.name) {
        return Err(invalid(&why));
    }
    if let Some(preview) = preview {
        stop_preview(preview, id);
    }
    let dir = project::dir(&root, &config.layout, id)?;
    if let Some(watching) = watching {
        watching.forget_under(&dir);
    }
    let rows = project::summaries(&root, &config.layout)?;
    let removed = trash::remove_project(&root, &config.layout, id)?;
    let (root, config) = if config.current_project.as_deref() == Some(id) {
        let next = next_open(&rows, id);
        session.update_config(|c| c.current_project = next)?
    } else {
        (root, config)
    };
    Ok(ProjectRemoved {
        removed,
        workspace: snapshot(session, &root, &config)?,
    })
}

/// Puts the removed project `trash` back and opens it.
pub(crate) fn restore(session: &Session, trash_id: &str) -> CommandResult<WorkspaceSnapshot> {
    let (root, config) = session.workspace()?;
    let id = trash::restore_project(&root, &config.layout, trash_id)?;
    session.save_repos(&root)?;
    let (root, config) = session.update_config(|c| c.current_project = Some(id))?;
    snapshot(session, &root, &config)
}

/// Moves a project to the workspace trash. Refused while work runs in it.
#[tauri::command]
#[specta::specta]
pub async fn project_remove(app: AppHandle, id: String) -> CommandResult<ProjectRemoved> {
    tauri::async_runtime::spawn_blocking(move || {
        let running = Running {
            runner: &app.state::<Pipelines>().0,
            lab: &app.state::<Lab>().0,
            studio: &app.state::<Studio>().0,
            building: building(&id),
        };
        let preview = app
            .try_state::<Preview>()
            .map(|p| std::sync::Arc::clone(&p.0));
        let watching = app.try_state::<PlanWatch>();
        remove(
            &app.state::<Session>(),
            &running,
            preview.as_deref(),
            watching.as_deref(),
            &id,
        )
    })
    .await
    .map_err(super::join_failed)?
}

/// The projects in the workspace trash, newest first.
#[tauri::command(async)]
#[specta::specta]
pub fn project_removed_list(
    session: tauri::State<'_, Session>,
) -> CommandResult<Vec<RemovedProject>> {
    let (root, _) = session.workspace()?;
    Ok(trash::list_removed(&root)?)
}

/// Puts a removed project back and opens it.
#[tauri::command]
#[specta::specta]
pub async fn project_restore(app: AppHandle, trash: String) -> CommandResult<WorkspaceSnapshot> {
    tauri::async_runtime::spawn_blocking(move || restore(&app.state::<Session>(), &trash))
        .await
        .map_err(super::join_failed)?
}

#[cfg(test)]
mod tests;
