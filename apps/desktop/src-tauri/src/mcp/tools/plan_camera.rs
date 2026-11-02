//! The camera plan of a website project for agents: reading it, proposing
//! a tour as the agent's own line of stop nodes beside the person's, and
//! reading what Apply would write to `layout.json`. Agents never apply: the
//! person does, with Apply in the app, and no tool here or elsewhere writes
//! `layout.json` from a plan. Stop settings of single nodes change through
//! update_nodes, whose `stop` field is [`StopChange`].

use std::path::PathBuf;

use maquette_core::ProjectTarget;
use maquette_core::layout::LayoutStop;
use maquette_core::plan_camera;
use maquette_core::plans::agent::{Agent, StopEdit};
use maquette_core::plans::camera::{self, CAMERA_PLAN_ID, CameraReview, ProposedStop};
use rmcp::RoleServer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Peer, schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::plans::{agent_name, project_dir};
use super::{McpTools, Out, ProjectArgs, ToolError, respond};
use crate::mcp::host::Host;

/// Said with every camera plan answer, so no agent goes looking for a way.
const PERSON_APPLIES: &str = "Only the person applies a camera plan, with Apply in the app.";

/// New camera settings for a stop node; only the fields given change.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StopChange {
    pub name: Option<String>,
    /// Where the camera stands, x, y, z in metres.
    pub position: Option<[f32; 3]>,
    /// What it looks at, x, y, z in metres.
    pub target: Option<[f32; 3]>,
    /// Field of view in degrees.
    pub fov: Option<f32>,
}

impl From<StopChange> for StopEdit {
    fn from(c: StopChange) -> Self {
        Self {
            name: c.name,
            position: c.position,
            target: c.target,
            fov: c.fov,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StopSpec {
    /// A stop id from get_camera_plan, or a new one (lowercase, digits and dashes).
    pub id: String,
    /// Needed for a new stop; given for a known one, it renames it.
    pub name: Option<String>,
    /// Needed for a new stop: where the camera stands, x, y, z in metres.
    pub position: Option<[f32; 3]>,
    /// Needed for a new stop: what it looks at.
    pub target: Option<[f32; 3]>,
    /// Field of view in degrees; a new stop gets 35.
    pub fov: Option<f32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProposeArgs {
    pub project: String,
    /// The whole tour in order, 1 to 24 stops. Known stops keep their settings unless given.
    pub stops: Vec<StopSpec>,
    /// Your name as people see it, such as "Camera agent". Defaults to your MCP client's name.
    pub agent: Option<String>,
    /// A run id you choose to group several calls, so a person can undo them together.
    pub run: Option<String>,
}

/// The project folder of a website project; others have no camera plan.
fn website_dir(host: &dyn Host, project: &str) -> Result<PathBuf, ToolError> {
    let (dir, manifest) = project_dir(host, project)?;
    if manifest.target != ProjectTarget::Website {
        return Err("Only a website project has a camera plan.".into());
    }
    Ok(dir)
}

fn round(v: f32) -> f64 {
    (f64::from(v) * 1000.0).round() / 1000.0
}

fn stop_view(node: &str, stop: &LayoutStop) -> Value {
    json!({
        "node": node, "id": stop.id, "name": stop.name,
        "position": stop.position.map(round), "target": stop.target.map(round),
        "fov": round(stop.fov),
    })
}

/// A review in a few fields: the counts, each stop that changes and why the
/// line can not be applied, when it can not.
fn review_view(review: &CameraReview) -> Value {
    let stops: Vec<Value> = review
        .diff
        .stops
        .iter()
        .filter(|s| s.added || s.removed || s.moved || !s.fields.is_empty())
        .map(|s| {
            let mut v = json!({ "id": s.id, "from": s.from, "to": s.to });
            for (key, on) in [
                ("added", s.added),
                ("removed", s.removed),
                ("moved", s.moved),
            ] {
                if on {
                    v[key] = json!(true);
                }
            }
            if !s.fields.is_empty() {
                let fields: Vec<&str> = s.fields.iter().map(|f| f.field.as_str()).collect();
                v["changed"] = json!(fields);
            }
            v
        })
        .collect();
    let mut out = json!({ "counts": review.diff.counts_line(), "stops": stops });
    if review.stale {
        out["stale"] = json!(true);
    }
    if !review.problems.is_empty() {
        out["problems"] = json!(review.problems);
    }
    out
}

#[tool_router(router = plan_camera_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Read a website project's camera plan (made from layout.json the first time): the main line Apply would write, its stops in order (node, id, name, position, target, fov), the other lines (agents' proposals with their agent, stops set aside), stale: true when layout.json changed since the plan was based on it, and the stops with a poster render. Change single stops with update_nodes (its stop field) on canvas camera-plan, propose a whole tour with propose_camera_plan.",
        annotations(read_only_hint = true)
    )]
    async fn get_camera_plan(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = plan_camera::view(&website_dir(host, &args.project)?)?;
                let plan = &view.doc.plan;
                let line = |ids: &[String]| -> Vec<Value> {
                    ids.iter()
                        .filter_map(|id| plan.nodes.iter().find(|n| &n.id == id))
                        .filter_map(|n| camera::stop_of(n).map(|s| stop_view(&n.id, s)))
                        .collect()
                };
                let lines: Vec<Value> = view
                    .lines
                    .iter()
                    .map(|l| json!({ "agent": l.agent, "stops": line(&l.nodes), "problem": l.problem }))
                    .collect();
                let posters: Vec<&str> = view.posters.iter().map(|p| p.stop.as_str()).collect();
                let mut out = json!({
                    "canvas": CAMERA_PLAN_ID, "main": line(&view.main), "lines": lines,
                    "posters": posters, "apply": PERSON_APPLIES,
                });
                if let Some(problem) = &view.problem {
                    out["problem"] = json!(problem);
                }
                if view.stale {
                    out["stale"] = json!(true);
                    out["outside"] = json!(view.outside.counts_line());
                }
                Ok(out)
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Propose a camera tour for a website project: the stops in order, as your own line of stop nodes below the person's main line on canvas camera-plan. Known stops take only the settings you give; a new stop needs a name, position and target. Your earlier proposal is replaced, except stops a person wrote in or linked. Never touches layout.json or the main line: the person can take the proposal's order, then reviews and applies it in the app; there is no apply tool. Answers with the new node ids, the change id and how the proposal compares with layout.json.",
        annotations(idempotent_hint = false)
    )]
    async fn propose_camera_plan(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<ProposeArgs>,
    ) -> Out {
        let name = agent_name(args.agent.as_deref(), &peer);
        let stops: Vec<ProposedStop> = args
            .stops
            .into_iter()
            .map(|s| ProposedStop {
                id: s.id,
                name: s.name,
                position: s.position,
                target: s.target,
                fov: s.fov,
            })
            .collect();
        let (project, run) = (args.project, args.run);
        let found = self
            .blocking(move |host, _| {
                let dir = website_dir(host, &project)?;
                let who = Agent {
                    name: &name,
                    run: run.as_deref(),
                };
                let made = camera::propose(&dir, &who, &stops)?;
                host.plan_changed(&project, CAMERA_PLAN_ID);
                let mut out = json!({
                    "nodes": made.nodes,
                    "change": made.changed.entry.as_ref().map(|e| e.id.clone()),
                    "vsLayout": review_view(&made.review),
                    "apply": PERSON_APPLIES,
                });
                if !made.kept.is_empty() {
                    out["kept"] = json!(made.kept);
                }
                Ok(out)
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read what Apply would write to layout.json from a website project's camera plan now: the counts (added, moved, removed, changed), each stop that changes with its place before and after and the settings that change, the main line's node ids, dropped: triggers and timeline keys that go with a removed stop, stale: true when layout.json changed since the plan was based on it (the person compares or rebases first), and problems that block Apply. Reading only: the person applies, with Apply in the app, after reviewing this same diff.",
        annotations(read_only_hint = true)
    )]
    async fn get_apply_preview(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let dir = website_dir(host, &args.project)?;
                camera::ensure(&dir, None)?;
                let review = camera::review(&dir, CAMERA_PLAN_ID)?;
                let mut out = review_view(&review);
                out["nodes"] = json!(review.chain);
                if review.dropped > 0 {
                    // Triggers and timeline keys that wait for a removed stop.
                    out["dropped"] = json!(review.dropped);
                }
                out["apply"] = json!(PERSON_APPLIES);
                Ok(out)
            })
            .await;
        respond(found)
    }
}
