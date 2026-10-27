//! Tools for the inbox: its checks, accepting a model and thumbnails.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use maquette_core::inbox::{self, AcceptRequest};
use maquette_core::thumbs::{self, Requested, ThumbTarget};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{McpTools, Out, failed, json_result, reason, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InboxItemArgs {
    /// The inbox item id (its file name), as list_inbox returns it.
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AcceptArgs {
    /// The inbox item id (its file name).
    pub id: String,
    /// The project that gets the model.
    pub project: String,
    /// An existing asset id to add a version to. Without it, the asset named
    /// `name` (or after the file) is found or created.
    pub asset: Option<String>,
    /// The asset name, when a new asset is made.
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailArgs {
    /// A .glb or .gltf inside the workspace, absolute or relative to the workspace folder.
    pub file: String,
    /// Seconds to wait for the render, 45 by default and 120 at most.
    pub wait_seconds: Option<u32>,
}

fn resolve_model(root: &Path, file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file.trim());
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    let inside = path
        .canonicalize()
        .ok()
        .zip(root.canonicalize().ok())
        .is_some_and(|(p, r)| p.starts_with(r));
    if !inside {
        return Err(format!(
            "{} is not a file inside the workspace.",
            path.display()
        ));
    }
    if !thumbs::is_renderable(&path) {
        return Err(format!("{} is not a .glb or .gltf model.", path.display()));
    }
    Ok(path)
}

#[tool_router(router = inbox_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List the models waiting in the workspace inbox, with their kind, size, triangle count and worst check.",
        annotations(read_only_hint = true)
    )]
    async fn list_inbox(&self) -> Out {
        let found = self
            .blocking(|host, _| {
                let (root, config) = workspace(host)?;
                let mut items = inbox::list(&root, &config).map_err(reason)?;
                host.studio().link_inbox(&mut items);
                let items: Vec<Value> = items
                    .iter()
                    .map(|i| {
                        json!({
                            "id": i.id,
                            "name": i.name,
                            "kind": to_value(i.kind),
                            "kindGuessed": i.kind_guessed,
                            "kindFromSend": i.kind_from_send,
                            "state": to_value(i.state),
                            "worst": to_value(i.worst),
                            "bytes": i.bytes,
                            "triangles": i.report.as_ref().map(|r| r.triangles),
                            "maxTexture": i.report.as_ref().map(|r| r.max_texture),
                            "error": i.error,
                            "sentFor": i.link.as_ref().map(|l| json!({
                                "job": l.job,
                                "project": l.project,
                                "asset": l.asset,
                                "assetName": l.asset_name,
                            })),
                        })
                    })
                    .collect();
                Ok(json!({
                    "inbox": host.session().display(&inbox::inbox_dir(&root, &config)),
                    "items": items,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Get the checks for one inbox model (triangles, textures, rig, scale, materials) and its analysis report.",
        annotations(read_only_hint = true)
    )]
    async fn get_inbox_checks(&self, Parameters(args): Parameters<InboxItemArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let item = inbox::item(&root, &config, &args.id).map_err(reason)?;
                Ok(json!({
                    "id": item.id,
                    "name": item.name,
                    "kind": to_value(item.kind),
                    "worst": to_value(item.worst),
                    "checks": to_value(&item.checks),
                    "report": to_value(&item.report),
                    "error": item.error,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Accept a model from the inbox into a project: it moves into the project as a new asset version on the Board, and its thumbnail is rendered headless. A By hand send of that asset still waiting for its model closes as done; a Tripo or Meshy send runs on."
    )]
    async fn accept_inbox_model(&self, Parameters(args): Parameters<AcceptArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                // A model from a send goes to the asset it was sent for.
                let mut item = inbox::item(&root, &config, &args.id).map_err(reason)?;
                host.studio().link_inbox(std::slice::from_mut(&mut item));
                let linked = item
                    .link
                    .filter(|l| l.project == args.project)
                    .map(|l| l.asset);
                let request = AcceptRequest {
                    item: args.id,
                    project: args.project.clone(),
                    asset: args.asset.or(linked),
                    name: args.name,
                    prep: None,
                };
                let mut result = inbox::accept(&root, &config, &request).map_err(reason)?;
                host.studio().accepted(
                    &request.item,
                    &request.project,
                    &result.asset.id,
                    result.version.version,
                );
                let target = ThumbTarget {
                    project: args.project,
                    asset: result.asset.id.clone(),
                    version: result.version.version,
                };
                let tools = host.toolbox();
                let file = PathBuf::from(&result.file);
                if let Some(project) = crate::commands::thumbs::for_version(
                    host.thumbs(),
                    &root,
                    &config,
                    &tools,
                    &file,
                    target,
                ) {
                    result.project = project;
                }
                host.project_changed(&result.project);
                Ok(json!({
                    "asset": { "id": result.asset.id, "name": result.asset.name, "stage": to_value(result.asset.stage) },
                    "version": result.version.version,
                    "file": result.file,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Render a studio thumbnail of a model inside the workspace with headless Blender and return the PNG's path. Renders are cached by file content."
    )]
    async fn render_thumbnail(&self, Parameters(args): Parameters<ThumbnailArgs>) -> Out {
        let wait = Duration::from_secs(u64::from(args.wait_seconds.unwrap_or(45).min(120)));
        let requested = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let file = resolve_model(&root, &args.file)?;
                let dir = thumbs::thumbs_dir(&root, &config);
                let tools = host.toolbox();
                let found = host
                    .thumbs()
                    .request(&dir, &file, None, &tools)
                    .map_err(reason)?;
                let png = thumbs::png_for(&dir, &file).map_err(reason)?;
                Ok((found, png))
            })
            .await;
        let (found, png) = match requested {
            Ok(found) => found,
            Err(message) => return failed(message),
        };
        match found {
            Requested::Ready(path) => json_result(&json!({ "status": "ready", "png": path })),
            Requested::Fallback(why) => failed(format!("No Blender render for this model. {why}")),
            Requested::Rendering => {
                let deadline = Instant::now() + wait;
                while Instant::now() < deadline {
                    if png.is_file() {
                        return json_result(&json!({ "status": "ready", "png": png }));
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                json_result(&json!({
                    "status": "rendering",
                    "png": png,
                    "note": "Blender is still rendering. The PNG appears at this path when it is done.",
                }))
            }
        }
    }
}
