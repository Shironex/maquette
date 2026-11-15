//! Tools for the inbox: its checks, accepting or discarding a model and thumbnails.

use crate::mcp::tools::ToolError;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use maquette_core::inbox::{self, AcceptRequest};
use maquette_core::thumbs::{self, Requested, ThumbTarget};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::mcp::host::Host;

use super::hints::{step, with_next};
use super::{McpTools, Out, failed, json_result, respond, to_value, workspace};

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
    /// The project that gets the model. By default the project of the asset it
    /// was sent for or attached to (sentFor or aimedAt in list_inbox); another
    /// project is refused unless asset is given too.
    pub project: Option<String>,
    /// An existing asset id to add a version to. Without it, the asset the model
    /// was sent for or attached to, else the asset named `name` (or after the
    /// file), found or created.
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

/// The code of an accept into a project other than the one the model was
/// sent for or attached to, without an asset to say it is meant.
pub(super) const OTHER_PROJECT: &str = "model-for-another-project";

/// The project and asset a model goes to: the one given, else the asset it
/// was sent for, else the one a download was attached to. A project other
/// than its own is refused unless an asset is given too, so a paid model
/// never lands as a new asset in the wrong project by mistake.
fn destination(
    item: &inbox::InboxItem,
    project: Option<String>,
    asset: Option<&str>,
) -> Result<(String, Option<String>), ToolError> {
    let home = item
        .link
        .as_ref()
        .map(|l| (l.project.clone(), l.asset.clone(), l.asset_name.clone()))
        .or_else(|| {
            item.aim
                .as_ref()
                .map(|a| (a.project.clone(), a.asset.clone(), a.asset_name.clone()))
        });
    match (project, home) {
        (Some(project), Some((own, _, name))) if project != own => {
            if asset.is_some() {
                return Ok((project, None));
            }
            Err(ToolError {
                message: format!(
                    "{} is for {name} in {own}, not {project}. Leave project out to accept it there, or give asset to file it in {project} on purpose.",
                    item.id
                ),
                code: OTHER_PROJECT.to_string(),
            })
        }
        (Some(project), Some((_, linked, _))) => Ok((project, Some(linked))),
        (Some(project), None) => Ok((project, None)),
        (None, Some((own, linked, _))) => Ok((own, Some(linked))),
        (None, None) => Err(format!(
            "{} was not sent for or attached to an asset; give project to accept it.",
            item.id
        )
        .into()),
    }
}

/// Accepts inbox item `args.id` into its project, as accept_inbox_model does,
/// and starts its thumbnail.
pub(super) fn accept_item(host: &dyn Host, args: AcceptArgs) -> Result<Value, ToolError> {
    let (root, config) = workspace(host)?;
    // A model from a send goes to the asset it was sent for; the
    // core falls back on the asset a download was attached to.
    let mut item = inbox::item(&root, &config, &args.id)?;
    host.studio().link_inbox(std::slice::from_mut(&mut item));
    let (project, linked) = destination(&item, args.project, args.asset.as_deref())?;
    let request = AcceptRequest {
        item: args.id,
        project: project.clone(),
        asset: args.asset.or(linked),
        name: args.name,
        prep: None,
    };
    let mut result = inbox::accept(&root, &config, &request)?;
    host.studio().accepted(
        &request.item,
        &request.project,
        &result.asset.id,
        result.version.version,
    );
    let target = ThumbTarget {
        project: project.clone(),
        asset: result.asset.id.clone(),
        version: result.version.version,
    };
    let tools = host.toolbox();
    let bake_tools = tools.clone();
    crate::commands::split::spawn_bakes(
        host.studio().clone(),
        host.thumbs().clone(),
        move || bake_tools,
        root.clone(),
        config.clone(),
    );
    let file = PathBuf::from(&result.file);
    if let Some(project) =
        crate::commands::thumbs::for_version(host.thumbs(), &root, &config, &tools, &file, target)
    {
        result.project = project;
    }
    host.project_changed(&result.project);
    Ok(json!({
        "project": project,
        "asset": { "id": result.asset.id, "name": result.asset.name, "stage": to_value(result.asset.stage) },
        "version": result.version.version,
        "file": result.file,
    }))
}

#[tool_router(router = inbox_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List the models waiting in the workspace inbox, which covers every project's models, not only the open one, with their kind, size, triangle count and worst check, the version Accept would make of the asset each goes to (nextVersion) and replacedBy, the newer version of that asset made after the model arrived (a redo that was accepted; the model then only takes room). discarded lists the models moved aside into inbox/.discarded/, most recent first, which restore_inbox_model puts back by name until expiresMs, when the 30 days they are kept run out.",
        annotations(read_only_hint = true)
    )]
    async fn list_inbox(&self) -> Out {
        let found = self
            .blocking(|host, _| {
                let (root, config) = workspace(host)?;
                let mut items = inbox::list(&root, &config)?;
                host.studio().link_inbox(&mut items);
                inbox::place(&root, &config, &mut items);
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
                            "aimedAt": i.aim.as_ref().map(|a| json!({
                                "project": a.project,
                                "asset": a.asset,
                                "assetName": a.asset_name,
                            })),
                            "nextVersion": i.next_version,
                            "replacedBy": i.replaced_by,
                        })
                    })
                    .collect();
                let discarded: Vec<Value> = inbox::list_discarded(&root, &config)?
                    .iter()
                    .map(|d| {
                        json!({
                            "name": d.name,
                            "id": d.id,
                            "bytes": d.bytes,
                            "discardedMs": d.discarded_ms,
                            "expiresMs": d.expires_ms,
                            "assetName": d.asset_name,
                            "replacedBy": d.replaced_by,
                        })
                    })
                    .collect();
                let empty = items.is_empty();
                let answer = json!({
                    "inbox": host.session().display(&inbox::inbox_dir(&root, &config)),
                    "items": items,
                    "discarded": discarded,
                });
                Ok(if empty {
                    with_next(
                        answer,
                        vec![
                            step(
                                "list_handoffs",
                                "See which By hand sends still wait for a model.",
                            ),
                            step(
                                "send_to_3d",
                                "Models land here after a send; by-hand is free.",
                            ),
                        ],
                    )
                } else {
                    answer
                })
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
                let mut item = inbox::item(&root, &config, &args.id)?;
                // As list_inbox does: a model from a send is checked against
                // what the send asked for (its kind, maps and budgets), so
                // both tools report the same checks and worst.
                host.studio().link_inbox(std::slice::from_mut(&mut item));
                Ok(json!({
                    "id": item.id,
                    "name": item.name,
                    "kind": to_value(item.kind),
                    "worst": to_value(item.worst),
                    "sentFor": item.link.as_ref().map(|l| json!({
                        "job": l.job,
                        "project": l.project,
                        "asset": l.asset,
                        "assetName": l.asset_name,
                    })),
                    "checks": to_value(&item.checks),
                    "report": to_value(&item.report),
                    "error": item.error,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Accept a model from the inbox into a project: it moves into the project as a new asset version on the Board, and its thumbnail is rendered headless. project defaults to the project of the asset the model was sent for or attached to (sentFor or aimedAt in list_inbox); another project is refused (code model-for-another-project) unless asset is given too. Without asset it goes to the asset the model was sent for, or the one a download was attached to. A By hand send of that asset still waiting for its model closes as done; a Tripo or Meshy send runs on."
    )]
    async fn accept_inbox_model(&self, Parameters(args): Parameters<AcceptArgs>) -> Out {
        let found = self.blocking(move |host, _| accept_item(host, args)).await;
        respond(found)
    }

    #[tool(
        description = "Discard an inbox model, such as the losing version of a redo or a bad download. The file is not deleted: it moves into inbox/.discarded/ in the workspace (a name already there gets a number) and restore_inbox_model puts it back for 30 days. Its cached checks, picked kind and aim are forgotten. A send that was waiting on this model in the Inbox ends as cancelled with the note Discarded from the Inbox (closedSend is its id, or null); the credits ledger and the asset's earlier versions are not touched. Spends nothing and needs no approval."
    )]
    async fn discard_inbox_model(&self, Parameters(args): Parameters<InboxItemArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let (done, closed) =
                    crate::commands::inbox::discard_model(host.studio(), &root, &config, &args.id)?;
                Ok(json!({ "id": done.id, "file": done.file, "closedSend": closed }))
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
                let found = host.thumbs().request(&dir, &file, None, &tools)?;
                let png = thumbs::png_for(&dir, &file)?;
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
