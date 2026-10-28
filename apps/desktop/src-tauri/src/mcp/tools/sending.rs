//! Tools that send assets to a 3D service and read the jobs.

use std::sync::Arc;

use maquette_core::project;
use maquette_core::providers::{JobState, Provider, ProviderJob, ProviderJobs};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::mcp::host::Host;
use crate::mcp::hub::Hub;

mod redo;
mod requests;
mod warnings;

use self::requests::{batch_request, send_request};
use super::{McpTools, Out, WAITING, failed, json_result, reason, respond, to_value, workspace};

/// Where a model gets made.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderArg {
    Tripo,
    Meshy,
    ByHand,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceArg {
    Tripo,
    Meshy,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum VariantArg {
    Prop,
    Character,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendTo3dArgs {
    /// The project id.
    pub project: String,
    /// The asset id, as list_assets returns it.
    pub asset: String,
    /// "tripo" or "meshy" spend credits and always wait for a person to approve
    /// them in the app; "by-hand" writes a handoff folder and spends nothing.
    pub provider: ProviderArg,
    /// By hand only: whose web app the settings card is for, "tripo" by default.
    pub service: Option<ServiceArg>,
    /// "prop" or "character". By default from the asset's kind.
    pub variant: Option<VariantArg>,
    /// Images relative to the project folder (for example "refs/lantern_front.png"),
    /// front first, at most 4. By default the asset's own refs.
    #[serde(default)]
    pub images: Vec<String>,
    /// Preset fields to change for this send only, named as in the workspace preset,
    /// for example {"faceLimit": 20000, "rig": false} for Tripo or
    /// {"aiModel": "meshy-6", "targetPolycount": 30000} for Meshy.
    pub overrides: Option<serde_json::Map<String, Value>>,
    /// Meshy only: text that guides the texture, at most 800 characters.
    pub prompt: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemArg {
    /// The asset id, as list_assets returns it.
    pub asset: String,
    /// "prop" or "character". By default from the asset's kind.
    pub variant: Option<VariantArg>,
    /// Images relative to the project folder, front first, at most 4. By default the
    /// asset's own refs.
    #[serde(default)]
    pub images: Vec<String>,
    /// Preset fields to change for this asset only, laid over the batch settings, for
    /// example {"faceLimit": 2000, "pbr": false}.
    pub overrides: Option<serde_json::Map<String, Value>>,
    /// Meshy only: text that guides this asset's texture, at most 800 characters.
    pub prompt: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendBatchArgs {
    /// The project id.
    pub project: String,
    /// The same provider for every asset. "tripo" or "meshy" spend credits and wait for one
    /// approval in the app for the whole batch; "by-hand" writes one handoff folder per asset.
    pub provider: ProviderArg,
    /// By hand only: whose web app the settings cards are for, "tripo" by default.
    pub service: Option<ServiceArg>,
    /// One model per asset, at most 24.
    pub items: Vec<BatchItemArg>,
    /// Preset fields to change for this batch only, laid over both the prop and the
    /// character preset, for example {"faceLimit": 20000}. An item's own overrides
    /// go on top of these.
    pub overrides: Option<serde_json::Map<String, Value>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderJobArgs {
    /// The job id that send_to_3d returned, for example "stone-lantern-2".
    pub id: Option<String>,
    /// Without an id: every send in this project, newest last. With an id:
    /// the project the send must belong to.
    pub project: Option<String>,
    /// With project: only this asset's sends.
    pub asset: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelJobsArgs {
    /// The job ids that send_to_3d, send_batch_to_3d or the redo tools
    /// returned, 1 to 50.
    pub ids: Vec<String>,
}

/// Most ids one cancel_provider_jobs call takes.
const MAX_CANCEL_IDS: usize = 50;

/// A provider job as agents read it.
pub(super) fn job_value(hub: &Hub, job: &ProviderJob) -> Value {
    let approval = hub.pending_for_job(&job.id);
    let status = if approval.is_some() {
        WAITING.to_string()
    } else {
        to_value(job.state).as_str().unwrap_or_default().to_string()
    };
    let mut out = json!({
        "status": status,
        "id": job.id,
        "project": job.project,
        "asset": job.asset,
        "assetName": job.asset_name,
        "provider": to_value(job.provider),
        "settings": job.summary(),
        "images": job.images,
        "phase": to_value(job.phase),
        "progress": job.progress,
        "credits": job.credits,
        "estimate": job.estimate,
        "redoOf": job.redo_of,
        "message": job.message,
        "inboxFile": job.inbox_file,
        "handoff": job.handoff,
        "version": job.version,
        "cancel": to_value(job.cancel),
    });
    if let Some(approval) = approval {
        out["approval"] = json!({
            "id": approval.id,
            "note": "A person approves or denies spending these credits in the Maquette app. Poll get_provider_job; do not send again.",
        });
    }
    out
}

/// Waits for the person's answer on its own thread, then releases or drops
/// every job it covers. Jobs cancelled from the app's queue withdraw the
/// approval once none is left waiting.
pub(super) fn await_send_approval(
    host: Arc<dyn Host>,
    hub: Arc<Hub>,
    approval: String,
    jobs: Vec<String>,
) {
    let spawned = std::thread::Builder::new()
        .name("send-approval".into())
        .spawn(move || {
            let waiting = |id: &String| {
                host.studio()
                    .get(id)
                    .is_some_and(|j| j.state == JobState::Approval)
            };
            let gone = || !jobs.iter().any(waiting);
            let answer = hub.wait_until(&approval, gone);
            for job in jobs.iter().filter(|j| waiting(j)) {
                let result = match answer {
                    Some(true) => host.studio().approve(job).map(|_| ()),
                    Some(false) => host.studio().deny(job).map(|_| ()),
                    None => Ok(()),
                };
                if let Err(err) = result {
                    tracing::warn!(job, error = %err, "could not answer a send approval");
                }
            }
        });
    if let Err(err) = spawned {
        tracing::warn!(error = %err, "could not wait for a send approval");
    }
}

#[tool_router(router = sending_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Send an asset's refs to 3D. \"tripo\" or \"meshy\" spend credits, so the send always waits for a person to approve it in the Maquette app and answers \"waiting for approval\"; follow it with get_provider_job. \"by-hand\" spends nothing: it writes a handoff folder with the images and a settings card, and the asset waits for a person to bring the GLB back. Settings start from the workspace preset (prop or character) and overrides change them for this send only. The images go front, left, back, right by the asset's ref views (get_ref_views), and refWarnings lists weak refs (small, cropped, busy background, one view only, a view that looks different from the front) without stopping the send."
    )]
    async fn send_to_3d(&self, Parameters(args): Parameters<SendTo3dArgs>) -> Out {
        let sent = self
            .blocking(move |host, hub| {
                let request = send_request(host, &args)?;
                let approval = request.provider != Provider::ByHand;
                let job = host
                    .studio()
                    .send(&request, approval, true)
                    .map_err(reason)?;
                let pending = (job.state == JobState::Approval).then(|| hub.request_send(&job));
                let warned = warnings::of(host, &job);
                Ok((job, pending, warned))
            })
            .await;
        let (job, pending, warned) = match sent {
            Ok(sent) => sent,
            Err(message) => return failed(message),
        };
        if let Some(approval) = pending {
            await_send_approval(
                Arc::clone(&self.host),
                Arc::clone(&self.hub),
                approval,
                vec![job.id.clone()],
            );
        }
        let mut out = job_value(&self.hub, &job);
        out["refWarnings"] = warned;
        if job.provider == Provider::ByHand {
            out["note"] = json!(format!(
                "Nothing is spent. A person makes the model in the {} web app with the settings card in the handoff folder, then drops the GLB on the asset's card in Maquette or saves it into the inbox as {}.glb.",
                job.service.label(),
                job.id
            ));
        }
        json_result(&out)
    }

    #[tool(
        description = "Send several assets to 3D at once, one model per asset, with the same provider and settings (each asset keeps its own refs and prop or character preset unless given). \"tripo\" or \"meshy\" spend credits, so the whole batch waits for one approval in the Maquette app that lists every asset; follow each job with get_provider_job. \"by-hand\" spends nothing and writes one handoff folder per asset. Every item is checked before any goes out. Each job reports refWarnings (weak refs, never a stop) and assetsWithRefWarnings counts the assets with any."
    )]
    async fn send_batch_to_3d(&self, Parameters(args): Parameters<SendBatchArgs>) -> Out {
        let sent = self
            .blocking(move |host, hub| {
                let request = batch_request(host, &args)?;
                let approval = request.provider != Provider::ByHand;
                let sent = host
                    .studio()
                    .send_batch(&request, approval, true)
                    .map_err(reason)?;
                let waiting: Vec<ProviderJob> = sent
                    .jobs
                    .iter()
                    .filter(|j| j.state == JobState::Approval)
                    .cloned()
                    .collect();
                let pending = hub.request_batch(&waiting);
                let ids = waiting.into_iter().map(|j| j.id).collect::<Vec<_>>();
                let warned: Vec<Value> = sent.jobs.iter().map(|j| warnings::of(host, j)).collect();
                Ok((sent, pending, ids, warned))
            })
            .await;
        let (sent, pending, ids, warned) = match sent {
            Ok(sent) => sent,
            Err(message) => return failed(message),
        };
        if let Some(approval) = pending {
            await_send_approval(Arc::clone(&self.host), Arc::clone(&self.hub), approval, ids);
        }
        let jobs: Vec<Value> = sent
            .jobs
            .iter()
            .zip(warned)
            .map(|(j, w)| {
                let mut out = job_value(&self.hub, j);
                out["refWarnings"] = w;
                out
            })
            .collect();
        let warned_assets = jobs
            .iter()
            .filter(|j| j["refWarnings"].as_array().is_some_and(|w| !w.is_empty()))
            .count();
        let by_hand = sent
            .jobs
            .first()
            .is_some_and(|j| j.provider == Provider::ByHand);
        let status = if by_hand || sent.jobs.is_empty() {
            "started"
        } else {
            WAITING
        };
        let mut out = json!({
            "status": status,
            "batch": sent.batch,
            "jobs": jobs,
            "assetsWithRefWarnings": warned_assets,
            "failed": sent.failed.iter().map(|f| json!({ "asset": f.asset, "message": f.message })).collect::<Vec<_>>(),
        });
        out["note"] = json!(if by_hand {
            "Nothing is spent. Each asset has its own handoff folder with its images and settings card; a person makes the models in the web app and brings each GLB back in Maquette."
        } else {
            "A person approves or denies the whole batch once in the Maquette app. Poll get_provider_job for each job; do not send again."
        });
        json_result(&out)
    }

    #[tool(
        description = "Get a send to 3D: its status (\"waiting for approval\", queued, running, waiting for a by hand model, inbox, accepted, failed or cancelled), phase, progress, credits spent and message. With project instead of id, every send in the project.",
        annotations(read_only_hint = true)
    )]
    async fn get_provider_job(&self, Parameters(args): Parameters<ProviderJobArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                if let Some(id) = args.id.as_deref() {
                    // Ids are unique across the workspace; a project given
                    // alongside must be the job's own.
                    let job = host
                        .studio()
                        .get(id.trim())
                        .filter(|j| args.project.as_deref().is_none_or(|p| j.project == p))
                        .ok_or_else(|| match args.project.as_deref() {
                            Some(project) => format!("There is no send {id} in {project}."),
                            None => format!("There is no send {id}."),
                        })?;
                    return Ok(job_value(hub, &job));
                }
                let Some(project) = args.project.as_deref() else {
                    return Err("Pass the job id, or a project to list its sends.".to_string());
                };
                let (root, config) = workspace(host)?;
                project::load(&root, &config.layout, project).map_err(reason)?;
                let jobs: Vec<Value> = host
                    .studio()
                    .list(Some(project))
                    .iter()
                    .filter(|j| args.asset.as_deref().is_none_or(|a| j.asset == a))
                    .map(|j| job_value(hub, j))
                    .collect();
                Ok(json!({ "project": project, "jobs": jobs }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Stop sends to 3D that are still in flight: ids is 1 to 50 job ids. Each id is stopped on its own and answers with its new status and message, or the reason it could not be stopped, so one failure never stops the others. A by hand send that waits for a model becomes cancelled (\"Stop waiting\"), a Tripo or Meshy send that has not started is cancelled for free, and Meshy gives the credits back when the task is called off before it starts. A send that cannot be stopped (Tripo once sent, a model that is downloading, a job that is over) is refused with why. A send waiting for approval is refused: a person approves or denies those in the Maquette app."
    )]
    async fn cancel_provider_jobs(&self, Parameters(args): Parameters<CancelJobsArgs>) -> Out {
        let mut ids: Vec<String> = Vec::new();
        for id in args.ids.iter().map(|id| id.trim()) {
            if id.is_empty() {
                return failed("Job ids cannot be empty.");
            }
            if !ids.iter().any(|seen| seen == id) {
                ids.push(id.to_string());
            }
        }
        if ids.is_empty() || ids.len() > MAX_CANCEL_IDS {
            return failed(format!(
                "Pass 1 to {MAX_CANCEL_IDS} job ids, got {}.",
                ids.len()
            ));
        }
        let found = self
            .blocking(move |host, hub| {
                let results: Vec<Value> = ids
                    .iter()
                    .map(|id| cancel_one(host.studio(), hub, id))
                    .collect();
                let stopped = results.iter().filter(|r| r["ok"] == json!(true)).count();
                Ok(json!({ "stopped": stopped, "refused": results.len() - stopped, "results": results }))
            })
            .await;
        respond(found)
    }
}

/// Stops one job for [`cancel_provider_jobs`]. A send that waits for approval
/// is left to the person, so an agent cannot deny its own sends.
fn cancel_one(jobs: &ProviderJobs, hub: &Hub, id: &str) -> Value {
    let refuse = |error: String| json!({ "id": id, "ok": false, "error": error });
    let Some(job) = jobs.get(id) else {
        return refuse(format!("There is no send {id}."));
    };
    if job.state == JobState::Approval || hub.pending_for_job(id).is_some() {
        return refuse(
            "This send is waiting for approval. A person denies it in the Maquette app; an agent cannot deny its own sends."
                .to_string(),
        );
    }
    match jobs.cancel(id) {
        Ok(job) => json!({
            "id": id,
            "ok": true,
            "status": to_value(job.state),
            "message": job.message,
        }),
        Err(err) => refuse(reason(err)),
    }
}
