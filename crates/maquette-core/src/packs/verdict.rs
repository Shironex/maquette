//! What the check of one model of a pack found, as `export_pack.py` reports
//! it, and the item's state and note that come of it.

use serde::Deserialize;

use super::PackSettings;
use super::record::{PackItem, PackItemState};

/// How far the measured height may drift from the real size.
const HEIGHT_SLACK: f32 = 0.02;
/// How far over its budget a decimated model may land, as a share of it:
/// a collapse rounds per mesh, so a model of many parts can land a little over.
const BUDGET_SLACK: f64 = 0.02;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Numbers {
    pub triangles: u32,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct Check {
    pub file: String,
    pub triangles: u32,
    pub height_m: Option<f32>,
    pub floor_m: Option<f32>,
    pub centre: Option<[f32; 2]>,
    pub unloaded: Vec<String>,
    pub missing: Vec<String>,
    pub clips: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Report {
    pub before: Numbers,
    pub after: Numbers,
    pub thin: bool,
    pub clips_removed: u32,
    pub files: Vec<String>,
    pub checks: Vec<Check>,
    pub pivot_slack: f32,
}

/// The item's state and note from what the check found.
pub(super) fn verdict(item: &PackItem, report: &Report, settings: &PackSettings) -> PackItem {
    let mut next = item.clone();
    next.triangles = Some(report.after.triangles);
    next.clips_left_out = report.clips_removed;
    next.suggest = None;
    let main = report.checks.first();
    next.height_m = main.and_then(|c| c.height_m);
    let (state, note) = judge(item, report, settings);
    next.state = state;
    next.note = Some(note);
    if state == PackItemState::Warn && report.thin {
        let budget = item.budget.unwrap_or(report.after.triangles);
        next.suggest = Some(report.before.triangles.min(budget.saturating_mul(2)));
    } else if state == PackItemState::Warn && over_budget(item, report).is_some() {
        next.suggest = Some(report.after.triangles);
    }
    next
}

/// The budget of a model that should have come down to it and did not: the
/// decimate leaves a mesh with shape keys alone.
fn over_budget(item: &PackItem, report: &Report) -> Option<u32> {
    item.budget.filter(|b| {
        !item.model.rigged
            && f64::from(report.after.triangles) > f64::from(*b) * (1.0 + BUDGET_SLACK)
    })
}

fn judge(item: &PackItem, report: &Report, settings: &PackSettings) -> (PackItemState, String) {
    let Some(main) = report.checks.first() else {
        return (PackItemState::Fail, "Nothing was written.".to_string());
    };
    if let Some(check) = report
        .checks
        .iter()
        .find(|c| !c.missing.is_empty() || !c.unloaded.is_empty())
    {
        let gone = check
            .missing
            .first()
            .or(check.unloaded.first())
            .cloned()
            .unwrap_or_default();
        return (
            PackItemState::Fail,
            format!(
                "Texture missing: {} names {gone}, which did not load. Export it again.",
                check.file
            ),
        );
    }
    if item.model.mixamo && report.checks.iter().any(|c| c.clips > 0) {
        return (
            PackItemState::Fail,
            "Mixamo clips are still in the file, so it is held back.".to_string(),
        );
    }
    let count = crate::assets::short_count;
    if report.thin {
        let budget = item.budget.unwrap_or(report.after.triangles);
        let keeps = report.before.triangles.min(budget.saturating_mul(2));
        return (
            PackItemState::Warn,
            format!(
                "Thin parts lost at {} tris; {} keeps more of them.",
                count(budget),
                count(keeps)
            ),
        );
    }
    if let Some(budget) = item
        .budget
        .filter(|b| item.model.rigged && report.after.triangles > *b)
    {
        return (
            PackItemState::Warn,
            format!(
                "Rigged, so not decimated: {} tris, over the {} budget.",
                count(report.after.triangles),
                count(budget)
            ),
        );
    }
    if let Some(budget) = over_budget(item, report) {
        return (
            PackItemState::Warn,
            format!(
                "Over budget: {} tris for {}. A mesh with shape keys is not decimated.",
                count(report.after.triangles),
                count(budget)
            ),
        );
    }
    let slack = report.pivot_slack.max(0.001);
    let off_floor = main.floor_m.is_some_and(|f| f.abs() > slack);
    let off_centre = main
        .centre
        .is_some_and(|[x, z]| x.abs() > slack || z.abs() > slack);
    if settings.pivot_bottom && (off_floor || off_centre) {
        return (
            PackItemState::Warn,
            "The pivot is not at the bottom centre after import.".to_string(),
        );
    }
    let wanted = item.model.height_m.filter(|_| settings.real_size);
    if let (Some(wanted), Some(got)) = (wanted, main.height_m)
        && wanted > 0.0
        && ((got - wanted) / wanted).abs() > HEIGHT_SLACK
    {
        return (
            PackItemState::Warn,
            format!("Measures {got:.2} m high; its real size says {wanted:.2} m."),
        );
    }
    let mut note = if report.after.triangles >= report.before.triangles && item.budget.is_some() {
        "Under budget, left as is".to_string()
    } else if settings.pivot_bottom {
        "Re-imported, Y up, on the floor".to_string()
    } else {
        "Re-imported, Y up".to_string()
    };
    if report.clips_removed > 0 {
        let n = report.clips_removed;
        note = format!(
            "{note}; {n} Mixamo clip{} left out",
            if n == 1 { "" } else { "s" }
        );
    }
    (PackItemState::Pass, note)
}
