//! What a pose row's numbers mean. Every word the sheets show about a row
//! comes from here, so a cell, a panel and a stop cannot disagree.

use super::{RigDefect, RigRow, RigRowChange, RigSheet, RigTrend};

/// A row is flagged when an edge stretches this far past what the body does
/// by itself (mm at a one metre tall model). A clean rig with cloth across
/// its armpits stays near 60 in big poses while the raw stretch reads 260.
pub const TEAR_MM: f64 = 80.0;
/// Seam copies this far apart read as a crack.
pub const SEAM_MM: f64 = 5.0;
/// A number has to move this far to count as changed: smaller moves are noise.
const SAME_MM: f64 = 5.0;

/// The stretch the weights answer for. A record from before the number
/// existed has only the raw stretch.
pub(super) fn past_body(row: &RigRow) -> f64 {
    row.beyond_body_mm.unwrap_or(row.stretch_mm)
}

/// What is wrong with a row by its numbers alone.
pub(super) fn problem(row: &RigRow) -> Option<String> {
    if row.seam_gap_mm >= SEAM_MM {
        Some(format!("seam opens {:.0} mm", row.seam_gap_mm))
    } else if past_body(row) >= TEAR_MM {
        Some(format!("stretches {:.0} mm past the body", past_body(row)))
    } else {
        None
    }
}

fn trend(before: f64, after: f64) -> RigTrend {
    if after - before >= SAME_MM {
        RigTrend::Worse
    } else if before - after >= SAME_MM {
        RigTrend::Better
    } else {
        RigTrend::Same
    }
}

fn word(trend: RigTrend) -> &'static str {
    match trend {
        RigTrend::Better => "better",
        RigTrend::Worse => "worse",
        RigTrend::Same => "same",
    }
}

/// Says on each before row what shows there. A defect's words win over the
/// numbers; a row with neither is the only kind that looks fine.
pub(super) fn mark_rows(rows: &mut [RigRow], defects: &[RigDefect]) {
    use super::DefectKind;
    for row in rows.iter_mut() {
        for d in defects {
            let shows = if d.kind == DefectKind::NeedsYes {
                row.id == "raise"
            } else if d.kind == DefectKind::Note {
                false
            } else if d.title.contains("arm bones") {
                row.id == "raise" || row.id == "forward"
            } else {
                row.id == "hand"
            };
            if shows {
                row.defects.push(d.number);
                row.note.get_or_insert(short(d));
            }
        }
        if row.note.is_none() {
            row.note = problem(row);
        }
    }
}

fn short(d: &RigDefect) -> String {
    if d.kind == super::DefectKind::NeedsYes {
        "joints off mesh".to_string()
    } else if d.title.starts_with("Hair") {
        "hair follows".to_string()
    } else {
        "cloth swings".to_string()
    }
}

/// Says on each after row how its numbers moved from the before sheet, and
/// answers the row still worst off, for the stop after the last round.
pub(super) fn mark_after(after: &mut RigSheet, before: &RigSheet) -> Option<(String, f64)> {
    let mut worst: Option<(String, f64)> = None;
    for row in &mut after.rows {
        if problem(row).is_some() {
            let mm = past_body(row).max(row.seam_gap_mm);
            if worst.as_ref().is_none_or(|w| mm > w.1) {
                worst = Some((row.id.clone(), mm));
            }
        }
        let Some(was) = before.rows.iter().find(|r| r.id == row.id) else {
            continue;
        };
        let change = RigRowChange {
            stretch_before: was.stretch_mm,
            beyond_body_before: was.beyond_body_mm,
            seam_gap_before: was.seam_gap_mm,
            stretch: trend(was.stretch_mm, row.stretch_mm),
            seam_gap: trend(was.seam_gap_mm, row.seam_gap_mm),
        };
        let mut said = Vec::new();
        if change.stretch != RigTrend::Same {
            said.push(format!(
                "stretch {:.0} to {:.0} mm, {}",
                was.stretch_mm,
                row.stretch_mm,
                word(change.stretch)
            ));
        }
        if change.seam_gap != RigTrend::Same {
            said.push(format!(
                "seam {:.0} to {:.0} mm, {}",
                was.seam_gap_mm,
                row.seam_gap_mm,
                word(change.seam_gap)
            ));
        }
        row.note = (!said.is_empty()).then(|| said.join("; "));
        row.change = Some(change);
    }
    worst
}

/// The row where `value` peaks, with its label, for a tile that says where.
pub(super) fn peak(rows: &[RigRow], value: impl Fn(&RigRow) -> f64) -> (String, f64) {
    rows.iter()
        .map(|r| (r.label.clone(), value(r)))
        .fold((String::new(), 0.0), |a, b| if b.1 > a.1 { b } else { a })
}

/// A tile's label with the pose its number came from. The numbers are of the
/// whole mesh in that pose, so every view of the pose shows it.
pub(super) fn at_pose(label: &str, pose: &str) -> String {
    if pose.is_empty() {
        label.to_string()
    } else {
        format!("{label}, {pose}, every view")
    }
}
