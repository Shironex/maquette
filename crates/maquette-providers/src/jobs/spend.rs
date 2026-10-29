//! Writing a job into the credits ledger as it spends: a `sent` line once
//! its first task exists on the provider, a `settled` line when it ends.
//! Jobs that never reached the provider (denied, cancelled for free) spend
//! nothing and are not written. By hand sends are written as subscription
//! spend: a `sent` line with the site's estimate when the handoff is
//! written, a `settled` line when the model comes back (or 0 when you stop
//! waiting).

use super::{JobState, Provider, Shared, Stored};
use crate::ledger::{self, LedgerLine, LineEvent, SpendOutcome};
use maquette_base::history::now_ms;

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
    if after.job.provider == Provider::ByHand {
        return by_hand_settled(before, after, at_ms);
    }
    let job = &after.job;
    let service = job.provider.service()?;
    let task = after.chain.model_task.clone()?;
    let (event, outcome) = if !before.over && over(job.state) {
        let outcome = match job.state {
            JobState::Inbox | JobState::Accepted => SpendOutcome::Succeeded,
            // Only a model task the provider called off gives the credits
            // back; one cancelled later (a resumed job, the rig step) is spent.
            JobState::Cancelled if after.chain.refunded => SpendOutcome::Refunded,
            JobState::Cancelled => SpendOutcome::Cancelled,
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

/// A By hand line, for sends whose card carries the site's estimate (older
/// handoffs do not, and stay out of the ledger).
fn by_hand_line(
    after: &Stored,
    event: LineEvent,
    at_ms: f64,
    outcome: SpendOutcome,
) -> Option<LedgerLine> {
    let job = &after.job;
    let card = job.card.as_ref()?;
    let estimate = card.credits?;
    let mut line = LedgerLine::subscription(event, at_ms, outcome);
    line.project.clone_from(&job.project);
    line.job.clone_from(&job.id);
    line.asset.clone_from(&job.asset);
    line.asset_name.clone_from(&job.asset_name);
    line.service = job.service;
    line.summary = format!(
        "By hand on the {} site, {}",
        job.service.label(),
        card.credits_note.as_deref().unwrap_or("one model")
    );
    line.estimate = Some(estimate);
    line.redo_of.clone_from(&job.redo_of);
    line.via_agent = job.via_agent;
    Some(line)
}

/// The `sent` line of a By hand send, written with its handoff.
pub(super) fn by_hand_sent(after: &Stored, at_ms: f64) -> Option<LedgerLine> {
    by_hand_line(after, LineEvent::Sent, at_ms, SpendOutcome::Pending)
}

/// A By hand send settles when its model is imported; stopped, it cost
/// nothing that Maquette can know of.
fn by_hand_settled(before: &Before, after: &Stored, at_ms: f64) -> Option<LedgerLine> {
    if before.over || !over(after.job.state) {
        return None;
    }
    let (outcome, credits) = match after.job.state {
        JobState::Inbox | JobState::Accepted => (SpendOutcome::Succeeded, None),
        JobState::Cancelled => (SpendOutcome::Cancelled, Some(0.0)),
        _ => (SpendOutcome::Failed, Some(0.0)),
    };
    let mut line = by_hand_line(after, LineEvent::Settled, at_ms, outcome)?;
    line.credits = credits;
    Some(line)
}

fn append(shared: &Shared, line: &LedgerLine) {
    let Some(workspace) = shared.workspace() else {
        return;
    };
    if let Err(err) = ledger::append(&workspace.root, line) {
        tracing::warn!(error = %err, "could not write the credits ledger");
    }
}

/// Appends the line a change calls for. A failed write is logged, never
/// raised: the job itself must go on.
pub(super) fn record(shared: &Shared, before: &Before, after: &Stored) {
    if let Some(line) = line(before, after, now_ms()) {
        append(shared, &line);
    }
}

/// Writes a new By hand send's `sent` line.
pub(super) fn record_by_hand(shared: &Shared, stored: &Stored) {
    if let Some(line) = by_hand_sent(stored, now_ms()) {
        append(shared, &line);
    }
}
