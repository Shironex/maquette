//! Tools that split a model into parts on Tripo or Meshy, and assemble one
//! prop from part models. A split spends credits, so it waits for a person
//! to approve it in the app like a send to 3D.

use std::sync::Arc;

use maquette_core::parts::assemble::{AssemblePart, AssembleRequest, Pivot};
use maquette_core::providers::split::{
    CompletionMode, Granularity, MeshySplit, MeshySplitMode, SegmentModel, TripoSplit,
};
use maquette_core::providers::{JobState, Provider, SplitRequest};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::json;

use crate::commands::split::{run_assembly, version_thumb};

use super::prepare::asset_id;
use super::sending::{ServiceArg, await_send_approval, job_value};
use super::{McpTools, Out, failed, json_result, reason, respond, workspace};

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SplitModelArg {
    /// Cuts where the shape changes.
    Geometry,
    /// Looks for parts that mean something, like a lid or a cage. Beta.
    Semantic,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum GranularityArg {
    Simple,
    Balanced,
    Detailed,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CompleteArg {
    AiCompletion,
    QuickCap,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum MeshyModeArg {
    Auto,
    ByParts,
    ByColor,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SplitArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// "tripo" or "meshy". Both spend credits and wait for a person to approve them.
    pub provider: ServiceArg,
    /// The version to split. By default the asset's current one.
    pub version: Option<u32>,
    /// An inbox model of the asset (its id in list_inbox) instead of a version.
    pub item: Option<String>,
    /// Tripo only: "geometry" (default) or "semantic" (beta).
    pub model: Option<SplitModelArg>,
    /// Tripo semantic only: "simple", "balanced" (default) or "detailed".
    pub granularity: Option<GranularityArg>,
    /// Tripo semantic only: also split by connected pieces. True by default.
    pub split_by_connectivity: Option<bool>,
    /// Tripo only: run part completion after the split, "ai-completion" or "quick-cap".
    /// It rebuilds faces one part hid in another, for more credits.
    pub complete: Option<CompleteArg>,
    /// Meshy only: "auto" (default), "by-parts" or "by-color".
    pub mode: Option<MeshyModeArg>,
    /// Meshy by-parts or by-color: 1 to 10 names of the parts to find.
    #[serde(default)]
    pub parts: Vec<String>,
}

impl SplitArgs {
    fn request(self, asset: String) -> SplitRequest {
        let provider = match self.provider {
            ServiceArg::Tripo => Provider::Tripo,
            ServiceArg::Meshy => Provider::Meshy,
        };
        let defaults = TripoSplit::default();
        let tripo = (provider == Provider::Tripo).then(|| TripoSplit {
            model: match self.model {
                Some(SplitModelArg::Semantic) => SegmentModel::Semantic,
                Some(SplitModelArg::Geometry) | None => SegmentModel::Geometry,
            },
            granularity: match self.granularity {
                Some(GranularityArg::Simple) => Granularity::Simple,
                Some(GranularityArg::Detailed) => Granularity::Detailed,
                Some(GranularityArg::Balanced) | None => defaults.granularity,
            },
            split_by_connectivity: self
                .split_by_connectivity
                .unwrap_or(defaults.split_by_connectivity),
            complete: self.complete.map(|c| match c {
                CompleteArg::AiCompletion => CompletionMode::AiCompletion,
                CompleteArg::QuickCap => CompletionMode::QuickCap,
            }),
        });
        let meshy = (provider == Provider::Meshy).then_some(MeshySplit {
            mode: match self.mode {
                Some(MeshyModeArg::ByParts) => MeshySplitMode::ByParts,
                Some(MeshyModeArg::ByColor) => MeshySplitMode::ByColor,
                Some(MeshyModeArg::Auto) | None => MeshySplitMode::Auto,
            },
            parts: self.parts,
        });
        SplitRequest {
            project: self.project,
            asset,
            version: self.version,
            item: self.item,
            provider,
            tripo,
            meshy,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PivotArg {
    /// The origin the model was made with.
    Origin,
    BottomCentre,
    Centre,
    /// The left edge at the bottom, seen from the front: a door's hinge.
    LeftHinge,
    RightHinge,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartArgs {
    /// The part's asset id as list_assets returns it, or its name.
    pub asset: String,
    /// Its version. By default the current one, so a redo of the part flows in.
    pub version: Option<u32>,
    /// The node name it ships as, for example "door_leaf".
    pub name: String,
    /// Where it turns about. "bottom-centre" by default.
    pub pivot: Option<PivotArg>,
    /// Where its pivot goes, in metres, Y up. By default where its model put it.
    pub at: Option<[f32; 3]>,
    /// Degrees about the up axis.
    #[serde(default)]
    pub turn: f32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssembleArgs {
    /// The project id.
    pub project: String,
    /// The prop's name: its root node, and the new asset's name when target is not given.
    pub name: String,
    /// The asset it becomes the next version of. By default one named after the prop,
    /// made when there is none.
    pub target: Option<String>,
    /// 2 to 16 parts, in node order.
    pub parts: Vec<PartArgs>,
}

fn pivot(arg: Option<PivotArg>) -> Pivot {
    match arg {
        Some(PivotArg::Origin) => Pivot::Origin,
        Some(PivotArg::Centre) => Pivot::Centre,
        Some(PivotArg::LeftHinge) => Pivot::LeftHinge,
        Some(PivotArg::RightHinge) => Pivot::RightHinge,
        Some(PivotArg::BottomCentre) | None => Pivot::BottomCentre,
    }
}

#[tool_router(router = splitting_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Split an asset's model into parts on Tripo (mesh segmentation, about 40 credits, optional part completion for more) or Meshy (Auto Split, about 10 credits). It spends credits, so it always waits for a person to approve it in the Maquette app and answers \"waiting for approval\"; follow it with get_provider_job. The split model lands in the inbox; accept_inbox_model makes it the asset's next version and the version split from stays. Meshy splits only models it made with meshy-6 or newer and drops the texture: once accepted, the texture bakes back from the version split from in headless Blender, free, as the version after. Tripo parts arrive as tripo_part_0, tripo_part_1 and on; name them with set_parts. A free split in Blender is a fix-up instead (add_fixups, op split-parts)."
    )]
    async fn split_with_provider(&self, Parameters(args): Parameters<SplitArgs>) -> Out {
        let sent = self
            .blocking(move |host, hub| {
                let asset = asset_id(host, &args.project, &args.asset)?;
                let request = args.request(asset);
                let job = host
                    .studio()
                    .send_split(&request, true, true)
                    .map_err(reason)?;
                let pending = (job.state == JobState::Approval).then(|| hub.request_send(&job));
                Ok((job, pending))
            })
            .await;
        let (job, pending) = match sent {
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
        json_result(&job_value(&self.hub, &job))
    }

    #[tool(
        description = "Assemble one prop from parts made as separate assets (each keeps its own versions): their models join headless in Blender (blender -b, never a window) into one GLB with a node per part under one root named after the prop, and it lands as the next version of the target asset (by default one named after the prop, made when there is none). Each part takes a pivot (left-hinge for a door that swings), an optional place in metres and a turn. The recipe is kept, so assembling again picks up a part's newer version. Spends nothing."
    )]
    async fn assemble_prop(&self, Parameters(args): Parameters<AssembleArgs>) -> Out {
        let made = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let target = match &args.target {
                    Some(t) => Some(asset_id(host, &args.project, t)?),
                    None => None,
                };
                let mut parts = Vec::new();
                for part in args.parts {
                    parts.push(AssemblePart {
                        asset: asset_id(host, &args.project, &part.asset)?,
                        version: part.version,
                        name: part.name,
                        pivot: pivot(part.pivot),
                        at: part.at,
                        turn: part.turn,
                    });
                }
                let request = AssembleRequest {
                    project: args.project,
                    asset: target,
                    name: args.name,
                    parts,
                };
                let tools = host.toolbox();
                let made = run_assembly(&root, &config, &tools, &request).map_err(|e| e.message)?;
                version_thumb(
                    host.thumbs(),
                    &root,
                    &config,
                    &tools,
                    &request.project,
                    &made.asset,
                    made.version.version,
                );
                host.project_changed(&made.project);
                Ok(json!({
                    "asset": made.asset,
                    "version": made.version.version,
                    "root": made.root,
                    "nodes": made.nodes,
                    "note": "Each part is a node under the root, named as given, so object states and explode tracks pick it by name.",
                }))
            })
            .await;
        respond(made)
    }
}

#[cfg(test)]
mod tests;
