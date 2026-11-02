//! Writing a change to a canvas: operations laid over the file as it is
//! now, under the same lock, checks and size cap as a save, then a line in
//! the change log. A person's edits, an agent's tools and an undo all come
//! through [`change`], so neither side writes over the other.

use std::collections::BTreeSet;
use std::path::Path;

use super::files::{doc, write_plan, writing};
use super::log::{self, PlanLogEntry};
use super::ops::{self, Merge, PlanOp};
use stack::{beneath, on_top};

mod stack;
use super::summary::{self, ChangeMark};
use super::{PlanDoc, PlanFile, PlanNode, load_file, normalized, path, validate};
use crate::error::{CoreError, CoreResult};
use crate::project::now_rfc3339;

/// Who writes a change and how it meets the file.
#[derive(Debug, Clone)]
pub struct Writer<'a> {
    /// An agent's name; `None` for a person.
    pub by: Option<&'a str>,
    /// An agent's run, grouping its calls in the log.
    pub run: Option<&'a str>,
    pub merge: Merge,
}

impl Writer<'_> {
    /// A person in the app: their values win.
    pub const PERSON: Writer<'static> = Writer {
        by: None,
        run: None,
        merge: Merge::Mine,
    };
}

/// What a change wrote.
#[derive(Debug, Clone)]
pub struct Changed {
    pub doc: PlanDoc,
    /// The log line, or `None` when nothing changed.
    pub entry: Option<PlanLogEntry>,
    /// Operations that did not apply whole (see [`ops::apply`]).
    pub skipped: u32,
}

/// What an undo says in the log instead of a summary of its operations.
#[derive(Debug, Clone, Default)]
pub(super) struct Undoing {
    pub(super) ids: Vec<String>,
    pub(super) summary: String,
    /// The summary when everything it would undo was changed since.
    pub(super) idle: String,
}

/// Whether two states of a node say the same (where it sits aside).
fn same_content(a: &PlanNode, b: &PlanNode) -> bool {
    a.kind == b.kind
        && a.title == b.title
        && a.text == b.text
        && a.image == b.image
        && a.url == b.url
        && a.asset == b.asset
        && a.direction == b.direction
}

/// Marks agent nodes a person wrote in: a person's write to the content
/// sets [`PlanNode::person_wrote`] and no agent's write clears it, so every
/// agent change on top of the person's words stays held for undo.
fn mark_writers(current: &PlanFile, next: &mut PlanFile, person: bool) {
    for node in next.nodes.iter_mut().filter(|n| n.by.is_some()) {
        if let Some(was) = current.nodes.iter().find(|c| c.id == node.id) {
            node.person_wrote = was.person_wrote || (person && !same_content(was, node));
        }
    }
}

/// A log line for `ops` written by `writer`.
fn entry_of(writer: &Writer<'_>, ops: Vec<PlanOp>, undoing: Option<Undoing>) -> PlanLogEntry {
    let (summary, mark, undoes) = match undoing {
        None => (summary::summary(&ops), summary::mark(&ops), Vec::new()),
        Some(u) => (u.summary, ChangeMark::Undo, u.ids),
    };
    PlanLogEntry {
        id: log::new_id("c"),
        at: now_rfc3339(),
        by: writer.by.map(str::to_string),
        run: writer.run.map(str::to_string),
        summary,
        mark,
        ops,
        undoes,
        keeps: Vec::new(),
        trimmed: false,
    }
}

/// Appends a log line; a log that cannot be written never fails the write
/// it describes, since the canvas file is what counts.
pub(super) fn note(project_dir: &Path, id: &str, entry: &PlanLogEntry) {
    if let Err(err) = log::append(project_dir, id, entry) {
        tracing::warn!(error = %err, canvas = id, "the canvas change log was not written");
    }
}

/// Refuses an agent's write whose undo the log could not keep: a line over
/// [`log::LINE_MAX_BYTES`], or a change to a person's content once the
/// keepers would pass [`log::KEEP_MAX_BYTES`]. So a person's earlier text
/// is never lost to trimming or rotation. An undo is never refused: it
/// holds nothing of a person's and frees what it undoes.
fn keep_room(project_dir: &Path, id: &str, entry: &PlanLogEntry) -> CoreResult<()> {
    if !entry.undoes.is_empty() {
        return Ok(());
    }
    let len = log::line_len(entry)?;
    if len > log::LINE_MAX_BYTES {
        return Err(CoreError::Invalid(format!(
            "This change is too big to keep for undo ({} KB; the most is {} KB). Send it in smaller calls, a few nodes at a time.",
            len / 1024,
            log::LINE_MAX_BYTES / 1024
        )));
    }
    if !log::is_keeper(entry, &BTreeSet::new()) {
        return Ok(());
    }
    if log::held_after(project_dir, id, entry, len)? > log::KEEP_MAX_BYTES {
        return Err(CoreError::Invalid(format!(
            "The change log already keeps {} MB of undo for agent changes to a person's content, the most it keeps. Ask the person to undo them or press Keep (or Keep run) in the Changes panel, which frees that room; undo_run of your own changes frees it too, and adding new nodes still works.",
            log::KEEP_MAX_BYTES / 1024 / 1024
        )));
    }
    Ok(())
}

/// Writes `next` over `current` (both the file's canvas) when it differs,
/// keeping the name, `created` and preset, and logs what changed.
pub(super) fn commit(
    project_dir: &Path,
    id: &str,
    current: PlanFile,
    next: &PlanFile,
    writer: &Writer<'_>,
    undoing: Option<Undoing>,
) -> CoreResult<(PlanDoc, Option<PlanLogEntry>)> {
    let mut next = normalized(next);
    next.name = current.name.clone();
    next.created = current.created.clone();
    next.preset = current.preset;
    next.edited = current.edited.clone();
    if undoing.is_none() {
        mark_writers(&current, &mut next, writer.by.is_none());
    }
    validate(&next)?;
    if next == current {
        let Some(undoing) = undoing else {
            return Ok((doc(project_dir, id, current), None));
        };
        // Everything it would undo was changed since: the line still
        // releases what it names, so nothing stays held for an undo that
        // can no longer happen.
        let idle = Undoing {
            summary: undoing.idle.clone(),
            ..undoing
        };
        let entry = entry_of(writer, Vec::new(), Some(idle));
        log::append(project_dir, id, &entry)?;
        return Ok((doc(project_dir, id, current), Some(entry)));
    }
    let done = ops::diff(&current, &next);
    let entry = entry_of(writer, done, undoing);
    if writer.by.is_some() {
        keep_room(project_dir, id, &entry)?;
    }
    next.edited = now_rfc3339();
    write_plan(&path(project_dir, id)?, &next)?;
    note(project_dir, id, &entry);
    Ok((doc(project_dir, id, next), Some(entry)))
}

/// Lays the operations `make` returns over the canvas `id` as the file
/// holds it now and writes the result. `make` sees that canvas, so an
/// agent places nodes against what is really there.
///
/// # Errors
///
/// Fails when the canvas is gone, `make` refuses, the result does not
/// validate or is over the size cap, or the file cannot be written.
pub fn change(
    project_dir: &Path,
    id: &str,
    writer: &Writer<'_>,
    make: impl FnOnce(&PlanFile) -> CoreResult<Vec<PlanOp>>,
) -> CoreResult<Changed> {
    change_with(project_dir, id, writer, |plan| Ok((make(plan)?, None)))
}

fn change_with(
    project_dir: &Path,
    id: &str,
    writer: &Writer<'_>,
    make: impl FnOnce(&PlanFile) -> CoreResult<(Vec<PlanOp>, Option<Undoing>)>,
) -> CoreResult<Changed> {
    let _lock = writing();
    let current = load_file(&path(project_dir, id)?)?;
    let (ops, undoing) = make(&current)?;
    let applied = ops::apply(&current, &ops, writer.merge);
    let (doc, entry) = commit(project_dir, id, current, &applied.plan, writer, undoing)?;
    Ok(Changed {
        doc,
        entry,
        skipped: applied.skipped,
    })
}

/// Undoes the log lines `ids` of the canvas `id`, newest first, carefully:
/// a field someone changed since stays as they left it, and a node changed
/// since is not removed. With `only_by` every line must be that agent's.
///
/// # Errors
///
/// Fails when a line is not in the log, was undone already, kept no
/// operations or (with `only_by`) is someone else's, and as [`change`].
pub fn undo(
    project_dir: &Path,
    id: &str,
    ids: &[String],
    by: Option<&str>,
    only_by: Option<&str>,
) -> CoreResult<Changed> {
    if ids.is_empty() {
        return Err(CoreError::Invalid("Name the changes to undo.".into()));
    }
    let writer = Writer {
        by,
        run: None,
        merge: Merge::Careful,
    };
    change_with(project_dir, id, &writer, |_| {
        let all = log::entries(project_dir, id)?;
        let undone = log::undone_ids(&all);
        let mut picked: Vec<(usize, &PlanLogEntry)> = Vec::new();
        for want in ids {
            let (at, entry) = all
                .iter()
                .enumerate()
                .find(|(_, e)| &e.id == want)
                .ok_or_else(|| CoreError::NotFound(format!("the change {want} in the log")))?;
            if undone.contains(want) {
                return Err(CoreError::Invalid(format!(
                    "The change {want} was undone already."
                )));
            }
            if !entry.undoes.is_empty() {
                return Err(CoreError::Invalid(format!(
                    "The change {want} is an undo, and an undo can not be undone. Redo your own edits with Redo on the canvas; an agent makes its change again instead."
                )));
            }
            if entry.trimmed || entry.ops.is_empty() {
                return Err(CoreError::Invalid(format!(
                    "The change {want} can not be undone: it changed no nodes or was too big to keep."
                )));
            }
            if let Some(agent) = only_by
                && entry.by.as_deref() != Some(agent)
            {
                return Err(CoreError::Invalid(format!(
                    "The change {want} was made by {}, not {agent}. Agents undo only changes made under their own name.",
                    entry.by.as_deref().unwrap_or("a person")
                )));
            }
            if !picked.iter().any(|(_, e)| e.id == entry.id) {
                picked.push((at, entry));
            }
        }
        let names: BTreeSet<&str> = picked.iter().map(|(_, e)| e.id.as_str()).collect();
        for (at, entry) in &picked {
            if let Some(later) = on_top(&all, *at, entry, &undone, &names) {
                return Err(CoreError::Invalid(format!(
                    "The change {} can not be undone yet: a later agent change ({}, {}) changed the same node or link on top of it. Undo the later change first, or both together.",
                    entry.id, later.id, later.summary
                )));
            }
        }
        // The log is oldest first: undo the newest first.
        picked.sort_by(|a, b| b.0.cmp(&a.0));
        let ops = picked
            .iter()
            .flat_map(|(_, e)| ops::invert(&e.ops))
            .collect();
        let (summary, idle) = match picked.as_slice() {
            [(_, one)] => (
                format!("Undid: {}", one.summary),
                format!("Nothing to undo, already changed: {}", one.summary),
            ),
            many => (
                format!("Undid {} changes", many.len()),
                format!("Nothing to undo, {} changes already changed", many.len()),
            ),
        };
        let ids = picked.iter().map(|(_, e)| e.id.clone()).collect();
        Ok((ops, Some(Undoing { ids, summary, idle })))
    })
}

/// What a Keep wrote, and the earlier changes it kept with the ones named.
#[derive(Debug, Clone)]
pub struct Kept {
    pub entry: PlanLogEntry,
    /// The summaries of earlier changes to the same fields, kept too.
    pub also: Vec<String>,
}

/// Keeps (accepts) the agent changes `ids` as a person: one log line that
/// releases them, so the room they held for undo is free again. The canvas
/// does not change, and each stays undoable while it is among the newest
/// lines. Earlier agent changes still held under them on the same fields
/// are kept too: the person accepted the newer content, and a kept change
/// rotating out must not leave an older one to release the person's words.
///
/// # Errors
///
/// Fails when the canvas is gone, a change is not in the log or holds
/// nothing of a person's that is still kept for undo, or the log cannot be
/// written.
pub fn keep(project_dir: &Path, id: &str, ids: &[String]) -> CoreResult<Kept> {
    if ids.is_empty() {
        return Err(CoreError::Invalid("Name the changes to keep.".into()));
    }
    let _lock = writing();
    load_file(&path(project_dir, id)?)?;
    let all = log::entries(project_dir, id)?;
    let released = log::released_ids(&all);
    let mut picked: Vec<&PlanLogEntry> = Vec::new();
    for want in ids {
        let entry = all
            .iter()
            .find(|e| &e.id == want)
            .ok_or_else(|| CoreError::NotFound(format!("the change {want} in the log")))?;
        if !log::is_keeper(entry, &released) {
            return Err(CoreError::Invalid(format!(
                "The change {want} has nothing to keep: it is not an agent's change to your content, or it was undone or kept already."
            )));
        }
        if !picked.iter().any(|e| e.id == entry.id) {
            picked.push(entry);
        }
    }
    // The person accepted the newer content, so the changes it sits on go too.
    let also = beneath(&all, &picked, &released);
    let summary = match (picked.as_slice(), also.as_slice()) {
        ([one], []) => format!("Kept: {}", one.summary),
        ([one], [under]) => format!("Kept: {}, and the earlier {}", one.summary, under.summary),
        (many, under) => format!("Kept {} agent changes", many.len() + under.len()),
    };
    let entry = PlanLogEntry {
        id: log::new_id("c"),
        at: now_rfc3339(),
        by: None,
        run: None,
        summary,
        mark: ChangeMark::Keep,
        ops: Vec::new(),
        undoes: Vec::new(),
        keeps: picked.iter().chain(&also).map(|e| e.id.clone()).collect(),
        trimmed: false,
    };
    log::append(project_dir, id, &entry)?;
    Ok(Kept {
        also: also.iter().map(|e| e.summary.clone()).collect(),
        entry,
    })
}

#[cfg(test)]
mod keep_tests;
#[cfg(test)]
mod redo_tests;
#[cfg(test)]
mod release_tests;
#[cfg(test)]
mod stack_tests;
#[cfg(test)]
mod tests;
