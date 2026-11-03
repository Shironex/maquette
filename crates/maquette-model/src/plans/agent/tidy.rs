//! An agent removing what it made and tidying its own nodes; a person's
//! nodes and links are never removed or moved.

use std::collections::BTreeSet;
use std::path::Path;

use super::super::change::{Changed, Say, change, change_with};
use super::super::ops::PlanOp;
use super::super::place::{Rect, free_spot, rect_of};
use super::super::{PlanFile, PlanNode};
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
            if node.person_wrote {
                return refuse(format!(
                    "A person wrote in the node {want} last, so it is theirs now. Agents never remove a person's work; ask the person to remove it."
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
        tidied: None,
    })
}

/// The agent's nodes (all of them, or those in `only`) that overlap another
/// node, each moved to a free spot beside what it links to, and their ids.
fn tidy_ops(plan: &PlanFile, agent: &Agent<'_>, only: &[String]) -> (Vec<PlanOp>, Vec<String>) {
    let mine = |n: &PlanNode| n.by.as_deref() == Some(agent.name);
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
    let mut moved = Vec::new();
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
        moved.push(node.id.clone());
        ops.push(PlanOp::SetNode {
            before: node.clone(),
            after,
        });
    }
    (ops, moved)
}

/// Moves the agent's nodes (all of them, or those in `only`) that overlap
/// another node to a free spot beside what they link to. Nodes already
/// clear stay where they are, and a person's nodes never move. Refused
/// when the person turned "Auto layout after agents" off for the canvas:
/// then they lay it out, and agents place nodes with x and y.
///
/// # Errors
///
/// Fails when a node in `only` is not the agent's, when auto layout is
/// off, and as [`change`].
pub fn layout(
    project_dir: &Path,
    id: &str,
    agent: &Agent<'_>,
    only: &[String],
) -> CoreResult<Done> {
    too_many(only.len(), "nodes")?;
    let mut moved_ids = Vec::new();
    let changed = change(project_dir, id, &agent.writer(), |plan| {
        if !plan.auto_layout {
            return refuse(
                "The person turned Auto layout after agents off for this canvas, so they lay it out. Place your nodes with x and y instead."
                    .into(),
            );
        }
        for want in only {
            match plan.nodes.iter().find(|n| &n.id == want) {
                Some(n) if n.by.as_deref() == Some(agent.name) => {}
                Some(_) => {
                    return refuse(format!(
                        "The node {want} is not yours. Layout moves only the nodes you added."
                    ));
                }
                None => return refuse(format!("The node {want} is not on this canvas.")),
            }
        }
        let (ops, moved) = tidy_ops(plan, agent, only);
        moved_ids = moved;
        Ok(ops)
    })?;
    let placed = u32::try_from(moved_ids.len()).unwrap_or(u32::MAX);
    Ok(Done {
        changed,
        nodes: moved_ids,
        edges: Vec::new(),
        placed,
        notes: Vec::new(),
        tidied: None,
    })
}

/// After an agent's write: when the canvas has "Auto layout after agents"
/// on, the agent's nodes among `ids` that now overlap another node move
/// clear, as a line of its own ("Moved 2 nodes with auto layout") the
/// person can undo apart from the write. A person's nodes never move.
/// `None` when nothing moved; a tidy that fails leaves the write as it is.
pub(super) fn after(
    project_dir: &Path,
    id: &str,
    agent: &Agent<'_>,
    ids: &[String],
) -> Option<Changed> {
    if ids.is_empty() {
        return None;
    }
    let tidied = change_with(project_dir, id, &agent.writer(), |plan| {
        if !plan.auto_layout {
            return Ok((Vec::new(), Say::Ops));
        }
        let (ops, moved) = tidy_ops(plan, agent, ids);
        let summary = match moved.len() {
            1 => "Moved 1 node with auto layout".to_string(),
            n => format!("Moved {n} nodes with auto layout"),
        };
        let resolves = Vec::new();
        Ok((ops, Say::Line { summary, resolves }))
    });
    match tidied {
        Ok(changed) => changed.entry.is_some().then_some(changed),
        Err(err) => {
            tracing::warn!(error = %err, canvas = id, "auto layout after an agent's write was left out");
            None
        }
    }
}

#[cfg(test)]
mod tests;
