//! Saving and loading `providers.json`, changing one job, and moving its
//! asset on the Board.

use std::fs;
use std::io;
use std::path::Path;

use super::spend;
use super::{
    CLOSED_WHILE_SENDING, Entry, FILE_VERSION, JOBS_FILE, JobKey, JobState, JobsFile,
    KEEP_FINISHED, ProviderEvent, Saving, Shared, Stored, Workspace, snapshot,
};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use maquette_base::history::now_ms;
use maquette_model::assets::{self, AssetStatus, Stage};
use maquette_model::project::{self, ProjectManifest};

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

/// Writes a project's jobs from `jobs`, the list as the caller changed it.
/// Keeps the newest finished ones only.
///
/// The caller holds `saving` from its change to the list until here, so
/// close cannot clear the list in between. It also holds the jobs lock
/// from its change through this write, so nobody reads a job's new state
/// (a send seen as finished, say) before it is on disk: another open of
/// the workspace, or the next start, would otherwise load the old state.
pub(super) fn persist(shared: &Shared, _saving: &Saving<'_>, jobs: &[Entry], project: &str) {
    #[cfg(test)]
    {
        let hook = shared
            .before_save
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        if let Some(hook) = hook {
            hook();
        }
    }
    let Some(workspace) = shared.workspace() else {
        return;
    };
    let Ok(dir) = workspace.project_dir(project) else {
        return;
    };
    let mine: Vec<Stored> = jobs
        .iter()
        .filter(|e| e.stored.job.project == project)
        .map(|e| e.stored.clone())
        .collect();
    write_project(&workspace.root, &dir, mine);
}

fn write_project(root: &Path, dir: &Path, mut mine: Vec<Stored>) {
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

/// Changes the job `key` names, saves it when `save` and tells the UI.
pub(super) fn update(
    shared: &Shared,
    key: &JobKey,
    save: bool,
    change: impl FnOnce(&mut Stored) -> CoreResult<()>,
) -> CoreResult<()> {
    update_with(shared, key, save, true, change)
}

/// [`update`], writing the credits ledger line the change calls for only
/// when `spend`.
pub(super) fn update_with(
    shared: &Shared,
    key: &JobKey,
    save: bool,
    spend: bool,
    change: impl FnOnce(&mut Stored) -> CoreResult<()>,
) -> CoreResult<()> {
    change_job(shared, None, key, save, spend, change)
}

/// [`update`] for a job thread or bake started under workspace
/// `generation`: refused once that workspace has closed, so it never
/// changes a job of the next one that has the same project and id.
pub(super) fn update_in(
    shared: &Shared,
    generation: u64,
    key: &JobKey,
    save: bool,
    change: impl FnOnce(&mut Stored) -> CoreResult<()>,
) -> CoreResult<()> {
    change_job(shared, Some(generation), key, save, true, change)
}

/// How a job that is not there reads.
pub(super) fn no_job(project: &str, id: &str) -> CoreError {
    CoreError::NotFound(format!("the job {id} in {project}"))
}

fn change_job(
    shared: &Shared,
    generation: Option<u64>,
    key: &JobKey,
    save: bool,
    spend: bool,
    change: impl FnOnce(&mut Stored) -> CoreResult<()>,
) -> CoreResult<()> {
    // A change that is not saved (progress) need not wait for a write.
    let saving = save.then(|| shared.saving());
    let (job, before, after) = {
        let mut jobs = shared.jobs();
        // Checked under the jobs lock, which close holds to empty the list
        // and forget the workspace, so the list is still this generation's.
        if generation.is_some_and(|g| shared.generation() != Some(g)) {
            return Err(CoreError::Unavailable(
                "The workspace this job ran in has closed.".to_string(),
            ));
        }
        let entry = jobs
            .iter_mut()
            .find(|e| e.is(&key.project, &key.id))
            .ok_or_else(|| no_job(&key.project, &key.id))?;
        let before = spend::before(&entry.stored);
        change(&mut entry.stored)?;
        let job = snapshot(&entry.stored);
        let after = entry.stored.clone();
        // Still under the jobs lock: the new state is on disk before
        // anyone can read it.
        if let Some(saving) = &saving {
            persist(shared, saving, &jobs, &after.job.project);
        }
        (job, before, after)
    };
    drop(saving);
    shared.emit(ProviderEvent::Job { job: Box::new(job) });
    if spend {
        spend::record(shared, &before, &after);
    }
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
    if let Some(manifest) = touch_in(&workspace, project, asset, stage, status) {
        shared.emit(ProviderEvent::Project { project: manifest });
    }
}

/// Moves an asset of `workspace` on the Board and sets its status. The
/// changed project, for the caller to tell the UI.
pub(super) fn touch_in(
    workspace: &Workspace,
    project: &str,
    asset: &str,
    stage: Option<Stage>,
    status: AssetStatus,
) -> Option<ProjectManifest> {
    let changed = project::modify(&workspace.root, &workspace.config.layout, project, |m| {
        if let Some(stage) = stage
            && assets::find(m, asset)?.stage < stage
        {
            assets::move_to(m, asset, stage)?;
        }
        assets::set_status(m, asset, status)
    });
    changed
        .inspect_err(|err| tracing::warn!(error = %err, "could not update the asset"))
        .ok()
}
