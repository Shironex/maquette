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

use super::super::files::{doc, write_plan, writing};
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

/// Writes the plan's base move and the person's log line for an Apply or
/// its undo.
fn note_plan(
    project_dir: &Path,
    id: &str,
    current: &PlanFile,
    base: Option<CameraBase>,
    entry: PlanLogEntry,
) -> CoreResult<(PlanDoc, PlanLogEntry)> {
    let mut next = current.clone();
    next.camera = base;
    write_person(project_dir, id, current, &next, entry)
}

/// Writes `next` over `current` (the file's canvas, read under the lock)
/// as the person's change, its operations in `entry`. The caller holds the
/// canvas lock.
pub(super) fn write_person(
    project_dir: &Path,
    id: &str,
    current: &PlanFile,
    next: &PlanFile,
    entry: PlanLogEntry,
) -> CoreResult<(PlanDoc, PlanLogEntry)> {
    let mut next = normalized(next);
    next.name.clone_from(&current.name);
    next.created.clone_from(&current.created);
    next.preset = current.preset;
    validate(&next)?;
    let mut entry = entry;
    entry.ops = ops::diff(current, &next);
    next.edited = now_rfc3339();
    write_plan(&path(project_dir, id)?, &next)?;
    log::append(project_dir, id, &entry)?;
    Ok((doc(project_dir, id, next), entry))
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
    let mut done: Option<(StopDiff, String, Vec<LayoutStop>)> = None;
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
        let file = project_dir.join(&history);
        if let Some(folder) = file.parent() {
            fs::create_dir_all(folder).map_err(|e| CoreError::io(folder, e))?;
        }
        crate::json::write_json(&file, current)?;
        let mut next = current.clone();
        next.stops = stops;
        layout::prune_stops(&mut next);
        done = Some((review.diff, layout_key(current), next.stops.clone()));
        Ok(next)
    })?;
    let Some((diff, before, stops)) = done else {
        return Err(stale_review());
    };
    let applied = Applied {
        history,
        before,
        after: layout_key(&written),
    };
    let mut entry = person_line(
        format!(
            "Applied the camera plan to layout.json: {}",
            diff.counts_line()
        ),
        ChangeMark::Apply,
        Some(applied),
    );
    entry.id = change_id;
    note_plan(project_dir, id, &plan, Some(CameraBase { stops }), entry)
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
    let file = project_dir.join(&applied.history);
    layout::update(project_dir, |current| {
        if layout_key(current) != applied.after {
            return Err(CoreError::Invalid(
                "layout.json changed since this Apply, so it was not put back: an undo would lose those changes. Change the stops by hand, or apply the plan again.".into(),
            ));
        }
        let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
        crate::json::from_json_with_defaults::<SceneLayout>(&text)
            .map_err(|e| CoreError::json(&file, e))
    })?;
    let back = ops::apply(&plan, &ops::invert(&entry.ops), Merge::Careful).plan;
    let mut line = person_line(format!("Undid: {}", entry.summary), ChangeMark::Undo, None);
    line.undoes = vec![change.to_string()];
    let base = back.camera.clone();
    note_plan(project_dir, id, &plan, base, line)
}

#[cfg(test)]
mod tests;
