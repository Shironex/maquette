//! Tools for going through the inbox many models at a time: accepting
//! several in one call and putting a discarded model back.

use crate::mcp::tools::ToolError;
use maquette_core::inbox;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::mcp::host::Host;

use super::inbox::{AcceptArgs, accept_item};
use super::{McpTools, Out, respond, workspace};

/// The most models one accept_inbox_models call takes.
const MAX_BATCH: usize = 200;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AcceptManyArgs {
    /// The inbox item ids (their file names), as list_inbox returns them, 1 to 200.
    pub ids: Vec<String>,
    /// The project that gets every model. Left out, each goes to the project
    /// of the asset it was sent for or attached to, and a model with neither
    /// fails on its own.
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RestoreArgs {
    /// The discarded model's file name, as list_inbox lists it under discarded.
    pub name: String,
}

/// The project inbox model `id` goes to without one given: its send's, else its aim's.
fn home_project(host: &dyn Host, id: &str) -> Result<String, ToolError> {
    let (root, config) = workspace(host)?;
    let mut item = inbox::item(&root, &config, id)?;
    host.studio().link_inbox(std::slice::from_mut(&mut item));
    item.link
        .map(|l| l.project)
        .or(item.aim.map(|a| a.project))
        .ok_or_else(|| {
            format!("{id} was not sent for or attached to an asset; give project to accept it.")
        })
        .map_err(ToolError::from)
}

/// Accepts one model as accept_inbox_model does and answers its result row.
fn accept_one(host: &dyn Host, id: &str, project: Option<&str>) -> Result<Value, ToolError> {
    let project = match project {
        Some(p) => p.to_string(),
        None => home_project(host, id)?,
    };
    let done = accept_item(
        host,
        AcceptArgs {
            id: id.to_string(),
            project: project.clone(),
            asset: None,
            name: None,
        },
    )?;
    Ok(json!({
        "id": id,
        "ok": true,
        "project": project,
        "asset": done["asset"],
        "version": done["version"],
        "file": done["file"],
    }))
}

#[tool_router(router = inbox_triage_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Accept several inbox models in one call (1 to 200 ids), each as accept_inbox_model does: it goes to the asset it was sent for or attached to, as that asset's next version, in that asset's project unless project is given. Each model answers on its own, with ok true and its project, asset, version and file, or ok false and why (an unreadable file, a model that goes nowhere without project), and one failure does not stop the rest; the answer also counts accepted and failed. A By hand send of an accepted asset still waiting closes as done. Spends nothing and needs no approval, like accept_inbox_model."
    )]
    async fn accept_inbox_models(&self, Parameters(args): Parameters<AcceptManyArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                if args.ids.is_empty() || args.ids.len() > MAX_BATCH {
                    return Err(format!("Give 1 to {MAX_BATCH} inbox ids.").into());
                }
                let results: Vec<Value> = args
                    .ids
                    .iter()
                    .map(|id| {
                        accept_one(host, id, args.project.as_deref()).unwrap_or_else(
                            |why| json!({ "id": id, "ok": false, "error": why.message }),
                        )
                    })
                    .collect();
                let accepted = results.iter().filter(|r| r["ok"] == json!(true)).count();
                Ok(json!({
                    "accepted": accepted,
                    "failed": results.len() - accepted,
                    "results": results,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Put a discarded inbox model back into the inbox: name is its file name in inbox/.discarded/, as list_inbox lists it under discarded. It returns under the inbox name it had (a name taken since gets a number) and is aimed at the asset it was going to, so accept_inbox_model goes there again; the send its discard closed stays closed. It answers with its inbox id and file. Spends nothing and needs no approval."
    )]
    async fn restore_inbox_model(&self, Parameters(args): Parameters<RestoreArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let back = inbox::restore(&root, &config, &args.name)?;
                Ok(json!({ "id": back.id, "file": back.file }))
            })
            .await;
        respond(found)
    }
}
