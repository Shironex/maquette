//! Tools that show and measure a model without a script of the agent's own:
//! render_views makes the standard pictures of an asset version in the page
//! light, beside its refs, and get_model_facts says what the model measures
//! and where it stands. Both carry the asset's ref images.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use maquette_core::facts::ModelFacts;
use maquette_core::views::cameras::{THREE_QUARTER, stands};
use maquette_core::views::{
    Asked, StandardView, ViewAngle, ViewSet, ViewTarget, ViewedModel, Views, standard_set,
};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::views::{self as commands, ViewsAsk, ViewsPlan};
use crate::mcp::host::Host;

use super::hints::{step, with_next};
use super::{McpTools, Out, ToolError, failed, json_result, respond, to_short_value, to_value};

/// How long render_views waits for Blender before it answers "rendering".
const DEFAULT_WAIT: u32 = 60;
const MAX_WAIT: u32 = 240;
const LIGHT: &str = "The page light (preview_rig.py): the sun and the even grey fill the Bake light step bakes into a model, shown through the page's tone mapping, three.js ACES Filmic. A colour that reads right in these pictures reads right on a page that shows the baked light.";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AngleArgs {
    /// Degrees round from the front towards the model's +X, the right of the front view.
    /// Negative for a ref drawn from the model's other side. -180 to 180.
    pub azimuth: f32,
    /// Degrees up, 0 to 80. Defaults to 22.
    pub elevation: Option<f32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloseUpArgs {
    /// The name of an object of the model (get_parts lists them): it and everything under it
    /// fills the frame.
    pub node: Option<String>,
    /// Two corners in shares of the bounding box, [[x0, y0, z0], [x1, y1, z1]], each 0 to 1: x
    /// from the left of the front view to its right, y from bottom to top, z from back to front.
    #[serde(rename = "box")]
    pub region: Option<[[f32; 3]; 2]>,
    /// 0 or 1: a half of the model along its longest side (left then right, top then bottom,
    /// front then back).
    pub half: Option<u8>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderViewsArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// The version to render. The newest when left out.
    pub version: Option<u32>,
    /// Another version of the asset, rendered with the very same cameras for a side by side
    /// (the cameras are fitted to both models together).
    pub compare: Option<u32>,
    /// The three quarter view's angle, to match a ref drawn from elsewhere. Defaults to 38
    /// degrees round and 22 up, the front right view object refs are asked for.
    pub angle: Option<AngleArgs>,
    /// What the two close ups frame, each { node } or { box } or { half }. Left out, they are
    /// the two halves of the model.
    #[serde(default)]
    pub close_ups: Vec<CloseUpArgs>,
    /// Renders the model files as they are, without the asset's fix-ups and real size.
    /// Defaults to false.
    #[serde(default)]
    pub as_is: bool,
    /// Stops the render of this set that is running, and renders nothing. Defaults to false.
    #[serde(default)]
    pub cancel: bool,
    /// Seconds to wait for Blender, 60 by default and 240 at most.
    pub wait_seconds: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModelFactsArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// The version to measure. The newest when left out.
    pub version: Option<u32>,
}

impl CloseUpArgs {
    fn target(&self) -> Result<ViewTarget, String> {
        match (&self.node, self.region, self.half) {
            (Some(node), None, None) => Ok(ViewTarget::Node(node.clone())),
            (None, Some(region), None) => Ok(ViewTarget::Box(region)),
            (None, None, Some(half)) => Ok(ViewTarget::Half(half)),
            _ => Err("A close up takes exactly one of node, box and half.".to_string()),
        }
    }
}

impl RenderViewsArgs {
    fn views(&self) -> Result<Vec<StandardView>, ToolError> {
        let angle = self.angle.as_ref().map(|a| ViewAngle {
            azimuth: a.azimuth,
            elevation: a.elevation.unwrap_or(THREE_QUARTER.elevation),
        });
        let close_ups = self
            .close_ups
            .iter()
            .map(CloseUpArgs::target)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(standard_set(angle, &close_ups)?)
    }
}

fn service(host: &dyn Host) -> Result<Views, ToolError> {
    host.views()
        .cloned()
        .ok_or_else(|| ToolError::from("The standard views are not available in this session."))
}

/// What a view is for, by its id.
fn about(id: &str) -> &'static str {
    match id {
        "three-quarter" => "The view object refs are drawn from: hold it beside the ref.",
        "front" => "The model's +Z side, which a front ref shows and which glTF calls the front.",
        "top" => "The footprint from above, the front at the bottom of the picture.",
        _ => "A closer look, from the three quarter side.",
    }
}

fn shots(model: &ViewedModel) -> Vec<Value> {
    model
        .shots
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "file": s.file,
                "about": about(&s.id),
                "camera": stands(s.azimuth, s.elevation),
                "frames": s.frames,
                "position": to_short_value(s.position),
                "target": to_short_value(s.target),
                "fov": to_short_value(s.fov),
            })
        })
        .collect()
}

/// What an agent reads about a set, finished or on its way.
fn answer(plan: &ViewsPlan, set: Option<&ViewSet>) -> Value {
    let found = &plan.found;
    let refs: Vec<Value> = found
        .refs
        .iter()
        .map(|r| {
            json!({
                "file": found.project_dir.join(&r.file),
                "view": to_value(r.view),
            })
        })
        .collect();
    let prepared = if plan.prepared {
        "The asset's fix-ups and real size are applied: the copy Build scene would place, in metres."
    } else {
        "The model file as it is: the asset has no fix-ups and no real size, or asIs left them out."
    };
    let mut out = json!({
        "project": plan.project,
        "asset": found.asset.id,
        "name": found.asset.name,
        "version": found.version.version,
        "compare": plan.compared.as_ref().map(|c| c.version),
        "status": if set.is_some() { "ready" } else { "rendering" },
        "dir": plan.request.dir,
        "size": plan.request.size,
        "prepared": prepared,
        "refs": refs,
        "light": LIGHT,
        "axes": maquette_core::facts::AXES,
    });
    let Some(set) = set else {
        out["note"] = json!(
            "Blender is still rendering. Call render_views again with the same arguments: it joins this render and answers with the pictures once they are in dir."
        );
        return out;
    };
    let lead = if plan.request.refs.is_empty() {
        "no ref leads it, since the asset has none"
    } else {
        "the asset's refs first"
    };
    out["views"] = json!(set.models.first().map(shots).unwrap_or_default());
    if let (Some(other), Some(compared)) = (set.models.get(1), &plan.compared) {
        out["compared"] = json!({
            "version": compared.version,
            "views": shots(other),
            "note": "Rendered with the same cameras as views, shot for shot.",
        });
    }
    out["sheet"] = json!({
        "file": set.sheet,
        "shows": format!(
            "Every picture on one PNG: {lead}, then the views in the order listed{}.",
            if set.models.len() > 1 { ", a row per version with the version asked for on top" } else { "" }
        ),
    });
    out["boundsRendered"] = json!(
        set.models
            .iter()
            .map(|m| json!({
                "version": m.version,
                "min": to_short_value(m.min),
                "max": to_short_value(m.max),
                "triangles": m.triangles,
            }))
            .collect::<Vec<_>>()
    );
    out["lightNumbers"] = to_short_value(&set.light);
    out["seconds"] = json!(set.seconds);
    with_next(
        out,
        vec![step(
            "get_model_facts",
            "The model's real size, its box, its counts, its facing and its layout entry.",
        )],
    )
}

fn facts_answer(found: &ModelFacts) -> Value {
    with_next(
        to_short_value(found),
        vec![step(
            "render_views",
            "See it from the standard views in the page light, beside its refs.",
        )],
    )
}

impl McpTools {
    /// Every prepare tool: materials, real size and fix-ups, parts, and the
    /// views and facts here.
    pub(super) fn prepare_routers() -> ToolRouter<Self> {
        Self::materials_router()
            + Self::prepare_router()
            + Self::parts_router()
            + Self::views_router()
    }
}

#[tool_router(router = views_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Render an asset version from the standard views, headless in Blender, in the light of its page: three-quarter (front right, the angle refs are drawn from; angle moves it), front, top (straight down, the front at the bottom) and two close ups (the two halves of the model, or closeUps names a part or a box). 1024 px PNGs, built models and provider models alike. They go to the project's exports/renders/<asset>/views/v<n>/ (another angle or closeUps: v<n>-<8 hex>/, a folder of their own), never outside the workspace, with sheet.png (every picture on one PNG, the asset's refs first) and views.json. compare renders another version with the same cameras into v<n>-vs-v<m>/, for a side by side of a rebuild and the model it replaces. The asset's fix-ups and real size are applied, as Build scene places it (asIs leaves them out). Answers each picture's path, the camera that took it and what it frames, and the paths of the asset's refs. A set that is already there answers at once; a render takes about a minute, and past waitSeconds the answer says rendering: call again with the same arguments. cancel stops it. Spends no credits and needs no approval."
    )]
    async fn render_views(&self, Parameters(args): Parameters<RenderViewsArgs>) -> Out {
        let wait = Duration::from_secs(u64::from(
            args.wait_seconds.unwrap_or(DEFAULT_WAIT).min(MAX_WAIT),
        ));
        let cancel = args.cancel;
        let started = self
            .blocking(move |host, _| {
                let views = service(host)?;
                let asset = super::prepare::asset_id(host, &args.project, &args.asset)?;
                let ask = ViewsAsk {
                    project: &args.project,
                    asset: &asset,
                    version: args.version,
                    compare: args.compare,
                    as_is: args.as_is,
                    views: args.views()?,
                };
                if cancel {
                    let dir = commands::set_folder(host.session(), &ask)?;
                    let stopped = views.cancel(&dir);
                    return Ok(Started::Cancelled(dir, stopped));
                }
                let tools = host.toolbox();
                let plan = commands::plan(host.session(), &tools, ask, views.size())?;
                let asked = views.request(&plan.request, &tools)?;
                Ok(Started::Asked(Box::new((views, plan, asked))))
            })
            .await;
        let (views, plan, asked) = match started {
            Err(err) => return failed(err),
            Ok(Started::Cancelled(dir, stopped)) => {
                return json_result(&json!({
                    "status": if stopped { "cancelled" } else { "nothing was rendering" },
                    "dir": dir,
                }));
            }
            Ok(Started::Asked(all)) => *all,
        };
        if let Asked::Ready(set) = asked {
            return json_result(&answer(&plan, Some(&set)));
        }
        let deadline = Instant::now() + wait;
        while views.is_running(&plan.request.dir) {
            if Instant::now() >= deadline {
                return json_result(&answer(&plan, None));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        // The run is over: the set is in its folder, or the service says why not.
        match views.outcome(&plan.request) {
            Ok(Some(set)) => json_result(&answer(&plan, Some(&set))),
            Ok(None) => json_result(&json!({
                "status": "cancelled",
                "dir": plan.request.dir,
                "note": "The render was stopped before it finished. Call render_views again to render the set.",
            })),
            Err(err) => failed(err),
        }
    }

    #[tool(
        description = "Get the measured facts of an asset version, built or from a provider, without Blender: counts (triangles, vertices, meshes, objects a page can move by name, primitives, materials), inFile (the model file's own box), realSize (what prepare.json holds, the scale Build scene gives the file to reach it and the origin fix-up's shift), atRealSize (the box the layout places, metres), facing (the axis its front points along in the file and in the scene, with how it was decided, rig or convention, and how sure, measured or assumed: a prop's front is never guessed silently), layout (the entry in layout.json: position, rotation in degrees, scale, and the box it fills in the scene), refs (the asset's ref images with their whole paths and views) and notes that say what to make of the numbers. axes says how to read them: glTF and three.js, y up, front +z, which Blender imports as -y. A per asset tool rather than fields on list_assets, since it reads the model file. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn get_model_facts(&self, Parameters(args): Parameters<ModelFactsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let asset = super::prepare::asset_id(host, &args.project, &args.asset)?;
                let facts =
                    commands::model_facts(host.session(), &args.project, &asset, args.version)?;
                Ok(facts_answer(&facts))
            })
            .await;
        respond(found)
    }
}

/// What starting render_views led to.
enum Started {
    Cancelled(PathBuf, bool),
    Asked(Box<(Views, ViewsPlan, Asked)>),
}
