//! Making a canvas: a free id from its name, the file, and the first line
//! of its change log, which holds the preset's starter notes so one undo
//! takes them back.

use std::fs;
use std::path::Path;

use super::files::{doc, plain_entry, write_plan, writing};
use super::log;
use super::ops::PlanOp;
use super::summary::ChangeMark;
use super::{
    Changed, PlanDoc, PlanFile, PlanPreset, change, check_name, dir, normalized, path, presets,
    validate,
};
use crate::error::{CoreError, CoreResult};
use crate::project::{latin_slug, windows_safe};
use crate::settings::game::ProjectTarget;

/// A free id for a canvas called `name`: its slug, then `-2`, `-3` and on,
/// never the camera plan's id, which only the built in plan takes.
pub(super) fn free_id(project_dir: &Path, name: &str) -> CoreResult<String> {
    let base = latin_slug(name).map_or_else(|| "canvas".to_string(), windows_safe);
    for n in 1..10_000 {
        let id = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        // A log left without its canvas counts as taken: it would pass to the new canvas.
        if id != super::camera::CAMERA_PLAN_ID
            && !path(project_dir, &id)?.exists()
            && !log::path(project_dir, &id)?.exists()
        {
            return Ok(id);
        }
    }
    Err(CoreError::Invalid(format!(
        "There are too many canvases called {name}."
    )))
}

/// Makes an empty canvas called `name` and writes its file. The app and
/// agents go through [`create_seeded`], which also lays out the preset's
/// starter notes; this keeps the preset on an empty canvas.
///
/// # Errors
///
/// Fails on a name [`check_name`] refuses or when the file cannot be written.
pub fn create(project_dir: &Path, name: &str, preset: PlanPreset) -> CoreResult<PlanDoc> {
    create_by(project_dir, name, preset, None)
}

/// [`create`], noting who made it (an agent's name) in the change log.
///
/// # Errors
///
/// As [`create`].
pub fn create_by(
    project_dir: &Path,
    name: &str,
    preset: PlanPreset,
    by: Option<&str>,
) -> CoreResult<PlanDoc> {
    Ok(make(project_dir, PlanFile::new(name, preset), by)?.doc)
}

/// Makes a canvas called `name` from `preset`, with the preset's starter
/// notes and links for a project of `target` ([`presets::seed`]), made by
/// `by` (an agent's name, or `None` for a person). The starters are the
/// first line of the change log, which [`Changed::entry`] names, so one
/// undo of that change id takes them all back. The line has no run.
///
/// # Errors
///
/// As [`create`], and when the starters' log line cannot be written: the
/// canvas is removed again rather than left with notes no undo can reach.
pub fn create_seeded(
    project_dir: &Path,
    name: &str,
    preset: PlanPreset,
    target: ProjectTarget,
    by: Option<&str>,
) -> CoreResult<Changed> {
    let mut plan = PlanFile::new(name, preset);
    (plan.nodes, plan.edges) = presets::seed(preset, target, by);
    make(project_dir, plan, by)
}

/// Writes `plan` as a new canvas under a free id made from its name, and
/// logs it as made, with an add for each node and edge it starts with.
/// A blank canvas only warns when its line cannot be written, as every
/// other plain line does; a canvas with starters is removed again then.
fn make(project_dir: &Path, plan: PlanFile, by: Option<&str>) -> CoreResult<Changed> {
    check_name(&plan.name)?;
    let plan = normalized(&plan);
    validate(&plan)?;
    let _lock = writing();
    let id = free_id(project_dir, &plan.name)?;
    let folder = dir(project_dir);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    write_plan(&path(project_dir, &id)?, &plan)?;
    let summary = if plan.nodes.is_empty() {
        format!("Made the canvas {}", plan.name)
    } else {
        format!(
            "Made the canvas {} from the {} preset",
            plan.name,
            presets::label(plan.preset)
        )
    };
    let mut line = plain_entry(by, summary, ChangeMark::Create);
    line.ops = plan
        .nodes
        .iter()
        .map(|n| PlanOp::AddNode { node: n.clone() })
        .chain(
            plan.edges
                .iter()
                .map(|e| PlanOp::AddEdge { edge: e.clone() }),
        )
        .collect();
    let file = path(project_dir, &id)?;
    if line.ops.is_empty() {
        change::note(project_dir, &id, &line);
    } else if let Err(err) = append(project_dir, &id, &line) {
        if let Err(stuck) = fs::remove_file(&file) {
            tracing::warn!("a canvas whose first line was not logged stayed: {stuck}");
        }
        return Err(err);
    }
    Ok(Changed {
        doc: doc(project_dir, &id, plan),
        entry: Some(line),
        skipped: 0,
    })
}

#[cfg(test)]
thread_local! {
    /// Makes the next log append here fail, for the test of that path.
    pub(super) static FAIL_LOG: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn append(project_dir: &Path, id: &str, line: &log::PlanLogEntry) -> CoreResult<()> {
    #[cfg(test)]
    if FAIL_LOG.with(|f| f.replace(false)) {
        return Err(CoreError::Invalid("the log could not be written".into()));
    }
    log::append(project_dir, id, line)
}
