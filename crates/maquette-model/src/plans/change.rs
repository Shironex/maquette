//! Writing a change to a canvas: operations laid over the file as it is
//! now, under the same lock, checks and size cap as a save, then a line in
//! the change log. A person's edits, an agent's tools and an undo all come
//! through [`change`], so neither side writes over the other.

use std::path::Path;

use super::files::{doc, write_plan, writing};
use super::log::{self, PlanLogEntry};
use super::ops::{self, Merge, PlanOp};
use super::summary::{self, ChangeMark};
use super::{PlanDoc, PlanFile, load_file, normalized, path, validate};
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
    validate(&next)?;
    if next == current {
        return Ok((doc(project_dir, id, current), None));
    }
    let done = ops::diff(&current, &next);
    next.edited = now_rfc3339();
    write_plan(&path(project_dir, id)?, &next)?;
    let entry = entry_of(writer, done, undoing);
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
            if entry.trimmed || entry.ops.is_empty() {
                return Err(CoreError::Invalid(format!(
                    "The change {want} can not be undone: it changed no nodes or was too big to keep."
                )));
            }
            if let Some(agent) = only_by
                && entry.by.as_deref() != Some(agent)
            {
                return Err(CoreError::Invalid(format!(
                    "The change {want} was made by {}, not {agent}. Agents undo only their own changes.",
                    entry.by.as_deref().unwrap_or("a person")
                )));
            }
            if !picked.iter().any(|(_, e)| e.id == entry.id) {
                picked.push((at, entry));
            }
        }
        // The log is oldest first: undo the newest first.
        picked.sort_by(|a, b| b.0.cmp(&a.0));
        let ops = picked
            .iter()
            .flat_map(|(_, e)| ops::invert(&e.ops))
            .collect();
        let summary = match picked.as_slice() {
            [(_, one)] => format!("Undid: {}", one.summary),
            many => format!("Undid {} changes", many.len()),
        };
        let ids = picked.iter().map(|(_, e)| e.id.clone()).collect();
        Ok((ops, Some(Undoing { ids, summary })))
    })
}

#[cfg(test)]
mod tests;
