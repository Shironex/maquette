//! The Lab queue's controls: Cancel, Retry, Cancel all, Clear and Run next.

use maquette_core::concepts::ConceptJob;
use tauri::State;

use super::{Lab, LabView, view};
use crate::commands::CommandResult;
use crate::session::Session;

#[tauri::command(async)]
#[specta::specta]
pub fn lab_cancel(lab: State<'_, Lab>, job_id: String) -> CommandResult<()> {
    Ok(lab.0.cancel(&job_id)?)
}

/// Runs a failed or cancelled job again: the same prompt, refs and file
/// under the same job, so no new direction. A job waiting out a rate limit
/// runs at once.
#[tauri::command(async)]
#[specta::specta]
pub fn lab_retry(lab: State<'_, Lab>, job_id: String) -> CommandResult<ConceptJob> {
    Ok(lab.0.retry(&job_id)?)
}

/// Stops every queued and running job of the project.
#[tauri::command(async)]
#[specta::specta]
pub fn lab_cancel_all(lab: State<'_, Lab>, project: String) -> u32 {
    u32::try_from(lab.0.cancel_all(&project)).unwrap_or(u32::MAX)
}

#[tauri::command(async)]
#[specta::specta]
pub fn lab_clear_finished(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    project: String,
) -> CommandResult<LabView> {
    lab.0.clear_finished(&project);
    view(&session, &lab.0, &project)
}

/// Run next: moves the project's queued jobs (or the ones among `jobs`) to
/// the front of the line, ahead of every other project's. Returns the jobs
/// that moved.
#[tauri::command(async)]
#[specta::specta]
pub fn lab_run_next(
    lab: State<'_, Lab>,
    project: String,
    jobs: Option<Vec<String>>,
) -> Vec<ConceptJob> {
    match jobs {
        None => lab.0.run_next_project(&project),
        Some(ids) => {
            let own: Vec<String> = lab
                .0
                .jobs(Some(&project))
                .into_iter()
                .map(|j| j.id)
                .filter(|id| ids.contains(id))
                .collect();
            lab.0.run_next(&own)
        }
    }
}
