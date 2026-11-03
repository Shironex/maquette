//! A person's settings on one canvas, each a logged change the person can
//! undo: the "Auto layout after agents" switch and a Game mechanics overlap
//! marked as fine. No agent tool reaches these.

use std::path::Path;

use super::change::{Changed, Say, Writer, change_with};
use super::ops::PlanOp;
use super::overlap::{FineMark, overlaps, pair_key};
use super::summary::node_name;
use crate::error::{CoreError, CoreResult};

/// Turns "Auto layout after agents" on or off for the canvas `id`, as the
/// person. Nothing is written when it is already so.
///
/// # Errors
///
/// Fails as [`super::change`].
pub fn set_auto_layout(project_dir: &Path, id: &str, on: bool) -> CoreResult<Changed> {
    change_with(project_dir, id, &Writer::PERSON, |plan| {
        let summary = if on {
            "Turned auto layout after agents on"
        } else {
            "Turned auto layout after agents off"
        };
        let ops = if plan.auto_layout == on {
            Vec::new()
        } else {
            vec![PlanOp::SetAutoLayout {
                before: plan.auto_layout,
                after: on,
            }]
        };
        Ok((
            ops,
            Say::Line {
                summary: summary.into(),
                resolves: Vec::new(),
            },
        ))
    })
}

/// Marks the overlap of the cards `a` and `b` as fine, as the person, for
/// the words both hold now: the flag stays away until either changes.
///
/// # Errors
///
/// Refuses two nodes that do not overlap now (their words changed since the
/// flag showed) or are marked already; fails as [`super::change`].
pub fn mark_fine(project_dir: &Path, id: &str, a: &str, b: &str) -> CoreResult<Changed> {
    let (a, b) = if a <= b { (a, b) } else { (b, a) };
    change_with(project_dir, id, &Writer::PERSON, |plan| {
        let found = overlaps(plan);
        let pair: Vec<_> = found.iter().filter(|o| o.a == a && o.b == b).collect();
        if pair.is_empty() {
            return Err(CoreError::Invalid(
                "These two cards do not overlap any more, so there is nothing to mark.".into(),
            ));
        }
        if pair.iter().all(|o| o.fine) {
            return Err(CoreError::Invalid(
                "This overlap is marked as fine already.".into(),
            ));
        }
        let node = |id: &str| plan.nodes.iter().find(|n| n.id == id);
        let (Some(na), Some(nb)) = (node(a), node(b)) else {
            return Err(CoreError::NotFound(format!("the cards {a} and {b}")));
        };
        let mut ops: Vec<PlanOp> = plan
            .fine
            .iter()
            .filter(|m| m.a == a && m.b == b)
            .map(|m| PlanOp::UnmarkFine { mark: m.clone() })
            .collect();
        ops.push(PlanOp::MarkFine {
            mark: FineMark {
                a: a.to_string(),
                b: b.to_string(),
                key: pair_key(na, nb),
            },
        });
        let summary = format!("Marked as fine: {} and {}", node_name(na), node_name(nb));
        Ok((
            ops,
            Say::Line {
                summary,
                resolves: Vec::new(),
            },
        ))
    })
}
