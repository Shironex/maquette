//! What agents do on a canvas over MCP: add nodes and links next to what
//! they relate to, edit and move nodes, remove only what they made, and
//! tidy their own nodes. Every write goes through [`super::change`] with
//! the agent's name as `by`, so it is checked, capped, locked and logged
//! like a person's save, and a person can undo it from the log.
//!
//! Agents never remove a person's node or link (`by` empty): that is
//! refused with a reason, and the person decides. An agent's name is a
//! label it gives itself, not an account: one agent can write under
//! another's name, so the only protection is the person's work.

use std::collections::BTreeSet;
use std::path::Path;

use super::change::{Changed, Writer, change};
use super::log::new_id;
use super::ops::{Merge, PlanOp};
use super::place::{Rect, default_width, free_spot, rect_of};
use super::{EdgeKind, NodeKind, PlanEdge, PlanFile, PlanNode, Side, is_project_relative};
use crate::error::{CoreError, CoreResult};

/// The most nodes or links one call adds, edits or removes.
pub const MAX_PER_CALL: usize = 100;

mod tidy;
pub use tidy::{layout, remove};

/// The agent writing, as the canvas and the log name it.
#[derive(Debug, Clone, Copy)]
pub struct Agent<'a> {
    pub name: &'a str,
    pub run: Option<&'a str>,
}

impl<'a> Agent<'a> {
    pub(super) fn writer(&self) -> Writer<'a> {
        Writer {
            by: Some(self.name),
            run: self.run,
            merge: Merge::Mine,
        }
    }
}

/// A node an agent adds. Without `at` it is placed beside `near` (or right
/// of everything) clear of other nodes.
#[derive(Debug, Clone, Default)]
pub struct NewNode {
    pub kind: Option<NodeKind>,
    pub title: String,
    pub text: String,
    pub url: Option<String>,
    pub image: Option<String>,
    pub asset: Option<String>,
    pub direction: Option<String>,
    pub at: Option<(i32, i32)>,
    pub width: Option<u32>,
}

/// A link an agent draws. `from` and `to` are node ids, or `$0`, `$1` for
/// the nodes of the same call in order.
#[derive(Debug, Clone)]
pub struct NewLink {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
    pub label: Option<String>,
}

/// A node an agent edits or moves: only the given fields change.
#[derive(Debug, Clone, Default)]
pub struct NodeEdit {
    pub id: String,
    pub title: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
    /// A stop node's camera settings.
    pub stop: Option<StopEdit>,
}

/// Changes to a stop node's camera stop: only the given settings change.
#[derive(Debug, Clone, Default)]
pub struct StopEdit {
    pub name: Option<String>,
    pub position: Option<[f32; 3]>,
    pub target: Option<[f32; 3]>,
    pub fov: Option<f32>,
}

impl StopEdit {
    /// `stop` with these changes laid over it.
    pub fn over(&self, stop: &crate::layout::LayoutStop) -> crate::layout::LayoutStop {
        let mut next = stop.clone();
        if let Some(name) = &self.name {
            next.name = super::trimmed(name).to_string();
        }
        next.position = self.position.unwrap_or(next.position);
        next.target = self.target.unwrap_or(next.target);
        next.fov = self.fov.unwrap_or(next.fov);
        next
    }
}

/// What an agent's write did, for a short answer.
#[derive(Debug, Clone)]
pub struct Done {
    pub changed: Changed,
    /// Nodes added or touched, in the order given.
    pub nodes: Vec<String>,
    pub edges: Vec<String>,
    /// Nodes placed or moved by the layout.
    pub placed: u32,
    /// Links left out, and why.
    pub notes: Vec<String>,
    /// The auto layout line that moved this write's nodes clear, when the
    /// canvas has "Auto layout after agents" on and any overlapped.
    pub tidied: Option<Changed>,
}

fn refuse<T>(message: String) -> CoreResult<T> {
    Err(CoreError::Invalid(message))
}

fn too_many(n: usize, what: &str) -> CoreResult<()> {
    if n > MAX_PER_CALL {
        return refuse(format!("Send at most {MAX_PER_CALL} {what} per call."));
    }
    Ok(())
}

/// The kind of a new node from its fields when not given.
fn kind_of(node: &NewNode) -> NodeKind {
    node.kind.unwrap_or(if node.asset.is_some() {
        NodeKind::Asset
    } else if node.direction.is_some() {
        NodeKind::Direction
    } else if node.image.is_some() {
        NodeKind::Image
    } else if node.url.is_some() {
        NodeKind::Link
    } else {
        NodeKind::Note
    })
}

/// The sides a link leaves and arrives at, from where the nodes sit.
fn sides(from: &Rect, to: &Rect) -> (Side, Side) {
    let dx = (to.x + to.w / 2) - (from.x + from.w / 2);
    let dy = (to.y + to.h / 2) - (from.y + from.h / 2);
    if dx.abs() >= dy.abs() {
        if dx >= 0 {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        }
    } else if dy >= 0 {
        (Side::Bottom, Side::Top)
    } else {
        (Side::Top, Side::Bottom)
    }
}

/// A picture an agent names must be in the project already.
fn check_picture(project_dir: &Path, image: &str) -> CoreResult<()> {
    if !is_project_relative(image) || !project_dir.join(image).is_file() {
        return refuse(format!(
            "The picture {image} is not in the project. Give a path relative to the project, such as refs/porch.png."
        ));
    }
    Ok(())
}

fn node_of(spec: &NewNode, id: String, agent: &Agent<'_>) -> PlanNode {
    let kind = kind_of(spec);
    let keeps = |own: NodeKind, value: &Option<String>| value.clone().filter(|_| kind == own);
    PlanNode {
        id,
        kind,
        x: 0,
        y: 0,
        width: spec.width,
        title: spec.title.clone(),
        text: spec.text.clone(),
        image: keeps(NodeKind::Image, &spec.image),
        url: keeps(NodeKind::Link, &spec.url),
        asset: keeps(NodeKind::Asset, &spec.asset),
        direction: keeps(NodeKind::Direction, &spec.direction),
        by: Some(agent.name.to_string()),
        person_wrote: false,
        stop: None,
    }
}

/// The column new nodes stack in: beside `near`, else right of everything.
fn column_anchor(plan: &PlanFile, near: Option<&str>) -> CoreResult<Rect> {
    if let Some(near) = near {
        let node =
            plan.nodes.iter().find(|n| n.id == near).ok_or_else(|| {
                CoreError::NotFound(format!("the node {near} on this canvas (near)"))
            })?;
        return Ok(rect_of(node));
    }
    let right = plan.nodes.iter().map(rect_of).map(|r| r.x + r.w).max();
    let top = plan.nodes.iter().map(|n| n.y).min().unwrap_or(0);
    // A point the first free column starts beside.
    Ok(Rect {
        x: right.unwrap_or(-64) - 1,
        y: top,
        w: 1,
        h: 1,
    })
}

/// Adds `nodes` and `links`. Placed nodes stack beside `near`.
///
/// # Errors
///
/// Fails on too many items, a `near` or link end that is not on the
/// canvas, a picture not in the project, and as [`change`].
pub fn add(
    project_dir: &Path,
    id: &str,
    agent: &Agent<'_>,
    near: Option<&str>,
    nodes: &[NewNode],
    links: &[NewLink],
) -> CoreResult<Done> {
    too_many(nodes.len(), "nodes")?;
    too_many(links.len(), "links")?;
    if nodes.is_empty() && links.is_empty() {
        return refuse("Give nodes or links to add.".into());
    }
    for image in nodes.iter().filter_map(|n| n.image.as_deref()) {
        check_picture(project_dir, image)?;
    }
    if nodes.iter().any(|n| n.kind == Some(NodeKind::Stop)) {
        return refuse(
            "Camera stops are proposed with propose_camera_plan, not added one by one.".into(),
        );
    }
    let mut made = Vec::new();
    let mut edges = Vec::new();
    let mut notes = Vec::new();
    let mut placed = 0;
    let changed = change(project_dir, id, &agent.writer(), |plan| {
        let anchor = column_anchor(plan, near)?;
        let mut taken: Vec<Rect> = plan.nodes.iter().map(rect_of).collect();
        let mut rects = std::collections::BTreeMap::new();
        for r in &plan.nodes {
            rects.insert(r.id.clone(), rect_of(r));
        }
        let mut ops = Vec::new();
        for spec in nodes {
            let mut node = node_of(spec, new_id("n"), agent);
            let w = node
                .width
                .and_then(|w| i32::try_from(w).ok())
                .unwrap_or_else(|| default_width(node.kind));
            let (x, y) = spec.at.unwrap_or_else(|| {
                placed += 1;
                let h = rect_of(&node).h;
                free_spot(Some(anchor), &taken, w, h)
            });
            (node.x, node.y) = (x, y);
            let rect = rect_of(&node);
            taken.push(rect);
            rects.insert(node.id.clone(), rect);
            made.push(node.id.clone());
            ops.push(PlanOp::AddNode { node });
        }
        let resolve = |end: &str| -> CoreResult<String> {
            if let Some(n) = end.strip_prefix('$') {
                let at: usize = n.parse().map_err(|_| {
                    CoreError::Invalid(format!("{end} is not $ and the number of a new node."))
                })?;
                return made.get(at).cloned().ok_or_else(|| {
                    CoreError::Invalid(format!("{end} names no node of this call."))
                });
            }
            if rects.contains_key(end) {
                return Ok(end.to_string());
            }
            refuse(format!("The node {end} is not on this canvas."))
        };
        let mut pairs: BTreeSet<(String, String)> = plan
            .edges
            .iter()
            .flat_map(|e| {
                [
                    (e.from.clone(), e.to.clone()),
                    (e.to.clone(), e.from.clone()),
                ]
            })
            .collect();
        for link in links {
            let (from, to) = (resolve(&link.from)?, resolve(&link.to)?);
            if from == to || pairs.contains(&(from.clone(), to.clone())) {
                notes.push(format!("{} to {} is linked already", link.from, link.to));
                continue;
            }
            pairs.insert((from.clone(), to.clone()));
            pairs.insert((to.clone(), from.clone()));
            let (from_side, to_side) = match (rects.get(&from), rects.get(&to)) {
                (Some(a), Some(b)) => {
                    let (s, t) = sides(a, b);
                    (Some(s), Some(t))
                }
                _ => (None, None),
            };
            let edge = PlanEdge {
                id: new_id("e"),
                from,
                to,
                from_side,
                to_side,
                kind: link.kind,
                label: link.label.clone(),
                by: Some(agent.name.to_string()),
            };
            edges.push(edge.id.clone());
            ops.push(PlanOp::AddEdge { edge });
        }
        Ok(ops)
    })?;
    let tidied = tidy::after(project_dir, id, agent, &made);
    Ok(Done {
        changed,
        nodes: made,
        edges,
        placed,
        notes,
        tidied,
    })
}

/// Edits or moves nodes: only the fields given change. Any node may be
/// edited, a person's too; the log keeps what it said before.
///
/// # Errors
///
/// Fails on too many edits, a node not on the canvas, and as [`change`].
pub fn update(
    project_dir: &Path,
    id: &str,
    agent: &Agent<'_>,
    edits: &[NodeEdit],
) -> CoreResult<Done> {
    too_many(edits.len(), "edits")?;
    if edits.is_empty() {
        return refuse("Give the nodes to change.".into());
    }
    let changed = change(project_dir, id, &agent.writer(), |plan| {
        let mut ops = Vec::new();
        for edit in edits {
            let before = plan.nodes.iter().find(|n| n.id == edit.id).ok_or_else(|| {
                CoreError::NotFound(format!("the node {} on this canvas", edit.id))
            })?;
            let mut after = before.clone();
            if let Some(t) = &edit.title {
                after.title.clone_from(t);
            }
            if let Some(t) = &edit.text {
                after.text.clone_from(t);
            }
            if let Some(u) = &edit.url {
                after.url = Some(u.clone());
            }
            after.x = edit.x.unwrap_or(after.x);
            after.y = edit.y.unwrap_or(after.y);
            if edit.width.is_some() {
                after.width = edit.width;
            }
            if let Some(change) = &edit.stop {
                let stop = super::camera::stop_of(before).ok_or_else(|| {
                    CoreError::Invalid(format!("The node {} is not a camera stop.", edit.id))
                })?;
                after.stop = Some(Box::new(change.over(stop)));
            }
            ops.push(PlanOp::SetNode {
                before: before.clone(),
                after,
            });
        }
        Ok(ops)
    })?;
    let nodes: Vec<String> = edits.iter().map(|e| e.id.clone()).collect();
    let tidied = tidy::after(project_dir, id, agent, &nodes);
    Ok(Done {
        changed,
        nodes,
        edges: Vec::new(),
        placed: 0,
        notes: Vec::new(),
        tidied,
    })
}

#[cfg(test)]
mod tests;
