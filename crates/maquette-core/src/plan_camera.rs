//! The camera plan as its canvas shows it: the plan file (made from
//! `layout.json` the first time), each stop's poster render where the
//! Capture posters step made one, which line Apply writes and what changed
//! in `layout.json` since the plan was based on it. The plan itself, its
//! review, Apply, undo and rebase are in [`crate::plans::camera`].

use std::path::Path;

use serde::Serialize;

use crate::inspect::stops::stop_posters;
use crate::layout;
use crate::plans::PlanDoc;
use crate::plans::camera::{self, StopDiff};

/// A stop's poster render on this computer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StopPoster {
    /// The stop's id in `layout.json`.
    pub stop: String,
    pub poster: String,
}

/// A line of stop nodes beside the main line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CameraLine {
    /// Its node ids, in tour order.
    pub nodes: Vec<String>,
    /// The agent that proposed it, when one agent made every stop.
    pub agent: Option<String>,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CameraView {
    pub doc: PlanDoc,
    pub posters: Vec<StopPoster>,
    /// The line Apply writes, its node ids in tour order; empty when the
    /// plan has none.
    pub main: Vec<String>,
    /// Why the main line can not be applied as it is (a fork, a loop).
    pub problem: Option<String>,
    /// The other lines: agents' proposals and stops left aside.
    pub lines: Vec<CameraLine>,
    /// `layout.json` changed since the plan was based on it.
    pub stale: bool,
    /// What changed in `layout.json` since: from the plan's base to now.
    pub outside: StopDiff,
}

/// The project's camera plan as the canvas shows it, made from
/// `layout.json` the first time it is opened.
///
/// # Errors
///
/// Fails when `layout.json` or the plan cannot be read, or the plan cannot
/// be made.
pub fn view(project_dir: &Path) -> crate::CoreResult<CameraView> {
    let doc = camera::ensure(project_dir, None)?;
    let layout = layout::load(project_dir)?;
    let plan = &doc.plan;
    let base = plan.camera.clone().map(|b| b.stops).unwrap_or_default();
    let main = camera::main_chain(plan);
    let lines = camera::chains(plan)
        .into_iter()
        .filter(|c| main.as_ref().is_none_or(|m| m.nodes != c.nodes))
        .map(|c| CameraLine {
            nodes: c.nodes,
            agent: c.agent,
            problem: c.problem,
        })
        .collect();
    let mut names: Vec<(String, String)> = plan
        .nodes
        .iter()
        .filter_map(camera::stop_of)
        .chain(&layout.stops)
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();
    names.sort();
    names.dedup_by(|a, b| a.0 == b.0);
    let posters = stop_posters(project_dir, &names)
        .into_iter()
        .map(|(stop, poster)| StopPoster { stop, poster })
        .collect();
    Ok(CameraView {
        stale: camera::is_stale(plan, &layout),
        outside: camera::diff_stops(&base, &layout.stops, &[]),
        problem: main.as_ref().and_then(|m| m.problem.clone()),
        main: main.map(|m| m.nodes).unwrap_or_default(),
        lines,
        posters,
        doc,
    })
}

pub fn register(types: specta::Types) -> specta::Types {
    types.register::<CameraView>()
}

#[cfg(test)]
mod tests;
