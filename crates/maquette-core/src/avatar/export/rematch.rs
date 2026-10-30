//! Holding the confirmed springs to the parts the finder sees now. A pick
//! names a part by its root bone and centre line; after the model changes
//! (a new reweight, a new rest pose) the finder may call the same part by
//! another id, kind or root. A pick whose part is still there as it was
//! stays as it is. One whose part moved is carried over to the candidate
//! that lies on its line and hangs from about the same place, keeping the
//! person's kind, joints and preset, when exactly one candidate does so
//! clearly. Otherwise the pick is stale and the person picks again.

use serde::Serialize;

use crate::avatar::SpringSpec;
use crate::avatar::springs::{FoundSprings, SpringCandidate};

/// A line this near a candidate's (mean distance, share of the height) is
/// that part, as the chain script holds it.
const SAME: f64 = 0.05;
/// A point this near the other line (share of the height) lies on it.
const NEAR: f64 = 0.05;
/// The share of each line that must lie on the other for a carry over.
const OVERLAP_MIN: f64 = 0.6;
/// Where the two hang from may lie this far apart (share of the height).
const ROOT_NEAR: f64 = 0.1;
/// A second candidate this close in overlap makes the carry unsure.
const CLEAR_BY: f64 = 0.15;

/// A pick carried over to the candidate its part is now.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpringCarried {
    /// The pick's id before.
    pub from: String,
    /// The candidate's id now, which the pick takes.
    pub to: String,
    /// The bone it hangs from now.
    pub root_bone: String,
    /// 0 to 1: how much of the two lines lie on each other.
    pub overlap: f64,
}

/// The picks held to the candidates found now.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Held {
    /// Every pick matches a candidate; `springs` are the picks to chain,
    /// those carried over with their candidate's id, root and line.
    Matched {
        springs: Vec<SpringSpec>,
        carried: Vec<SpringCarried>,
    },
    /// These picks (by id) match no candidate clearly.
    Stale(Vec<String>),
}

fn distance_to_segment(p: [f64; 3], a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let r = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let len2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    let t = if len2 > 1e-18 {
        ((r[0] * d[0] + r[1] * d[1] + r[2] * d[2]) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let q = [r[0] - t * d[0], r[1] - t * d[1], r[2] - t * d[2]];
    (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt()
}

/// How far `p` lies from the polyline `line`.
fn distance_to_line(p: [f64; 3], line: &[[f64; 3]]) -> f64 {
    match line {
        [] => f64::INFINITY,
        [only] => distance_to_segment(p, *only, *only),
        _ => line
            .windows(2)
            .map(|w| distance_to_segment(p, w[0], w[1]))
            .fold(f64::INFINITY, f64::min),
    }
}

/// How far, on average, the points of `a` lie from the line `b`.
pub(super) fn line_gap(a: &[[f64; 3]], b: &[[f64; 3]]) -> f64 {
    if a.is_empty() || b.len() < 2 {
        return f64::INFINITY;
    }
    #[expect(clippy::cast_precision_loss, reason = "a line holds at most 64 points")]
    let count = a.len() as f64;
    a.iter().map(|p| distance_to_line(*p, b)).sum::<f64>() / count
}

/// The share of `a`'s points within `near` of the line `b`.
fn on_line(a: &[[f64; 3]], b: &[[f64; 3]], near: f64) -> f64 {
    if a.is_empty() || b.len() < 2 {
        return 0.0;
    }
    let close = a
        .iter()
        .filter(|p| distance_to_line(**p, b) <= near)
        .count();
    #[expect(clippy::cast_precision_loss, reason = "a line holds at most 64 points")]
    let share = close as f64 / a.len() as f64;
    share
}

/// How much of two lines lie on each other: the smaller share either way.
fn overlap(a: &[[f64; 3]], b: &[[f64; 3]], near: f64) -> f64 {
    on_line(a, b, near).min(on_line(b, a, near))
}

fn gap(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The candidate a pick still names as it is: the same root, its line on
/// the candidate's.
fn same<'a>(pick: &SpringSpec, found: &'a FoundSprings) -> Option<&'a SpringCandidate> {
    let h = found.height.max(1e-6);
    found
        .candidates
        .iter()
        .filter(|c| c.root_bone == pick.root_bone)
        .map(|c| (c, line_gap(&pick.line, &c.line)))
        .filter(|(_, g)| *g <= SAME * h)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(c, _)| c)
}

/// The one candidate a pick's part clearly is now, whatever its root, and
/// how much the two lines overlap.
fn closest<'a>(
    pick: &SpringSpec,
    found: &'a FoundSprings,
    taken: &[&str],
) -> Option<(&'a SpringCandidate, f64)> {
    let h = found.height.max(1e-6);
    let start = *pick.line.first()?;
    let mut scored: Vec<(&SpringCandidate, f64)> = found
        .candidates
        .iter()
        .filter(|c| !taken.contains(&c.id.as_str()))
        .filter(|c| {
            c.line
                .first()
                .is_some_and(|s| gap(*s, start) <= ROOT_NEAR * h)
        })
        .map(|c| (c, overlap(&pick.line, &c.line, NEAR * h)))
        .filter(|(_, o)| *o >= OVERLAP_MIN)
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    match scored.as_slice() {
        [best] => Some(*best),
        [best, next, ..] if best.1 - next.1 >= CLEAR_BY => Some(*best),
        _ => None,
    }
}

/// Holds `picks` to the candidates in `found`.
pub(super) fn hold(picks: &[SpringSpec], found: &FoundSprings) -> Held {
    let mut springs: Vec<SpringSpec> = Vec::with_capacity(picks.len());
    let mut carried = Vec::new();
    let mut stale = Vec::new();
    let mut taken: Vec<String> = Vec::new();
    // Picks that still match as they are claim their parts first.
    let kept: Vec<Option<&SpringCandidate>> = picks.iter().map(|p| same(p, found)).collect();
    taken.extend(kept.iter().flatten().map(|c| c.id.clone()));
    for (pick, kept) in picks.iter().zip(&kept) {
        if kept.is_some() {
            springs.push(pick.clone());
            continue;
        }
        let names: Vec<&str> = taken.iter().map(String::as_str).collect();
        let Some((candidate, share)) = closest(pick, found, &names) else {
            stale.push(pick.id.clone());
            continue;
        };
        taken.push(candidate.id.clone());
        // The candidate's id unless another pick already goes by it.
        let clash = picks
            .iter()
            .any(|p| p.id == candidate.id && p.id != pick.id);
        let id = if clash {
            pick.id.clone()
        } else {
            candidate.id.clone()
        };
        carried.push(SpringCarried {
            from: pick.id.clone(),
            to: id.clone(),
            root_bone: candidate.root_bone.clone(),
            overlap: (share * 100.0).round() / 100.0,
        });
        springs.push(SpringSpec {
            id,
            root_bone: candidate.root_bone.clone(),
            line: candidate.line.clone(),
            ..pick.clone()
        });
    }
    if stale.is_empty() {
        Held::Matched { springs, carried }
    } else {
        Held::Stale(stale)
    }
}

#[cfg(test)]
mod tests;
