//! The By hand hand-offs tools: which sends wait for a model made on the
//! provider's site, the file name each waits for, and what the Downloads
//! watcher matched, with the downloads waiting for a look or asking which
//! asset they are. attach_download and dismiss_download answer for them;
//! nothing is ever attached without one of those calls, and nothing is
//! sent or spent.

use maquette_core::providers::arrivals::pending_handoffs;
use maquette_core::providers::{Arrival, ArrivalState, Arrivals};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{McpTools, Out, reason, respond, to_value};
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
        description = "List the By hand hand-offs: every send waiting for a model made by a person on the Tripo or Meshy site (job, asset, provider, the export file name it waits for as maq_<asset>_v<n>_<provider>, its handoff folder with send.json, its front ref, about what the site's subscription charges), and the Downloads watcher: whether it is on and watching, and the recent files it saw. Only a file named for its send goes into the inbox by itself. needsLook lists the downloads whose name is only close to a waiting send's (or that came for the only send waiting): each names the asset it looks like and why, and waits. unmatched lists the downloads it could not place. Answer either with attach_download or dismiss_download. Reads only: Maquette never opens or drives the provider's site, and nothing is spent.",
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
                Ok(json!({ "pending": pending, "watcher": watcher }))
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
                let arrival = watcher(host)?
                    .attach(&args.arrival, &args.project, &args.asset)
                    .map_err(reason)?;
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
        description = "Leave a download from list_handoffs alone for good (Not this one for a needsLook download, Not a Maquette model for an unmatched one): nothing is copied, its send keeps waiting, and the watcher never asks about the same file again. The file stays where it is."
    )]
    async fn dismiss_download(&self, Parameters(args): Parameters<DismissDownloadArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let arrival = watcher(host)?.dismiss(&args.arrival).map_err(reason)?;
                Ok(json!({ "arrival": to_value(&arrival) }))
            })
            .await;
        respond(found)
    }
}
