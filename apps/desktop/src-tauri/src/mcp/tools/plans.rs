//! Planning canvases for agents: listing and reading a project's canvases
//! and their change log, and making a canvas. The edits are in
//! `plan_edits.rs`. Answers are short on purpose (ids, counts, one line per
//! node) so planning with an agent stays cheap; `full` gives whole texts.

use std::path::PathBuf;

use maquette_core::plans::{self, NodeKind, PlanEdge, PlanNode, PlanPreset};
use maquette_core::{ProjectManifest, project};
use rmcp::RoleServer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Peer, schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{McpTools, Out, ProjectArgs, ToolError, respond, workspace};
use crate::mcp::host::Host;

/// How much of a node's text a short read shows.
const SHORT_TEXT: usize = 120;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CanvasArgs {
    /// The project id.
    pub project: String,
    /// The canvas id, as list_canvases returns it (its file name without .plan.json).
    pub canvas: String,
    /// Whole node texts instead of the first 120 characters.
    #[serde(default)]
    pub full: bool,
    /// Only these node ids.
    #[serde(default)]
    pub nodes: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangesArgs {
    /// The project id.
    pub project: String,
    /// The canvas id.
    pub canvas: String,
    /// The newest this many changes, 20 by default, at most 200.
    pub limit: Option<u32>,
    /// Only changes after this RFC 3339 time.
    pub since: Option<String>,
    /// Only changes by this agent name, or "person" for a person's.
    pub by: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCanvasArgs {
    /// The project id.
    pub project: String,
    /// The canvas name, one line, up to 120 characters. Its id is made from it.
    pub name: String,
    /// What the canvas starts with: brainstorm (blank, the default; blank is
    /// accepted too), story (beats left to right, characters above, places
    /// below) or mechanics (one card per system with its verb, look and input).
    pub preset: Option<String>,
    /// Your name as people see it in the canvas and its change log, such as "Research agent".
    /// Defaults to your MCP client's name.
    pub agent: Option<String>,
}

/// The project folder of `project`, which must exist.
pub(super) fn project_dir(
    host: &dyn Host,
    id: &str,
) -> Result<(PathBuf, ProjectManifest), ToolError> {
    let (root, config) = workspace(host)?;
    let manifest = project::load(&root, &config.layout, id)?;
    Ok((project::dir(&root, &config.layout, id)?, manifest))
}

/// The preset an agent names: `brainstorm` (also `blank`, or none),
/// `story` or `mechanics`, else refused with the list.
pub(super) fn preset_of(given: Option<&str>) -> Result<PlanPreset, ToolError> {
    let named = given.map(|g| g.trim().to_ascii_lowercase());
    match named.as_deref() {
        None | Some("blank" | "brainstorm") => Ok(PlanPreset::Brainstorm),
        Some("story") => Ok(PlanPreset::Story),
        Some("mechanics") => Ok(PlanPreset::Mechanics),
        Some(_) => Err(ToolError::from(format!(
            "{} is not a preset. Use brainstorm (blank, the default), story or mechanics.",
            given.unwrap_or_default().trim()
        ))),
    }
}

/// The agent's name for `by`: the one given, else the MCP client's title
/// or name, one line and at most 64 characters.
pub(super) fn agent_name(given: Option<&str>, peer: &Peer<RoleServer>) -> String {
    let client = peer.peer_info().map(|info| {
        let c = &info.client_info;
        c.title.clone().unwrap_or_else(|| c.name.clone())
    });
    let name = given
        .map(str::to_string)
        .or(client)
        .unwrap_or_default()
        .split(char::is_control)
        .collect::<String>();
    let name = name.trim();
    let name = if name.is_empty() { "Agent" } else { name };
    name.chars().take(plans::BY_MAX_CHARS).collect()
}

fn kind_word(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Note => "note",
        NodeKind::Image => "image",
        NodeKind::Link => "link",
        NodeKind::Asset => "asset",
        NodeKind::Direction => "direction",
        NodeKind::Stop => "stop",
    }
}

fn clipped(text: &str, full: bool) -> String {
    if full || text.chars().count() <= SHORT_TEXT {
        return text.to_string();
    }
    text.chars().take(SHORT_TEXT - 3).collect::<String>() + "..."
}

/// One node in a few fields; empty ones are left out.
pub(super) fn node_view(node: &PlanNode, manifest: &ProjectManifest, full: bool) -> Value {
    let mut out = Map::new();
    out.insert("id".into(), json!(node.id));
    out.insert("kind".into(), json!(kind_word(node.kind)));
    let title = node.title.trim();
    if !title.is_empty() {
        out.insert("title".into(), json!(title));
    }
    if !node.text.is_empty() {
        out.insert("text".into(), json!(clipped(&node.text, full)));
    }
    for (key, value) in [
        ("url", &node.url),
        ("image", &node.image),
        ("direction", &node.direction),
    ] {
        if let Some(v) = value {
            out.insert(key.into(), json!(v));
        }
    }
    if let Some(asset) = &node.asset {
        let live = manifest.assets.iter().find(|a| &a.id == asset);
        out.insert(
            "asset".into(),
            live.map_or_else(
                || json!({ "id": asset, "gone": true }),
                |a| json!({ "id": a.id, "name": a.name, "stage": a.stage, "versions": a.versions.len() }),
            ),
        );
    }
    out.insert("x".into(), json!(node.x));
    out.insert("y".into(), json!(node.y));
    if let Some(by) = &node.by {
        out.insert("by".into(), json!(by));
    }
    if node.person_wrote {
        // A person wrote in this agent node last: it is theirs now.
        out.insert("personWrote".into(), json!(true));
    }
    Value::Object(out)
}

fn edge_view(edge: &PlanEdge) -> Value {
    let mut out = Map::new();
    out.insert("id".into(), json!(edge.id));
    out.insert("from".into(), json!(edge.from));
    out.insert("to".into(), json!(edge.to));
    if edge.kind != plans::EdgeKind::Relates {
        out.insert("kind".into(), json!(edge.kind));
    }
    if let Some(label) = &edge.label {
        out.insert("label".into(), json!(label));
    }
    if let Some(by) = &edge.by {
        out.insert("by".into(), json!(by));
    }
    Value::Object(out)
}

#[tool_router(router = plans_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List a project's planning canvases: each id (for the other plan tools), name, preset, node and link counts and when it last changed. A canvas file that cannot be read lists with its problem.",
        annotations(read_only_hint = true)
    )]
    async fn list_canvases(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (dir, _) = project_dir(host, &args.project)?;
                let canvases: Vec<Value> = plans::list(&dir)?
                    .iter()
                    .map(|c| match &c.problem {
                        Some(problem) => json!({ "id": c.id, "problem": problem }),
                        None => json!({
                            "id": c.id, "name": c.name, "preset": c.preset,
                            "nodes": c.nodes, "links": c.edges, "edited": c.edited,
                        }),
                    })
                    .collect();
                Ok(json!({ "canvases": canvases }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read a planning canvas: its nodes (id, kind, title, text, url, picture path, linked asset with its live name, stage and version count, Lab direction, x and y, and by: the agent that made it, absent for a person; personWrote: true on an agent's node a person wrote in, which makes it theirs) and its links between nodes (from, to, kind when not relates, label, by). Texts are cut at 120 characters unless full; nodes narrows it to some ids. Read this before add_nodes so new nodes go near the right one.",
        annotations(read_only_hint = true)
    )]
    async fn get_canvas(&self, Parameters(args): Parameters<CanvasArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (dir, manifest) = project_dir(host, &args.project)?;
                let plan = plans::get(&dir, &args.canvas)?.plan;
                let wanted = |id: &str| args.nodes.is_empty() || args.nodes.iter().any(|n| n == id);
                let nodes: Vec<Value> = plan
                    .nodes
                    .iter()
                    .filter(|n| wanted(&n.id))
                    .map(|n| node_view(n, &manifest, args.full))
                    .collect();
                let links: Vec<Value> = plan
                    .edges
                    .iter()
                    .filter(|e| wanted(&e.from) || wanted(&e.to))
                    .map(edge_view)
                    .collect();
                Ok(json!({
                    "id": args.canvas, "name": plan.name, "preset": plan.preset,
                    "edited": plan.edited, "nodes": nodes, "links": links,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read a canvas's change log, newest first: each change's id (for undo_run), time, by (an agent's name, absent for a person), run, a one line summary and whether it was undone. Every write by a person or agent is logged.",
        annotations(read_only_hint = true)
    )]
    async fn get_changes(&self, Parameters(args): Parameters<ChangesArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (dir, _) = project_dir(host, &args.project)?;
                let limit = usize::try_from(args.limit.unwrap_or(20).clamp(1, 200)).unwrap_or(20);
                let changes: Vec<Value> = plans::log::lines(&dir, &args.canvas, 500)?
                    .into_iter()
                    .filter(|l| {
                        args.since
                            .as_ref()
                            .is_none_or(|t| l.at.as_str() > t.as_str())
                    })
                    .filter(|l| match args.by.as_deref() {
                        None => true,
                        Some("person") => l.by.is_none(),
                        Some(name) => l.by.as_deref() == Some(name),
                    })
                    .take(limit)
                    .map(|l| {
                        let mut v = json!({ "id": l.id, "at": l.at, "summary": l.summary });
                        for (key, value) in [("by", l.by), ("run", l.run)] {
                            if let Some(value) = value {
                                v[key] = json!(value);
                            }
                        }
                        if l.undone {
                            v["undone"] = json!(true);
                        }
                        v
                    })
                    .collect();
                Ok(json!({ "changes": changes }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Make a planning canvas in a project, as Plan > New canvas does; writes one file, plans/<id>.plan.json. preset sets only the start, every canvas holds every kind of node: brainstorm (blank, the default), story (starter notes for beats left to right joined in order, characters above and places below, worded for a game or a website by the project's target) or mechanics (a card per system with its verb, look and input, and a note on keeping them apart). Preset names are not case sensitive. The starter notes carry your name and are one change in the log with no run, so the person can undo them at once and you can undo_run them by the change id this answers with. Answers with its id, the preset, the node and link counts and change (left out for a blank canvas, which has nothing to undo).",
        annotations(idempotent_hint = false)
    )]
    async fn create_canvas(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<CreateCanvasArgs>,
    ) -> Out {
        let agent = agent_name(args.agent.as_deref(), &peer);
        let found = self
            .blocking(move |host, _| {
                let (dir, manifest) = project_dir(host, &args.project)?;
                let preset = preset_of(args.preset.as_deref())?;
                let changed =
                    plans::create_seeded(&dir, &args.name, preset, manifest.target, Some(&agent))?;
                let made = changed.doc;
                host.plan_changed(&args.project, &made.id);
                let mut out = json!({
                    "id": made.id, "file": made.file, "preset": made.plan.preset,
                    "nodes": made.plan.nodes.len(), "links": made.plan.edges.len(),
                });
                // A blank canvas has no starters, so no change id to undo.
                if let Some(entry) = changed.entry {
                    out["change"] = json!(entry.id);
                }
                Ok(out)
            })
            .await;
        respond(found)
    }
}
