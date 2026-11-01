//! Tools for an asset's refs and their views: read and set the views,
//! draw the missing ones from the front, clean a ref's background and take
//! a candidate into the refs. The drawing and cleaning are Lab jobs on the
//! Codex lane, the same as start_concepts.

use crate::mcp::tools::ToolError;
use maquette_core::concepts::{CleanGround, NewCleanRef, NewOtherViews};
use maquette_core::refs::{AssetRef, RefView};
use maquette_core::{assets, project};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::concepts::job_value;
use super::image_api::{BackendArg, backend};
use super::{McpTools, Out, respond, to_value, workspace};
use crate::commands::refs as shared;
use crate::mcp::host::Host;

/// The view a ref shows.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ViewArg {
    Front,
    Left,
    Back,
    Right,
    Other,
}

impl From<ViewArg> for RefView {
    fn from(value: ViewArg) -> Self {
        match value {
            ViewArg::Front => RefView::Front,
            ViewArg::Left => RefView::Left,
            ViewArg::Back => RefView::Back,
            ViewArg::Right => RefView::Right,
            ViewArg::Other => RefView::Other,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefViewArg {
    /// The ref, relative to the project folder, for example "refs/frost_lantern.png".
    pub file: String,
    pub view: ViewArg,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetRefViewsArgs {
    /// The project id.
    pub project: String,
    /// The asset id, as list_assets returns it.
    pub asset: String,
    /// Every ref of the asset with its view. At most one per front, left, back and right;
    /// "other" refs stay with the asset but never go as views. A ref left out is no longer
    /// the asset's (the file stays in refs/).
    pub refs: Vec<RefViewArg>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MakeViewsArgs {
    /// The project id.
    pub project: String,
    /// The asset id, as list_assets returns it.
    pub asset: String,
    /// "left", "back" or "right" to draw. By default every empty slot; naming a filled one
    /// draws it again.
    #[serde(default)]
    pub views: Vec<ViewArg>,
    /// Candidates per view, 1 to 4. 2 by default.
    pub count: Option<u32>,
    /// false draws them without the project's style board. On by default.
    pub use_style: Option<bool>,
    /// Extra instructions for every view, for example "the sword in the left hand" or "the
    /// cape over the right shoulder". One line, at most 400 characters; a retry keeps them.
    pub notes: Option<String>,
    /// "codex" or "openai-images" (OpenAI's Image API, paid per image from the OpenAI key,
    /// within the monthly cap). By default the backend Settings picks for this kind.
    pub backend: Option<BackendArg>,
}

/// The background a clean copy gets.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum GroundArg {
    StudioGrey,
    Transparent,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CleanRefArgs {
    /// The project id.
    pub project: String,
    /// The asset id, as list_assets returns it.
    pub asset: String,
    /// One of the asset's refs, for example "refs/lantern_kit_crop.png".
    pub file: String,
    /// "studio-grey" (default, the grey of object refs) or "transparent".
    pub background: Option<GroundArg>,
    /// true centers the object and pads the image to a square.
    pub pad: Option<bool>,
    /// true draws what the frame cut off; by default nothing is invented.
    pub extend: Option<bool>,
    /// "codex" or "openai-images" (OpenAI's Image API, paid per image from the OpenAI key,
    /// within the monthly cap). By default the backend Settings picks for this kind.
    pub backend: Option<BackendArg>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UseCandidateArgs {
    /// The project id.
    pub project: String,
    /// The asset id, as list_assets returns it.
    pub asset: String,
    /// A candidate that make_other_views or clean_ref_background made, as get_ref_views
    /// lists it, for example "concepts/frost-lantern-views/frost-lantern-views_01.png".
    pub file: String,
    /// For a drawn view: the slot it fills, by default the view it was drawn for.
    pub view: Option<ViewArg>,
    /// For a drawn view whose slot is filled: true (default) replaces the old ref, which leaves
    /// the asset's refs and moves to refs/originals (it stays in place when another asset
    /// shows it or a send lists it); false keeps it with the asset as other.
    pub replace: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefViewsArgs {
    /// The project id.
    pub project: String,
    /// The asset id, as list_assets returns it.
    pub asset: String,
}

/// An asset's refs as the tools report them.
fn refs_value(view: &shared::RefsView) -> Value {
    let candidates: Vec<Value> = view
        .directions
        .iter()
        .flat_map(|d| {
            d.images.iter().map(move |i| {
                json!({
                    "file": i.file,
                    "kind": to_value(d.refs_job.as_ref().map(|j| j.kind)),
                    "for": i.title,
                    "usedAs": i.ref_file,
                })
            })
        })
        .collect();
    json!({
        "project": view.project.id,
        "asset": view.asset.id,
        "tagged": view.tagged,
        "refs": to_value(&view.refs),
        "candidates": candidates,
        "jobs": view.jobs.iter().filter(|j| !j.state.is_finished()).map(job_value).collect::<Vec<_>>(),
        "note": "Sends carry front, left, back and right in that order; other refs stay home.",
    })
}

fn after(host: &dyn Host, project_id: &str, asset_id: &str) -> Result<Value, ToolError> {
    let view = shared::view(host.session(), host.concepts(), project_id, asset_id)?;
    host.project_changed(&view.project);
    for d in &view.directions {
        host.concepts_changed(project_id, d);
    }
    Ok(refs_value(&view))
}

/// The asset's id, checked to be on the project's Board.
fn check_asset(host: &dyn Host, project_id: &str, asset_id: &str) -> Result<(), ToolError> {
    let (root, config) = workspace(host)?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    assets::find(&manifest, asset_id)?;
    Ok(())
}

#[tool_router(router = refs_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Read an asset's refs and the view each shows (front, left, back, right or other), whether they were tagged (tagged false: read from the refs' order, the first is the front), the candidates make_other_views and clean_ref_background made for it, and their jobs still running.",
        annotations(read_only_hint = true)
    )]
    async fn get_ref_views(&self, Parameters(args): Parameters<RefViewsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view =
                    shared::view(host.session(), host.concepts(), &args.project, &args.asset)?;
                Ok(refs_value(&view))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Set the views of an asset's refs, as the refs sheet does: every ref with its view, at most one per front, left, back and right. Sends then carry front, left, back and right in that order and leave other refs home; By hand handoffs name the files by view. Everything is checked before anything is saved; it spends nothing and the app updates live.",
        annotations(idempotent_hint = true, destructive_hint = false)
    )]
    async fn set_ref_views(&self, Parameters(args): Parameters<SetRefViewsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let refs: Vec<AssetRef> = args
                    .refs
                    .into_iter()
                    .map(|r| AssetRef {
                        file: r.file,
                        view: r.view.into(),
                    })
                    .collect();
                maquette_core::refs::set_views(
                    &root,
                    &config.layout,
                    &args.project,
                    &args.asset,
                    &refs,
                )?;
                after(host, &args.project, &args.asset)
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Make the other views of an asset: the image backend draws the views it is missing (left, back, right) from its front ref, count candidates each, with the front attached and the project's style board unless useStyle is false. Filled views are skipped unless named. notes adds extra instructions to every view's prompt (\"the sword in the left hand\"). The candidates land in the Lab direction \"<asset> views\"; nothing reaches the refs until use_ref_candidate. The backend is the one Settings picks for other views (get_image_backend), or the one backend names: codex spends Codex usage; openai-images is paid per image from the OpenAI key within the monthly cap, and since an agent asked, the round waits for a person to approve it in the Maquette app: the answer is \"waiting for approval\" with approval.estimateUsd and the jobs are in state approval, so poll get_concepts and never start the round again (a denied round's jobs end cancelled)."
    )]
    async fn make_other_views(&self, Parameters(args): Parameters<MakeViewsArgs>) -> Out {
        let project = args.project.clone();
        let found = self
            .blocking(move |host, _| {
                check_asset(host, &args.project, &args.asset)?;
                let request = NewOtherViews {
                    project: args.project.clone(),
                    asset: args.asset.clone(),
                    views: args.views.into_iter().map(RefView::from).collect(),
                    count: args.count.unwrap_or(2),
                    use_style: args.use_style.unwrap_or(true),
                    notes: args.notes,
                };
                let tools = host.toolbox();
                let (direction, jobs) =
                    shared::make_views(host.session(), host.concepts(), &tools, &request, backend(args.backend))?;
                Ok(json!({
                    "status": "started",
                    "direction": { "id": direction.id, "name": direction.name },
                    "styleRevision": direction.style_revision,
                    "jobs": jobs.iter().map(|j| json!({ "id": j.id, "view": j.title, "file": j.file, "state": to_value(j.state), "backend": to_value(j.backend) })).collect::<Vec<_>>(),
                    "note": "Read the candidates with get_ref_views once they are made, then put one in its slot with use_ref_candidate.",
                }))
            })
            .await;
        respond(self.ask_for_round(project, found).await)
    }

    #[tool(
        description = "Clean the background of one of an asset's refs: the image backend makes a copy with the object kept and the clutter removed, on studio grey (default) or transparent, optionally centered and padded to a square, and drawing what the frame cut off only when extend is true. The copy lands in the Lab direction \"<asset> clean\"; use_ref_candidate puts it in the ref's place with its view, and the original moves to refs/originals, kept. The backend is the one Settings picks for clean background (get_image_backend), or the one backend names: codex spends Codex usage; openai-images is paid per image from the OpenAI key within the monthly cap, and since an agent asked, the round waits for a person to approve it in the Maquette app: the answer is \"waiting for approval\" with approval.estimateUsd and the jobs are in state approval, so poll get_concepts and never start the round again (a denied round's jobs end cancelled)."
    )]
    async fn clean_ref_background(&self, Parameters(args): Parameters<CleanRefArgs>) -> Out {
        let project = args.project.clone();
        let found = self
            .blocking(move |host, _| {
                check_asset(host, &args.project, &args.asset)?;
                let request = NewCleanRef {
                    project: args.project.clone(),
                    asset: args.asset.clone(),
                    file: args.file,
                    ground: match args.background {
                        Some(GroundArg::Transparent) => CleanGround::Transparent,
                        _ => CleanGround::StudioGrey,
                    },
                    pad: args.pad.unwrap_or(false),
                    extend: args.extend.unwrap_or(false),
                };
                let tools = host.toolbox();
                let (direction, jobs) =
                    shared::clean(host.session(), host.concepts(), &tools, &request, backend(args.backend))?;
                Ok(json!({
                    "status": "started",
                    "direction": { "id": direction.id, "name": direction.name },
                    "jobs": jobs.iter().map(|j| json!({ "id": j.id, "file": j.file, "state": to_value(j.state), "backend": to_value(j.backend) })).collect::<Vec<_>>(),
                    "note": "Once it is made, put the copy in place with use_ref_candidate; until then the original stays as it is.",
                }))
            })
            .await;
        respond(self.ask_for_round(project, found).await)
    }

    #[tool(
        description = "Take a candidate into an asset's refs: a drawn view goes into refs/ and into its slot, a clean copy takes the place and view of the ref it cleans while the original moves to refs/originals. A drawn view replaces the ref that held its slot: that ref leaves the asset's refs and moves to refs/originals, or stays in place when another asset shows it or a send lists it; replace false keeps it as other instead. replaced says what moved where. The other candidates stay in the Lab. Spends nothing; the app updates live.",
        annotations(destructive_hint = false)
    )]
    async fn use_ref_candidate(&self, Parameters(args): Parameters<UseCandidateArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (_, replaced) = shared::use_candidate(
                    host.session(),
                    host.studio(),
                    &args.project,
                    &args.asset,
                    &args.file,
                    args.view.map(RefView::from),
                    args.replace.unwrap_or(true),
                )?;
                let mut out = after(host, &args.project, &args.asset)?;
                out["replaced"] = to_value(&replaced);
                Ok(out)
            })
            .await;
        respond(found)
    }
}
