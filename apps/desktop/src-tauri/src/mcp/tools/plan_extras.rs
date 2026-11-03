//! An agent's extras on a planning canvas: asking the person to remove
//! nodes it may not remove itself (`propose_removal`), and starting a new
//! canvas from a node and the nodes linked to it (`start_from_node`). Only
//! the person answers a proposal, in the app's Changes panel: no tool here
//! or anywhere accepts or dismisses one, an agent's own included.

use maquette_core::plans::{self, agent::Agent, removal};
use rmcp::RoleServer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Peer, schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::json;

use super::plan_edits::write;
use super::plans::{agent_name, project_dir};
use super::{McpTools, Out, respond};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProposeRemovalArgs {
    pub project: String,
    pub canvas: String,
    /// The node ids to ask about, 1 to 100.
    pub nodes: Vec<String>,
    /// Why, in a sentence the person reads beside Keep it and Remove (up to 300 characters).
    #[serde(default)]
    pub reason: String,
    pub agent: Option<String>,
    pub run: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartFromNodeArgs {
    pub project: String,
    /// The canvas the node is on.
    pub canvas: String,
    /// The node id to start from. Not a camera stop.
    pub node: String,
    /// The new canvas's name, one line; left out, the node's name.
    pub name: Option<String>,
    pub agent: Option<String>,
}

#[tool_router(router = plan_extras_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Ask the person to remove nodes you may not remove yourself (a person's node, or one a person wrote in): the canvas does not change. The person sees the nodes marked and your reason in the Changes panel and answers Keep it or Remove; a Remove is their own change, which they can undo. No tool answers a proposal, yours included: only the person decides. A proposal goes stale, and Remove is refused, when a node's content changes or it goes before the person answers (moving it does not). At most 50 proposals wait per canvas, 20 of them under one agent name; get_canvas lists the open ones. Your own nodes you remove with remove_nodes. Answers with the proposal id.",
        annotations(idempotent_hint = false)
    )]
    async fn propose_removal(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<ProposeRemovalArgs>,
    ) -> Out {
        let name = agent_name(args.agent.as_deref(), &peer);
        let (nodes, reason, run) = (args.nodes, args.reason, args.run);
        write(self, args.project, args.canvas, move |dir, canvas| {
            let who = Agent {
                name: &name,
                run: run.as_deref(),
            };
            let line = removal::propose(dir, canvas, &who, &nodes, &reason)?;
            Ok(json!({ "proposal": line.id, "summary": line.summary }))
        })
        .await
    }

    #[tool(
        description = "Start a new planning canvas from a node: the node and the nodes linked to it are copied (with their links among them, the node at 0,0) into a new canvas, as one change carrying your name that the person or you can undo. The source canvas is never changed: nothing is moved or removed from it. Asset and Lab direction nodes stay links by id; copies keep who made them, so a person's note stays theirs. The new canvas takes the source's preset (a brainstorm from the camera plan). A camera stop cannot start one, and linked stops are left out (leftOut). Answers with the new canvas id, its node and link counts and the change id.",
        annotations(idempotent_hint = false)
    )]
    async fn start_from_node(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<StartFromNodeArgs>,
    ) -> Out {
        let agent = agent_name(args.agent.as_deref(), &peer);
        let found = self
            .blocking(move |host, _| {
                let (dir, _) = project_dir(host, &args.project)?;
                let started = plans::start_from(
                    &dir,
                    &args.canvas,
                    &args.node,
                    args.name.as_deref(),
                    Some(&agent),
                )?;
                let made = started.changed.doc;
                host.plan_changed(&args.project, &made.id);
                let mut out = json!({
                    "id": made.id, "name": made.plan.name, "preset": made.plan.preset,
                    "nodes": made.plan.nodes.len(), "links": made.plan.edges.len(),
                    "change": started.changed.entry.map(|e| e.id),
                });
                if !started.left_out.is_empty() {
                    out["leftOut"] = json!(started.left_out);
                }
                Ok(out)
            })
            .await;
        respond(found)
    }
}
