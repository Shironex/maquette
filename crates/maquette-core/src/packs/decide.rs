//! A person's decisions on a pack's models, and how a pack the app closed
//! on reads.

use serde::{Deserialize, Serialize};

use super::record::{PackItemState, PackRecord, PackState};
use crate::error::{CoreError, CoreResult};

/// A person's answer to a model the check flagged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "choice", rename_all = "kebab-case")]
pub enum PackDecision {
    /// Export it again at a higher budget.
    Raise { key: String, triangles: u32 },
    /// Keep it as the check found it.
    Keep { key: String },
    /// Export it again as it is (after fixing its source, say).
    Again { key: String },
}

impl PackDecision {
    pub(super) fn key(&self) -> &str {
        match self {
            Self::Raise { key, .. } | Self::Keep { key } | Self::Again { key } => key,
        }
    }
}

/// The model that runs next, counted from 1: the first not settled. A
/// model cut short counts as not run, so going on checks it again.
fn next_model(record: &PackRecord) -> Option<usize> {
    record
        .items
        .iter()
        .position(|i| !i.state.is_settled())
        .map(|at| at + 1)
}

/// What to call a pack that stopped before every model ran.
pub(super) fn stopped_at(record: &PackRecord, why: &str) -> String {
    let done = record.done();
    let all = record.items.len();
    match next_model(record) {
        Some(next) => format!("{why} with {done} of {all} done, model {next} is next"),
        None => format!("{why} with {done} of {all} done"),
    }
}

/// A record left running by an app that closed reads as stopped. One that
/// closed with every model run has nothing to go on with, so it waits for
/// the zip again.
pub(super) fn settle_closed(mut record: PackRecord) -> PackRecord {
    if record.state.is_active() {
        if next_model(&record).is_none() {
            record.message =
                Some("Maquette closed while finishing the pack. Zip it again.".to_string());
            record.state = PackState::Waiting;
            return record;
        }
        record.message = Some(stopped_at(&record, "Maquette closed"));
        record.state = PackState::Stopped;
        for item in &mut record.items {
            if item.state == PackItemState::Running {
                item.state = PackItemState::Queued;
            }
        }
    }
    record
}

pub(super) fn apply(record: &mut PackRecord, decision: &PackDecision) -> CoreResult<()> {
    let key = decision.key();
    let item = record
        .items
        .iter_mut()
        .find(|i| i.model.key == key)
        .ok_or_else(|| CoreError::NotFound(format!("the model {key} in this pack")))?;
    if item.state == PackItemState::Running {
        return Err(CoreError::Invalid(format!(
            "{} is running now.",
            item.model.name
        )));
    }
    match decision {
        PackDecision::Raise { triangles, .. } => {
            if item.budget.is_some_and(|b| *triangles <= b) {
                return Err(CoreError::Invalid(
                    "Raise the budget above the one it has.".to_string(),
                ));
            }
            item.budget = Some(*triangles);
            item.state = PackItemState::Queued;
        }
        PackDecision::Keep { .. } => {
            if item.state != PackItemState::Warn {
                return Err(CoreError::Invalid(format!(
                    "{} has no warning to keep.",
                    item.model.name
                )));
            }
            item.state = PackItemState::Kept;
        }
        PackDecision::Again { .. } => item.state = PackItemState::Queued,
    }
    Ok(())
}
