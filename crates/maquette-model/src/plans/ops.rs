//! Changes to a canvas as a list of operations by node and edge id, so two
//! writers (a person in the app and an agent over MCP) never write over
//! each other's work: each sends what it changed, and it is applied to the
//! canvas as the file holds it now.
//!
//! A change of fields carries the node or edge before and after it, so the
//! same list can be turned around to undo it ([`invert`]) and written to
//! the change log as it is. [`diff`] finds the operations between two
//! canvases; [`apply`] lays them over a third, field by field.

use serde::{Deserialize, Serialize};

use super::{PlanEdge, PlanFile, PlanNode};

/// One change to one node or edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum PlanOp {
    AddNode {
        node: PlanNode,
    },
    /// The node as it was when removed.
    RemoveNode {
        node: PlanNode,
    },
    /// Only the fields that differ between `before` and `after` change.
    SetNode {
        before: PlanNode,
        after: PlanNode,
    },
    AddEdge {
        edge: PlanEdge,
    },
    RemoveEdge {
        edge: PlanEdge,
    },
    SetEdge {
        before: PlanEdge,
        after: PlanEdge,
    },
}

/// How operations meet a canvas that changed since they were made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Merge {
    /// The writer's values win: a person's save. A field changed by both
    /// takes the writer's value, and an edit to a node removed meanwhile
    /// puts the node back with the edit.
    Mine,
    /// Only what still holds the value the operation expects changes: an
    /// undo. A field someone changed since stays as they left it, and a
    /// node changed since is not removed.
    Careful,
}

/// The canvas after [`apply`] and how many operations did not apply whole.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub plan: PlanFile,
    pub skipped: u32,
}

/// Sets each field `before` and `after` disagree on from `after`; with
/// [`Merge::Careful`] only where `current` still holds `before`'s value.
/// Says whether a field was left because it changed meanwhile.
fn merge_node(current: &mut PlanNode, before: &PlanNode, after: &PlanNode, merge: Merge) -> bool {
    let mut held = false;
    macro_rules! field {
        ($($f:ident),*) => {$(
            if before.$f != after.$f {
                if merge == Merge::Mine || current.$f == before.$f {
                    current.$f = after.$f.clone();
                } else {
                    held = true;
                }
            }
        )*};
    }
    field!(kind, x, y, width, title, text, image, url, asset, direction);
    // Whether a person wrote in it follows the content: it changes only
    // when every field the operation changes did, and is never a reason to
    // call the operation held.
    if !held
        && before.person_wrote != after.person_wrote
        && (merge == Merge::Mine || current.person_wrote == before.person_wrote)
    {
        current.person_wrote = after.person_wrote;
    }
    held
}

fn merge_edge(current: &mut PlanEdge, before: &PlanEdge, after: &PlanEdge, merge: Merge) -> bool {
    let mut held = false;
    macro_rules! field {
        ($($f:ident),*) => {$(
            if before.$f != after.$f {
                if merge == Merge::Mine || current.$f == before.$f {
                    current.$f = after.$f.clone();
                } else {
                    held = true;
                }
            }
        )*};
    }
    field!(from_side, to_side, kind, label);
    held
}

/// Applies `ops` in order to `plan`. Nothing fails here: an operation that
/// no longer fits (a link to a node gone, a careful change to something
/// changed since) is skipped and counted, and the result is validated when
/// it is written.
pub fn apply(plan: &PlanFile, ops: &[PlanOp], merge: Merge) -> Applied {
    let mut next = plan.clone();
    let mut skipped = 0;
    for op in ops {
        if !apply_one(&mut next, op, merge) {
            skipped += 1;
        }
    }
    Applied {
        plan: next,
        skipped,
    }
}

fn node_at(plan: &PlanFile, id: &str) -> Option<usize> {
    plan.nodes.iter().position(|n| n.id == id)
}

fn edge_at(plan: &PlanFile, id: &str) -> Option<usize> {
    plan.edges.iter().position(|e| e.id == id)
}

/// Applies one operation; false when it was skipped or only partly applied.
fn apply_one(plan: &mut PlanFile, op: &PlanOp, merge: Merge) -> bool {
    match op {
        PlanOp::AddNode { node } => match node_at(plan, &node.id) {
            Some(_) if merge == Merge::Careful => false,
            Some(at) => {
                plan.nodes[at] = node.clone();
                true
            }
            None => {
                plan.nodes.push(node.clone());
                true
            }
        },
        PlanOp::RemoveNode { node } => match node_at(plan, &node.id) {
            Some(at) if merge == Merge::Careful && plan.nodes[at] != *node => false,
            // Careful (an undo): a node someone else linked since stays, and so do their links.
            Some(_)
                if merge == Merge::Careful
                    && plan
                        .edges
                        .iter()
                        .any(|e| e.from == node.id || e.to == node.id) =>
            {
                false
            }
            Some(at) => {
                plan.nodes.remove(at);
                plan.edges.retain(|e| e.from != node.id && e.to != node.id);
                true
            }
            None => false,
        },
        PlanOp::SetNode { before, after } => match node_at(plan, &after.id) {
            Some(at) => !merge_node(&mut plan.nodes[at], before, after, merge),
            None if merge == Merge::Mine => {
                plan.nodes.push(after.clone());
                true
            }
            None => false,
        },
        PlanOp::AddEdge { edge } => {
            let joins = node_at(plan, &edge.from).is_some() && node_at(plan, &edge.to).is_some();
            match edge_at(plan, &edge.id) {
                _ if !joins => false,
                Some(_) if merge == Merge::Careful => false,
                Some(at) => {
                    plan.edges[at] = edge.clone();
                    true
                }
                None => {
                    plan.edges.push(edge.clone());
                    true
                }
            }
        }
        PlanOp::RemoveEdge { edge } => match edge_at(plan, &edge.id) {
            Some(at) if merge == Merge::Careful && plan.edges[at] != *edge => false,
            Some(at) => {
                plan.edges.remove(at);
                true
            }
            None => false,
        },
        PlanOp::SetEdge { before, after } => match edge_at(plan, &after.id) {
            Some(at) => !merge_edge(&mut plan.edges[at], before, after, merge),
            None if merge == Merge::Mine => apply_one(
                plan,
                &PlanOp::AddEdge {
                    edge: after.clone(),
                },
                merge,
            ),
            None => false,
        },
    }
}

/// The operations that turn `from` into `to`, in an order that applies
/// cleanly and turns around cleanly: edges and nodes that went, nodes that
/// came and changed, then edges that came and changed.
pub fn diff(from: &PlanFile, to: &PlanFile) -> Vec<PlanOp> {
    let find_node = |plan: &PlanFile, id: &str| plan.nodes.iter().find(|n| n.id == id).cloned();
    let find_edge = |plan: &PlanFile, id: &str| plan.edges.iter().find(|e| e.id == id).cloned();
    let mut ops = Vec::new();
    for edge in &from.edges {
        if find_edge(to, &edge.id).is_none() {
            ops.push(PlanOp::RemoveEdge { edge: edge.clone() });
        }
    }
    for node in &from.nodes {
        if find_node(to, &node.id).is_none() {
            ops.push(PlanOp::RemoveNode { node: node.clone() });
        }
    }
    for node in &to.nodes {
        match find_node(from, &node.id) {
            None => ops.push(PlanOp::AddNode { node: node.clone() }),
            Some(before) if before != *node => ops.push(PlanOp::SetNode {
                before,
                after: node.clone(),
            }),
            Some(_) => {}
        }
    }
    for edge in &to.edges {
        match find_edge(from, &edge.id) {
            None => ops.push(PlanOp::AddEdge { edge: edge.clone() }),
            Some(before) if before != *edge => ops.push(PlanOp::SetEdge {
                before,
                after: edge.clone(),
            }),
            Some(_) => {}
        }
    }
    ops
}

/// The operations that undo `ops`: each turned around, last first.
pub fn invert(ops: &[PlanOp]) -> Vec<PlanOp> {
    ops.iter()
        .rev()
        .map(|op| match op.clone() {
            PlanOp::AddNode { node } => PlanOp::RemoveNode { node },
            PlanOp::RemoveNode { node } => PlanOp::AddNode { node },
            PlanOp::SetNode { before, after } => PlanOp::SetNode {
                before: after,
                after: before,
            },
            PlanOp::AddEdge { edge } => PlanOp::RemoveEdge { edge },
            PlanOp::RemoveEdge { edge } => PlanOp::AddEdge { edge },
            PlanOp::SetEdge { before, after } => PlanOp::SetEdge {
                before: after,
                after: before,
            },
        })
        .collect()
}
