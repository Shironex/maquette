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
    fn key(&self) -> &str {
        match self {
            Self::Raise { key, .. } | Self::Keep { key } | Self::Again { key } => key,
        }
    }
}

/// What to call a pack that stopped before every model ran.
pub(super) fn stopped_at(record: &PackRecord, why: &str) -> String {
    let at = record.done() + 1;
    format!("{why} at model {at} of {}", record.items.len())
}

/// A record left running by an app that closed reads as stopped.
pub(super) fn settle_closed(mut record: PackRecord) -> PackRecord {
    if record.state.is_active() {
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
