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
    Ok(make(project_dir, PlanFile::new(name, preset), by, None)?.doc)
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
    make(project_dir, plan, by, None)
}

/// Writes `plan` as a new canvas under a free id made from its name, and
/// logs it as made (`summary`, else a line naming its preset), with an add
/// for each node and edge it starts with.
/// A blank canvas only warns when its line cannot be written, as every
/// other plain line does, and answers with no change id, since its line
/// undoes nothing; a canvas with starters is taken back then ([`unmake`]).
pub(super) fn make(
    project_dir: &Path,
    plan: PlanFile,
    by: Option<&str>,
    summary: Option<String>,
) -> CoreResult<Changed> {
    check_name(&plan.name)?;
    let plan = normalized(&plan);
    validate(&plan)?;
    let _lock = writing();
    let id = free_id(project_dir, &plan.name)?;
    let folder = dir(project_dir);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    write_plan(&path(project_dir, &id)?, &plan)?;
    let summary = if let Some(summary) = summary {
        summary
    } else if plan.nodes.is_empty() {
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
    let entry = if line.ops.is_empty() {
        change::note(project_dir, &id, &line);
        None
    } else {
        if let Err(err) = append(project_dir, &id, &line) {
            unmake(project_dir, &id, &line, err)?;
        }
        Some(line)
    };
    Ok(Changed {
        doc: doc(project_dir, &id, plan),
        entry,
        skipped: 0,
    })
}

/// After the starters' line of the new canvas `id` failed to append: takes
/// back what this call made, so no starter is left without an undo. Any
/// log on the id is this call's torn line ([`free_id`] saw none under the
/// same lock), so it goes and the slug stays free. The canvas goes next,
/// tried again for a moment while something holds it (Windows); when it
/// still stays, its line is written once more so one undo takes the
/// starters back. Ok only in that case: the canvas is there with its line.
fn unmake(
    project_dir: &Path,
    id: &str,
    line: &log::PlanLogEntry,
    err: CoreError,
) -> CoreResult<()> {
    let torn = log::path(project_dir, id)?;
    if let Err(stuck) = remove_if_there(&torn) {
        tracing::warn!("the torn first line of a new canvas stayed: {stuck}");
    }
    let file = path(project_dir, id)?;
    let Err(stuck) = remove(&file) else {
        return Err(err);
    };
    if append(project_dir, id, line).is_ok() {
        tracing::warn!(
            "a new canvas could not be taken back ({stuck}); its first line was logged on a second try"
        );
        return Ok(());
    }
    tracing::warn!("a canvas whose first line was not logged stayed: {stuck}");
    Err(err)
}

#[cfg(test)]
thread_local! {
    /// Makes the next removal here fail, as a lock on Windows would.
    pub(super) static FAIL_REMOVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Makes the next log append here write half a line before it fails.
    pub(super) static TEAR_LOG: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn remove_if_there(file: &Path) -> std::io::Result<()> {
    if file.exists() { remove(file) } else { Ok(()) }
}

fn remove(file: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if FAIL_REMOVE.with(|f| f.replace(false)) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "held open",
        ));
    }
    maquette_base::fs::remove_retrying(file)
}

#[cfg(test)]
thread_local! {
    /// Makes the next log append here fail, for the test of that path.
    pub(super) static FAIL_LOG: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn append(project_dir: &Path, id: &str, line: &log::PlanLogEntry) -> CoreResult<()> {
    #[cfg(test)]
    if TEAR_LOG.with(|f| f.replace(false)) {
        fs::write(log::path(project_dir, id)?, "{\"id\":\"c-torn").expect("torn line");
        return Err(CoreError::Invalid("the log could not be written".into()));
    }
    #[cfg(test)]
    if FAIL_LOG.with(|f| f.replace(false)) {
        return Err(CoreError::Invalid("the log could not be written".into()));
    }
    log::append(project_dir, id, line)
}
