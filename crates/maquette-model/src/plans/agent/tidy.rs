//! An agent removing what it made and tidying its own nodes; a person's
//! nodes and links are never removed or moved.

use std::collections::BTreeSet;
use std::path::Path;

use super::super::PlanNode;
use super::super::change::change;
use super::super::ops::PlanOp;
use super::super::place::{Rect, free_spot, rect_of};
use super::{Agent, Done, refuse, too_many};
use crate::error::CoreResult;

fn whose(by: Option<&String>) -> String {
    by.map_or_else(|| "a person".to_string(), |b| b.clone())
}

/// Removes nodes and links the agent made. A node goes with its links; a
/// person's node or link, or another agent's, is refused whole.
///
/// # Errors
///
/// Fails when anything named is not the agent's, and as [`change`].
pub fn remove(
    project_dir: &Path,
    id: &str,
    agent: &Agent<'_>,
    nodes: &[String],
    edges: &[String],
) -> CoreResult<Done> {
    too_many(nodes.len() + edges.len(), "nodes and links")?;
    let mine = |by: Option<&String>| by.map(String::as_str) == Some(agent.name);
    let mut notes = Vec::new();
    let changed = change(project_dir, id, &agent.writer(), |plan| {
        let mut ops = Vec::new();
        let mut gone_edges = BTreeSet::new();
        for want in edges {
            let Some(edge) = plan.edges.iter().find(|e| &e.id == want) else {
                notes.push(format!("{want} is not on the canvas"));
                continue;
            };
            if !mine(edge.by.as_ref()) {
                return refuse(format!(
                    "The link {want} was drawn by {}. Agents remove only what they made; ask the person.",
                    whose(edge.by.as_ref())
                ));
            }
            gone_edges.insert(edge.id.clone());
            ops.push(PlanOp::RemoveEdge { edge: edge.clone() });
        }
        for want in nodes {
            let Some(node) = plan.nodes.iter().find(|n| &n.id == want) else {
                notes.push(format!("{want} is not on the canvas"));
                continue;
            };
            if !mine(node.by.as_ref()) {
                return refuse(format!(
                    "The node {want} was made by {}. Agents remove only nodes made under their own name, and never a person's; ask the person to remove it.",
                    whose(node.by.as_ref())
                ));
            }
            for edge in plan
                .edges
                .iter()
                .filter(|e| e.from == node.id || e.to == node.id)
            {
                if !mine(edge.by.as_ref()) {
                    return refuse(format!(
                        "The node {want} has a link {} drew ({}). Agents remove only what they made; ask the person.",
                        whose(edge.by.as_ref()),
                        edge.id
                    ));
                }
                if gone_edges.insert(edge.id.clone()) {
                    ops.push(PlanOp::RemoveEdge { edge: edge.clone() });
                }
            }
            ops.push(PlanOp::RemoveNode { node: node.clone() });
        }
        Ok(ops)
    })?;
    Ok(Done {
        changed,
        nodes: nodes.to_vec(),
        edges: edges.to_vec(),
        placed: 0,
        notes,
    })
}

/// Moves the agent's nodes (all of them, or those in `only`) that overlap
/// another node to a free spot beside what they link to. Nodes already
/// clear stay where they are, and a person's nodes never move.
///
/// # Errors
///
/// Fails when a node in `only` is not the agent's, and as [`change`].
pub fn layout(
    project_dir: &Path,
    id: &str,
    agent: &Agent<'_>,
    only: &[String],
) -> CoreResult<Done> {
    too_many(only.len(), "nodes")?;
    let mut moved_ids = Vec::new();
    let changed = change(project_dir, id, &agent.writer(), |plan| {
        let mine = |n: &PlanNode| n.by.as_deref() == Some(agent.name);
        for want in only {
            match plan.nodes.iter().find(|n| &n.id == want) {
                Some(n) if mine(n) => {}
                Some(_) => {
                    return refuse(format!(
                        "The node {want} is not yours. Layout moves only the nodes you added."
                    ));
                }
                None => return refuse(format!("The node {want} is not on this canvas.")),
            }
        }
        let mut targets: Vec<&PlanNode> = plan
            .nodes
            .iter()
            .filter(|n| mine(n) && (only.is_empty() || only.contains(&n.id)))
            .collect();
        targets.sort_by_key(|n| (n.y, n.x));
        let target_ids: BTreeSet<&str> = targets.iter().map(|n| n.id.as_str()).collect();
        let mut taken: Vec<Rect> = plan
            .nodes
            .iter()
            .filter(|n| !target_ids.contains(n.id.as_str()))
            .map(rect_of)
            .collect();
        let mut ops = Vec::new();
        for node in targets {
            let rect = rect_of(node);
            let clear = taken.iter().all(|t| {
                rect.x + rect.w + 32 <= t.x
                    || t.x + t.w + 32 <= rect.x
                    || rect.y + rect.h + 32 <= t.y
                    || t.y + t.h + 32 <= rect.y
            });
            if clear {
                taken.push(rect);
                continue;
            }
            let anchor = plan
                .edges
                .iter()
                .filter_map(|e| match (e.from == node.id, e.to == node.id) {
                    (true, _) => Some(&e.to),
                    (_, true) => Some(&e.from),
                    _ => None,
                })
                .filter(|other| !target_ids.contains(other.as_str()))
                .find_map(|other| plan.nodes.iter().find(|n| &n.id == other))
                .map(rect_of);
            let (x, y) = free_spot(anchor, &taken, rect.w, rect.h);
            let mut after = node.clone();
            (after.x, after.y) = (x, y);
            taken.push(rect_of(&after));
            moved_ids.push(node.id.clone());
            ops.push(PlanOp::SetNode {
                before: node.clone(),
                after,
            });
        }
        Ok(ops)
    })?;
    let placed = u32::try_from(moved_ids.len()).unwrap_or(u32::MAX);
    Ok(Done {
        changed,
        nodes: moved_ids,
        edges: Vec::new(),
        placed,
        notes: Vec::new(),
    })
}
