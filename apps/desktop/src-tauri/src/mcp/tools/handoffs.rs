//! The By hand hand-offs tools: which sends wait for a model made on the
//! provider's site, the file name each waits for, and what the Downloads
//! watcher matched, with the downloads waiting for a look or asking which
//! asset they are. attach_download and dismiss_download answer for them;
//! nothing is ever attached without one of those calls, and nothing is
//! sent or spent.

use std::path::Path;

use maquette_core::providers::arrivals::pending_handoffs;
use maquette_core::providers::own_file::{self, OwnFile};
use maquette_core::providers::{Arrival, ArrivalState, Arrivals};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::hints::{step, with_next};
use super::inbox::{AcceptArgs, accept_item};
use super::{McpTools, Out, respond, to_value, workspace};
use crate::mcp::host::Host;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandoffsArgs {
    /// Only this project's hand-offs and downloads.
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachDownloadArgs {
    /// The download's id, from list_handoffs' needsLook or unmatched.
    pub arrival: String,
    /// The project the asset is in.
    pub project: String,
    /// The asset id (or name) that gets the model.
    pub asset: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddModelFileArgs {
    /// The project the asset is in.
    pub project: String,
    /// The asset id (or name) that gets the model as its next version.
    pub asset: String,
    /// The absolute path to a .glb, .gltf, .fbx or .obj outside the
    /// workspace's inbox and the projects' models folders.
    pub path: String,
    /// Accept it into the asset right away (false by default: it waits in
    /// the inbox, checked, for accept_inbox_model).
    pub accept: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DismissDownloadArgs {
    /// The download's id, from list_handoffs' needsLook or unmatched.
    pub arrival: String,
}

fn watcher(host: &dyn Host) -> Result<&Arrivals, String> {
    host.arrivals()
        .ok_or_else(|| "The Downloads watcher does not run here.".to_string())
}

/// The downloads in `state`, for the listed projects.
fn waiting_in(
    arrivals: &[Arrival],
    state: ArrivalState,
    mine: impl Fn(Option<&str>) -> bool,
) -> Vec<Value> {
    arrivals
        .iter()
        .filter(|a| a.state == state && (a.project.is_none() || mine(a.project.as_deref())))
        .map(to_value)
        .collect()
}

#[tool_router(router = handoffs_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List the By hand hand-offs: every send waiting for a model a person makes on the Tripo or Meshy site (job, asset, provider, the export file name it waits for as maq_<asset>_v<n>_<provider>, its handoff folder, its front ref, about what the site's subscription charges), and the Downloads watcher with the recent files it saw. Only a file named for its send lands in the inbox by itself. needsLook lists downloads only close to a waiting send by name, the only send waiting or picture; unmatched lists ones it could not place. Each carries suggestions (waiting sends ranked best first with score, picture and name), picture (queued, ready, failed or skipped) and render (the PNG). Nothing is attached by itself: answer with attach_download or dismiss_download. Reads only: Maquette never opens or drives the provider's site, and nothing is spent.",
        annotations(read_only_hint = true)
    )]
    async fn list_handoffs(&self, Parameters(args): Parameters<HandoffsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let mine = |project: Option<&str>| {
                    args.project.as_deref().is_none_or(|p| project == Some(p))
                };
                let pending: Vec<Value> = pending_handoffs(host.studio())
                    .into_iter()
                    .filter(|h| mine(Some(&h.project)))
                    .map(to_value)
                    .collect();
                let watcher = host.arrivals().map_or(Value::Null, |arrivals| {
                    let status = arrivals.status();
                    let recent: Vec<Value> = status
                        .arrivals
                        .iter()
                        .filter(|a| a.project.is_none() || mine(a.project.as_deref()))
                        .map(to_value)
                        .collect();
                    json!({
                        "enabled": status.enabled,
                        "watching": status.watching,
                        "error": status.error,
                        "needsLook": waiting_in(&status.arrivals, ArrivalState::Review, mine),
                        "unmatched": waiting_in(&status.arrivals, ArrivalState::Unmatched, mine),
                        "arrivals": recent,
                    })
                });
                let empty = pending.is_empty();
                let answer = json!({ "pending": pending, "watcher": watcher });
                Ok(if empty {
                    with_next(
                        answer,
                        vec![step(
                            "send_to_3d",
                            "Nothing waits. Provider by-hand writes a handoff folder for free.",
                        )],
                    )
                } else {
                    answer
                })
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Attach a download from list_handoffs (needsLook or unmatched) to an asset: it goes into the inbox for that asset as a named export does, for the asset's newest waiting By hand send, or aimed at the asset when none waits. The usual checks run there; accept it with accept_inbox_model (its id is inboxFile), which goes to that asset by default and closes the asset's By hand sends still waiting. Check a needsLook download before attaching it: its name was only close. The download itself stays where it is. Spends nothing."
    )]
    async fn attach_download(&self, Parameters(args): Parameters<AttachDownloadArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let arrival = watcher(host)?.attach(&args.arrival, &args.project, &args.asset)?;
                Ok(json!({
                    "inboxFile": arrival.inbox_file,
                    "job": arrival.job,
                    "asset": { "id": arrival.asset, "name": arrival.asset_name },
                    "arrival": to_value(&arrival),
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Add a model file someone made or fixed themselves (an absolute path to a .glb, .gltf, .fbx or .obj, outside the inbox and the projects' models folders) as an asset's next version. It is copied, never moved: a .glb as it is, the others turned into a GLB by headless Blender. It goes into the inbox for the asset as attach_download does, for the asset's newest waiting By hand send when one waits (accepting closes it) or aimed at the asset when none does, and the usual checks run there. With accept true it is accepted into the asset right away and the answer carries the version. Spends nothing."
    )]
    async fn add_model_file(&self, Parameters(args): Parameters<AddModelFileArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let added = own_file::add(
                    host.studio(),
                    host.runner(),
                    &host.toolbox(),
                    &OwnFile {
                        root: &root,
                        config: &config,
                        project: &args.project,
                        asset: &args.asset,
                        file: Path::new(args.path.trim()),
                    },
                )?;
                let accepted = if args.accept.unwrap_or(false) {
                    let accept = AcceptArgs {
                        id: added.inbox_file.clone(),
                        project: args.project.clone(),
                        asset: Some(added.asset.clone()),
                        name: None,
                    };
                    Some(accept_item(host, accept)?)
                } else {
                    None
                };
                Ok(json!({
                    "inboxFile": added.inbox_file,
                    "job": added.job,
                    "asset": { "id": added.asset, "name": added.asset_name },
                    "accepted": accepted,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Leave a download from list_handoffs alone for good (Not this one for a needsLook download, Not a Maquette model for an unmatched one): nothing is copied, its send keeps waiting, and the watcher never asks about the same file again. The file stays where it is."
    )]
    async fn dismiss_download(&self, Parameters(args): Parameters<DismissDownloadArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let arrival = watcher(host)?.dismiss(&args.arrival)?;
                Ok(json!({ "arrival": to_value(&arrival) }))
            })
            .await;
        respond(found)
    }
}
