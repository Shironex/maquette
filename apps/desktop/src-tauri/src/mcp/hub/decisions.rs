//! Answering approvals, and keeping a record of every answer: approve, deny,
//! a cancel before anyone answered, and what was still waiting when the app
//! quit or the workspace changed. Only the app answers: no MCP tool reaches
//! [`Hub::decide_by`].
//!
//! The record is built and written under the hub's lock, before any waiter
//! wakes, into the workspace that was open when the question was raised. A
//! yes that cannot be written is refused: nothing paid goes out without its
//! record. A no, a cancel or an expiry still applies, with a warning.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use maquette_core::pipeline::history::now_ms;
use maquette_core::providers::ledger::{DecidedBy, Decision, DecisionRecord, DecisionSource};

use super::{Approval, ApprovalKind, CallOutcome, Hub, Inner};

/// The kebab-case name of an approval kind, as the record keeps it.
fn kind_name(kind: ApprovalKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// The record of `decision` on `approval`.
fn record_of(approval: &Approval, decision: Decision, by: DecidedBy) -> DecisionRecord {
    let mut record = DecisionRecord::new(
        &approval.id,
        &kind_name(approval.kind),
        decision,
        by,
        now_ms(),
    );
    record.project.clone_from(&approval.project);
    record.jobs = approval.jobs();
    record.concept_jobs.clone_from(&approval.concept_jobs);
    record.run_id.clone_from(&approval.run_id);
    record.provider.clone_from(&approval.service);
    record.asset = approval.asset_name.clone().or_else(|| {
        let names: Vec<&str> = approval
            .items
            .iter()
            .map(|i| i.asset_name.as_str())
            .collect();
        (!names.is_empty()).then(|| names.join(", "))
    });
    record.estimate = approval.estimate;
    record.estimate_usd = approval.estimate_usd;
    record.requested_at_ms = approval.requested_at_ms;
    record
}

/// What a call that raised an approval says once it is answered.
fn answered(decision: Decision) -> &'static str {
    match decision {
        Decision::Approved => "Approved in the app",
        Decision::Denied => "Denied in the app",
        Decision::Expired => "Still waiting when Maquette quit",
        Decision::Cancelled => "Cancelled before anyone answered",
    }
}

/// Takes the approval out of the waiting list, and marks the calls that
/// raised it as answered. `None` when it was not waiting.
fn take(inner: &mut Inner, approval_id: &str, decision: Decision) -> Option<Approval> {
    let at = inner.approvals.iter().position(|a| a.id == approval_id)?;
    let approval = inner.approvals.remove(at);
    for call in inner
        .calls
        .iter_mut()
        .filter(|c| c.approval.as_deref() == Some(approval_id))
    {
        if call.outcome == CallOutcome::Waiting {
            call.outcome = CallOutcome::Ok;
        }
        call.message = Some(answered(decision).to_string());
    }
    Some(approval)
}

/// Where answers are written: the app's is the workspace's `approvals.jsonl`.
pub trait DecisionLog: Send + Sync {
    /// Where answers to a question raised now go: the open workspace.
    fn place(&self) -> Option<PathBuf>;
    /// Appends one record at `place`.
    fn write(&self, place: &Path, record: &DecisionRecord) -> Result<(), String>;
}

/// Why an answer did not go through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// Answered already, or withdrawn.
    Gone,
    /// A yes whose record could not be written; it still waits.
    Unrecorded(String),
}

impl Hub {
    /// Where every answer goes from now on.
    pub fn set_log(&self, log: impl DecisionLog + 'static) {
        *self.log.lock().unwrap_or_else(|p| p.into_inner()) = Some(Arc::new(log));
    }

    fn current_log(&self) -> Option<Arc<dyn DecisionLog>> {
        self.log.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Where a question raised now has its answer written.
    pub(super) fn place_now(&self) -> Option<PathBuf> {
        self.current_log().and_then(|log| log.place())
    }

    /// Writes the record of `decision` on `approval` to the place captured
    /// when it was raised. The caller holds the lock.
    fn write(
        &self,
        inner: &Inner,
        approval: &Approval,
        decision: Decision,
        by: DecidedBy,
    ) -> Result<(), String> {
        let Some(log) = self.current_log() else {
            return Ok(());
        };
        let Some(place) = inner.places.get(&approval.id).cloned().flatten() else {
            return Err("No workspace was open when this was asked.".to_string());
        };
        log.write(&place, &record_of(approval, decision, by))
    }

    /// Ends an approval that no one answered (cancelled or expired): the
    /// record is written, a waiter gets no. The caller holds the lock.
    fn end_unanswered(&self, inner: &mut Inner, id: &str, decision: Decision, by: DecidedBy) {
        let Some(approval) = inner.approvals.iter().find(|a| a.id == id).cloned() else {
            return;
        };
        if let Err(err) = self.write(inner, &approval, decision, by) {
            tracing::warn!(error = %err, approval = id, "could not record an approval's end");
        }
        take(inner, id, decision);
        inner.places.remove(id);
        inner.unrecorded.remove(id);
    }

    /// Approves or denies, without saying from where. False when the
    /// approval is gone or the yes could not be recorded.
    pub fn decide(&self, approval_id: &str, approve: bool) -> bool {
        self.decide_by(
            approval_id,
            approve,
            DecidedBy::from(DecisionSource::Unknown),
        )
        .is_ok()
    }

    /// Approves or denies, and records who answered, from where and how.
    pub fn decide_by(
        &self,
        approval_id: &str,
        approve: bool,
        by: DecidedBy,
    ) -> Result<(), Refused> {
        let decision = if approve {
            Decision::Approved
        } else {
            Decision::Denied
        };
        self.settle(approval_id, approve, decision, by)
    }

    /// Why the last yes to `approval_id` was refused, while it still waits.
    pub fn unrecorded(&self, approval_id: &str) -> Option<String> {
        self.lock().unrecorded.get(approval_id).cloned()
    }

    /// Ends an approval with `decision`, written before a waiter gets
    /// `answer`. An approval that cannot be written stays waiting.
    pub(super) fn settle(
        &self,
        approval_id: &str,
        answer: bool,
        decision: Decision,
        by: DecidedBy,
    ) -> Result<(), Refused> {
        let mut inner = self.lock();
        let approval = inner
            .approvals
            .iter()
            .find(|a| a.id == approval_id)
            .cloned()
            .ok_or(Refused::Gone)?;
        if let Err(err) = self.write(&inner, &approval, decision, by) {
            if decision == Decision::Approved {
                let why = format!(
                    "Not approved: the answer could not be written to approvals.jsonl, so nothing was sent. {err}"
                );
                inner
                    .unrecorded
                    .insert(approval_id.to_string(), why.clone());
                drop(inner);
                (self.on_change)();
                return Err(Refused::Unrecorded(why));
            }
            tracing::warn!(error = %err, approval = approval_id, "could not record an answer");
        }
        take(&mut inner, approval_id, decision);
        inner.places.remove(approval_id);
        inner.unrecorded.remove(approval_id);
        inner.decisions.insert(approval_id.to_string(), answer);
        drop(inner);
        self.decided.notify_all();
        (self.on_change)();
        Ok(())
    }

    /// Records a question a newer one replaced, and takes it out of the
    /// list; the caller holds the lock and tells its waiter.
    pub(super) fn withdraw(&self, inner: &mut Inner, id: &str) {
        self.end_unanswered(
            inner,
            id,
            Decision::Cancelled,
            DecidedBy::from(DecisionSource::QuestionChanged),
        );
    }

    /// Everything still waiting, recorded as expired: the app quits
    /// (`Quit`) or another workspace opens (`WorkspaceChanged`). Every
    /// waiter is told no, so nothing waits on a question no card shows.
    pub fn expire_all(&self, source: DecisionSource) {
        let mut inner = self.lock();
        let waiting: Vec<(String, ApprovalKind)> = inner
            .approvals
            .iter()
            .map(|a| (a.id.clone(), a.kind))
            .collect();
        if waiting.is_empty() {
            return;
        }
        for (id, kind) in &waiting {
            self.end_unanswered(&mut inner, id, Decision::Expired, DecidedBy::from(source));
            inner.decisions.insert(id.clone(), false);
            if *kind == ApprovalKind::PagePreview {
                // Not a person's no: the next start asks again.
                inner.preview_withdrawn.insert(id.clone());
            }
        }
        drop(inner);
        self.decided.notify_all();
        (self.on_change)();
    }

    /// Blocks until the approval is decided. `None` when the run was
    /// cancelled first; the approval is then withdrawn.
    pub fn wait_decision(&self, approval_id: &str, cancelled: &AtomicBool) -> Option<bool> {
        self.wait_until(approval_id, || cancelled.load(Ordering::SeqCst))
    }

    /// Like [`Hub::wait_decision`], with any test for "it went away first".
    /// What went away is recorded as cancelled.
    pub fn wait_until(&self, approval_id: &str, gone: impl Fn() -> bool) -> Option<bool> {
        let mut inner = self.lock();
        loop {
            if let Some(answer) = inner.decisions.remove(approval_id) {
                return Some(answer);
            }
            if gone() {
                let kind = inner
                    .approvals
                    .iter()
                    .find(|a| a.id == approval_id)
                    .map(|a| a.kind);
                if let Some(kind) = kind {
                    let source = if kind == ApprovalKind::Ship {
                        DecisionSource::RunCancelled
                    } else {
                        DecisionSource::JobsCancelled
                    };
                    let by = DecidedBy::from(source);
                    self.end_unanswered(&mut inner, approval_id, Decision::Cancelled, by);
                }
                drop(inner);
                (self.on_change)();
                return None;
            }
            inner = self
                .decided
                .wait_timeout(inner, Duration::from_millis(100))
                .map(|(guard, _)| guard)
                .unwrap_or_else(|p| p.into_inner().0);
        }
    }
}
