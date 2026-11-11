//! Finding a send by the id an agent gives. Job ids are numbered per
//! project, so an id names one send only with its project: the tools take
//! `project`, and an id that is a send of several projects without it is
//! refused with the projects to choose from and what to pass.

use maquette_core::providers::{JobState, ProviderJob, ProviderJobs};
use serde_json::{Value, json};

use crate::mcp::hub::Hub;
use crate::mcp::tools::{ToolError, errors, to_value};

/// The code of a job id given without its project while several projects
/// have a send with that id.
pub(in crate::mcp::tools) const SEVERAL_PROJECTS: &str = "job-in-several-projects";

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
/// only send with that id. A send that waits for approval is left to the
/// person, so an agent cannot deny its own sends.
pub(super) fn cancel_one(jobs: &ProviderJobs, hub: &Hub, id: &str, project: Option<&str>) -> Value {
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
    if job.state == JobState::Approval || hub.pending_for_job(&job.project, id).is_some() {
        return refuse(
            "This send is waiting for approval. A person denies it in the Maquette app; an agent cannot deny its own sends."
                .to_string(),
        );
    }
    match jobs.cancel(&job.project, id) {
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
