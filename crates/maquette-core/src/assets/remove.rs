//! Removing Board assets from a project, and putting them back.
//!
//! A removed asset leaves `project.json` and every file that is its own
//! moves into the workspace trash, `<workspace>/trash/<project>/<asset>-<when>/`:
//! its refs, model versions, thumbnails, By hand handoff folders and built
//! scripts under `files/` at the paths they had in the project, with
//! `removed.json` beside them holding the asset entry, those paths and what
//! the project's other files said about it (its placement, camera stops,
//! path ends, object states, clips, explode track, real size and fix-ups,
//! material tweaks, built source and game model). Nothing is deleted, so
//! [`restore`] can put it all back.
//!
//! Sends waiting for a model (By hand, or a paid send still waiting for
//! approval or its turn) are stopped first, the way the 3D queue's Cancel
//! and Stop waiting do. A paid send that is running refuses the remove, so
//! no credits are lost to a model with nowhere to land, and so does a
//! pipeline run going in the project.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::Asset;
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest};
use crate::providers::{JobState, ProviderJob, ProviderJobs};
use crate::workspace::FolderLayout;

mod files;
mod links;
mod restore;
mod trash;

#[cfg(test)]
mod tests;

pub use restore::{AssetRestored, restore};
pub use trash::{TRASH_DIR, TrashEntry, list_trash};

/// The most assets one remove takes.
pub const MAX_REMOVE: usize = 50;

/// What else is going on in the project, which a remove has to wait for.
#[derive(Debug, Clone, Copy, Default)]
pub struct Busy<'a> {
    /// The pipeline run going in the project, if one is.
    pub run: Option<&'a str>,
    /// The built assets building in Blender now.
    pub building: &'a [String],
}

/// The sends a remove looks at and stops: the app's provider jobs, or a
/// stand in for tests.
pub trait Sends {
    /// Every send of `project`, as the 3D queue lists them.
    fn jobs(&self, project: &str) -> Vec<ProviderJob>;
    /// Cancels a send or stops waiting for its model.
    fn stop(&self, id: &str) -> CoreResult<ProviderJob>;
}

impl Sends for ProviderJobs {
    fn jobs(&self, project: &str) -> Vec<ProviderJob> {
        self.list(Some(project))
    }

    fn stop(&self, id: &str) -> CoreResult<ProviderJob> {
        self.cancel(id)
    }
}

/// How the remove of one asset went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RemoveState {
    Removed,
    /// The project has no asset with that id; nothing changed.
    NotFound,
    /// Something is still working on it; `reason` says what to do.
    Refused,
}

/// One asset of a remove.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoveOutcome {
    /// The asset id asked for.
    pub asset: String,
    /// Its name on the Board, when it was there.
    pub name: Option<String>,
    pub state: RemoveState,
    /// Why it was refused.
    pub reason: Option<String>,
    /// The trash id [`restore`] takes.
    pub trash: Option<String>,
    /// The trash folder, relative to the workspace, with `/` separators.
    pub trash_path: Option<String>,
    /// The files and folders moved to the trash.
    pub files: u32,
    /// The sends that were stopped, by job id.
    pub stopped: Vec<String>,
    /// What could not be cleaned up; the asset is removed all the same.
    pub warnings: Vec<String>,
}

impl RemoveOutcome {
    fn new(asset: &str, name: Option<&str>, state: RemoveState) -> Self {
        Self {
            asset: asset.to_string(),
            name: name.map(str::to_string),
            state,
            reason: None,
            trash: None,
            trash_path: None,
            files: 0,
            stopped: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn refused(asset: &Asset, reason: impl Into<String>) -> Self {
        Self {
            reason: Some(reason.into()),
            ..Self::new(&asset.id, Some(&asset.name), RemoveState::Refused)
        }
    }
}

/// What a remove did, with the project as it is now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetsRemoved {
    pub project: ProjectManifest,
    /// One per asset asked for, in the order asked.
    pub outcomes: Vec<RemoveOutcome>,
}

/// Where a remove or restore works.
pub(crate) struct Place<'a> {
    pub root: &'a Path,
    pub layout: &'a FolderLayout,
    pub project: &'a str,
    pub dir: std::path::PathBuf,
}

impl<'a> Place<'a> {
    pub(crate) fn new(
        root: &'a Path,
        layout: &'a FolderLayout,
        project: &'a str,
    ) -> CoreResult<Self> {
        let dir = project::dir(root, layout, project)?;
        Ok(Self {
            root,
            layout,
            project,
            dir,
        })
    }
}

pub(crate) fn refuse_while_running(manifest: &ProjectManifest, busy: &Busy<'_>) -> CoreResult<()> {
    match busy.run {
        Some(run) => Err(CoreError::Invalid(format!(
            "A pipeline run ({run}) is going in {}. Wait for it to finish or cancel it, then try again.",
            manifest.name
        ))),
        None => Ok(()),
    }
}

/// Removes the assets `ids` from the project `project_id`, each on its own:
/// one that is not there answers not found and one that is still being
/// made answers refused, and neither stops the others.
///
/// # Errors
///
/// Refuses the whole call, changing nothing, when it names no asset or more
/// than [`MAX_REMOVE`], when a pipeline run is going in the project, or
/// when the project cannot be read.
pub fn remove_assets(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    ids: &[String],
    busy: &Busy<'_>,
    sends: &dyn Sends,
) -> CoreResult<AssetsRemoved> {
    if ids.is_empty() {
        return Err(CoreError::Invalid(
            "Name at least one asset to remove.".to_string(),
        ));
    }
    if ids.len() > MAX_REMOVE {
        return Err(CoreError::Invalid(format!(
            "Remove at most {MAX_REMOVE} assets at a time."
        )));
    }
    let place = Place::new(root, layout, project_id)?;
    let manifest = project::load(root, layout, project_id)?;
    refuse_while_running(&manifest, busy)?;
    let mut outcomes: Vec<RemoveOutcome> = Vec::new();
    for id in ids {
        if outcomes.iter().any(|o| &o.asset == id) {
            continue;
        }
        outcomes.push(remove_one(&place, id, busy, sends));
    }
    Ok(AssetsRemoved {
        project: project::load(root, layout, project_id)?,
        outcomes,
    })
}

/// Why a send refuses the remove: it is being made and paid for now.
fn running_reason(asset: &Asset, job: &ProviderJob) -> String {
    format!(
        "{} is making the model of {} now (send {}). Wait for it to reach the Inbox, or cancel it in the 3D queue, then remove the asset.",
        job.provider.label(),
        asset.name,
        job.id
    )
}

fn remove_one(place: &Place<'_>, id: &str, busy: &Busy<'_>, sends: &dyn Sends) -> RemoveOutcome {
    let manifest = match project::load(place.root, place.layout, place.project) {
        Ok(m) => m,
        Err(err) => {
            let mut out = RemoveOutcome::new(id, None, RemoveState::Refused);
            out.reason = Some(err.to_string());
            return out;
        }
    };
    let Ok(asset) = super::find(&manifest, id).cloned() else {
        return RemoveOutcome::new(id, None, RemoveState::NotFound);
    };
    if busy.building.iter().any(|b| b == id) {
        return RemoveOutcome::refused(
            &asset,
            format!(
                "{} is building in Blender now. Remove it once the build is done.",
                asset.name
            ),
        );
    }
    let jobs = sends.jobs(place.project);
    let mine: Vec<&ProviderJob> = jobs.iter().filter(|j| j.asset == asset.id).collect();
    if let Some(running) = mine.iter().find(|j| j.state == JobState::Running) {
        return RemoveOutcome::refused(&asset, running_reason(&asset, running));
    }
    let mut stopped = Vec::new();
    for job in mine.iter().filter(|j| {
        matches!(
            j.state,
            JobState::Approval | JobState::Queued | JobState::Waiting
        )
    }) {
        if let Err(err) = sends.stop(&job.id) {
            let mut out = RemoveOutcome::refused(
                &asset,
                format!("Could not stop the send {}. {err}", job.id),
            );
            out.stopped = stopped;
            return out;
        }
        stopped.push(job.id.clone());
    }
    let mut out = match trash::move_to_trash(place, &manifest, &asset, &jobs, &stopped) {
        Ok(out) => out,
        Err(err) => RemoveOutcome::refused(&asset, err.to_string()),
    };
    out.stopped = stopped;
    out
}
