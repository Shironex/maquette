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
    check!(kind, x, y, width, title, text, image, url, asset, direction);
    fields
}

/// The node and link ids, with the fields, `entry`'s operations touch.
fn touched(entry: &PlanLogEntry) -> BTreeSet<(&str, &'static str)> {
    let mut out = BTreeSet::new();
    for op in &entry.ops {
        match op {
            PlanOp::AddNode { node } | PlanOp::RemoveNode { node } => {
                out.insert((node.id.as_str(), WHOLE));
            }
            PlanOp::SetNode { before, after } => {
                for field in node_fields(before, after) {
                    out.insert((after.id.as_str(), field));
                }
            }
            PlanOp::AddEdge { edge } | PlanOp::RemoveEdge { edge } => {
                out.insert((edge.id.as_str(), WHOLE));
            }
            PlanOp::SetEdge { after, .. } => {
                out.insert((after.id.as_str(), WHOLE));
            }
        }
    }
    out
}

fn overlap(a: &BTreeSet<(&str, &str)>, b: &BTreeSet<(&str, &str)>) -> bool {
    a.iter().any(|(id, field)| {
        b.iter()
            .any(|(other, f)| id == other && (field == f || *field == WHOLE || *f == WHOLE))
    })
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
/// those under them: keeping the newer content keeps what it sits on.
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
