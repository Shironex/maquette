//! The By hand hand-offs tool: which sends wait for a model made on the
//! provider's site, the file name each waits for, and what the Downloads
//! watcher matched. Reads only; nothing is sent, copied or spent.

use maquette_core::providers::arrivals::pending_handoffs;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{McpTools, Out, respond, to_value};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandoffsArgs {
    /// Only this project's hand-offs and downloads.
    pub project: Option<String>,
}

#[tool_router(router = handoffs_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List the By hand hand-offs: every send waiting for a model made by a person on the Tripo or Meshy site (job, asset, provider, the export file name it waits for as maq_<asset>_v<n>_<provider>, its handoff folder with send.json, about what the site's subscription charges), and the Downloads watcher: whether it is on and watching, and the recent files it matched, copied into the inbox, left for a GLB or is asking about. Reads only: Maquette never opens or drives the provider's site, and nothing is spent.",
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
                        "arrivals": recent,
                    })
                });
                Ok(json!({ "pending": pending, "watcher": watcher }))
            })
            .await;
        respond(found)
    }
}
