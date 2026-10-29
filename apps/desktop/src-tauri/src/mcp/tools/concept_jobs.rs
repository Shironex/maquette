//! The tools that control Lab jobs as the Lab queue does: Retry, Cancel and
//! Run next.

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
    /// Which jobs run again when jobs is left out: failed (failed or cancelled, the default),
    /// held (waiting out a Codex rate limit or usage limit) or all (both).
    #[serde(default)]
    pub include: RetryInclude,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RetryInclude {
    #[default]
    Failed,
    Held,
    All,
}

impl RetryInclude {
    fn takes(self, state: ConceptJobState) -> bool {
        let failed = matches!(state, ConceptJobState::Failed | ConceptJobState::Cancelled);
        let held = state == ConceptJobState::RateLimited;
        match self {
            Self::Failed => failed,
            Self::Held => held,
            Self::All => failed || held,
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::Held => "held",
            Self::All => "failed or held",
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobsArgs {
    /// The project id.
    pub project: String,
    /// The job ids, as get_concepts lists them. Left out: every job of the project the tool
    /// applies to.
    #[serde(default)]
    pub jobs: Option<Vec<String>>,
}

#[tool_router(router = concept_jobs_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Run failed or cancelled Lab jobs again, as Retry in the Lab queue does: each reruns its own prompt, refs and target file under the same job id, so no new direction is made (object ref jobs too). Without jobs, every failed or cancelled job of the project; include \"held\" takes every job waiting out a Codex rate limit or usage limit instead, and \"all\" takes both. Jobs the app closed on come back failed (Interrupted by a restart) and run again here. A held job (it runs again on its own at retryAtMs) runs at once. A queued, running, waiting for approval or done job is refused, and nothing runs when any id is refused. Each job runs on the backend it ran on before: a codex job spends Codex usage; an openai-images job is paid from the OpenAI key, so its retry waits for a person to approve it in the Maquette app and the answer is \"waiting for approval\" (poll get_concepts, never retry it again meanwhile). Returns the jobs; follow them with get_concepts.",
        annotations(idempotent_hint = false)
    )]
    async fn retry_concept_jobs(&self, Parameters(args): Parameters<RetryJobsArgs>) -> Out {
        let project = args.project.clone();
        let found = self
            .blocking(move |host, _| {
                let concepts = host.concepts();
                let own = concepts.jobs(Some(&args.project));
                let ids: Vec<String> = match args.jobs {
                    Some(ids) => ids,
                    None => own
                        .iter()
                        .filter(|j| args.include.takes(j.state))
                        .map(|j| j.id.clone())
                        .collect(),
                };
                if ids.is_empty() {
                    return Err(format!(
                        "{} has no {} Lab job to retry.",
                        args.project,
                        args.include.noun()
                    ));
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
                    .map(|id| {
                        // An agent's retry on the Image API waits for approval again.
                        concepts
                            .retry_with(id, true)
                            .map(|j| job_value(&j))
                            .map_err(reason)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(json!({ "project": args.project, "jobs": jobs }))
            })
            .await;
        respond(self.ask_for_round(project, found).await)
    }

    #[tool(
        description = "Cancel Lab jobs, as Cancel in the Lab queue does. Without jobs, every unfinished job of the project (Cancel all). A queued job is dropped, a running one is stopped (it shows running until its run ends, then cancelled), and a job waiting out a rate or usage limit, or waiting for approval (an agent's own OpenAI Images round), is cancelled with nothing sent or spent. Each job answers on its own with its state now, or why it was not cancelled (unknown, or already done, failed or cancelled), so one refusal never stops the others. Cancelled jobs can run again with retry_concept_jobs.",
        annotations(destructive_hint = true, idempotent_hint = true)
    )]
    async fn cancel_concept_jobs(&self, Parameters(args): Parameters<JobsArgs>) -> Out {
        respond(
            self.blocking(move |host, _| {
                let concepts = host.concepts();
                let own = concepts.jobs(Some(&args.project));
                let ids: Vec<String> = match args.jobs {
                    Some(ids) => ids,
                    None => own
                        .iter()
                        .filter(|j| !j.state.is_finished())
                        .map(|j| j.id.clone())
                        .collect(),
                };
                if ids.is_empty() {
                    return Err(format!("{} has no unfinished Lab job.", args.project));
                }
                let jobs: Vec<_> = ids
                    .iter()
                    .map(|id| {
                        let Some(job) = own.iter().find(|j| &j.id == id) else {
                            return json!({ "id": id, "cancelled": false, "reason": format!("There is no Lab job {id:?} in {}.", args.project) });
                        };
                        if job.state.is_finished() {
                            return json!({ "id": id, "cancelled": false, "state": to_value(job.state), "reason": format!("{} is already {}.", job.file, to_value(job.state).as_str().unwrap_or_default()) });
                        }
                        match concepts.cancel(id) {
                            Ok(()) => {
                                let now = concepts.jobs(Some(&args.project)).into_iter().find(|j| &j.id == id).unwrap_or_else(|| job.clone());
                                let mut value = job_value(&now);
                                value["cancelled"] = json!(true);
                                value
                            }
                            Err(err) => json!({ "id": id, "cancelled": false, "state": to_value(job.state), "reason": reason(err) }),
                        }
                    })
                    .collect();
                Ok(json!({ "project": args.project, "jobs": jobs }))
            })
            .await,
        )
    }

    #[tool(
        description = "Run Lab jobs next, as Run next in the Lab queue does: the project's queued jobs move to the front of the line, ahead of every other project's, in the order they were queued (Codex jobs on the Codex lane, openai-images jobs on the Image API's line). Without jobs, every queued job of the project. Running, waiting (rate or usage limit, approval) and finished jobs stay where they are and are listed under skipped with why. Spends nothing; returns the jobs that moved.",
        annotations(idempotent_hint = true)
    )]
    async fn prioritize_concept_jobs(&self, Parameters(args): Parameters<JobsArgs>) -> Out {
        respond(
            self.blocking(move |host, _| {
                let concepts = host.concepts();
                let own = concepts.jobs(Some(&args.project));
                let moved = match &args.jobs {
                    None => concepts.run_next_project(&args.project),
                    Some(ids) => {
                        let ids: Vec<String> = ids.iter().filter(|id| own.iter().any(|j| &j.id == *id)).cloned().collect();
                        concepts.run_next(&ids)
                    }
                };
                let skipped: Vec<_> = args
                    .jobs
                    .iter()
                    .flatten()
                    .filter(|id| !moved.iter().any(|j| &j.id == *id))
                    .map(|id| match own.iter().find(|j| &j.id == id) {
                        None => json!({ "id": id, "reason": format!("There is no Lab job {id:?} in {}.", args.project) }),
                        Some(job) => json!({ "id": id, "state": to_value(job.state), "reason": format!("{} is {}; only a queued job moves.", job.file, to_value(job.state).as_str().unwrap_or_default()) }),
                    })
                    .collect();
                if moved.is_empty() && skipped.is_empty() {
                    return Err(format!("{} has no queued Lab job to run next.", args.project));
                }
                Ok(json!({
                    "project": args.project,
                    "moved": moved.iter().map(job_value).collect::<Vec<_>>(),
                    "skipped": skipped,
                }))
            })
            .await,
        )
    }
}
