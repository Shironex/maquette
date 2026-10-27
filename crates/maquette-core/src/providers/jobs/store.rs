//! Saving and loading `providers.json`, changing one job, and moving its
//! asset on the Board.

use std::fs;
use std::io;
use std::path::Path;

use super::spend;
use super::{
    CLOSED_WHILE_SENDING, Entry, FILE_VERSION, JOBS_FILE, JobState, JobsFile, KEEP_FINISHED,
    ProviderEvent, Shared, Stored, snapshot,
};
use crate::assets::{self, AssetStatus, Stage};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::pipeline::history::now_ms;
use crate::project;

/// Fixes a saved job up for a fresh start. True when it changed.
pub(super) fn resume_fixup(stored: &mut Stored) -> bool {
    let job = &mut stored.job;
    match job.state {
        JobState::Approval => {
            job.state = JobState::Failed;
            job.message = Some(
                "Maquette closed before you approved this send. Nothing was spent.".to_string(),
            );
            job.finished_at_ms = Some(now_ms());
            true
        }
        JobState::Running if stored.chain.sending && stored.chain.model_task.is_none() => {
            job.state = JobState::Failed;
            job.message = Some(CLOSED_WHILE_SENDING.to_string());
            job.finished_at_ms = Some(now_ms());
            true
        }
        JobState::Running => {
            // Queued again; the chain says where to pick up.
            job.state = JobState::Queued;
            job.message = Some(if stored.chain.model_task.is_some() {
                "Picking up where it left off".to_string()
            } else {
                "Starting again".to_string()
            });
            true
        }
        _ => false,
    }
}

/// A project's saved jobs, with each By hand folder as a path on this
/// computer.
/// True as well when a folder was saved the older way, as an absolute
/// path, so the file is written again in the new form.
pub(super) fn load(root: &Path, project_dir: &Path) -> (Vec<Stored>, bool) {
    let mut jobs = read(project_dir);
    let mut older = false;
    for stored in &mut jobs {
        let job = &mut stored.job;
        if let Some(recorded) = job.handoff.take() {
            older |= crate::fs::looks_absolute(&recorded);
            job.handoff = Some(handoff_here(root, project_dir, &job.id, &recorded));
        }
    }
    (jobs, older)
}

/// Where a recorded handoff folder is on this computer. Files save it
/// relative to the workspace; an older file's absolute path is used as it
/// is when it is inside this workspace or still there, and otherwise (a
/// path from another computer) the job's own folder stands in for it when
/// that is here.
fn handoff_here(root: &Path, project_dir: &Path, job: &str, recorded: &str) -> String {
    let path = crate::fs::from_workspace(root, recorded);
    if !crate::fs::looks_absolute(recorded) || path.starts_with(root) || path.is_dir() {
        return path.display().to_string();
    }
    let own = project_dir
        .join(super::super::handoff::HANDOFF_DIR)
        .join(job);
    if own.is_dir() { own } else { path }.display().to_string()
}

fn read(project_dir: &Path) -> Vec<Stored> {
    let path = project_dir.join(JOBS_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            tracing::warn!(error = %err, path = %path.display(), "could not read the provider jobs");
            return Vec::new();
        }
    };
    match serde_json::from_str::<JobsFile>(&text) {
        Ok(file) if file.version <= FILE_VERSION => file.jobs,
        Ok(_) => {
            tracing::warn!(path = %path.display(), "provider jobs from a newer Maquette, ignored");
            Vec::new()
        }
        Err(err) => {
            tracing::warn!(error = %err, path = %path.display(), "unreadable provider jobs");
            Vec::new()
        }
    }
}

/// Writes a project's jobs. Keeps the newest finished ones only.
pub(super) fn persist(shared: &Shared, project: &str) {
    let Some(workspace) = shared.workspace() else {
        return;
    };
    let Ok(dir) = workspace.project_dir(project) else {
        return;
    };
    let jobs = shared.jobs();
    write_project(&workspace.root, &dir, &jobs, project);
}

fn write_project(root: &Path, dir: &Path, jobs: &[Entry], project: &str) {
    let mut mine: Vec<Stored> = jobs
        .iter()
        .filter(|e| e.stored.job.project == project)
        .map(|e| e.stored.clone())
        .collect();
    // Saved relative to the workspace, so the file reads on every computer.
    for stored in &mut mine {
        if let Some(folder) = stored.job.handoff.as_deref() {
            stored.job.handoff = Some(crate::fs::to_workspace(root, Path::new(folder)));
        }
    }
    let finished = mine.iter().filter(|s| !s.job.state.is_active()).count();
    if finished > KEEP_FINISHED {
        let mut drop = finished - KEEP_FINISHED;
        mine.retain(|s| {
            if drop > 0 && !s.job.state.is_active() && s.job.state != JobState::Inbox {
                drop -= 1;
                false
            } else {
                true
            }
        });
    }
    let file = JobsFile {
        version: FILE_VERSION,
        jobs: mine,
    };
    if let Err(err) = write_json(&dir.join(JOBS_FILE), &file) {
        tracing::warn!(error = %err, "could not save the provider jobs");
    }
}

/// Changes one job, saves it when `save` and tells the UI.
pub(super) fn update(
    shared: &Shared,
    id: &str,
    save: bool,
    change: impl FnOnce(&mut Stored) -> CoreResult<()>,
) -> CoreResult<()> {
    let (job, project, before, after) = {
        let mut jobs = shared.jobs();
        let entry = jobs
            .iter_mut()
            .find(|e| e.stored.job.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
        let before = spend::before(&entry.stored);
        change(&mut entry.stored)?;
        (
            snapshot(&entry.stored),
            entry.stored.job.project.clone(),
            before,
            entry.stored.clone(),
        )
    };
    if save {
        persist(shared, &project);
    }
    shared.emit(ProviderEvent::Job { job: Box::new(job) });
    spend::record(shared, &before, &after);
    Ok(())
}

pub(super) fn touch_asset(
    shared: &Shared,
    project: &str,
    asset: &str,
    stage: Option<Stage>,
    status: AssetStatus,
) {
    let Some(workspace) = shared.workspace() else {
        return;
    };
    let changed = project::modify(&workspace.root, &workspace.config.layout, project, |m| {
        if let Some(stage) = stage
            && assets::find(m, asset)?.stage < stage
        {
            assets::move_to(m, asset, stage)?;
        }
        assets::set_status(m, asset, status)
    });
    match changed {
        Ok(manifest) => shared.emit(ProviderEvent::Project { project: manifest }),
        Err(err) => tracing::warn!(error = %err, "could not update the asset"),
    }
}
