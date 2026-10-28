//! Tools that list projects and assets and edit the scene layout.

use maquette_core::layout::{Desk, LayoutPatch, PlacementPatch, StopPatch};
use maquette_core::project;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::{assets, layout};

use super::motion::{ClipEdit, StatesEdit, clips_patch, states_patch};
use super::paths::{EnergizeEdit, PathEdit, energize_patch, paths_patch};
use super::timeline::{ExplodeEdit, explode_patch};
use super::{
    McpTools, Out, ProjectArgs, explicit, reason, respond, to_short_value, to_value, workspace,
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlacementEdit {
    /// The asset id, as get_layout lists it in `models`.
    pub asset: String,
    /// Where the model's origin stands, in metres: [x, y, z] with y up (glTF axes).
    pub position: Option<[f32; 3]>,
    /// Rotation in degrees about X, then Y, then Z.
    pub rotation: Option<[f32; 3]>,
    /// Uniform scale, 1 is the model's own size.
    pub scale: Option<f32>,
    /// Drop the placement, so the model stands side by side with the other unplaced ones.
    #[serde(default)]
    pub remove: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StopEdit {
    /// An existing stop id changes that stop. Leave it out to add a stop, which then needs
    /// name, position and target.
    pub id: Option<String>,
    pub name: Option<String>,
    /// The camera position in metres, y up.
    pub position: Option<[f32; 3]>,
    /// The point the camera looks at, in metres.
    pub target: Option<[f32; 3]>,
    /// Vertical field of view in degrees, 35 when a new stop leaves it out.
    pub fov: Option<f32>,
    /// The asset ids this stop counts for the first view budget, set by hand. null goes back
    /// to measuring what the camera sees.
    #[serde(default, deserialize_with = "explicit")]
    pub models: Option<Option<Vec<String>>>,
    /// Remove the stop with this id.
    #[serde(default)]
    pub remove: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeskEdit {
    /// Width along X in metres.
    pub width: f32,
    /// Depth along Z in metres.
    pub depth: f32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetLayoutArgs {
    /// The project id.
    pub project: String,
    /// Placements to change. Only the fields you give change.
    #[serde(default)]
    pub models: Vec<PlacementEdit>,
    /// Camera stops to add, change or remove, in this order.
    #[serde(default)]
    pub stops: Vec<StopEdit>,
    /// Every stop id in the new tour order. The first stop is the view the page opens on.
    pub stop_order: Option<Vec<String>>,
    /// The desk the models stand on (its top is at y = 0). null removes it.
    #[serde(default, deserialize_with = "explicit")]
    pub desk: Option<Option<DeskEdit>>,
    /// Object states per asset: each entry replaces that asset's states, transitions and
    /// triggers whole (remove drops them). Parts must be ones get_motion lists for the model.
    #[serde(default)]
    pub states: Vec<StatesEdit>,
    /// Whether animation clips ship. ship false keeps a clip off the site.
    #[serde(default)]
    pub clips: Vec<ClipEdit>,
    /// Explode tracks of the scroll timeline per asset: each entry replaces that asset's
    /// track whole (remove drops it). Keys name camera stops that are in the layout.
    #[serde(default)]
    pub explode: Vec<ExplodeEdit>,
    /// Paths through the scene to add, change or remove, in this order.
    #[serde(default)]
    pub paths: Vec<PathEdit>,
    /// Energize tracks of the scroll timeline per path: each entry replaces that path's track
    /// whole (remove drops it). Keys name camera stops that are in the layout.
    #[serde(default)]
    pub energize: Vec<EnergizeEdit>,
}

impl SetLayoutArgs {
    fn patch(&self) -> LayoutPatch {
        LayoutPatch {
            models: self
                .models
                .iter()
                .map(|m| {
                    let change = (!m.remove).then_some(PlacementPatch {
                        position: m.position,
                        rotation: m.rotation,
                        scale: m.scale,
                    });
                    (m.asset.trim().to_string(), change)
                })
                .collect(),
            stops: self
                .stops
                .iter()
                .map(|s| StopPatch {
                    id: s.id.as_ref().map(|id| id.trim().to_string()),
                    name: s.name.clone(),
                    position: s.position,
                    target: s.target,
                    fov: s.fov,
                    models: s.models.clone(),
                    remove: s.remove,
                })
                .collect(),
            stop_order: self.stop_order.clone(),
            desk: self.desk.as_ref().map(|d| {
                d.as_ref().map(|d| Desk {
                    size: [d.width, d.depth],
                })
            }),
            states: states_patch(&self.states),
            clips: clips_patch(&self.clips),
            explode: explode_patch(&self.explode),
            paths: paths_patch(&self.paths),
            path_tracks: energize_patch(&self.energize),
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CopyAssetArgs {
    /// The project the asset is in.
    pub project: String,
    /// The asset id, as list_assets returns it.
    pub asset: String,
    /// The project id to copy it to.
    pub to: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAssetsArgs {
    /// The project id, as list_projects returns it.
    pub project: String,
    /// Only assets at this Board stage, for example "refs", "model" or "shipped".
    pub stage: Option<String>,
}

#[tool_router(router = projects_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List the projects in the open Maquette workspace: template, target (website or game), site repo on this computer (the game repo for a game; siteRepoMissing is true when one was picked on another computer or the folder is gone, and the app then asks the user where it is), created, asset count, \
the picture the app shows for it, what runs in it now and which one is open in the app. Every other tool takes \
a project id; no tool changes which project the app has open.",
        annotations(read_only_hint = true)
    )]
    async fn list_projects(&self) -> Out {
        let found = self
            .blocking(|host, _| {
                let (root, config) = workspace(host)?;
                let rows = crate::commands::project::summaries(
                    host.session(),
                    host.runner(),
                    host.concepts(),
                    host.studio(),
                )
                .map_err(reason)?;
                // The app opens the remembered project, else the oldest one.
                let current = config
                    .current_project
                    .as_deref()
                    .filter(|id| rows.iter().any(|r| r.id == *id))
                    .or_else(|| rows.first().map(|r| r.id.as_str()))
                    .map(str::to_string);
                let projects: Vec<Value> = rows
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "name": p.name,
                            "template": to_value(p.template),
                            "target": to_value(p.target),
                            "siteRepo": p.site_repo,
                            "siteRepoName": p.site_repo_name,
                            "siteRepoState": to_value(p.site_repo_state),
                            "siteRepoMissing": p.site_repo_state.is_missing(),
                            "createdAt": p.created_at,
                            "assets": p.asset_count,
                            "thumb": p.thumb,
                            "activity": to_value(&p.activity),
                            "current": current.as_deref() == Some(p.id.as_str()),
                        })
                    })
                    .collect();
                Ok(json!({
                    "workspace": host.session().display(&root),
                    "projects": projects,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Get one project's manifest: name, template, target (website or game), site repo (the game repo for a game), source folder, the game settings of a game project and its Board assets with every version.",
        annotations(read_only_hint = true)
    )]
    async fn get_project(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let manifest =
                    project::load(&root, &config.layout, &args.project).map_err(reason)?;
                let dir = project::dir(&root, &config.layout, &args.project).map_err(reason)?;
                Ok(json!({
                    "folder": host.session().display(&dir),
                    "project": to_value(&manifest),
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "List a project's Board assets: kind, stage, status, version count, the latest version's file and stats, and its refs with the view each shows (front, left, back, right or other; refsTagged false means the views are read from the refs' order).",
        annotations(read_only_hint = true)
    )]
    async fn list_assets(&self, Parameters(args): Parameters<ListAssetsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let manifest =
                    project::load(&root, &config.layout, &args.project).map_err(reason)?;
                let dir = project::dir(&root, &config.layout, &args.project).map_err(reason)?;
                let stage = args.stage.as_deref().map(str::to_ascii_lowercase);
                let assets: Vec<Value> = manifest
                    .assets
                    .iter()
                    .filter(|a| {
                        stage
                            .as_deref()
                            .is_none_or(|s| to_value(a.stage).as_str() == Some(s))
                    })
                    .map(|a| {
                        json!({
                            "id": a.id,
                            "name": a.name,
                            "kind": to_value(a.kind),
                            "stage": to_value(a.stage),
                            "status": to_value(a.status),
                            "versions": a.versions.len(),
                            "latest": a.latest().map(to_value),
                            "thumbnail": a.thumbnail,
                            "refs": to_value(crate::commands::refs::asset_refs(&dir, a)),
                            "refsTagged": !a.refs.is_empty(),
                        })
                    })
                    .collect();
                Ok(json!({ "project": manifest.id, "assets": assets }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Get a project's scene layout: where each model stands (position in metres with y up, rotation in degrees X then Y then Z, uniform scale), the camera stops in tour order, the desk, every placeable model with its bounds and size, the first view (what the first stop loads against its budget), the object states with their transitions and triggers, the clips kept off the site (clips with no entry ship), the paths (layout.paths: curves of points in metres the site draws as glowing tubes, with the models their ends follow) and the scroll timeline (layout.timeline.explode: per asset, how far its parts spread apart at each camera stop; layout.timeline.paths: per path, how much of it is lit at each camera stop). A model without a placement stands side by side with the others.",
        annotations(read_only_hint = true)
    )]
    async fn get_layout(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = layout::get(host.session(), &args.project).map_err(|e| e.message)?;
                Ok(to_short_value(&view))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Change a project's scene layout, as the Layout editor does. models changes placements per asset (only the fields you give change; remove drops one). stops adds a stop (name, position, target, optional fov and models), changes one by id, or removes one by id. stopOrder lists every stop id in the new order. desk sets the desk size or null removes it. states sets an asset's object states whole: states in order (the first is the rest pose; each moves named parts by a position offset in metres and a rotation in degrees from rest), transitions between two states (duration, delay and stagger in seconds, easing linear, ease-in, ease-out or ease-in-out) and triggers (stop with a stop id, scroll with start and end percent, click or hover with an optional part); remove drops them. clips sets whether a model's clip ships (ship false keeps it off the site from the next Build scene). explode sets an asset's explode track whole: distance in metres, easing and keys of an amount from 0 (parts together) to 1 (spread the whole distance) at camera stop ids, eased between stops as the page scrolls; remove drops it. paths adds a path (name and 2 to 128 points in metres, optional curve polyline or catmull-rom, width in metres, color and glow as #rrggbb, start and end asset ids its first and last point follow), changes one by id (only the fields you give change) or removes one by id with its track. energize sets a path's energize track whole: easing and keys of an amount from 0 (unlit) to 1 (lit from the first point to the last) at camera stop ids, eased between stops as the page scrolls; remove drops it. Moving a model moves the path ends anchored to it. Removing a stop drops the triggers and keys that wait for it. Unknown assets or stops are refused and nothing changes. The open app updates live. The next Build scene and Render stops use it. Returns the layout like get_layout.",
        annotations(idempotent_hint = true)
    )]
    async fn set_layout(&self, Parameters(args): Parameters<SetLayoutArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = layout::patch(host.session(), &args.project, &args.patch())
                    .map_err(|e| e.message)?;
                host.layout_changed(&args.project);
                Ok(to_short_value(&view))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Copy an asset to another project of the workspace: its refs, every model version, their thumbnails and its finished sends come along under a name and id that are free there. The asset stays where it is. Returns the copy."
    )]
    async fn copy_asset(&self, Parameters(args): Parameters<CopyAssetArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let copied = assets::copy_to(
                    host.session(),
                    host.studio(),
                    &args.project,
                    &args.asset,
                    &args.to,
                )
                .map_err(|e| e.message)?;
                host.project_changed(&copied.to);
                Ok(json!({
                    "project": copied.to.id,
                    "asset": {
                        "id": copied.asset.id,
                        "name": copied.asset.name,
                        "stage": to_value(copied.asset.stage),
                        "versions": copied.asset.versions.len(),
                    },
                    "refs": copied.refs.iter().map(|(_, new)| new.clone()).collect::<Vec<_>>(),
                }))
            })
            .await;
        respond(found)
    }
}
