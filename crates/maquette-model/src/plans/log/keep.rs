//! Keepers: an agent's lines that hold a person's earlier content, the only
//! copy of it, so rotation never drops them until they are released. A
//! line is released when a later line undoes it (`undoes`, by a person or
//! the agent) or a person keeps it (`keeps`, Keep in the Changes panel).
//!
//! What counts as a person's content follows who wrote in it, not who
//! made the node: a person's node or link (`by` is none), or an agent's
//! node whose content a person wrote ([`PlanNode::person_wrote`]).
//!
//! The sizes of the keepers and of the other lines are kept in memory per
//! log, so an append neither reads the log again to check the budget nor
//! to decide whether it must rotate. The index is read again from the file
//! whenever the file's size or time is not what the last write left, or
//! the file grew by more than the line just appended (a sync tool or
//! another app wrote to it meanwhile).

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::SystemTime;

use super::super::PlanNode;
use super::super::ops::PlanOp;
use super::{PlanLogEntry, READ_MAX_BYTES, line_len, read_tail};
use crate::error::{CoreError, CoreResult};

/// Whether `node` is a person's content: theirs, or an agent's node a
/// person wrote in last.
pub fn protected_node(node: &PlanNode) -> bool {
    node.by.is_none() || node.person_wrote
}

/// The lines `entry` releases: those it undoes and those it keeps.
fn releases(entry: &PlanLogEntry) -> impl Iterator<Item = &String> {
    entry.undoes.iter().chain(entry.keeps.iter())
}

/// The ids some line undid or kept.
pub fn released_ids(entries: &[PlanLogEntry]) -> BTreeSet<String> {
    entries.iter().flat_map(|e| releases(e).cloned()).collect()
}

/// Whether `entry` is a keeper: an agent's change (not an undo) to a
/// person's content that no line in `released` has released.
pub fn is_keeper(entry: &PlanLogEntry, released: &BTreeSet<String>) -> bool {
    entry.by.is_some()
        && !entry.trimmed
        && entry.undoes.is_empty()
        && entry.keeps.is_empty()
        && !released.contains(&entry.id)
        && entry.ops.iter().any(|op| match op {
            PlanOp::SetNode { before, .. } | PlanOp::RemoveNode { node: before } => {
                protected_node(before)
            }
            PlanOp::SetEdge { before, .. } | PlanOp::RemoveEdge { edge: before } => {
                before.by.is_none()
            }
            PlanOp::AddNode { .. }
            | PlanOp::AddEdge { .. }
            | PlanOp::SetCamera { .. }
            | PlanOp::SetAutoLayout { .. }
            | PlanOp::MarkFine { .. }
            | PlanOp::UnmarkFine { .. } => false,
        })
}

/// How many bytes the keepers of `entries` hold.
pub fn keeper_bytes(entries: &[PlanLogEntry]) -> u64 {
    let released = released_ids(entries);
    entries
        .iter()
        .filter(|e| is_keeper(e, &released))
        .map(|e| bytes(line_len(e).unwrap_or(0) + 1))
        .sum()
}

fn bytes(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// The log's size and time, to tell whether the index still describes it.
pub(super) type Stamp = (u64, Option<SystemTime>);

pub(super) fn stamp(file: &Path) -> Stamp {
    fs::metadata(file).map_or((0, None), |m| (m.len(), m.modified().ok()))
}

/// The keepers' sizes by id and the other lines' total, for one log.
struct Index {
    stamp: Stamp,
    keepers: HashMap<String, u64>,
    rest: u64,
}

impl Index {
    fn total(&self) -> u64 {
        self.keepers.values().sum()
    }
}

static INDEX: LazyLock<Mutex<HashMap<PathBuf, Index>>> = LazyLock::new(Mutex::default);

fn build(file: &Path) -> CoreResult<Index> {
    let stamp = stamp(file);
    let text = read_tail(file, READ_MAX_BYTES)?;
    let parsed: Vec<(u64, PlanLogEntry)> = text
        .lines()
        .filter_map(|l| {
            serde_json::from_str(l)
                .ok()
                .map(|e| (bytes(l.len() + 1), e))
        })
        .collect();
    let all: Vec<PlanLogEntry> = parsed.iter().map(|(_, e)| e.clone()).collect();
    let released = released_ids(&all);
    let mut index = Index {
        stamp,
        keepers: HashMap::new(),
        rest: 0,
    };
    for (len, entry) in parsed {
        if is_keeper(&entry, &released) {
            index.keepers.insert(entry.id, len);
        } else {
            index.rest += len;
        }
    }
    Ok(index)
}

/// Runs `f` on the index of `file`, read again when the file changed
/// under it.
fn with_index<T>(file: &Path, f: impl FnOnce(&mut Index) -> T) -> CoreResult<T> {
    let mut all = INDEX.lock().unwrap_or_else(PoisonError::into_inner);
    let fresh = all.get(file).is_some_and(|i| i.stamp == stamp(file));
    if !fresh {
        all.insert(file.to_path_buf(), build(file)?);
    }
    all.get_mut(file)
        .map(f)
        .ok_or_else(|| CoreError::Invalid("The change log index was lost.".into()))
}

/// The bytes the keepers would hold once `entry` (its line `len` bytes)
/// is appended: what it releases goes, and it adds itself when it keeps a
/// person's content.
pub(super) fn held_after(file: &Path, entry: &PlanLogEntry, len: usize) -> CoreResult<u64> {
    let keeper = is_keeper(entry, &BTreeSet::new());
    with_index(file, |index| {
        let freed: u64 = releases(entry).filter_map(|id| index.keepers.get(id)).sum();
        let added = if keeper { bytes(len + 1) } else { 0 };
        index.total().saturating_sub(freed) + added
    })
}

/// Takes in a line just appended (`len` bytes with its line break, after
/// `written` bytes went to the file; a keeper unless its operations were
/// dropped) when the index described the file as it was `before`, else
/// reads the file again. Answers the bytes of
/// the lines that are not keepers, which decide when the log rotates.
pub(super) fn noted(
    file: &Path,
    entry: &PlanLogEntry,
    keeper: bool,
    len: usize,
    written: usize,
    before: Stamp,
) -> CoreResult<u64> {
    let mut all = INDEX.lock().unwrap_or_else(PoisonError::into_inner);
    // One look at the file, so a line another app appends after it shows
    // as a different size on the next write.
    let now = stamp(file);
    match all.get_mut(file) {
        // Only our own line landed since the index was right: anything else
        // (another app appending meanwhile) reads the file again.
        Some(index) if index.stamp == before && now.0 == before.0 + bytes(written) => {
            for id in releases(entry) {
                if let Some(size) = index.keepers.remove(id) {
                    index.rest += size;
                }
            }
            if keeper && is_keeper(entry, &BTreeSet::new()) {
                index.keepers.insert(entry.id.clone(), bytes(len));
            } else {
                index.rest += bytes(len);
            }
            index.stamp = now;
            Ok(index.rest)
        }
        _ => {
            let index = build(file)?;
            let rest = index.rest;
            all.insert(file.to_path_buf(), index);
            Ok(rest)
        }
    }
}

/// Drops the index of `file`, after a rewrite or a move.
pub(super) fn forget(file: &Path) {
    INDEX
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(file);
}
