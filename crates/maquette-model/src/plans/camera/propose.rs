//! An agent's camera plan proposal: its own line of stop nodes beside the
//! person's main line, never the main line itself and never `layout.json`.
//! A new proposal from the same agent name replaces its last one. The
//! person reads it on the canvas, can take it into the main line, and only
//! the person applies.

use std::collections::BTreeSet;
use std::path::Path;

use super::super::agent::Agent;
use super::super::change::{Changed, change};
use super::super::ops::PlanOp;
use super::{
    CAMERA_PLAN_ID, CameraReview, chain_stops, ensure, lead, main_chain, review_of, stop_node,
    stop_of,
};
use crate::error::{CoreError, CoreResult};
use crate::layout::{self, DEFAULT_FOV, LayoutStop, MAX_STOPS};

/// How far below everything a new proposal is laid, in canvas pixels.
const BELOW: i32 = 320;

/// One stop of a proposal: a stop of the plan or `layout.json` by its id,
/// with any settings changed, or a new one with its name, position and
/// target.
#[derive(Debug, Clone, Default)]
pub struct ProposedStop {
    pub id: String,
    pub name: Option<String>,
    pub position: Option<[f32; 3]>,
    pub target: Option<[f32; 3]>,
    pub fov: Option<f32>,
}

/// What a proposal wrote and how it compares with `layout.json`.
#[derive(Debug, Clone)]
pub struct Proposal {
    pub changed: Changed,
    /// Its stop nodes in tour order.
    pub nodes: Vec<String>,
    /// Stop nodes of an earlier proposal that stayed: a person linked or
    /// wrote in them.
    pub kept: Vec<String>,
    pub review: CameraReview,
}

fn refuse<T>(message: String) -> CoreResult<T> {
    Err(CoreError::Invalid(message))
}

fn stop_from(want: &ProposedStop, known: Option<&LayoutStop>) -> CoreResult<LayoutStop> {
    let stop = match known {
        Some(stop) => stop.clone(),
        None => {
            let (Some(name), Some(position), Some(target)) =
                (&want.name, want.position, want.target)
            else {
                return refuse(format!(
                    "The stop {} is not in the plan or layout.json. A new stop needs a name, a position and a target.",
                    want.id
                ));
            };
            LayoutStop {
                id: want.id.clone(),
                name: name.clone(),
                position,
                target,
                fov: DEFAULT_FOV,
                models: None,
            }
        }
    };
    let edit = super::super::agent::StopEdit {
        name: want.name.clone(),
        position: want.position,
        target: want.target,
        fov: want.fov,
    };
    let next = edit.over(&stop);
    layout::check_stop(&next)?;
    Ok(next)
}

/// Writes `stops` as `agent`'s proposal in the project's camera plan
/// (made from `layout.json` first when the project has none).
///
/// # Errors
///
/// Refuses no stops, too many, a stop twice, a new stop without its
/// camera or a setting the layout refuses; fails as [`change`].
pub fn propose(
    project_dir: &Path,
    agent: &Agent<'_>,
    stops: &[ProposedStop],
) -> CoreResult<Proposal> {
    if stops.is_empty() {
        return refuse("Give the stops of the proposal, in tour order.".into());
    }
    if stops.len() > MAX_STOPS {
        return refuse(format!("A tour has at most {MAX_STOPS} stops."));
    }
    let mut ids = BTreeSet::new();
    if let Some(twice) = stops.iter().find(|s| !ids.insert(s.id.as_str())) {
        return refuse(format!("The stop {} is in the proposal twice.", twice.id));
    }
    ensure(project_dir, Some(agent.name))?;
    let mut made = Vec::new();
    let mut kept = Vec::new();
    let changed = change(project_dir, CAMERA_PLAN_ID, &agent.writer(), |plan| {
        let main = main_chain(plan);
        let main_stops = main
            .as_ref()
            .map(|c| chain_stops(plan, c))
            .unwrap_or_default();
        let base = plan.camera.clone().map(|b| b.stops).unwrap_or_default();
        let on_main: BTreeSet<&String> = main.iter().flat_map(|c| c.nodes.iter()).collect();
        let mut ops = Vec::new();
        for node in plan
            .nodes
            .iter()
            .filter(|n| stop_of(n).is_some() && !on_main.contains(&n.id))
        {
            let linked = plan.edges.iter().any(|e| {
                (e.from == node.id || e.to == node.id) && e.by.as_deref() != Some(agent.name)
            });
            if node.by.as_deref() != Some(agent.name) {
                continue;
            }
            if node.person_wrote || linked {
                kept.push(node.id.clone());
            } else {
                ops.push(PlanOp::RemoveNode { node: node.clone() });
            }
        }
        let x = main
            .as_ref()
            .and_then(|c| c.nodes.first())
            .and_then(|id| plan.nodes.iter().find(|n| &n.id == id))
            .map_or(0, |n| n.x);
        let y = plan.nodes.iter().map(|n| n.y).max().unwrap_or(0) + BELOW;
        let mut nodes = Vec::new();
        for (want, i) in stops.iter().zip(0..) {
            let known = main_stops.iter().chain(&base).find(|s| s.id == want.id);
            let stop = stop_from(want, known)?;
            nodes.push(stop_node(&stop, x + i * super::STEP_X, y, Some(agent.name)));
        }
        let edges: Vec<_> = nodes
            .windows(2)
            .map(|w| lead(&w[0].id, &w[1].id, Some(agent.name)))
            .collect();
        made = nodes.iter().map(|n| n.id.clone()).collect();
        ops.extend(nodes.into_iter().map(|node| PlanOp::AddNode { node }));
        ops.extend(edges.into_iter().map(|edge| PlanOp::AddEdge { edge }));
        Ok(ops)
    })?;
    let layout = layout::load(project_dir)?;
    let review = review_of(&changed.doc.plan, &layout, made.first().map(String::as_str));
    Ok(Proposal {
        changed,
        nodes: made,
        kept,
        review,
    })
}
