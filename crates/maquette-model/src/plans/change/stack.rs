//! Which later agent change sits on top of one being undone. An undo
//! merges carefully, so a field a later change rewrote stays as that change
//! left it; undoing the earlier change alone would then release what it
//! holds while its old value lives nowhere else. Such an undo waits until
//! the later change is undone first, or both go together.

use std::collections::BTreeSet;

use super::super::PlanNode;
use super::super::log::{PlanLogEntry, is_keeper};
use super::super::ops::PlanOp;

/// Stands for every field of a node or link: added, removed or a link.
const WHOLE: &str = "*";

/// What an id names: node and link ids are only unique among their own
/// kind, so a key carries which one it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Of {
    Node,
    Link,
    /// The camera plan's base.
    Camera,
}

type Key<'a> = (Of, &'a str, &'static str);

/// The fields of a node an operation changed.
fn node_fields(before: &PlanNode, after: &PlanNode) -> Vec<&'static str> {
    let mut fields = Vec::new();
    macro_rules! check {
        ($($f:ident),*) => {$(
            if before.$f != after.$f {
                fields.push(stringify!($f));
            }
        )*};
    }
    check!(
        kind, x, y, width, title, text, image, url, asset, direction, stop
    );
    fields
}

/// The nodes and links, with the fields, `entry`'s operations touch.
fn touched(entry: &PlanLogEntry) -> BTreeSet<Key<'_>> {
    let mut out = BTreeSet::new();
    for op in &entry.ops {
        match op {
            PlanOp::AddNode { node } | PlanOp::RemoveNode { node } => {
                out.insert((Of::Node, node.id.as_str(), WHOLE));
            }
            PlanOp::SetNode { before, after } => {
                for field in node_fields(before, after) {
                    out.insert((Of::Node, after.id.as_str(), field));
                }
            }
            PlanOp::AddEdge { edge } | PlanOp::RemoveEdge { edge } => {
                out.insert((Of::Link, edge.id.as_str(), WHOLE));
            }
            PlanOp::SetEdge { after, .. } => {
                out.insert((Of::Link, after.id.as_str(), WHOLE));
            }
            PlanOp::SetCamera { .. } => {
                out.insert((Of::Camera, "", WHOLE));
            }
        }
    }
    out
}

fn overlap(a: &BTreeSet<Key<'_>>, b: &BTreeSet<Key<'_>>) -> bool {
    a.iter().any(|(of, id, field)| {
        b.iter().any(|(other_of, other, f)| {
            of == other_of && id == other && (field == f || *field == WHOLE || *f == WHOLE)
        })
    })
}

/// The parts of the canvas `entry` changed, for a person to read: "text",
/// "title", or "whole node" and "link" for adds, removes and links.
pub(super) fn parts(entry: &PlanLogEntry) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (of, _, field) in touched(entry) {
        let name = match (of, field) {
            (Of::Node, WHOLE) => "whole node".to_string(),
            (Of::Link, _) => "link".to_string(),
            (Of::Camera, _) => "camera base".to_string(),
            (Of::Node, f) => f.to_string(),
        };
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// A later agent change, not undone and not part of this undo, that
/// touched a field of a node or link `entry` changed.
pub(super) fn on_top<'a>(
    all: &'a [PlanLogEntry],
    at: usize,
    entry: &PlanLogEntry,
    undone: &BTreeSet<String>,
    picked: &BTreeSet<&str>,
) -> Option<&'a PlanLogEntry> {
    let mine = touched(entry);
    all[at + 1..].iter().find(|later| {
        later.by.is_some()
            && later.undoes.is_empty()
            && !undone.contains(&later.id)
            && !picked.contains(later.id.as_str())
            && overlap(&mine, &touched(later))
    })
}

/// The earlier changes still held under `picked` on the same fields, and
/// those under them (each on its own fields, so the chain can reach other
/// nodes): a Keep of `picked` asks the person to keep these too.
pub(super) fn beneath<'a>(
    all: &'a [PlanLogEntry],
    picked: &[&'a PlanLogEntry],
    released: &BTreeSet<String>,
) -> Vec<&'a PlanLogEntry> {
    let mut found: Vec<&PlanLogEntry> = Vec::new();
    let mut open: Vec<&PlanLogEntry> = picked.to_vec();
    while let Some(top) = open.pop() {
        let at = all.iter().position(|e| e.id == top.id).unwrap_or(0);
        let mine = touched(top);
        for earlier in &all[..at] {
            let seen = picked.iter().chain(&found).any(|e| e.id == earlier.id);
            if !seen && is_keeper(earlier, released) && overlap(&mine, &touched(earlier)) {
                found.push(earlier);
                open.push(earlier);
            }
        }
    }
    found
}
