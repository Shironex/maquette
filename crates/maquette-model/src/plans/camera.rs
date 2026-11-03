//! The camera plan: the one built in canvas of a website project where the
//! tour's camera stops are planned before they reach `layout.json`.
//!
//! Each stop is a `stop` node holding the stop whole, as `layout.json` keeps
//! it, and the tour order is the line of `leads-to` edges between stop
//! nodes. The file's `camera` keeps the stops of `layout.json` the plan was
//! last built, rebased or applied from (its base), so when the layout
//! changes underneath (a person in the Inspector, an agent's `set_layout`)
//! the plan shows it is behind and offers to compare or rebase ([`rebase`]),
//! and never writes over either side.
//!
//! A plan may hold several lines: the main line (the one with the most of
//! the person's stops) and agents' proposals beside it. Only the main line
//! is applied, and only by the person, through a review that shows every
//! real change ([`review`]) and refuses when `layout.json` or the plan
//! changed since it was shown ([`apply`]). An Apply is logged as the
//! person's change, and its undo puts the old `layout.json` back while the
//! file is still as the Apply left it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::files::{doc, plain_entry, write_plan, writing};
use super::log::new_id;
use super::ops::PlanOp;
use super::summary::ChangeMark;
use super::{EdgeKind, NodeKind, PlanDoc, PlanEdge, PlanFile, PlanNode, PlanPreset, Side, dir};
use crate::error::{CoreError, CoreResult};
use crate::layout::{self, LayoutStop, MAX_STOPS, SceneLayout};

mod apply;
mod diff;
mod propose;
mod rebase;

pub use apply::{CameraReview, apply, review, review_of, undo_apply};
pub use diff::{DiffKind, DiffLine, FieldChange, StopChange, StopDiff, diff_stops};
pub use propose::{Proposal, ProposedStop, propose};
pub use rebase::{RebaseMode, RebaseReport, rebase, rebased, started_over};

/// The camera plan's file id: `plans/camera-plan.plan.json`.
pub const CAMERA_PLAN_ID: &str = "camera-plan";
pub const CAMERA_PLAN_NAME: &str = "Camera plan";
/// Where the layouts an Apply replaced are kept, inside `plans/`.
pub const HISTORY_DIR: &str = "history";
/// How far apart stop nodes are laid along a line, in canvas pixels.
pub const STEP_X: i32 = 280;

/// The stops of `layout.json` the plan was last built, rebased or applied
/// from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CameraBase {
    pub stops: Vec<LayoutStop>,
}

/// What an Apply wrote, kept in its log line for its undo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    /// The layout it replaced, relative to the project.
    pub history: String,
    /// [`layout_key`] of `layout.json` before and after the Apply.
    pub before: String,
    pub after: String,
}

/// One line of stop nodes joined by `leads-to` edges, in tour order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    pub nodes: Vec<String>,
    /// Why it is not one plain line (a fork, a loop, a stop twice).
    pub problem: Option<String>,
    /// How many of its stops a person made.
    pub people: usize,
    /// The agent that made every stop of it, for a proposal.
    pub agent: Option<String>,
}

/// A short, stable fingerprint of any value: the first 16 hex digits of the
/// SHA-256 of its JSON.
pub fn fingerprint<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    hex::encode(Sha256::digest(&bytes))[..16].to_string()
}

/// What a review compares an Apply against: the whole layout, so any change
/// to `layout.json` since the review was shown refuses the Apply.
pub fn layout_key(layout: &SceneLayout) -> String {
    fingerprint(layout)
}

fn invalid<T>(message: String) -> CoreResult<T> {
    Err(CoreError::Invalid(message))
}

/// The camera plan's own checks: stop nodes only in the camera plan, and a
/// base of valid stops with unique ids.
pub(super) fn check_plan(plan: &PlanFile) -> CoreResult<()> {
    let stops = plan.nodes.iter().any(|n| n.kind == NodeKind::Stop);
    let Some(base) = &plan.camera else {
        if stops {
            return invalid("Only the camera plan holds camera stops.".into());
        }
        return Ok(());
    };
    if base.stops.len() > MAX_STOPS {
        return invalid(format!(
            "The camera plan's base holds over {MAX_STOPS} stops."
        ));
    }
    let mut ids = BTreeSet::new();
    for stop in &base.stops {
        layout::check_stop(stop)?;
        if !ids.insert(stop.id.as_str()) {
            return invalid(format!(
                "The camera plan's base has the stop {} twice.",
                stop.id
            ));
        }
    }
    Ok(())
}

/// The stop of a stop node.
pub fn stop_of(node: &PlanNode) -> Option<&LayoutStop> {
    node.stop.as_deref().filter(|_| node.kind == NodeKind::Stop)
}

fn name_of(plan: &PlanFile, id: &str) -> String {
    plan.nodes
        .iter()
        .find(|n| n.id == id)
        .and_then(stop_of)
        .map_or_else(|| id.to_string(), |s| s.name.clone())
}

/// Every line of stop nodes, each in tour order. A stop with no `leads-to`
/// link is a line of its own.
pub fn chains(plan: &PlanFile) -> Vec<Chain> {
    let mut stops: Vec<&PlanNode> = plan.nodes.iter().filter(|n| stop_of(n).is_some()).collect();
    stops.sort_by_key(|n| (n.x, n.y));
    let known: BTreeSet<&str> = stops.iter().map(|n| n.id.as_str()).collect();
    let mut next: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut prev: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for e in plan.edges.iter().filter(|e| e.kind == EdgeKind::LeadsTo) {
        if known.contains(e.from.as_str()) && known.contains(e.to.as_str()) && e.from != e.to {
            next.entry(&e.from).or_default().push(&e.to);
            prev.entry(&e.to).or_default().push(&e.from);
        }
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut out = Vec::new();
    for start in &stops {
        if seen.contains(start.id.as_str()) {
            continue;
        }
        let mut part = vec![start.id.as_str()];
        seen.insert(&start.id);
        let mut at = 0;
        while let Some(&id) = part.get(at) {
            at += 1;
            let around = next.get(id).into_iter().chain(prev.get(id)).flatten();
            for &other in around {
                if seen.insert(other) {
                    part.push(other);
                }
            }
        }
        out.push(chain_from(plan, &part, &next, &prev));
    }
    out
}

fn chain_from(
    plan: &PlanFile,
    part: &[&str],
    next: &BTreeMap<&str, Vec<&str>>,
    prev: &BTreeMap<&str, Vec<&str>>,
) -> Chain {
    let count = |map: &BTreeMap<&str, Vec<&str>>, id: &str| map.get(id).map_or(0, Vec::len);
    let mut problem = None;
    if let Some(fork) = part.iter().find(|id| count(next, id) > 1) {
        problem = Some(format!(
            "{} leads to more than one stop. Keep one line: remove a link.",
            name_of(plan, fork)
        ));
    } else if let Some(join) = part.iter().find(|id| count(prev, id) > 1) {
        problem = Some(format!(
            "More than one stop leads to {}. Keep one line: remove a link.",
            name_of(plan, join)
        ));
    }
    let heads: Vec<&str> = part
        .iter()
        .copied()
        .filter(|id| count(prev, id) == 0)
        .collect();
    let mut nodes: Vec<String> = part.iter().map(|s| (*s).to_string()).collect();
    if problem.is_none() {
        match heads.as_slice() {
            [head] => {
                nodes = vec![(*head).to_string()];
                while let Some(&to) = next
                    .get(nodes[nodes.len() - 1].as_str())
                    .and_then(|n| n.first())
                {
                    nodes.push(to.to_string());
                }
            }
            _ => {
                problem = Some(format!(
                    "The stops from {} on loop back to the start. Remove a link so the line has a first stop.",
                    name_of(plan, part[0])
                ));
            }
        }
    }
    let of: Vec<&PlanNode> = nodes
        .iter()
        .filter_map(|id| plan.nodes.iter().find(|n| &n.id == id))
        .collect();
    if problem.is_none() {
        let mut ids = BTreeSet::new();
        if let Some(twice) = of
            .iter()
            .filter_map(|n| stop_of(n))
            .find(|s| !ids.insert(s.id.as_str()))
        {
            problem = Some(format!(
                "The stop {} is in this line twice. Remove one of them.",
                twice.name
            ));
        }
    }
    let people = of.iter().filter(|n| n.by.is_none()).count();
    let agents: BTreeSet<&str> = of.iter().filter_map(|n| n.by.as_deref()).collect();
    let agent = match (people, agents.iter().next()) {
        (0, Some(one)) if agents.len() == 1 => Some((*one).to_string()),
        _ => None,
    };
    Chain {
        nodes,
        problem,
        people,
        agent,
    }
}

/// The line Apply writes: the one with the most of the person's stops, then
/// the longest, then the leftmost.
pub fn main_chain(plan: &PlanFile) -> Option<Chain> {
    let x_of = |c: &Chain| {
        plan.nodes
            .iter()
            .find(|n| Some(&n.id) == c.nodes.first())
            .map_or(0, |n| n.x)
    };
    chains(plan)
        .into_iter()
        .filter(|c| c.people > 0)
        .min_by_key(|c| {
            (
                std::cmp::Reverse(c.people),
                std::cmp::Reverse(c.nodes.len()),
                x_of(c),
            )
        })
}

/// The line holding the node `id`.
pub fn chain_of(plan: &PlanFile, id: &str) -> Option<Chain> {
    chains(plan)
        .into_iter()
        .find(|c| c.nodes.iter().any(|n| n == id))
}

/// The stops of `chain`, in its order.
pub fn chain_stops(plan: &PlanFile, chain: &Chain) -> Vec<LayoutStop> {
    chain
        .nodes
        .iter()
        .filter_map(|id| plan.nodes.iter().find(|n| &n.id == id))
        .filter_map(stop_of)
        .cloned()
        .collect()
}

/// A stop node for `stop` at `(x, y)`, made by `by`.
pub fn stop_node(stop: &LayoutStop, x: i32, y: i32, by: Option<&str>) -> PlanNode {
    PlanNode {
        id: new_id("n"),
        kind: NodeKind::Stop,
        x,
        y,
        width: None,
        title: String::new(),
        text: String::new(),
        image: None,
        url: None,
        asset: None,
        direction: None,
        by: by.map(str::to_string),
        person_wrote: false,
        person_placed: false,
        stop: Some(Box::new(stop.clone())),
    }
}

/// A `leads-to` edge from `from` to `to`, right side to left side.
pub fn lead(from: &str, to: &str, by: Option<&str>) -> PlanEdge {
    PlanEdge {
        id: new_id("e"),
        from: from.to_string(),
        to: to.to_string(),
        from_side: Some(Side::Right),
        to_side: Some(Side::Left),
        kind: EdgeKind::LeadsTo,
        label: None,
        by: by.map(str::to_string),
    }
}

/// One line of stop nodes for `stops`, from `(x, y)` to the right.
pub fn line_of(
    stops: &[LayoutStop],
    x: i32,
    y: i32,
    by: Option<&str>,
) -> (Vec<PlanNode>, Vec<PlanEdge>) {
    let nodes: Vec<PlanNode> = stops
        .iter()
        .zip(0..)
        .map(|(stop, i)| stop_node(stop, x + i * STEP_X, y, by))
        .collect();
    let edges = nodes
        .windows(2)
        .map(|w| lead(&w[0].id, &w[1].id, by))
        .collect();
    (nodes, edges)
}

/// The project's camera plan, made from `layout.json` the first time: one
/// stop node per stop, joined in tour order, as the person's main line.
/// `by` names an agent that made it in the log; a person passes none. A
/// change log found on the id with no canvas is the plan's own when any line
/// shows it (its file deleted by hand, or a sync bringing the log first):
/// the plan is made again onto it. A log that clearly belongs to another
/// canvas moves into `plans/history/` first and the plan starts its own.
///
/// # Errors
///
/// Fails when `layout.json` cannot be read or the file cannot be written.
pub fn ensure(project_dir: &Path, by: Option<&str>) -> CoreResult<PlanDoc> {
    let _lock = writing();
    let file = super::path(project_dir, CAMERA_PLAN_ID)?;
    if file.exists() {
        let plan = super::load_file(&file)?;
        if plan.camera.is_some() {
            return Ok(doc(project_dir, CAMERA_PLAN_ID, plan));
        }
        // A canvas from before the id was kept for the camera plan: it moves, never blocks.
        let moved = super::files::move_aside(project_dir, &plan)?;
        tracing::info!("moved the canvas on the camera plan's id to {moved}");
    } else if !own_log(project_dir)? {
        // No canvas, yet another canvas's log: kept aside, never the new plan's.
        if let Some(kept) = super::files::set_aside_stray_log(project_dir)? {
            tracing::warn!(
                "a change log of another canvas on the camera plan's id moved to {kept}"
            );
        }
    }
    let stops = layout::load(project_dir)?.stops;
    // The line mirrors layout.json, the person's, whoever opened it first.
    let (nodes, edges) = line_of(&stops, 0, 0, None);
    let mut plan = PlanFile::new(CAMERA_PLAN_NAME, PlanPreset::Brainstorm);
    plan.nodes = nodes;
    plan.edges = edges;
    plan.camera = Some(CameraBase { stops });
    let plan = super::normalized(&plan);
    super::validate(&plan)?;
    let folder = dir(project_dir);
    std::fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    write_plan(&file, &plan)?;
    let count = plan.nodes.len();
    let line = plain_entry(
        by,
        format!("Made the camera plan from layout.json ({count} stops)"),
        ChangeMark::Create,
    );
    super::change::note(project_dir, CAMERA_PLAN_ID, &line);
    Ok(doc(project_dir, CAMERA_PLAN_ID, plan))
}

/// Whether the change log on the camera plan's id is the plan's own: no log
/// or no readable line counts as its own (nothing to inherit), else a line
/// must show the camera plan (made as it, an Apply or rebase, its base, or a
/// stop node). Only a log with readable lines and none of those is another
/// canvas's.
fn own_log(project_dir: &Path) -> CoreResult<bool> {
    let entries = super::log::entries(project_dir, CAMERA_PLAN_ID)?;
    if entries.is_empty() {
        return Ok(true);
    }
    let stop = |n: &PlanNode| n.kind == NodeKind::Stop;
    Ok(entries.iter().any(|e| {
        e.summary.starts_with("Made the camera plan")
            || matches!(e.mark, ChangeMark::Apply | ChangeMark::Rebase)
            || e.ops.iter().any(|op| match op {
                PlanOp::SetCamera { .. } => true,
                PlanOp::AddNode { node } | PlanOp::RemoveNode { node } => stop(node),
                PlanOp::SetNode { before, after } => stop(before) || stop(after),
                _ => false,
            })
    }))
}

/// True when the plan was based on other stops than `layout` has now.
pub fn is_stale(plan: &PlanFile, layout: &SceneLayout) -> bool {
    plan.camera
        .as_ref()
        .is_some_and(|b| b.stops != layout.stops)
}

#[cfg(test)]
mod tests;
