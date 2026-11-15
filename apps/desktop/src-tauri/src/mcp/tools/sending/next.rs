//! What an agent can do after a send: the tools that follow a by hand wait,
//! and a retry for a failed or stopped send.

use maquette_core::providers::{JobState, Provider, ProviderJob};
use serde_json::{Value, json};

use crate::mcp::tools::hints::person_step;
use crate::mcp::tools::hints::table::step_with;

/// True for a By hand send still waiting for its model.
pub(super) fn waits_for_model(job: &ProviderJob) -> bool {
    job.provider == Provider::ByHand && job.state == JobState::Waiting
}

/// What an agent can do next with a send that waits for a model, failed or
/// stopped; `None` while it is under way or done. Each step carries the
/// send's project, and its asset or job id where the tool takes one.
pub(in crate::mcp::tools) fn job_next(job: &ProviderJob) -> Option<Vec<Value>> {
    let project = || json!({ "project": job.project });
    let asset = || json!({ "project": job.project, "asset": job.asset });
    if waits_for_model(job) {
        return Some(vec![
            step_with(
                "list_handoffs",
                "See the By hand sends waiting and the downloads that may be their models.",
                project(),
            ),
            step_with(
                "add_model_file",
                "When you have the model file yourself, add it as the asset's next version (give path).",
                asset(),
            ),
            step_with(
                "list_inbox",
                "A model the person brought back lands here to accept.",
                json!({}),
            ),
            step_with(
                "attach_download",
                "Attach a download list_handoffs found to this asset (give arrival).",
                asset(),
            ),
        ]);
    }
    let retry = || {
        if job.split.is_some() {
            step_with(
                "split_with_provider",
                "Ask for the split again; a paid split waits for a person again.",
                asset(),
            )
        } else {
            step_with(
                "retry_provider_jobs",
                "Send it again with the same settings; a paid send waits for a person again.",
                json!({ "project": job.project, "ids": [job.id] }),
            )
        }
    };
    match job.state {
        JobState::Failed => Some(vec![retry()]),
        JobState::Cancelled if job.denied => Some(vec![
            person_step(
                "A person denied this send. Ask them before sending it again; do not retry on your own.",
            ),
            retry(),
        ]),
        JobState::Cancelled => Some(vec![retry()]),
        _ => None,
    }
    .filter(|_| job.retexture.is_none())
}
