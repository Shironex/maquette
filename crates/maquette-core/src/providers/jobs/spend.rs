//! Writing a job into the credits ledger as it spends: a `sent` line once
//! its first task exists on the provider, a `settled` line when it ends.
//! Jobs that never reached the provider (denied, cancelled for free, By
//! hand) spend nothing and are not written.

use super::{JobState, Shared, Stored};
use crate::pipeline::history::now_ms;
use crate::providers::ledger::{self, LedgerLine, LineEvent, SpendOutcome};

/// What a job looked like before a change, to tell what the change did.
pub(super) struct Before {
    task: bool,
    over: bool,
}

fn over(state: JobState) -> bool {
    matches!(
        state,
        JobState::Inbox | JobState::Accepted | JobState::Failed | JobState::Cancelled
    )
}

pub(super) fn before(stored: &Stored) -> Before {
    Before {
        task: stored.chain.model_task.is_some(),
        over: over(stored.job.state),
    }
}

/// The ledger line a change calls for, if any.
pub(super) fn line(before: &Before, after: &Stored, at_ms: f64) -> Option<LedgerLine> {
    let job = &after.job;
    let service = job.provider.service()?;
    let task = after.chain.model_task.clone()?;
    let (event, outcome) = if !before.over && over(job.state) {
        let outcome = match job.state {
            JobState::Inbox | JobState::Accepted => SpendOutcome::Succeeded,
            JobState::Cancelled => SpendOutcome::Refunded,
            _ => SpendOutcome::Failed,
        };
        (LineEvent::Settled, outcome)
    } else if !before.task {
        (LineEvent::Sent, SpendOutcome::Pending)
    } else {
        return None;
    };
    let mut line = LedgerLine::new(event, at_ms, outcome);
    line.project.clone_from(&job.project);
    line.job.clone_from(&job.id);
    line.asset.clone_from(&job.asset);
    line.asset_name.clone_from(&job.asset_name);
    line.service = service;
    line.task = Some(task);
    line.summary = job.summary();
    line.estimate = job.estimate.or_else(|| job.estimate());
    line.credits = job.credits;
    line.redo_of.clone_from(&job.redo_of);
    line.via_agent = job.via_agent;
    Some(line)
}

/// Appends the line a change calls for. A failed write is logged, never
/// raised: the job itself must go on.
pub(super) fn record(shared: &Shared, before: &Before, after: &Stored) {
    let Some(line) = line(before, after, now_ms()) else {
        return;
    };
    let Some(workspace) = shared.workspace() else {
        return;
    };
    if let Err(err) = ledger::append(&workspace.root, &line) {
        tracing::warn!(error = %err, "could not write the credits ledger");
    }
}
