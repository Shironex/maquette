//! Bringing the camera plan up to a `layout.json` that changed underneath
//! it, without losing either side.
//!
//! A rebase compares three states of each stop: the base (what the layout
//! had when the plan was built or last applied), the plan and the layout
//! now. What only the layout changed comes into the plan; what only the
//! plan changed stays; where both changed the same setting to different
//! values the plan's value stays and the clash is listed, so nothing is
//! dropped unseen. A stop the layout removed goes from the plan only when
//! the plan left it as it was; stops the layout added join the main line
//! after the stop they follow there; a new tour order comes in only when
//! the plan kept the old one. Start over instead builds the main line
//! again from the layout; the person's old stops stay undoable in the log.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::files::writing;
use super::super::{EdgeKind, PlanDoc, PlanFile, load_file, path};
use super::apply::{person_line, write_person};
use super::{CameraBase, STEP_X, lead, line_of, main_chain, stop_node, stop_of};
use crate::error::CoreResult;
use crate::layout::{self, LayoutStop};
use crate::plans::summary::ChangeMark;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RebaseMode {
    /// Keep the plan's changes and take the layout's.
    Rebase,
    /// Build the main line again from the layout.
    StartOver,
}

/// What a rebase takes in, keeps and could not settle, one line each.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RebaseReport {
    /// Settings and order taken from `layout.json`: "Moon: fov".
    pub taken: Vec<String>,
    /// Stops the layout added, joined to the main line.
    pub added: Vec<String>,
    /// Stops the layout removed, gone from the plan.
    pub removed: Vec<String>,
    /// Where both sides changed the same thing; the plan's side stays.
    pub clashes: Vec<String>,
    /// The log line, once written.
    pub change: Option<String>,
}

fn find<'a>(stops: &'a [LayoutStop], id: &str) -> Option<&'a LayoutStop> {
    stops.iter().find(|s| s.id == id)
}

/// Three way merge of one stop's settings; the plan's side wins a clash.
fn merge_stop(
    plan: &mut LayoutStop,
    base: &LayoutStop,
    now: &LayoutStop,
    report: &mut RebaseReport,
) {
    let name = plan.name.clone();
    macro_rules! field {
        ($($f:ident),*) => {$(
            if now.$f != base.$f {
                if plan.$f == base.$f {
                    plan.$f = now.$f.clone();
                    report.taken.push(format!("{name}: {}", stringify!($f)));
                } else if plan.$f != now.$f {
                    report.clashes.push(format!(
                        "{name}: {} changed in both; the plan's value stays",
                        stringify!($f)
                    ));
                }
            }
        )*};
    }
    field!(name, position, target, fov, models);
}

/// Removes the stop node `id`, joining the stops before and after it.
fn cut(plan: &mut PlanFile, id: &str) {
    let lead_of = |e: &&super::super::PlanEdge| e.kind == EdgeKind::LeadsTo;
    let before = plan
        .edges
        .iter()
        .filter(lead_of)
        .find(|e| e.to == id)
        .map(|e| e.from.clone());
    let after = plan
        .edges
        .iter()
        .filter(lead_of)
        .find(|e| e.from == id)
        .map(|e| e.to.clone());
    plan.nodes.retain(|n| n.id != id);
    plan.edges.retain(|e| e.from != id && e.to != id);
    if let (Some(from), Some(to)) = (before, after) {
        plan.edges.push(lead(&from, &to, None));
    }
}

/// The plan rebased on the layout's stops `now`, and what it did.
pub fn rebased(plan: &PlanFile, now: &[LayoutStop]) -> (PlanFile, RebaseReport) {
    let base = plan.camera.clone().map(|b| b.stops).unwrap_or_default();
    let mut next = plan.clone();
    let mut report = RebaseReport::default();
    let mut gone = Vec::new();
    for node in &mut next.nodes {
        let Some(mut stop) = stop_of(node).cloned() else {
            continue;
        };
        match (find(&base, &stop.id), find(now, &stop.id)) {
            (Some(b), Some(l)) => merge_stop(&mut stop, b, l, &mut report),
            (Some(b), None) if *b == stop => {
                gone.push(node.id.clone());
                if !report.removed.contains(&stop.name) {
                    report.removed.push(stop.name.clone());
                }
            }
            (Some(_), None) => report.clashes.push(format!(
                "{}: removed from layout.json but changed in the plan; it stays, and Apply would add it back",
                stop.name
            )),
            (None, Some(l)) if *l != stop => report.clashes.push(format!(
                "{}: layout.json added a stop with the same id; the plan's stays",
                stop.name
            )),
            _ => {}
        }
        node.stop = Some(Box::new(stop));
    }
    for id in &gone {
        cut(&mut next, id);
    }
    join_new(&mut next, &base, now, &mut report);
    reorder(&mut next, &base, now, &mut report);
    next.camera = Some(CameraBase {
        stops: now.to_vec(),
    });
    (next, report)
}

/// The stop ids of the main line, with their node ids.
fn main_line(plan: &PlanFile) -> Option<Vec<(String, String)>> {
    let chain = main_chain(plan).filter(|c| c.problem.is_none())?;
    Some(
        chain
            .nodes
            .iter()
            .filter_map(|id| plan.nodes.iter().find(|n| &n.id == id))
            .filter_map(|n| stop_of(n).map(|s| (s.id.clone(), n.id.clone())))
            .collect(),
    )
}

/// Lays the main line out again from its first stop, one step apart, and
/// joins it in the order `line` gives.
fn relay(plan: &mut PlanFile, line: &[String]) {
    let Some(head) = line
        .first()
        .and_then(|id| plan.nodes.iter().find(|n| &n.id == id))
    else {
        return;
    };
    let (x, y) = (head.x, head.y);
    plan.edges.retain(|e| {
        !(e.kind == EdgeKind::LeadsTo && line.contains(&e.from) && line.contains(&e.to))
    });
    for (node, i) in line.iter().zip(0..) {
        if let Some(n) = plan.nodes.iter_mut().find(|n| &n.id == node) {
            (n.x, n.y) = (x + i * STEP_X, y);
        }
    }
    for pair in line.windows(2) {
        plan.edges.push(lead(&pair[0], &pair[1], None));
    }
}

fn join_new(
    plan: &mut PlanFile,
    base: &[LayoutStop],
    now: &[LayoutStop],
    report: &mut RebaseReport,
) {
    let fresh: Vec<&LayoutStop> = now.iter().filter(|s| find(base, &s.id).is_none()).collect();
    let Some(mut line) = main_line(plan) else {
        if !fresh.is_empty() {
            report.clashes.push("The main line has a fork or a loop, so the stops layout.json added were not joined to it".into());
        }
        return;
    };
    let mut changed = false;
    for stop in fresh {
        if line.iter().any(|(sid, _)| *sid == stop.id) {
            continue;
        }
        let at = now.iter().position(|s| s.id == stop.id).unwrap_or(0);
        let after = now[..at]
            .iter()
            .rev()
            .find_map(|s| line.iter().position(|(sid, _)| *sid == s.id));
        let node = stop_node(stop, 0, 0, None);
        let slot = after.map_or(0, |i| i + 1);
        line.insert(slot, (stop.id.clone(), node.id.clone()));
        plan.nodes.push(node);
        report.added.push(stop.name.clone());
        changed = true;
    }
    if changed {
        let ids: Vec<String> = line.into_iter().map(|(_, node)| node).collect();
        relay(plan, &ids);
    }
}

fn reorder(
    plan: &mut PlanFile,
    base: &[LayoutStop],
    now: &[LayoutStop],
    report: &mut RebaseReport,
) {
    let Some(line) = main_line(plan) else {
        return;
    };
    let common = |ids: Vec<&str>| -> Vec<String> {
        ids.into_iter()
            .filter(|id| find(base, id).is_some() && find(now, id).is_some())
            .filter(|id| line.iter().any(|(sid, _)| sid == id))
            .map(str::to_string)
            .collect()
    };
    let in_plan = common(line.iter().map(|(sid, _)| sid.as_str()).collect());
    let in_base = common(base.iter().map(|s| s.id.as_str()).collect());
    let in_layout = common(now.iter().map(|s| s.id.as_str()).collect());
    if in_layout == in_base {
        return;
    }
    if in_plan != in_base {
        report
            .clashes
            .push("The tour order changed in both; the plan's order stays".into());
        return;
    }
    let mut wanted = in_layout.into_iter();
    let ordered: Vec<String> = line
        .iter()
        .map(|(sid, node)| {
            if in_plan.contains(sid) {
                let want = wanted.next().unwrap_or_default();
                line.iter()
                    .find(|(s, _)| *s == want)
                    .map_or_else(|| node.clone(), |(_, n)| n.clone())
            } else {
                node.clone()
            }
        })
        .collect();
    relay(plan, &ordered);
    report.taken.push("the tour order".into());
}

/// The plan with its main line built again from `now`, where the old main
/// line began. Agents' proposals stay.
pub fn started_over(plan: &PlanFile, now: &[LayoutStop]) -> PlanFile {
    let mut next = plan.clone();
    let head = main_chain(plan)
        .and_then(|c| c.nodes.first().cloned())
        .and_then(|id| plan.nodes.iter().find(|n| n.id == id))
        .map_or((0, 0), |n| (n.x, n.y));
    let mine: Vec<String> = plan
        .nodes
        .iter()
        .filter(|n| stop_of(n).is_some() && n.by.is_none())
        .map(|n| n.id.clone())
        .collect();
    next.nodes.retain(|n| !mine.contains(&n.id));
    next.edges
        .retain(|e| !mine.contains(&e.from) && !mine.contains(&e.to));
    let (nodes, edges) = line_of(now, head.0, head.1, None);
    next.nodes.extend(nodes);
    next.edges.extend(edges);
    next.camera = Some(CameraBase {
        stops: now.to_vec(),
    });
    next
}

/// Rebases (or starts over) the camera plan `id` on `layout.json` as it is
/// now, as the person. With `dry` nothing is written and the report says
/// what would happen.
///
/// # Errors
///
/// Fails when the canvas or `layout.json` cannot be read, or the result
/// does not validate or cannot be written.
pub fn rebase(
    project_dir: &Path,
    id: &str,
    mode: RebaseMode,
    dry: bool,
) -> CoreResult<(PlanDoc, RebaseReport)> {
    let _lock = writing();
    let current = load_file(&path(project_dir, id)?)?;
    let now = layout::load(project_dir)?.stops;
    let (next, mut report) = match mode {
        RebaseMode::Rebase => rebased(&current, &now),
        RebaseMode::StartOver => (started_over(&current, &now), RebaseReport::default()),
    };
    if dry {
        return Ok((super::super::files::doc(project_dir, id, current), report));
    }
    let summary = match mode {
        RebaseMode::StartOver => "Started the camera plan over from layout.json".to_string(),
        RebaseMode::Rebase => format!(
            "Rebased the camera plan on layout.json: {} taken, {} added, {} removed, {} kept from the plan",
            report.taken.len(),
            report.added.len(),
            report.removed.len(),
            report.clashes.len()
        ),
    };
    let (doc, entry) = write_person(
        project_dir,
        id,
        &current,
        &next,
        person_line(summary, ChangeMark::Rebase, None),
    )?;
    report.change = Some(entry.id);
    Ok((doc, report))
}

#[cfg(test)]
mod tests;
