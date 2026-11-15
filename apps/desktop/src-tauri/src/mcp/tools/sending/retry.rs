//! Retry: send a failed or stopped send again, with the request it was
//! made with (provider, images, kind, settings, prompt and Studio tab), so
//! an agent never has to rebuild it. A paid retry asks a person again,
//! like any send an agent asks for.

use std::sync::Arc;

use maquette_core::providers::{
    BatchItem, BatchRequest, JobKey, JobState, Provider, ProviderJob, SendRequest, StudioChoice,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::by_id::find_job;
use super::{await_send_approval, guard_by_hand, job_value};
use crate::mcp::host::Host;
use crate::mcp::tools::{
    McpTools, Out, ToolError, WAITING, WAITING_FOR_MODEL, failed, json_result,
};

/// Most sends one retry_provider_jobs call takes, as a batch does.
const MAX_RETRY_IDS: usize = 24;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetryJobsArgs {
    /// Job ids of failed or cancelled sends, 1 to 24, all of one project, as
    /// get_provider_job returns them.
    pub(crate) ids: Vec<String>,
    /// The project the sends are in. Job ids are numbered per project; left
    /// out, each id must be a send of one project only.
    pub(crate) project: Option<String>,
}

/// The send `id` names, if it can be sent again.
fn retryable(host: &dyn Host, id: &str, project: Option<&str>) -> Result<ProviderJob, ToolError> {
    let job = find_job(host.studio(), id, project)?;
    if job.split.is_some() {
        return Err(format!(
            "{} is a split; ask for it again with split_with_provider.",
            job.id
        )
        .into());
    }
    if job.retexture.is_some() {
        return Err(format!(
            "{} is a re-texture, which this tool does not repeat.",
            job.id
        )
        .into());
    }
    if !matches!(job.state, JobState::Failed | JobState::Cancelled) {
        return Err(format!(
            "{} is not failed or cancelled, so there is nothing to retry. Poll it with get_provider_job.",
            job.id
        )
        .into());
    }
    Ok(job)
}

/// The Studio tab and rig the earlier send asked for.
fn studio_of(job: &ProviderJob) -> Option<StudioChoice> {
    job.studio.map(|s| StudioChoice {
        mesh: Some(s.mesh),
        rig: Some(s.rig),
    })
}

fn retry_request(job: &ProviderJob) -> SendRequest {
    SendRequest {
        project: job.project.clone(),
        asset: job.asset.clone(),
        provider: job.provider,
        service: job.service,
        variant: job.variant,
        images: job.images.clone(),
        tripo: job.tripo.clone(),
        meshy: job.meshy.clone(),
        prompt: job.prompt.clone(),
        redo_of: None,
        studio: studio_of(job),
    }
}

fn retry_batch(jobs: &[ProviderJob]) -> Result<BatchRequest, ToolError> {
    let Some(first) = jobs.first() else {
        return Err("Pass at least one job id.".into());
    };
    if let Some(other) = jobs.iter().find(|j| j.project != first.project) {
        return Err(format!(
            "{} and {} are sends of different projects; retry each project's sends in its own call.",
            first.id, other.id
        )
        .into());
    }
    if let Some(other) = jobs
        .iter()
        .find(|j| j.provider != first.provider || j.service != first.service)
    {
        return Err(format!(
            "{} and {} went to different providers; retry each provider's sends in its own call.",
            first.id, other.id
        )
        .into());
    }
    for (n, job) in jobs.iter().enumerate() {
        if jobs[..n].iter().any(|j| j.asset == job.asset) {
            return Err(format!(
                "{} and another send are both of {}; retry one send per asset.",
                job.id, job.asset_name
            )
            .into());
        }
    }
    Ok(BatchRequest {
        project: first.project.clone(),
        provider: first.provider,
        service: first.service,
        items: jobs
            .iter()
            .map(|job| BatchItem {
                asset: job.asset.clone(),
                variant: job.variant,
                images: job.images.clone(),
                tripo: job.tripo.clone(),
                meshy: job.meshy.clone(),
                prompt: job.prompt.clone(),
                redo_of: None,
                studio: studio_of(job),
            })
            .collect(),
        tripo_prop: None,
        tripo_character: None,
        meshy_prop: None,
        meshy_character: None,
        studio: None,
    })
}

/// What a retry sent: its jobs, the approval they wait for and what failed.
struct Retried {
    jobs: Vec<ProviderJob>,
    failed: Vec<Value>,
    approval: Option<String>,
    batch: Option<String>,
}

fn send_again(
    host: &dyn Host,
    hub: &crate::mcp::hub::Hub,
    earlier: &[ProviderJob],
) -> Result<Retried, ToolError> {
    let first = earlier.first().ok_or("Pass at least one job id.")?;
    let assets: Vec<&str> = earlier.iter().map(|j| j.asset.as_str()).collect();
    guard_by_hand(host, &first.project, first.provider, &assets)?;
    let approval = first.provider != Provider::ByHand;
    if let [only] = earlier {
        let job = host.studio().send(&retry_request(only), approval, true)?;
        let pending = (job.state == JobState::Approval).then(|| hub.request_send(&job));
        return Ok(Retried {
            jobs: vec![job],
            failed: Vec::new(),
            approval: pending,
            batch: None,
        });
    }
    let sent = host
        .studio()
        .send_batch(&retry_batch(earlier)?, approval, true)?;
    let waiting: Vec<ProviderJob> = sent
        .jobs
        .iter()
        .filter(|j| j.state == JobState::Approval)
        .cloned()
        .collect();
    Ok(Retried {
        approval: hub.request_batch(&waiting),
        failed: sent
            .failed
            .iter()
            .map(|f| json!({ "asset": f.asset, "message": f.message }))
            .collect(),
        jobs: sent.jobs,
        batch: Some(sent.batch),
    })
}

#[tool_router(router = retry_router, vis = "pub(crate)")]
impl McpTools {
    #[tool(
        description = "Send failed or cancelled sends again (a failure, a send Maquette closed on before it was approved, a withdrawn or denied one) with the request each was made with: provider, images, prop or character, settings, Meshy prompt and Tripo Studio tab. ids is 1 to 24 job ids of one project and one provider (pass project; job ids are numbered per project). One id is one new send; several go as one batch. \"tripo\" or \"meshy\" spend credits, so the retry waits for a person to approve it in the Maquette app (one approval for a batch); a denied send was a person's no, so ask them first. A by hand retry writes a new handoff folder and is refused while the asset already waits for a by hand model. Splits and re-textures are not retried here. retries names the earlier sends; follow each new job with get_provider_job."
    )]
    async fn retry_provider_jobs(&self, Parameters(args): Parameters<RetryJobsArgs>) -> Out {
        let mut ids: Vec<String> = Vec::new();
        for id in args.ids.iter().map(|id| id.trim()) {
            if id.is_empty() {
                return failed("Job ids cannot be empty.");
            }
            if !ids.iter().any(|seen| seen == id) {
                ids.push(id.to_string());
            }
        }
        if ids.is_empty() || ids.len() > MAX_RETRY_IDS {
            return failed(format!(
                "Pass 1 to {MAX_RETRY_IDS} job ids, got {}.",
                ids.len()
            ));
        }
        let project = args.project.as_deref().map(str::trim).map(str::to_string);
        let sent = self
            .blocking(move |host, hub| {
                let earlier = ids
                    .iter()
                    .map(|id| retryable(host, id, project.as_deref()))
                    .collect::<Result<Vec<_>, _>>()?;
                let retried = send_again(host, hub, &earlier)?;
                let retries: Vec<String> = earlier.into_iter().map(|j| j.id).collect();
                Ok((retried, retries))
            })
            .await;
        let (retried, retries) = match sent {
            Ok(sent) => sent,
            Err(err) => return failed(err),
        };
        let keys: Vec<JobKey> = retried
            .jobs
            .iter()
            .filter(|j| j.state == JobState::Approval)
            .map(JobKey::of)
            .collect();
        let waiting = retried.approval.is_some();
        if let Some(approval) = retried.approval {
            await_send_approval(
                Arc::clone(&self.host),
                Arc::clone(&self.hub),
                approval,
                keys,
            );
        }
        let by_hand = retried
            .jobs
            .first()
            .is_some_and(|j| j.provider == Provider::ByHand);
        let jobs: Vec<Value> = retried
            .jobs
            .iter()
            .map(|j| job_value(&self.hub, j))
            .collect();
        let mut out = if let ([only], None) = (jobs.as_slice(), &retried.batch) {
            only.clone()
        } else {
            json!({
                "status": if waiting { WAITING } else if by_hand { WAITING_FOR_MODEL } else { "started" },
                "batch": retried.batch,
                "jobs": jobs,
                "failed": retried.failed,
            })
        };
        out["retries"] = json!(retries);
        out["note"] = json!(if by_hand {
            "Nothing is spent. A new handoff folder holds the images and settings card; the person makes the model again."
        } else {
            "A person approves or denies the retry in the Maquette app. Poll get_provider_job; do not send again."
        });
        json_result(&out)
    }
}
