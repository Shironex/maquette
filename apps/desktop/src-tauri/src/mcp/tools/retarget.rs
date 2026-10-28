//! Tools that put library clips on a rigged model: a headless Blender bake
//! onto a copy of the newest version, reviewed clip by clip, that becomes
//! the model's next version with the failed clips left out. Nothing here
//! spends credits.

use maquette_core::animations::retarget::{ClipSetting, MapChoice, RetargetReview, UseClips};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::animations as commands;

use super::{McpTools, Out, respond, to_short_value, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UseClipArg {
    /// The library clip's id, as list_animations returns it.
    pub clip: String,
    /// Its name in the model, snake_case. Defaults to the clip's name with
    /// its pack's words dropped.
    pub name: Option<String>,
    /// The first and last frame to keep. Defaults to the whole clip.
    pub start: Option<u32>,
    pub end: Option<u32>,
    pub loops: Option<bool>,
    /// Keep the root's travel (true) or play in place (false). Defaults to
    /// whether the clip travels.
    pub root_motion: Option<bool>,
    /// 0.25 to 4. Defaults to 1.
    pub speed: Option<f32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BoneArg {
    /// The humanoid (Mixamo) bone name, like Neck or LeftArm.
    pub bone: String,
    /// The model's bone it drives; leave it out to keep the bone at rest.
    pub target: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UseAnimationsArgs {
    /// The project id.
    pub project: String,
    /// The asset id of a rigged model, as list_assets returns it.
    pub asset: String,
    /// The clips to bake, with their settings on this model.
    pub clips: Vec<UseClipArg>,
    /// Bones to map by hand, over the saved map and the suggestion.
    #[serde(default)]
    pub map: Vec<BoneArg>,
    /// Keep the map for this model's next retarget. Defaults to true.
    pub save_map: Option<bool>,
    /// Only bake and review, without adding the version.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetargetReviewArgs {
    pub project: String,
    pub asset: String,
}

fn review_value(review: &RetargetReview) -> Value {
    json!({
        "clips": review.clips.iter().map(|c| json!({
            "clip": c.clip,
            "name": c.name,
            "status": to_value(c.status),
            "note": c.note,
            "frames": c.frames,
            "travel": c.travel.map(to_short_value),
        })).collect::<Vec<_>>(),
        "seconds": to_short_value(review.seconds),
        "bonesDriven": review.driven,
        "bonesAtRest": review.at_rest,
        "alignedDegrees": to_short_value(review.aligned_deg),
        "hipsScale": to_short_value(review.hips_scale),
    })
}

fn setting(arg: &UseClipArg, base: &ClipSetting) -> ClipSetting {
    ClipSetting {
        clip: arg.clip.clone(),
        name: arg.name.clone().unwrap_or_else(|| base.name.clone()),
        start: arg.start.unwrap_or(base.start),
        end: arg.end.unwrap_or(base.end),
        loops: arg.loops.unwrap_or(base.loops),
        root_motion: arg.root_motion.unwrap_or(base.root_motion),
        speed: arg.speed.unwrap_or(base.speed),
    }
}

#[tool_router(router = retarget_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Put animation library clips on a rigged model in a project. The clip's skeleton is mapped onto the model's rig (the map saved for the model, then the bone names, Mixamo and Tripo names included; map overrides bones by hand, and bones left unmapped stay at rest), each clip gets its name, frames, loop, root motion or in place, and speed on this model only, and Blender bakes them headless onto a copy of the newest version. The review grades each clip good, look (a planted foot slides) or failed (keys in the wrong units, nothing mapped); failed clips are left out and the rest become the model's next version through the usual checks, the older version kept. dryRun bakes and reviews without adding the version. Spends nothing."
    )]
    async fn use_animations(&self, Parameters(args): Parameters<UseAnimationsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                workspace(host)?;
                let ids: Vec<String> = args.clips.iter().map(|c| c.clip.clone()).collect();
                let picked: Vec<MapChoice> = args
                    .map
                    .iter()
                    .map(|b| MapChoice {
                        bone: b.bone.clone(),
                        target: b.target.clone(),
                    })
                    .collect();
                let plan = commands::retarget_plan(
                    host.session(),
                    (&args.project, &args.asset),
                    &ids,
                    &picked,
                )
                .map_err(|e| e.message)?;
                let settings = args
                    .clips
                    .iter()
                    .zip(&plan.settings)
                    .map(|(arg, base)| setting(arg, base))
                    .collect();
                let request = UseClips {
                    project: args.project.clone(),
                    asset: args.asset.clone(),
                    settings,
                    map: picked,
                    save_map: args.save_map.unwrap_or(true),
                };
                let tools = host.toolbox();
                let draft =
                    commands::retarget(host.session(), &tools, &request).map_err(|e| e.message)?;
                let map: Vec<Value> = plan
                    .map
                    .iter()
                    .map(|p| json!({ "bone": p.bone, "source": p.source, "target": p.target, "how": to_value(p.how) }))
                    .collect();
                if args.dry_run || draft.review.baked().next().is_none() {
                    return Ok(json!({
                        "dryRun": args.dry_run,
                        "version": null,
                        "review": review_value(&draft.review),
                        "map": map,
                    }));
                }
                let baked = commands::bake(host.session(), &draft.token).map_err(|e| e.message)?;
                host.project_changed(&baked.project);
                host.library_changed();
                Ok(json!({
                    "version": baked.version.version,
                    "file": baked.version.file,
                    "warnings": baked.warnings,
                    "review": review_value(&baked.review),
                    "map": map,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "The last retarget review of a model: each library clip baked onto it as good, look or failed with a one line note, the bones driven and left at rest, and the version it made. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn get_retarget_review(&self, Parameters(args): Parameters<RetargetReviewArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let dir = maquette_core::project::dir(&root, &config.layout, &args.project)
                    .map_err(|e| e.to_string())?;
                let entry =
                    maquette_core::animations::retarget::uses::of_asset(&dir, &args.asset)
                        .map_err(|e| e.to_string())?;
                let baked: Vec<Value> = entry
                    .baked
                    .iter()
                    .map(|b| json!({ "clip": b.setting.clip, "name": b.setting.name, "version": b.version, "revision": b.revision }))
                    .collect();
                Ok(json!({
                    "version": entry.review_version,
                    "review": entry.review.as_ref().map(review_value),
                    "baked": baked,
                }))
            })
            .await;
        respond(found)
    }
}
