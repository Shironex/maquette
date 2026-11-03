//! Reviewing and applying the camera plan's main line to `layout.json`, and
//! undoing an Apply.
//!
//! A review is the exact diff against `layout.json` as it is now, with two
//! keys: one of the whole layout and one of what the plan would write. An
//! Apply takes both back and refuses when either changed since the review
//! was shown, so a stale diff is never written. It writes through
//! [`layout::update`] (its lock, checks and atomic write), keeps the layout
//! it replaced in `plans/history/`, moves the plan's base to the new stops
//! and logs the Apply as the person's change. Its undo puts the old layout
//! back only while `layout.json` is still exactly what the Apply wrote.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::check::is_project_relative;
use super::super::files::{doc, plan_text, write_plan, writing};
use super::super::log::{self, PlanLogEntry};
use super::super::ops::{self, Merge};
use super::super::summary::ChangeMark;
use super::super::{PLANS_DIR, PlanDoc, PlanFile, load_file, normalized, path, validate};
use super::{
    Applied, CameraBase, HISTORY_DIR, StopDiff, chain_of, chain_stops, diff_stops, fingerprint,
    is_stale, layout_key, main_chain,
};
use crate::error::{CoreError, CoreResult};
use crate::layout::{self, LayoutStop, MAX_STOPS, SceneLayout};
use crate::project::now_rfc3339;

/// What Apply would write, as the review dialog and agents read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CameraReview {
    pub diff: StopDiff,
    /// The line reviewed, its node ids in tour order.
    pub chain: Vec<String>,
    /// Triggers and timeline keys that wait for a stop the Apply removes,
    /// which go with it.
    pub dropped: u32,
    /// The plan is based on other stops than `layout.json` has now.
    pub stale: bool,
    /// Why it can not be applied; empty when it can.
    pub problems: Vec<String>,
    /// Give both back to Apply: it refuses when either changed since.
    pub layout_key: String,
    pub plan_key: String,
}

/// The review of the plan's line holding `from` (its main line when none)
/// against `layout`.
pub fn review_of(plan: &PlanFile, layout: &SceneLayout, from: Option<&str>) -> CameraReview {
    reviewed(plan, layout, from).0
}

/// The review and the stops the line would write.
fn reviewed(
    plan: &PlanFile,
    layout: &SceneLayout,
    from: Option<&str>,
) -> (CameraReview, Vec<LayoutStop>) {
    let chain = match from {
        Some(id) => chain_of(plan, id),
        None => main_chain(plan),
    };
    let mut problems = Vec::new();
    let (nodes, stops): (Vec<String>, Vec<LayoutStop>) = match &chain {
        Some(c) => (c.nodes.clone(), chain_stops(plan, c)),
        None => (Vec::new(), Vec::new()),
    };
    if let Some(problem) = chain.as_ref().and_then(|c| c.problem.clone()) {
        problems.push(problem);
    }
    if stops.is_empty() {
        problems.push("The plan has no line of stops to apply. Add a stop first.".into());
    }
    if stops.len() > MAX_STOPS {
        problems.push(format!("Keep the line to {MAX_STOPS} stops or fewer."));
    }
    let stale = is_stale(plan, layout);
    if stale {
        problems.push("layout.json changed since this plan was based on it. Compare or rebase first, then review again.".into());
    }
    let diff = diff_stops(&layout.stops, &stops, &nodes);
    let mut next = layout.clone();
    next.stops.clone_from(&stops);
    let dropped = u32::try_from(layout::prune_stops(&mut next)).unwrap_or(u32::MAX);
    if problems.is_empty() {
        if diff.is_empty() {
            problems.push("Nothing to apply: layout.json already has this line.".into());
        } else if let Err(err) = layout::validate(&next) {
            problems.push(err.to_string());
        }
    }
    let review = CameraReview {
        plan_key: fingerprint(&(&nodes, &stops)),
        layout_key: layout_key(layout),
        diff,
        chain: nodes,
        dropped,
        stale,
        problems,
    };
    (review, stops)
}

/// The review of the camera plan `id`'s main line against `layout.json`.
///
/// # Errors
///
/// Fails when the canvas or `layout.json` cannot be read.
pub fn review(project_dir: &Path, id: &str) -> CoreResult<CameraReview> {
    let plan = load_file(&path(project_dir, id)?)?;
    let layout = layout::load(project_dir)?;
    Ok(review_of(&plan, &layout, None))
}

fn stale_review() -> CoreError {
    CoreError::Invalid(
        "layout.json changed since this review was shown, so nothing was applied. Review again to see the changes as they are now.".into(),
    )
}

/// A person's change checked and ready to write: the canvas, its text and
/// its log line. Everything that can refuse it has run.
pub(super) struct Ready {
    plan: PlanFile,
    text: String,
    entry: PlanLogEntry,
}

/// Checks `next` over `current` (the file's canvas, read under the lock)
/// as the person's change, its operations in `entry`, without writing:
/// refused when the canvas would be invalid or too big, or the log line too
/// long to keep its operations, since a line without them can not be undone.
pub(super) fn prepare_person(
    project_dir: &Path,
    id: &str,
    current: &PlanFile,
    next: &PlanFile,
    entry: PlanLogEntry,
) -> CoreResult<Ready> {
    let mut next = normalized(next);
    next.name.clone_from(&current.name);
    next.created.clone_from(&current.created);
    next.preset = current.preset;
    validate(&next)?;
    let mut entry = entry;
    entry.ops = ops::diff(current, &next);
    let len = log::line_len(&entry)?;
    if len > log::LINE_MAX_BYTES {
        return Err(CoreError::Invalid(format!(
            "This change is too big to keep for undo ({} KB; the most is {} KB), so nothing was written. Change fewer stops at a time.",
            len / 1024,
            log::LINE_MAX_BYTES / 1024
        )));
    }
    next.edited = now_rfc3339();
    let text = plan_text(&path(project_dir, id)?, &next)?;
    Ok(Ready {
        plan: next,
        text,
        entry,
    })
}

/// Writes a checked change and its log line. When the line can not be
/// written, the canvas goes back to `current`, so the file never moves
/// without its line. The caller holds the canvas lock.
pub(super) fn commit_person(
    project_dir: &Path,
    id: &str,
    current: &PlanFile,
    ready: Ready,
) -> CoreResult<(PlanDoc, PlanLogEntry)> {
    let file = path(project_dir, id)?;
    maquette_base::fs::write_atomic(&file, ready.text.as_bytes())
        .map_err(|e| CoreError::io(&file, e))?;
    if let Err(err) = log::append(project_dir, id, &ready.entry) {
        if let Err(back) = write_plan(&file, current) {
            tracing::warn!("the camera plan could not be put back after its log failed: {back}");
        }
        return Err(err);
    }
    Ok((doc(project_dir, id, ready.plan), ready.entry))
}

/// [`prepare_person`] then [`commit_person`].
pub(super) fn write_person(
    project_dir: &Path,
    id: &str,
    current: &PlanFile,
    next: &PlanFile,
    entry: PlanLogEntry,
) -> CoreResult<(PlanDoc, PlanLogEntry)> {
    let ready = prepare_person(project_dir, id, current, next, entry)?;
    commit_person(project_dir, id, current, ready)
}

/// Puts the layout kept in `file` back as `layout.json`, only while
/// `layout.json` is still the one whose key is `after`. Answers with the
/// layout it replaced (seen under the layout lock) and the one it wrote.
fn put_back(
    project_dir: &Path,
    file: &Path,
    after: &str,
    changed: &str,
) -> CoreResult<(SceneLayout, SceneLayout)> {
    let mut replaced = None;
    let written = layout::update(project_dir, |current| {
        if layout_key(current) != after {
            return Err(CoreError::Invalid(changed.into()));
        }
        let text = fs::read_to_string(file).map_err(|e| CoreError::io(file, e))?;
        let kept = crate::json::from_json_with_defaults::<SceneLayout>(&text)
            .map_err(|e| CoreError::json(file, e))?;
        replaced = Some(current.clone());
        Ok(kept)
    })?;
    let replaced = replaced.ok_or_else(|| CoreError::Invalid(changed.into()))?;
    Ok((replaced, written))
}

/// Writes `replaced` back as `layout.json` after an undo that could not be
/// logged, only while `layout.json` is still `written` (the undo's own
/// write); the check and the write share the layout lock, so a newer
/// layout from another writer is never written over.
fn roll_back(project_dir: &Path, written: &SceneLayout, replaced: &SceneLayout) -> CoreResult<()> {
    let key = layout_key(written);
    layout::update(project_dir, |current| {
        if layout_key(current) != key {
            return Err(CoreError::Invalid(
                "layout.json changed after the undo, so it was left as it is now.".into(),
            ));
        }
        Ok(replaced.clone())
    })
    .map(|_| ())
}

/// The kept layout an Apply line names, refused unless it sits in
/// `plans/history/` inside the project.
fn history_file(project_dir: &Path, history: &str) -> CoreResult<std::path::PathBuf> {
    let folder = format!("{PLANS_DIR}/{HISTORY_DIR}/");
    if !is_project_relative(history) || !history.starts_with(&folder) {
        return Err(CoreError::Invalid(format!(
            "The Apply line names {history}, which is not a kept layout in {folder}, so nothing was put back."
        )));
    }
    Ok(project_dir.join(history))
}

pub(super) fn person_line(
    summary: String,
    mark: ChangeMark,
    applied: Option<Applied>,
) -> PlanLogEntry {
    PlanLogEntry {
        id: log::new_id("c"),
        at: now_rfc3339(),
        by: None,
        run: None,
        summary,
        mark,
        ops: Vec::new(),
        undoes: Vec::new(),
        keeps: Vec::new(),
        trimmed: false,
        applied,
    }
}

/// Applies the camera plan `id`'s main line to `layout.json`, as the
/// person. `layout_key` and `plan_key` are the review's.
///
/// # Errors
///
/// Refuses, writing nothing, when `layout.json` or the plan changed since
/// the review, or the review has problems; fails when a file can not be
/// read or written.
pub fn apply(
    project_dir: &Path,
    id: &str,
    reviewed_layout: &str,
    plan_key: &str,
) -> CoreResult<(PlanDoc, PlanLogEntry)> {
    let _lock = writing();
    let plan = load_file(&path(project_dir, id)?)?;
    let change_id = log::new_id("c");
    let history = format!("{PLANS_DIR}/{HISTORY_DIR}/{change_id}.layout.json");
    let kept = project_dir.join(&history);
    let mut ready: Option<Ready> = None;
    let written = layout::update(project_dir, |current| {
        if layout_key(current) != reviewed_layout {
            return Err(stale_review());
        }
        let (review, stops) = reviewed(&plan, current, None);
        if review.plan_key != plan_key {
            return Err(CoreError::Invalid(
                "The camera plan changed since this review was shown, so nothing was applied. Review again.".into(),
            ));
        }
        if let Some(problem) = review.problems.first() {
            return Err(CoreError::Invalid(problem.clone()));
        }
        let mut next = current.clone();
        next.stops = stops;
        layout::prune_stops(&mut next);
        // The key of the bytes the write stores (version, tidied clips), so the
        // undo's check matches the file as it reads back, hand edited or not.
        let applied = Applied {
            history: history.clone(),
            before: layout_key(current),
            after: layout_key(&layout::stored(&next)),
        };
        let mut entry = person_line(
            format!(
                "Applied the camera plan to layout.json: {}",
                review.diff.counts_line()
            ),
            ChangeMark::Apply,
            Some(applied),
        );
        entry.id.clone_from(&change_id);
        let mut based = plan.clone();
        based.camera = Some(CameraBase {
            stops: next.stops.clone(),
        });
        // Everything that can refuse the plan and its line runs before layout.json is written.
        ready = Some(prepare_person(project_dir, id, &plan, &based, entry)?);
        if let Some(folder) = kept.parent() {
            fs::create_dir_all(folder).map_err(|e| CoreError::io(folder, e))?;
        }
        crate::json::write_json(&kept, current)?;
        Ok(next)
    });
    let written = match written {
        Ok(written) => written,
        Err(err) => {
            // Nothing was applied: no copy of a layout that was never replaced stays behind.
            let _ = fs::remove_file(&kept);
            return Err(err);
        }
    };
    let Some(ready) = ready else {
        return Err(stale_review());
    };
    commit_person(project_dir, id, &plan, ready).map_err(|err| {
        let changed = "layout.json changed again while the Apply was being written.";
        match put_back(project_dir, &kept, &layout_key(&written), changed) {
            Ok(_) => {
                let _ = fs::remove_file(&kept);
                CoreError::Invalid(format!(
                    "The Apply could not be logged, so layout.json was put back as it was: {err}"
                ))
            }
            Err(back) => CoreError::Invalid(format!(
                "The Apply could not be logged ({err}) and layout.json could not be put back ({back}). The layout from before is kept at {history}."
            )),
        }
    })
}

/// Undoes the Apply logged as `change`: puts back the `layout.json` it
/// replaced, only while the file is still what the Apply wrote, and moves
/// the plan's base back.
///
/// # Errors
///
/// Refuses when the line is no Apply, was undone already, or `layout.json`
/// changed since; fails when a file can not be read or written.
pub fn undo_apply(
    project_dir: &Path,
    id: &str,
    change: &str,
) -> CoreResult<(PlanDoc, PlanLogEntry)> {
    let _lock = writing();
    let plan = load_file(&path(project_dir, id)?)?;
    let all = log::entries(project_dir, id)?;
    let entry = all
        .iter()
        .find(|e| e.id == change)
        .ok_or_else(|| CoreError::NotFound(format!("the change {change} in the log")))?;
    let Some(applied) = &entry.applied else {
        return Err(CoreError::Invalid(format!(
            "The change {change} is not an Apply of the camera plan."
        )));
    };
    if log::undone_ids(&all).contains(change) {
        return Err(CoreError::Invalid(format!(
            "The Apply {change} was undone already."
        )));
    }
    let file = history_file(project_dir, &applied.history)?;
    let back = ops::apply(&plan, &ops::invert(&entry.ops), Merge::Careful).plan;
    let mut line = person_line(format!("Undid: {}", entry.summary), ChangeMark::Undo, None);
    line.undoes = vec![change.to_string()];
    let mut based = plan.clone();
    based.camera = back.camera;
    let ready = prepare_person(project_dir, id, &plan, &based, line)?;
    let (replaced, written) = put_back(
        project_dir,
        &file,
        &applied.after,
        "layout.json changed since this Apply, so it was not put back: an undo would lose those changes. Change the stops by hand, or apply the plan again.",
    )?;
    // Nothing that can fail sits between putting the layout back and logging it.
    commit_person(project_dir, id, &plan, ready).inspect_err(|_| {
        // The undo could not be logged: layout.json goes back to what the Apply wrote.
        if let Err(back) = roll_back(project_dir, &written, &replaced) {
            tracing::warn!("layout.json could not be put back after a failed undo: {back}");
        }
    })
}

#[cfg(test)]
mod tests;
