//! Tools that star Lab concepts and pick them onto the Board, the agent's
//! side of Star and Send picks to Refs. They spend nothing.

use maquette_core::concepts::{self, Direction, ImageRef, PickRequest, Picked};
use maquette_core::{AssetKind, project};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{McpTools, Out, reason, respond, to_value, workspace};
use crate::mcp::host::Host;

/// What a new asset becomes on the Board.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum KindArg {
    Prop,
    Character,
    Scene,
}

impl From<KindArg> for AssetKind {
    fn from(value: KindArg) -> Self {
        match value {
            KindArg::Prop => AssetKind::Prop,
            KindArg::Character => AssetKind::Character,
            KindArg::Scene => AssetKind::Scene,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PickArg {
    /// The concept as get_concepts lists it, relative to the project folder, for example
    /// "concepts/stone-lantern/stone-lantern_02.png". Or give direction and index instead.
    pub file: Option<String>,
    /// Without file: the direction, by id or name.
    pub direction: Option<String>,
    /// Without file: the image's index in that direction, as get_concepts lists it.
    pub index: Option<u32>,
    /// true stars the image, false unstars it. Left out, the star stays.
    pub star: Option<bool>,
    /// The asset's name, 80 characters at most. By default the object's name in a direction
    /// of object refs, else the direction's name. Images with the same name join one asset.
    /// A new asset's name starts with a capital letter ("desk lamp" makes "Desk lamp").
    pub name: Option<String>,
    /// "prop", "character" or "scene" (not for object refs), for a new asset only. By default
    /// the kind of an object refs direction, else "prop".
    pub kind: Option<KindArg>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PickConceptsArgs {
    /// The project id.
    pub project: String,
    /// The images to pick, 1 to 24.
    pub images: Vec<PickArg>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StarArg {
    /// The concept as get_concepts lists it, relative to the project folder. Or give
    /// direction and index instead.
    pub file: Option<String>,
    /// Without file: the direction, by id or name.
    pub direction: Option<String>,
    /// Without file: the image's index in that direction.
    pub index: Option<u32>,
    /// true stars the image, false unstars it.
    pub starred: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StarConceptsArgs {
    /// The project id.
    pub project: String,
    /// The images to star or unstar, 1 to 24.
    pub images: Vec<StarArg>,
}

/// An image by its file, else by direction and index.
fn image_ref(
    file: Option<String>,
    direction: Option<String>,
    index: Option<u32>,
) -> Result<ImageRef, String> {
    match (file.filter(|f| !f.trim().is_empty()), direction, index) {
        (Some(file), _, _) => Ok(ImageRef::File(file)),
        (None, Some(direction), Some(index)) => Ok(ImageRef::Numbered { direction, index }),
        _ => Err("Give each image a file, or a direction and an index.".to_string()),
    }
}

fn direction_value(d: &Direction) -> Value {
    let starred: Vec<&str> = d
        .images
        .iter()
        .filter(|i| i.starred)
        .map(|i| i.file.as_str())
        .collect();
    json!({ "id": d.id, "name": d.name, "starred": starred })
}

/// The answer to pick_concepts: every image, and each asset once with the
/// refs this call gave it.
fn picked_value(project_id: &str, picked: &Picked) -> Value {
    let images: Vec<Value> = picked
        .images
        .iter()
        .map(|i| {
            json!({
                "file": i.file,
                "direction": i.direction,
                "index": i.index,
                "refFile": i.ref_file,
                "copied": i.copied,
                "asset": i.asset.id,
            })
        })
        .collect();
    let mut assets: Vec<Value> = Vec::new();
    for image in &picked.images {
        let a = &image.asset;
        if let Some(seen) = assets.iter_mut().find(|v| v["id"] == json!(a.id)) {
            if let Some(refs) = seen["refs"].as_array_mut()
                && !refs.contains(&json!(image.ref_file))
            {
                refs.push(json!(image.ref_file));
            }
            continue;
        }
        assets.push(json!({
            "id": a.id,
            "name": a.name,
            "kind": to_value(a.kind),
            "stage": to_value(a.stage),
            "thumbnail": a.thumbnail,
            "refs": [image.ref_file],
        }));
    }
    json!({
        "project": project_id,
        "images": images,
        "assets": assets,
        "directions": picked.directions.iter().map(direction_value).collect::<Vec<_>>(),
        "note": "Send an asset with send_to_3d or several with send_batch_to_3d; by default they take the refs listed here.",
    })
}

fn pick(host: &dyn Host, args: PickConceptsArgs) -> Result<Value, String> {
    let (root, config) = workspace(host)?;
    let requests = args
        .images
        .into_iter()
        .map(|i| {
            Ok(PickRequest {
                image: image_ref(i.file, i.direction, i.index)?,
                star: i.star,
                name: i.name,
                kind: i.kind.map(AssetKind::from),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let picked =
        concepts::pick_images(&root, &config.layout, &args.project, &requests).map_err(reason)?;
    host.project_changed(&picked.project);
    for direction in &picked.directions {
        host.concepts_changed(&args.project, direction);
    }
    Ok(picked_value(&args.project, &picked))
}

fn star(host: &dyn Host, args: StarConceptsArgs) -> Result<Value, String> {
    let (root, config) = workspace(host)?;
    project::load(&root, &config.layout, &args.project).map_err(reason)?;
    let dir = project::dir(&root, &config.layout, &args.project).map_err(reason)?;
    let stars = args
        .images
        .into_iter()
        .map(|i| Ok((image_ref(i.file, i.direction, i.index)?, i.starred)))
        .collect::<Result<Vec<_>, String>>()?;
    let changed = concepts::star_images(&dir, &stars).map_err(reason)?;
    for direction in &changed {
        host.concepts_changed(&args.project, direction);
    }
    Ok(json!({
        "project": args.project,
        "directions": changed.iter().map(direction_value).collect::<Vec<_>>(),
    }))
}

#[tool_router(router = picks_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Pick Lab concepts onto the Board, as Send picks to Refs does in the app: each image is copied into the project's refs/ folder and becomes, or joins, a Board asset at the Refs stage (an existing asset of that name moves up from Concept). By default an object ref becomes an asset named after its object, any other concept the asset named after its direction; a new asset's name starts with a capital letter (\"desk lamp\" makes \"Desk lamp\"), an existing one keeps its own. An image picked before keeps its ref and answers with the same asset, so calling again makes no duplicates. Can star the images too. Every image is checked before anything is written; it spends nothing. Returns each image's ref file and asset, and each asset once with its refs, ready for send_to_3d.",
        annotations(idempotent_hint = true, destructive_hint = false)
    )]
    async fn pick_concepts(&self, Parameters(args): Parameters<PickConceptsArgs>) -> Out {
        respond(self.blocking(move |host, _| pick(host, args)).await)
    }

    #[tool(
        description = "Star or unstar Lab concepts, as Star does in the app, to shortlist them. Nothing goes to the Board; pick_concepts does that. Every image is checked before any star changes.",
        annotations(idempotent_hint = true, destructive_hint = false)
    )]
    async fn star_concepts(&self, Parameters(args): Parameters<StarConceptsArgs>) -> Out {
        respond(self.blocking(move |host, _| star(host, args)).await)
    }
}
