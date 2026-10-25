//! Redo from the Inbox: send an inbox model's asset again, prefilled from
//! the send it came from (provider, images, kind, settings and prompt),
//! with any overrides on top. The new send is the asset's next version;
//! the old model stays in the inbox so a person can compare and accept one.
//! A paid redo waits for approval in the app, like any send an agent asks for.

use std::sync::Arc;

use maquette_core::inbox;
use maquette_core::providers::{
    BatchItem, BatchRequest, JobState, PresetVariant, Provider, ProviderId, ProviderJob,
    SendRequest,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::requests::with_overrides;
use super::{ProviderArg, VariantArg, await_send_approval, job_value};
use crate::mcp::host::Host;
use crate::mcp::tools::{McpTools, Out, WAITING, failed, json_result, reason, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RedoArgs {
    /// The inbox item id (its file name), as list_inbox returns it. It must have come
    /// from a send (list_inbox shows sentFor).
    pub(crate) id: String,
    /// Another provider for the redo. By default the one it was sent to; "tripo" or
    /// "meshy" spend credits and wait for approval, "by-hand" writes a new handoff folder.
    pub(crate) provider: Option<ProviderArg>,
    /// "prop" or "character". By default as it was sent.
    pub(crate) variant: Option<VariantArg>,
    /// Images relative to the project folder, front first, at most 4. By default the
    /// images it was sent with.
    #[serde(default)]
    pub(crate) images: Vec<String>,
    /// Preset fields to change, laid over the settings it was sent with, for example
    /// {"faceLimit": 20000} or {"pbr": true}.
    pub(crate) overrides: Option<serde_json::Map<String, Value>>,
    /// Meshy only: the texture prompt. By default the one it was sent with; "" clears it.
    pub(crate) prompt: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RedoManyArgs {
    /// Inbox item ids, all sent for assets of the same project, at most 24.
    pub(crate) ids: Vec<String>,
    /// One provider for every redo. By default the one they were sent to, when they
    /// share it.
    pub(crate) provider: Option<ProviderArg>,
    /// Preset fields to change for every redo, laid over each one's own settings.
    pub(crate) overrides: Option<serde_json::Map<String, Value>>,
}

/// The send an inbox model came from.
fn record(host: &dyn Host, id: &str) -> Result<ProviderJob, String> {
    let (root, config) = workspace(host)?;
    let mut items = inbox::list(&root, &config).map_err(reason)?;
    host.studio().link_inbox(&mut items);
    let item = items
        .iter()
        .find(|i| i.id == id.trim())
        .ok_or_else(|| format!("There is no inbox model {id}. list_inbox lists them."))?;
    let link = item.link.as_ref().ok_or_else(|| {
        format!(
            "{id} did not come from a send, so there are no settings to repeat. Use send_to_3d."
        )
    })?;
    host.studio().get(&link.job).ok_or_else(|| {
        format!(
            "The send {} behind {id} is no longer on record. Use send_to_3d.",
            link.job
        )
    })
}

fn provider_of(arg: ProviderArg, earlier: &ProviderJob) -> (Provider, ProviderId) {
    match arg {
        ProviderArg::Tripo => (Provider::Tripo, ProviderId::Tripo),
        ProviderArg::Meshy => (Provider::Meshy, ProviderId::Meshy),
        ProviderArg::ByHand => (Provider::ByHand, earlier.service),
    }
}

type Settings = (
    Option<maquette_core::providers::TripoPreset>,
    Option<maquette_core::providers::MeshyPreset>,
);

/// The settings it was sent with when the service stays, else the saved
/// preset, with `overrides` laid over them.
fn settings(
    host: &dyn Host,
    earlier: &ProviderJob,
    service: ProviderId,
    variant: PresetVariant,
    overrides: Option<&serde_json::Map<String, Value>>,
) -> Result<Settings, String> {
    let (_, config) = workspace(host)?;
    let same = service == earlier.service && variant == earlier.variant;
    Ok(match service {
        ProviderId::Tripo => {
            let base = earlier
                .tripo
                .clone()
                .filter(|_| same)
                .unwrap_or_else(|| config.providers.tripo(variant).clone());
            (Some(with_overrides(&base, overrides)?), None)
        }
        ProviderId::Meshy => {
            let base = earlier
                .meshy
                .clone()
                .filter(|_| same)
                .unwrap_or_else(|| config.providers.meshy(variant).clone());
            (None, Some(with_overrides(&base, overrides)?))
        }
    })
}

fn redo_request(host: &dyn Host, args: &RedoArgs) -> Result<(SendRequest, String), String> {
    let earlier = record(host, &args.id)?;
    let (provider, service) = args.provider.map_or_else(
        || (earlier.provider, earlier.service),
        |p| provider_of(p, &earlier),
    );
    let variant = match args.variant {
        Some(VariantArg::Prop) => PresetVariant::Prop,
        Some(VariantArg::Character) => PresetVariant::Character,
        None => earlier.variant,
    };
    let (tripo, meshy) = settings(host, &earlier, service, variant, args.overrides.as_ref())?;
    let prompt = match &args.prompt {
        Some(text) => Some(text.clone()),
        None => earlier.prompt.clone(),
    }
    .filter(|_| service == ProviderId::Meshy);
    let images = if args.images.is_empty() {
        earlier.images.clone()
    } else {
        args.images.clone()
    };
    Ok((
        SendRequest {
            project: earlier.project.clone(),
            asset: earlier.asset.clone(),
            provider,
            service,
            variant,
            images,
            tripo,
            meshy,
            prompt,
            redo_of: Some(earlier.id.clone()),
        },
        earlier.id,
    ))
}

fn redo_batch(host: &dyn Host, args: &RedoManyArgs) -> Result<BatchRequest, String> {
    let earlier = args
        .ids
        .iter()
        .map(|id| record(host, id))
        .collect::<Result<Vec<_>, _>>()?;
    let Some(first) = earlier.first() else {
        return Err("Pass at least one inbox model id.".to_string());
    };
    if let Some(other) = earlier.iter().find(|j| j.project != first.project) {
        return Err(format!(
            "{} and {} were sent from different projects; redo each project's models in its own batch.",
            first.asset_name, other.asset_name
        ));
    }
    let (provider, service) = match args.provider {
        Some(p) => provider_of(p, first),
        None if earlier.iter().all(|j| j.provider == first.provider) => {
            (first.provider, first.service)
        }
        None => {
            return Err(
                "These models were sent to different providers. Pass provider for the batch."
                    .to_string(),
            );
        }
    };
    let mut items = Vec::new();
    for job in &earlier {
        let (tripo, meshy) = settings(host, job, service, job.variant, args.overrides.as_ref())?;
        items.push(BatchItem {
            asset: job.asset.clone(),
            variant: job.variant,
            images: job.images.clone(),
            tripo,
            meshy,
            prompt: job.prompt.clone().filter(|_| service == ProviderId::Meshy),
            redo_of: Some(job.id.clone()),
        });
    }
    Ok(BatchRequest {
        project: first.project.clone(),
        provider,
        service,
        items,
        tripo_prop: None,
        tripo_character: None,
        meshy_prop: None,
        meshy_character: None,
    })
}

#[tool_router(router = redo_router, vis = "pub(crate)")]
impl McpTools {
    #[tool(
        description = "Redo an inbox model: send its asset again as a new version, prefilled from the send it came from (provider, images, prop or character, settings and Meshy texture prompt), with overrides on top. \"tripo\" or \"meshy\" spend credits, so the redo always waits for a person to approve it in the Maquette app and answers \"waiting for approval\"; follow it with get_provider_job. \"by-hand\" writes a new handoff folder. The old model stays in the inbox, so a person can compare both and accept one."
    )]
    async fn redo_inbox_model(&self, Parameters(args): Parameters<RedoArgs>) -> Out {
        let sent = self
            .blocking(move |host, hub| {
                let (request, earlier) = redo_request(host, &args)?;
                let approval = request.provider != Provider::ByHand;
                let job = host
                    .studio()
                    .send(&request, approval, true)
                    .map_err(reason)?;
                let pending = (job.state == JobState::Approval).then(|| hub.request_send(&job));
                Ok((job, pending, earlier))
            })
            .await;
        let (job, pending, earlier) = match sent {
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
        out["redoes"] = json!(earlier);
        out["note"] = json!(if job.provider == Provider::ByHand {
            "Nothing is spent. A new handoff folder holds the images and settings card; the earlier model stays in the inbox."
        } else {
            "A person approves or denies the redo in the Maquette app. The earlier model stays in the inbox until one of them is accepted. Poll get_provider_job; do not send again."
        });
        json_result(&out)
    }

    #[tool(
        description = "Redo several inbox models at once, each prefilled from its own send, as one batch under one approval in the Maquette app (\"tripo\" or \"meshy\" spend credits). All must be from the same project. Overrides go on top of each model's own settings. The old models stay in the inbox."
    )]
    async fn redo_inbox_models(&self, Parameters(args): Parameters<RedoManyArgs>) -> Out {
        let sent = self
            .blocking(move |host, hub| {
                let request = redo_batch(host, &args)?;
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
                Ok((sent, pending, ids))
            })
            .await;
        let (sent, pending, ids) = match sent {
            Ok(sent) => sent,
            Err(message) => return failed(message),
        };
        let waiting = pending.is_some();
        if let Some(approval) = pending {
            await_send_approval(Arc::clone(&self.host), Arc::clone(&self.hub), approval, ids);
        }
        let jobs: Vec<Value> = sent.jobs.iter().map(|j| job_value(&self.hub, j)).collect();
        json_result(&json!({
            "status": if waiting { WAITING } else { "started" },
            "batch": sent.batch,
            "jobs": jobs,
            "failed": sent.failed.iter().map(|f| json!({ "asset": f.asset, "message": f.message })).collect::<Vec<_>>(),
            "note": "The earlier models stay in the inbox. A paid batch waits for one approval in the Maquette app; poll get_provider_job for each job.",
        }))
    }
}
