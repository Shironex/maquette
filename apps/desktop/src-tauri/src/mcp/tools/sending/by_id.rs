//! Finding a send by the id an agent gives. Job ids are numbered per
//! project, so an id names one send only with its project: the tools take
//! `project`, and an id that is a send of several projects without it is
//! refused with the projects to choose from and what to pass.

use maquette_core::providers::{JobState, Provider, ProviderJob, ProviderJobs};
use serde_json::{Value, json};

use crate::mcp::tools::{ToolError, errors, to_value};

/// The code of a job id given without its project while several projects
/// have a send with that id.
pub(in crate::mcp::tools) const SEVERAL_PROJECTS: &str = "job-in-several-projects";

/// The code of a By hand send an agent asked for while the asset already
/// has one waiting for its model.
pub(in crate::mcp::tools) const BY_HAND_WAITING: &str = "by-hand-already-waiting";

/// Refuses a By hand send of `asset` in `project` while one already waits
/// for its model, naming that send, so an agent that polls, sees nothing
/// and sends again does not pile up waits for the same asset.
pub(super) fn refuse_second_wait(
    jobs: &ProviderJobs,
    project: &str,
    assets: &[&str],
) -> Result<(), ToolError> {
    let waiting = jobs.list(Some(project)).into_iter().find(|j| {
        j.provider == Provider::ByHand
            && j.state == JobState::Waiting
            && assets.contains(&j.asset.as_str())
    });
    match waiting {
        None => Ok(()),
        Some(job) => Err(ToolError {
            message: format!(
                "{} already waits for a by hand model: send {} in {}. Poll it with get_provider_job, or stop it with cancel_provider_jobs before sending again with other settings.",
                job.asset_name, job.id, job.project
            ),
            code: BY_HAND_WAITING.to_string(),
        }),
    }
}

/// The send `id`: the one in `project` when given, else the only send of
/// the workspace with that id. Ids are numbered per project, so an id two
/// projects have is refused without one, with the projects to choose from.
pub(super) fn find_job(
    jobs: &ProviderJobs,
    id: &str,
    project: Option<&str>,
) -> Result<ProviderJob, ToolError> {
    let id = id.trim();
    if let Some(project) = project.map(str::trim) {
        return jobs
            .get(project, id)
            .ok_or_else(|| format!("There is no send {id} in {project}.").into());
    }
    let mut found = jobs.with_id(id);
    if found.len() <= 1 {
        return found
            .pop()
            .ok_or_else(|| format!("There is no send {id}.").into());
    }
    let mut projects: Vec<&str> = found.iter().map(|j| j.project.as_str()).collect();
    projects.sort_unstable();
    Err(ToolError {
        message: format!(
            "{id} is a send in {} projects ({}): job ids are numbered per project. Pass project to say which one.",
            projects.len(),
            projects.join(", ")
        ),
        code: SEVERAL_PROJECTS.to_string(),
    })
}

/// Stops one job for [`cancel_provider_jobs`]: the one in `project`, or the
/// only send with that id. A send that waits for approval is withdrawn: it
/// spends nothing and is recorded as taken back, not as a person's no. The
/// approval thread sees it leave the approval state and drops the prompt.
pub(super) fn cancel_one(jobs: &ProviderJobs, id: &str, project: Option<&str>) -> Value {
    let refuse = |error: String| json!({ "id": id, "ok": false, "error": error });
    let job = match find_job(jobs, id, project) {
        Ok(job) => job,
        Err(err) => {
            // The whole error: its code, and the fix when there is one.
            let mut out = errors::body(&err);
            out["id"] = json!(id);
            out["ok"] = json!(false);
            return out;
        }
    };
    let stopped = if job.state == JobState::Approval {
        jobs.withdraw(&job.project, id)
    } else {
        jobs.cancel(&job.project, id)
    };
    match stopped {
        Ok(job) => json!({
            "id": id,
            "project": job.project,
            "ok": true,
            "status": to_value(job.state),
            "message": job.message,
        }),
        Err(err) => refuse(maquette_core::ErrorPayload::from(err).message),
    }
}
