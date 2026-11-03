//! Removal proposals: an agent asks the person to remove nodes it may not
//! remove itself (a person's nodes, nodes a person wrote in, another agent's,
//! or any node it would rather the person decided on). Agents never delete a
//! person's work; they ask, and only the person answers.
//!
//! A proposal is a line in the change log ([`super::log`]) with `proposes`:
//! the nodes with each one's name and a key of its content as it was, and the
//! agent's reason. Nothing on the canvas changes. The person answers in the
//! app: Remove takes the nodes out with their links as the person's own
//! change, which names the proposal in `resolves` and can be undone like any
//! other; Keep it writes a line that only names it. No agent tool answers a
//! proposal, its own or another's, and an agent's name is only a label.
//!
//! A proposal goes stale when one of its nodes changed (its content, not
//! where it sits) or is gone: it still lists, marked so, and Remove is
//! refused, so nothing the agent did not see is removed; the person keeps
//! it or the agent asks again. The log keeps an open proposal whatever its
//! age, apart from the room it keeps for undo.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::agent::{Agent, MAX_PER_CALL};
use super::change::{Changed, Say, Writer, change_with};
use super::files::writing;
use super::log::{self, PlanLogEntry};
use super::ops::PlanOp;
use super::overlap::fnv;
use super::summary::{ChangeMark, node_name};
use super::{PlanNode, TITLE_MAX_CHARS, load_file, path};
use crate::error::{CoreError, CoreResult};
use crate::project::now_rfc3339;

/// The most proposals a canvas holds open; past it agents wait for the person.
pub const MAX_OPEN: usize = 50;

/// One node an agent asks to remove, as it was when asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AskedNode {
    pub id: String,
    /// Its name when asked, for the person to read once it changed or went.
    pub name: String,
    /// [`content_key`] of the node when asked.
    pub key: String,
}

/// What a removal proposal asks, as its log line keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Removal {
    pub nodes: Vec<AskedNode>,
    /// Why, in the agent's words; may be empty.
    #[serde(default)]
    pub reason: String,
}

/// Where a proposed node stands now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum AskedState {
    /// As it was when asked.
    Here,
    /// Its content changed since.
    Changed,
    /// No longer on the canvas.
    Gone,
}

/// A proposed node as the review shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AskedNodeView {
    pub id: String,
    pub name: String,
    pub state: AskedState,
}

/// A removal proposal waiting for the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RemovalProposal {
    /// The proposal's log line id, which Remove and Keep it name.
    pub id: String,
    pub at: String,
    /// The agent that asked.
    pub by: String,
    pub run: Option<String>,
    pub reason: String,
    pub nodes: Vec<AskedNodeView>,
    /// A node changed or went since it was asked: only Keep it is offered.
    pub stale: bool,
}

/// A key of what a node says (its kind, words, picture, address, links to an
/// asset or direction, stop and who made it), not where it sits.
pub fn content_key(node: &PlanNode) -> String {
    let content = PlanNode {
        x: 0,
        y: 0,
        width: None,
        ..node.clone()
    };
    fnv(&serde_json::to_string(&content).unwrap_or_default())
}

/// The proposals of `entries` no line answered, oldest first.
fn open_lines(entries: &[PlanLogEntry]) -> Vec<&PlanLogEntry> {
    let released = log::released_ids(entries);
    entries
        .iter()
        .filter(|e| log::is_open_proposal(e, &released))
        .collect()
}

fn view(entry: &PlanLogEntry, nodes: &[PlanNode]) -> Option<RemovalProposal> {
    let asked = entry.proposes.as_ref()?;
    let views: Vec<AskedNodeView> = asked
        .nodes
        .iter()
        .map(|a| {
            let state = match nodes.iter().find(|n| n.id == a.id) {
                None => AskedState::Gone,
                Some(n) if content_key(n) != a.key => AskedState::Changed,
                Some(_) => AskedState::Here,
            };
            AskedNodeView {
                id: a.id.clone(),
                name: a.name.clone(),
                state,
            }
        })
        .collect();
    Some(RemovalProposal {
        id: entry.id.clone(),
        at: entry.at.clone(),
        by: entry.by.clone().unwrap_or_default(),
        run: entry.run.clone(),
        reason: asked.reason.clone(),
        stale: views.iter().any(|v| v.state != AskedState::Here),
        nodes: views,
    })
}

/// The canvas's removal proposals waiting for the person, oldest first,
/// each with where its nodes stand now.
///
/// # Errors
///
/// Fails when the canvas or its log cannot be read.
pub fn open(project_dir: &Path, id: &str) -> CoreResult<Vec<RemovalProposal>> {
    let plan = load_file(&path(project_dir, id)?)?;
    let entries = log::entries(project_dir, id)?;
    Ok(open_lines(&entries)
        .into_iter()
        .filter_map(|e| view(e, &plan.nodes))
        .collect())
}

/// An agent asks the person to remove `nodes` of the canvas `id`, saying
/// why in `reason`. Nothing on the canvas changes; the log keeps the ask
/// until the person answers in the app.
///
/// # Errors
///
/// Refuses no nodes, more than [`MAX_PER_CALL`], a node not on the canvas,
/// a reason over 300 characters, or a canvas already holding [`MAX_OPEN`]
/// open proposals; fails when the log cannot be written.
pub fn propose(
    project_dir: &Path,
    id: &str,
    agent: &Agent<'_>,
    nodes: &[String],
    reason: &str,
) -> CoreResult<PlanLogEntry> {
    let reason = super::trimmed(reason);
    if nodes.is_empty() {
        return Err(CoreError::Invalid(
            "Name the nodes to propose removing.".into(),
        ));
    }
    if nodes.len() > MAX_PER_CALL {
        return Err(CoreError::Invalid(format!(
            "Propose at most {MAX_PER_CALL} nodes per call."
        )));
    }
    if reason.chars().count() > TITLE_MAX_CHARS {
        return Err(CoreError::Invalid(format!(
            "Keep the reason to {TITLE_MAX_CHARS} characters."
        )));
    }
    let _lock = writing();
    let plan = load_file(&path(project_dir, id)?)?;
    let entries = log::entries(project_dir, id)?;
    if open_lines(&entries).len() >= MAX_OPEN {
        return Err(CoreError::Invalid(format!(
            "This canvas already has {MAX_OPEN} removal proposals waiting for the person. Ask them to review the ones in the Changes panel first."
        )));
    }
    let mut seen = BTreeSet::new();
    let mut asked = Vec::new();
    for want in nodes {
        let node = plan
            .nodes
            .iter()
            .find(|n| &n.id == want)
            .ok_or_else(|| CoreError::NotFound(format!("the node {want} on this canvas")))?;
        if seen.insert(want.as_str()) {
            asked.push(AskedNode {
                id: node.id.clone(),
                name: node_name(node),
                key: content_key(node),
            });
        }
    }
    let summary = match asked.as_slice() {
        [one] => format!("Asked to remove: {}", one.name),
        many => format!("Asked to remove {} nodes", many.len()),
    };
    let entry = PlanLogEntry {
        id: log::new_id("c"),
        at: now_rfc3339(),
        by: Some(agent.name.to_string()),
        run: agent.run.map(str::to_string),
        summary,
        mark: ChangeMark::Propose,
        ops: Vec::new(),
        undoes: Vec::new(),
        keeps: Vec::new(),
        trimmed: false,
        applied: None,
        proposes: Some(Removal {
            nodes: asked,
            reason: reason.to_string(),
        }),
        resolves: Vec::new(),
    };
    if log::line_len(&entry)? > log::LINE_MAX_BYTES {
        return Err(CoreError::Invalid(
            "This proposal is too long to keep. Propose fewer nodes per call.".into(),
        ));
    }
    log::append(project_dir, id, &entry)?;
    Ok(entry)
}

/// The open proposal `proposal` of `entries`, refused when it is not one.
fn find_open<'a>(entries: &'a [PlanLogEntry], proposal: &str) -> CoreResult<&'a PlanLogEntry> {
    let entry = entries
        .iter()
        .find(|e| e.id == proposal && e.proposes.is_some())
        .ok_or_else(|| CoreError::NotFound(format!("the removal proposal {proposal}")))?;
    if !log::is_open_proposal(entry, &log::released_ids(entries)) {
        return Err(CoreError::Invalid(
            "This removal proposal was answered already.".into(),
        ));
    }
    Ok(entry)
}

/// The person accepts the proposal: its nodes go with their links, as the
/// person's change, which names the proposal and can be undone. Only the
/// app calls this; no agent tool reaches it.
///
/// # Errors
///
/// Refuses a proposal that is not open, or is stale (a node changed or went
/// since it was asked); fails as [`super::change`].
pub fn accept(project_dir: &Path, id: &str, proposal: &str) -> CoreResult<Changed> {
    change_with(project_dir, id, &Writer::PERSON, |plan| {
        let entries = log::entries(project_dir, id)?;
        let entry = find_open(&entries, proposal)?;
        let shown = view(entry, &plan.nodes)
            .ok_or_else(|| CoreError::NotFound(format!("the removal proposal {proposal}")))?;
        if shown.stale {
            return Err(CoreError::Invalid(
                "A node of this proposal changed or went since the agent asked, so nothing was removed. Keep it, or ask the agent to propose again.".into(),
            ));
        }
        let mut ops = Vec::new();
        let mut gone = BTreeSet::new();
        for asked in &shown.nodes {
            for edge in plan
                .edges
                .iter()
                .filter(|e| e.from == asked.id || e.to == asked.id)
            {
                if gone.insert(edge.id.clone()) {
                    ops.push(PlanOp::RemoveEdge { edge: edge.clone() });
                }
            }
            if let Some(node) = plan.nodes.iter().find(|n| n.id == asked.id) {
                ops.push(PlanOp::RemoveNode { node: node.clone() });
            }
        }
        let summary = match shown.nodes.as_slice() {
            [one] => format!("Removed {}, as {} asked", one.name, shown.by),
            many => format!("Removed {} nodes, as {} asked", many.len(), shown.by),
        };
        Ok((
            ops,
            Say::Line {
                summary,
                resolves: vec![proposal.to_string()],
            },
        ))
    })
}

/// The person keeps the nodes: a line that answers the proposal and
/// changes nothing. Only the app calls this.
///
/// # Errors
///
/// Refuses a proposal that is not open; fails when the canvas or its log
/// cannot be read or written.
pub fn dismiss(project_dir: &Path, id: &str, proposal: &str) -> CoreResult<PlanLogEntry> {
    let _lock = writing();
    load_file(&path(project_dir, id)?)?;
    let entries = log::entries(project_dir, id)?;
    let entry = find_open(&entries, proposal)?;
    let asked = entry
        .proposes
        .as_ref()
        .map_or(&[][..], |r| r.nodes.as_slice());
    let who = entry.by.as_deref().unwrap_or("an agent");
    let summary = match asked {
        [one] => format!("Kept {}, which {who} asked to remove", one.name),
        many => format!("Kept {} nodes {who} asked to remove", many.len()),
    };
    let line = PlanLogEntry {
        id: log::new_id("c"),
        at: now_rfc3339(),
        by: None,
        run: None,
        summary,
        mark: ChangeMark::Keep,
        ops: Vec::new(),
        undoes: Vec::new(),
        keeps: Vec::new(),
        trimmed: false,
        applied: None,
        proposes: None,
        resolves: vec![proposal.to_string()],
    };
    log::append(project_dir, id, &line)?;
    Ok(line)
}

#[cfg(test)]
mod tests;
