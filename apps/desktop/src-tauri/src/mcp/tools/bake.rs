//! Tools that read and change how the Bake light step bakes: the project's
//! lightmap size, samples and denoiser, and which assets skip the bake.

use maquette_core::pipeline::bake::{BakePatch, BakeView};
use maquette_core::prepare::{self, PrepPatch};
use maquette_core::project;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::pipeline;
use crate::commands::prepare as prep_commands;
use crate::mcp::host::Host;

use super::materials::find_asset;
use super::{McpTools, Out, ProjectArgs, reason, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetBakeArgs {
    /// The project id.
    pub project: String,
    /// The lightmap edge in pixels: 256, 512, 1024, 2048 or 4096.
    pub size: Option<u32>,
    /// Cycles samples per lightmap, 1 to 4096.
    pub samples: Option<u32>,
    /// Run OpenImageDenoise over each lightmap.
    pub denoise: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SkipBakeArgs {
    /// The project id.
    pub project: String,
    /// The asset ids as list_assets returns them, or the assets' names.
    pub assets: Vec<String>,
    /// true leaves the assets out of the bake, false bakes them again.
    pub skip: bool,
}

const NOTE: &str = "The settings are the Bake light step's --size, --samples and --denoise args in pipeline.json; the next run_pipeline bakes with them. Time grows about linearly with the samples and the number of baked meshes. Assets that skip the bake keep their own material lighting and ship no lightmap and no second UV set; meshes whose materials glow on their own (an emissive sky) are never baked.";

fn bake_value(project: &str, bake: Option<&BakeView>) -> Result<Value, String> {
    let bake = bake.ok_or_else(|| {
        format!("{project}'s pipeline has no Bake light step that runs tools/bake_light.py, so it has no bake settings.")
    })?;
    Ok(json!({
        "project": project,
        "size": bake.settings.size,
        "samples": bake.settings.samples,
        "denoise": bake.settings.denoise,
        "canDenoise": bake.can_denoise,
        "script": to_value(bake.script),
        "skipped": bake.skipped,
        "note": NOTE,
    }))
}

/// The asset ids for `wanted` (ids or names), refusing the first unknown.
fn asset_ids(host: &dyn Host, project_id: &str, wanted: &[String]) -> Result<Vec<String>, String> {
    let (root, config) = workspace(host)?;
    let manifest = project::load(&root, &config.layout, project_id).map_err(reason)?;
    wanted
        .iter()
        .map(|w| find_asset(&manifest, w).map(|a| a.id.clone()))
        .collect()
}

#[tool_router(router = bake_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get a project's bake settings: the lightmap size in pixels, the Cycles samples, whether each lightmap is denoised, whether its tools/bake_light.py can denoise (an older copy cannot until it is updated), how that script compares with the shipped starter (current, outdated, edited or missing), and the assets set to skip the bake.",
        annotations(read_only_hint = true)
    )]
    async fn get_bake_settings(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = pipeline::view(host.session(), host.runner(), &args.project)
                    .map_err(|e| e.message)?;
                bake_value(&args.project, view.bake.as_ref())
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Set a project's bake settings, as the Pipeline view's Bake settings do: size (256, 512, 1024, 2048 or 4096 px), samples (1 to 4096) and denoise. Only the given fields change. Denoise on an older tools/bake_light.py is refused until the script is updated. It spends nothing; the next run bakes with them. Returns the settings like get_bake_settings.",
        annotations(idempotent_hint = true)
    )]
    async fn set_bake_settings(&self, Parameters(args): Parameters<SetBakeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let patch = BakePatch {
                    size: args.size,
                    samples: args.samples,
                    denoise: args.denoise,
                };
                if patch == BakePatch::default() {
                    return Err("Give size, samples or denoise to change.".to_string());
                }
                let view =
                    pipeline::save_bake(host.session(), host.runner(), &args.project, &patch)
                        .map_err(|e| e.message)?;
                host.pipeline_changed(&args.project);
                bake_value(&args.project, view.bake.as_ref())
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Set whether assets skip the bake, as the Inspector's Skip bake switch does, for one or more assets at once. Skipped assets keep their own material lighting and ship with no lightmap and no second UV set; they still cast light and shadow on the baked ones. A model accepted from the Inbox (a Tripo or Meshy scan) skips the bake unless set otherwise. Nothing is written when an asset is unknown. Build scene and the bake rerun on the next run_pipeline. Returns the assets changed and the project's bake settings like get_bake_settings.",
        annotations(idempotent_hint = true)
    )]
    async fn set_skip_bake(&self, Parameters(args): Parameters<SkipBakeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                if args.assets.is_empty() {
                    return Err("Name at least one asset.".to_string());
                }
                let ids = asset_ids(host, &args.project, &args.assets)?;
                let patch = PrepPatch {
                    skip_bake: Some(Some(args.skip)),
                    ..PrepPatch::default()
                };
                prep_commands::update(host.session(), &args.project, |now| {
                    ids.iter().try_fold(now.clone(), |next, id| {
                        prepare::apply_patch(&next, id, &patch)
                    })
                })
                .map_err(|e| e.message)?;
                host.prepare_changed(&args.project);
                let view = pipeline::view(host.session(), host.runner(), &args.project)
                    .map_err(|e| e.message)?;
                Ok(json!({
                    "project": args.project,
                    "assets": ids,
                    "skip": args.skip,
                    "bake": bake_value(&args.project, view.bake.as_ref()).ok(),
                }))
            })
            .await;
        respond(found)
    }
}
