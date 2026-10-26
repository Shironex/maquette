//! The tool that runs failed Lab jobs again, as the Lab queue's Retry does.

use maquette_core::concepts::ConceptJobState;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::json;

use super::concepts::job_value;
use super::{McpTools, Out, reason, respond, to_value};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetryJobsArgs {
    /// The project id.
    pub project: String,
    /// The job ids, as get_concepts lists them under failed. Left out: every failed or
    /// cancelled job of the project.
    #[serde(default)]
    pub jobs: Option<Vec<String>>,
}

#[tool_router(router = concept_jobs_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Run failed or cancelled Lab jobs again, as Retry in the Lab queue does: each reruns its own prompt, refs and target file under the same job id, so no new direction is made (object ref jobs too). Without jobs, every failed or cancelled job of the project. A job waiting out a Codex rate limit (it runs again on its own at retryAtMs) runs at once. A queued, running or done job is refused, and nothing runs when any id is refused. Each run spends Codex usage. Returns the jobs; follow them with get_concepts.",
        annotations(idempotent_hint = false)
    )]
    async fn retry_concept_jobs(&self, Parameters(args): Parameters<RetryJobsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let concepts = host.concepts();
                let own = concepts.jobs(Some(&args.project));
                let ids: Vec<String> = match args.jobs {
                    Some(ids) => ids,
                    None => own
                        .iter()
                        .filter(|j| {
                            matches!(
                                j.state,
                                ConceptJobState::Failed | ConceptJobState::Cancelled
                            )
                        })
                        .map(|j| j.id.clone())
                        .collect(),
                };
                if ids.is_empty() {
                    return Err(format!("{} has no failed Lab job to retry.", args.project));
                }
                for id in &ids {
                    let job = own.iter().find(|j| &j.id == id).ok_or_else(|| {
                        format!("There is no Lab job {id:?} in {}.", args.project)
                    })?;
                    if !matches!(
                        job.state,
                        ConceptJobState::Failed
                            | ConceptJobState::Cancelled
                            | ConceptJobState::RateLimited
                    ) {
                        return Err(format!(
                            "{} is {}; only a failed, cancelled or rate limited job runs again.",
                            job.file,
                            to_value(job.state).as_str().unwrap_or_default()
                        ));
                    }
                }
                let jobs = ids
                    .iter()
                    .map(|id| concepts.retry(id).map(|j| job_value(&j)).map_err(reason))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(json!({ "project": args.project, "jobs": jobs }))
            })
            .await;
        respond(found)
    }
}
