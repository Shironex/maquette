//! A person keeping (accepting) agent changes: one log line that releases
//! them from the room the log holds for undo, with every earlier change
//! still held under them named and confirmed first.

use std::collections::BTreeSet;
use std::path::Path;

use super::super::files::writing;
use super::super::log::{self, PlanLogEntry};
use super::super::summary::ChangeMark;
use super::super::{load_file, path};
use super::stack::{self, beneath};
use crate::error::{CoreError, CoreResult};
use crate::project::now_rfc3339;

/// An earlier change a Keep must take along, for the person to confirm.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KeepNeed {
    pub id: String,
    /// When it was made (RFC 3339), so changes with the same summary differ.
    pub at: String,
    pub summary: String,
    /// The parts it changed: field names, "whole node" or "link".
    pub parts: Vec<String>,
}

/// Keeps (accepts) the agent changes `ids` as a person: one log line that
/// releases them, so the room they held for undo is free again. The canvas
/// does not change, and each stays undoable while it is among the newest
/// lines. Earlier agent changes still held under them on the same fields
/// ([`keep_needs`]) must be named too: a kept change rotating out must not
/// leave an older one whose undo would release the person's words, and
/// nothing is kept that the person did not confirm.
///
/// # Errors
///
/// Fails when the canvas is gone, a change is not in the log or holds
/// nothing of a person's that is still kept for undo, or the log cannot be
/// written.
///
/// It accepts any keepable ids, ones the person never saw listed too, so it
/// must never be exposed to agents. The app keeps through
/// [`keep_confirmed`], which holds the list to what the person confirmed.
pub fn keep(project_dir: &Path, id: &str, ids: &[String]) -> CoreResult<PlanLogEntry> {
    let _lock = writing();
    keep_locked(project_dir, id, ids)
}

/// Keeps the changes the person clicked (`clicked`) with the earlier ones
/// they confirmed (`confirmed`), refused unless `confirmed` is exactly what
/// [`keep_needs`] lists for `clicked` now: one undone, kept or added since
/// the person saw the list means they review it again.
///
/// # Errors
///
/// Refuses a list that changed since; fails as [`keep`].
pub fn keep_confirmed(
    project_dir: &Path,
    id: &str,
    clicked: &[String],
    confirmed: &[String],
) -> CoreResult<PlanLogEntry> {
    let _lock = writing();
    let all = log::entries(project_dir, id)?;
    let released = log::released_ids(&all);
    let picked: Vec<&PlanLogEntry> = all.iter().filter(|e| clicked.contains(&e.id)).collect();
    let needs: BTreeSet<&str> = beneath(&all, &picked, &released)
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    let seen: BTreeSet<&str> = confirmed.iter().map(String::as_str).collect();
    if needs != seen {
        return Err(CoreError::Invalid(
            "The list changed since you saw it, so nothing was kept. Review again.".into(),
        ));
    }
    let ids: Vec<String> = clicked.iter().chain(confirmed).cloned().collect();
    keep_locked(project_dir, id, &ids)
}

fn keep_locked(project_dir: &Path, id: &str, ids: &[String]) -> CoreResult<PlanLogEntry> {
    if ids.is_empty() {
        return Err(CoreError::Invalid("Name the changes to keep.".into()));
    }
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
    let missing: Vec<&str> = beneath(&all, &picked, &released)
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(CoreError::Invalid(format!(
            "Keeping these also needs the earlier changes {} kept: they sit under them on the same parts of the canvas. Name them too once the person confirmed.",
            missing.join(", ")
        )));
    }
    let summary = match picked.as_slice() {
        [one] => format!("Kept: {}", one.summary),
        many => format!("Kept {} agent changes", many.len()),
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
        keeps: picked.iter().map(|e| e.id.clone()).collect(),
        trimmed: false,
        applied: None,
        proposes: None,
        resolves: Vec::new(),
    };
    log::append(project_dir, id, &entry)?;
    Ok(entry)
}

/// The earlier agent changes a Keep of `ids` must take along: still held,
/// under them on the same fields (and under those). Empty when the Keep
/// can go as it is.
///
/// # Errors
///
/// Fails when the log cannot be read.
pub fn keep_needs(project_dir: &Path, id: &str, ids: &[String]) -> CoreResult<Vec<KeepNeed>> {
    let all = log::entries(project_dir, id)?;
    let released = log::released_ids(&all);
    let picked: Vec<&PlanLogEntry> = all.iter().filter(|e| ids.contains(&e.id)).collect();
    Ok(beneath(&all, &picked, &released)
        .into_iter()
        .map(|e| KeepNeed {
            id: e.id.clone(),
            at: e.at.clone(),
            summary: e.summary.clone(),
            parts: stack::parts(e),
        })
        .collect())
}
