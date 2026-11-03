//! Starting a new canvas from a node of another: the node and the nodes
//! linked to it by one link, with the links among them, laid out as they
//! were around it. It goes through the usual create path (a free id, never
//! the camera plan's, the checks and the size cap), and the copies are the
//! new canvas's first log line, one undo for all of them, under the person
//! or the agent who started it. Each copy keeps who made it, so a person's
//! note stays theirs. The source canvas is only read: nothing moves or goes.
//! Asset and Lab direction nodes stay links by id. Camera stops live only in
//! the camera plan, so a stop can not start a canvas and linked stops are
//! left out.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::agent::MAX_PER_CALL;
use super::change::Changed;
use super::create::make;
use super::log::new_id;
use super::summary::node_name;
use super::{NAME_MAX_CHARS, NodeKind, PlanFile, PlanNode, PlanPreset, load_file, path};
use crate::error::{CoreError, CoreResult};

/// What starting from a node made.
#[derive(Debug, Clone)]
pub struct Started {
    pub changed: Changed,
    /// The linked nodes left out (camera stops, or past the cap), by id.
    pub left_out: Vec<String>,
}

/// Makes a canvas from the node `node` of the canvas `source` and the nodes
/// linked to it, called `name` (else the node's name), made by `by` (an
/// agent's name, or `None` for a person).
///
/// # Errors
///
/// Refuses a node that is not on the canvas or is a camera stop; fails as
/// [`super::create_seeded`] does.
pub fn start_from(
    project_dir: &Path,
    source: &str,
    node: &str,
    name: Option<&str>,
    by: Option<&str>,
) -> CoreResult<Started> {
    let from = load_file(&path(project_dir, source)?)?;
    let seed = from
        .nodes
        .iter()
        .find(|n| n.id == node)
        .ok_or_else(|| CoreError::NotFound(format!("the node {node} on {source}")))?;
    if seed.kind == NodeKind::Stop {
        return Err(CoreError::Invalid(
            "A camera stop can not start a canvas: stops live only in the camera plan. Start from a note, picture, link, asset or direction instead.".into(),
        ));
    }
    let mut kept: BTreeSet<&str> = BTreeSet::from([seed.id.as_str()]);
    let mut left_out = Vec::new();
    for edge in &from.edges {
        let other = match (edge.from == seed.id, edge.to == seed.id) {
            (true, _) => &edge.to,
            (_, true) => &edge.from,
            _ => continue,
        };
        let Some(next) = from.nodes.iter().find(|n| &n.id == other) else {
            continue;
        };
        if kept.contains(next.id.as_str()) {
            continue;
        }
        if next.kind == NodeKind::Stop || kept.len() >= MAX_PER_CALL {
            left_out.push(next.id.clone());
        } else {
            kept.insert(next.id.as_str());
        }
    }
    left_out.sort();
    left_out.dedup();
    let fresh: BTreeMap<&str, String> = kept.iter().map(|id| (*id, new_id("n"))).collect();
    let nodes: Vec<PlanNode> = from
        .nodes
        .iter()
        .filter_map(|n| {
            let id = fresh.get(n.id.as_str())?;
            Some(PlanNode {
                id: id.clone(),
                x: n.x - seed.x,
                y: n.y - seed.y,
                ..n.clone()
            })
        })
        .collect();
    let edges = from
        .edges
        .iter()
        .filter_map(|e| {
            let (a, b) = (fresh.get(e.from.as_str())?, fresh.get(e.to.as_str())?);
            Some(super::PlanEdge {
                id: new_id("e"),
                from: a.clone(),
                to: b.clone(),
                ..e.clone()
            })
        })
        .collect();
    let seed_name = node_name(seed);
    let name = name
        .map(super::trimmed)
        .filter(|n| !n.is_empty())
        .map_or_else(|| seed_name.clone(), str::to_string);
    let name: String = name.chars().take(NAME_MAX_CHARS).collect();
    // The camera plan is no preset: a canvas started from it is a brainstorm.
    let preset = if from.camera.is_some() {
        PlanPreset::Brainstorm
    } else {
        from.preset
    };
    let mut plan = PlanFile::new(&name, preset);
    plan.nodes = nodes;
    plan.edges = edges;
    let summary = format!("Started from {seed_name} on {}", from.name);
    let changed = make(project_dir, plan, by, Some(summary))?;
    Ok(Started { changed, left_out })
}

#[cfg(test)]
mod tests;
