//! An agent's edits on a planning canvas through `maquette_core::plans::agent`:
//! add nodes and links (placed beside a node), edit and move nodes, remove
//! only what it made, tidy its own nodes and undo its own changes. Every
//! write carries the agent's name as `by`, goes through the same checks,
//! lock and size cap as the app's saves, lands in the change log and tells
//! an open canvas to reload, which keeps a person's unsaved edits. Agent
//! names are labels, not accounts; only a person's work is protected.

use maquette_core::plans::agent::{self, Agent, Done, NewLink, NewNode, NodeEdit};
use maquette_core::plans::{self, EdgeKind, NodeKind};
use rmcp::RoleServer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Peer, schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::plan_camera::StopChange;
use super::plans::{agent_name, project_dir};
use super::{McpTools, Out, ToolError, respond};
use crate::mcp::host::Host;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeSpec {
    /// note, image, link, asset or direction. Left out, it follows the fields: asset, direction,
    /// image, url, else note.
    pub kind: Option<String>,
    /// A link's title or a picture's caption; a note may have one. Asset and direction nodes
    /// take none: their names are read from the project.
    #[serde(default)]
    pub title: String,
    /// A note's text or a link's short note, up to 20000 characters.
    #[serde(default)]
    pub text: String,
    /// A link node's http or https address.
    pub url: Option<String>,
    /// An image node's picture, a path relative to the project that exists (refs/porch.png).
    pub image: Option<String>,
    /// An asset node's asset id, as list_assets returns it.
    pub asset: Option<String>,
    /// A Lab direction node's direction id, as get_concepts returns it.
    pub direction: Option<String>,
    /// Where it goes, in canvas pixels. Leave both out to have it placed beside near.
    pub x: Option<i32>,
    pub y: Option<i32>,
    /// Width in pixels, 120 to 2000. Left out keeps the kind's width.
    pub width: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkSpec {
    /// A node id on the canvas, or $0, $1 for the nodes of this call in order.
    pub from: String,
    pub to: String,
    /// relates (default), leads-to or source.
    pub kind: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddNodesArgs {
    pub project: String,
    pub canvas: String,
    /// A node id the new nodes relate to: they stack in a column to its right, clear of other
    /// nodes. Left out, they go right of everything.
    pub near: Option<String>,
    /// Up to 100 nodes.
    #[serde(default)]
    pub nodes: Vec<NodeSpec>,
    /// Up to 100 links. A pair linked already is left out and noted.
    #[serde(default)]
    pub links: Vec<LinkSpec>,
    /// Your name as people see it, such as "Research agent". Defaults to your MCP client's name.
    pub agent: Option<String>,
    /// A run id you choose to group several calls, so a person can undo them together.
    pub run: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeChange {
    pub id: String,
    pub title: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
    /// A stop node's camera settings on the camera plan.
    pub stop: Option<StopChange>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNodesArgs {
    pub project: String,
    pub canvas: String,
    /// Up to 100 nodes; only the fields given change.
    pub nodes: Vec<NodeChange>,
    pub agent: Option<String>,
    pub run: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoveNodesArgs {
    pub project: String,
    pub canvas: String,
    /// Node ids you added; each goes with its links.
    #[serde(default)]
    pub nodes: Vec<String>,
    /// Link ids you drew.
    #[serde(default)]
    pub links: Vec<String>,
    pub agent: Option<String>,
    pub run: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LayoutArgs {
    pub project: String,
    pub canvas: String,
    /// Only these of your node ids; left out, all of yours.
    #[serde(default)]
    pub nodes: Vec<String>,
    pub agent: Option<String>,
    pub run: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UndoArgs {
    pub project: String,
    pub canvas: String,
    /// Undo every change of yours in this run that is not undone yet.
    pub run: Option<String>,
    /// Or these change ids, as get_changes and the edit tools return them.
    #[serde(default)]
    pub changes: Vec<String>,
    pub agent: Option<String>,
}

fn parse<T: serde::de::DeserializeOwned>(
    word: &str,
    what: &str,
    valid: &str,
) -> Result<T, ToolError> {
    serde_json::from_value(json!(word))
        .map_err(|_| ToolError::from(format!("{word} is not a {what}: {valid}.")))
}

fn new_node(spec: NodeSpec) -> Result<NewNode, ToolError> {
    let kind = spec
        .kind
        .as_deref()
        .map(|k| parse::<NodeKind>(k, "node kind", "note, image, link, asset or direction"))
        .transpose()?;
    let at = match (spec.x, spec.y) {
        (Some(x), Some(y)) => Some((x, y)),
        (None, None) => None,
        _ => return Err("Give both x and y, or neither to have the node placed.".into()),
    };
    Ok(NewNode {
        kind,
        title: spec.title,
        text: spec.text,
        url: spec.url,
        image: spec.image,
        asset: spec.asset,
        direction: spec.direction,
        at,
        width: spec.width,
    })
}

fn new_link(spec: LinkSpec) -> Result<NewLink, ToolError> {
    let kind = spec
        .kind
        .as_deref()
        .map(|k| parse::<EdgeKind>(k, "link kind", "relates, leads-to or source"))
        .transpose()?
        .unwrap_or(EdgeKind::Relates);
    Ok(NewLink {
        from: spec.from,
        to: spec.to,
        kind,
        label: spec.label,
    })
}

/// The change id and, when some operations did not apply, a word on it.
fn change_fields(done: &Done, out: &mut Value) {
    out["change"] = json!(done.changed.entry.as_ref().map(|e| e.id.clone()));
    if !done.notes.is_empty() {
        out["notes"] = json!(done.notes);
    }
    if let Some(line) = done.tidied.as_ref().and_then(|t| t.entry.as_ref()) {
        out["autoLayout"] = json!({ "change": line.id, "moved": line.ops.len() });
    }
}

/// Runs an agent write on a blocking thread and tells an open canvas.
pub(super) async fn write(
    tools: &McpTools,
    project: String,
    canvas: String,
    job: impl FnOnce(&std::path::Path, &str) -> Result<Value, ToolError> + Send + 'static,
) -> Out {
    let found = tools
        .blocking(move |host: &dyn Host, _| {
            let (dir, _) = project_dir(host, &project)?;
            let out = job(&dir, &canvas)?;
            host.plan_changed(&project, &canvas);
            Ok(out)
        })
        .await;
    respond(found)
}

#[tool_router(router = plan_edits_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Add nodes (note, image, link, asset, direction) and links to a planning canvas. Nodes without x and y are laid out in a column beside near, clear of other nodes, never stacked. Links join node ids or $0, $1 for this call's nodes. Everything is checked before anything is written; it all carries your name (by) and lands in the change log, so a person can undo it. Answers with the new ids and the change id. When the person has Auto layout after agents on (the default), your new nodes that overlap another node then move clear as a change of its own (autoLayout: its change id and how many moved); a person's nodes never move.",
        annotations(idempotent_hint = false)
    )]
    async fn add_nodes(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<AddNodesArgs>,
    ) -> Out {
        let name = agent_name(args.agent.as_deref(), &peer);
        let nodes = args
            .nodes
            .into_iter()
            .map(new_node)
            .collect::<Result<Vec<_>, _>>();
        let links = args
            .links
            .into_iter()
            .map(new_link)
            .collect::<Result<Vec<_>, _>>();
        let (near, run) = (args.near, args.run);
        write(self, args.project, args.canvas, move |dir, canvas| {
            let who = Agent {
                name: &name,
                run: run.as_deref(),
            };
            let done = agent::add(dir, canvas, &who, near.as_deref(), &nodes?, &links?)?;
            let mut out =
                json!({ "added": done.nodes, "links": done.edges, "placed": done.placed });
            change_fields(&done, &mut out);
            Ok(out)
        })
        .await
    }

    #[tool(
        description = "Edit or move nodes on a planning canvas: title, text, url, x, y, width or a camera plan stop's settings (stop: name, position, target, fov); only the fields given change. Stop edits stay in the plan until the person applies it. Any node may be edited, a person's too; the change log keeps what it said before, so it can be undone. If the person edits the same field before saving, their value wins. A call whose undo record would pass 256 KB is refused with nothing written (send fewer nodes), and so are edits to a person's content once agents' changes to it not undone or kept yet hold 4 MB (the person frees that room with Undo or Keep in the Changes panel; your undo_run frees it too). With Auto layout after agents on, your own nodes left overlapping move clear as a change of its own (autoLayout).",
        annotations(idempotent_hint = true)
    )]
    async fn update_nodes(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<UpdateNodesArgs>,
    ) -> Out {
        let name = agent_name(args.agent.as_deref(), &peer);
        let run = args.run;
        let edits: Vec<NodeEdit> = args
            .nodes
            .into_iter()
            .map(|c| NodeEdit {
                id: c.id,
                title: c.title,
                text: c.text,
                url: c.url,
                x: c.x,
                y: c.y,
                width: c.width,
                stop: c.stop.map(Into::into),
            })
            .collect();
        write(self, args.project, args.canvas, move |dir, canvas| {
            let who = Agent {
                name: &name,
                run: run.as_deref(),
            };
            let done = agent::update(dir, canvas, &who, &edits)?;
            let mut out = json!({ "updated": done.nodes.len() });
            change_fields(&done, &mut out);
            Ok(out)
        })
        .await
    }

    #[tool(
        description = "Remove nodes and links added under your agent name (by). A node goes with its links. A person's node or link, or an agent's node a person wrote in (personWrote), is always refused with why: agents never delete a person's work; ask the person instead. Agent names are labels, not accounts: a node made under another name is refused too, but that is a courtesy, not a protection. Ids no longer on the canvas are noted, not refused.",
        annotations(destructive_hint = true, idempotent_hint = true)
    )]
    async fn remove_nodes(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<RemoveNodesArgs>,
    ) -> Out {
        let name = agent_name(args.agent.as_deref(), &peer);
        let (nodes, links, run) = (args.nodes, args.links, args.run);
        write(self, args.project, args.canvas, move |dir, canvas| {
            let who = Agent {
                name: &name,
                run: run.as_deref(),
            };
            let done = agent::remove(dir, canvas, &who, &nodes, &links)?;
            let left = done.changed.doc.plan.nodes.len();
            let mut out = json!({ "left": left });
            change_fields(&done, &mut out);
            Ok(out)
        })
        .await
    }

    #[tool(
        description = "Tidy the nodes you added on a planning canvas: each one overlapping another node moves to a free spot beside the node it links to. Nodes already clear stay, and a person's nodes never move. Refused when the person turned Auto layout after agents off for the canvas (get_canvas autoLayout: false): place nodes with x and y then. Answers with how many moved.",
        annotations(idempotent_hint = true)
    )]
    async fn layout_canvas(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<LayoutArgs>,
    ) -> Out {
        let name = agent_name(args.agent.as_deref(), &peer);
        let (only, run) = (args.nodes, args.run);
        write(self, args.project, args.canvas, move |dir, canvas| {
            let who = Agent {
                name: &name,
                run: run.as_deref(),
            };
            let done = agent::layout(dir, canvas, &who, &only)?;
            let mut out = json!({ "moved": done.placed });
            change_fields(&done, &mut out);
            Ok(out)
        })
        .await
    }

    #[tool(
        description = "Undo changes made under your agent name on a planning canvas: a whole run, or change ids from get_changes (at most 100). Undone carefully: a field someone changed since stays as they left it, and a node a person edited or linked since is kept with their links (counted in kept). A person's change, or one under another agent name, is refused; agent names are labels, so only the person's work is protected. When everything was changed since, nothing changes (nothingChanged: true) but the changes count as undone. A change a later agent change rewrote on top of is refused until that later change is undone first; pass both (or the run holding both) to undo them together. An undo line itself can not be undone; make the change again instead. A change the person kept is theirs and is refused (a run holding one too): ask the person, who can undo it in the Changes panel.",
        annotations(idempotent_hint = false)
    )]
    async fn undo_run(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<UndoArgs>,
    ) -> Out {
        let name = agent_name(args.agent.as_deref(), &peer);
        let (run, changes) = (args.run, args.changes);
        write(self, args.project, args.canvas, move |dir, canvas| {
            let mut ids = changes;
            if let Some(run) = &run {
                let lines = plans::log::lines(dir, canvas, 500)?;
                ids.extend(
                    lines
                        .into_iter()
                        .filter(|l| l.undoable && l.run.as_ref() == Some(run) && l.by.as_deref() == Some(name.as_str()))
                        .map(|l| l.id),
                );
            }
            if ids.is_empty() {
                return Err("Nothing of yours to undo: give a run with changes not undone yet, or change ids.".into());
            }
            if ids.len() > plans::agent::MAX_PER_CALL {
                return Err(format!(
                    "{} changes is more than one call undoes ({}). Undo them in smaller groups by id.",
                    ids.len(),
                    plans::agent::MAX_PER_CALL
                )
                .into());
            }
            let done = plans::undo(dir, canvas, &ids, Some(&name), Some(&name))?;
            // Everything was changed since: the line still marks them undone.
            let nothing = done.entry.as_ref().is_some_and(|e| e.ops.is_empty());
            Ok(json!({
                "undone": ids.len(),
                "change": done.entry.map(|e| e.id),
                "kept": done.skipped,
                "nothingChanged": nothing,
            }))
        })
        .await
    }
}
