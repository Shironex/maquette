//! The last few overlap answers, so a canvas read again and again (every
//! save, undo and reload answers with its doc) is checked once per change
//! of what the check reads: each node's id, whether it is a note, its title
//! and text, and the fine marks. Positions, links and the rest never count.

use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Mutex, PoisonError};

use super::super::{NodeKind, PlanFile};
use super::Overlap;

/// Canvases remembered; a person has a handful open across projects.
const KEEP: usize = 16;

static ANSWERS: Mutex<VecDeque<(u64, Vec<Overlap>)>> = Mutex::new(VecDeque::new());

/// A key of everything [`super::find`] reads in `plan`.
pub(super) fn key_of(plan: &PlanFile) -> u64 {
    let mut h = DefaultHasher::new();
    plan.nodes.len().hash(&mut h);
    for node in &plan.nodes {
        (
            &node.id,
            node.kind == NodeKind::Note,
            &node.title,
            &node.text,
        )
            .hash(&mut h);
    }
    plan.fine.len().hash(&mut h);
    for mark in &plan.fine {
        (&mark.a, &mark.b, &mark.key).hash(&mut h);
    }
    h.finish()
}

/// The overlaps of `plan`, from a remembered answer when nothing they read
/// changed, else from `find` (remembered for next time).
pub(super) fn answer(
    plan: &PlanFile,
    find: impl FnOnce(&PlanFile) -> Vec<Overlap>,
) -> Vec<Overlap> {
    let key = key_of(plan);
    {
        let answers = ANSWERS.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((_, found)) = answers.iter().find(|(k, _)| *k == key) {
            return found.clone();
        }
    }
    let found = find(plan);
    let mut answers = ANSWERS.lock().unwrap_or_else(PoisonError::into_inner);
    answers.retain(|(k, _)| *k != key);
    answers.push_front((key, found.clone()));
    answers.truncate(KEEP);
    found
}
