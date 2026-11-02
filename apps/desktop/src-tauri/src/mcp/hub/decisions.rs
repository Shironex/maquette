//! Answering approvals, and keeping a record of every answer: approve, deny,
//! a cancel before anyone answered, and what was still waiting when the app
//! quit. Only the app answers: no MCP tool reaches [`Hub::decide_by`].

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

impl Hub {
    /// Where every answer goes from now on. The app writes them to the
    /// workspace's `approvals.jsonl`.
    pub fn set_recorder(&self, recorder: impl Fn(&DecisionRecord) + Send + Sync + 'static) {
        *self.recorder.lock().unwrap_or_else(|p| p.into_inner()) = Some(Arc::new(recorder));
    }

    /// Called outside the hub's lock, so a slow disk never holds a waiter.
    fn record(&self, approval: &Approval, decision: Decision, by: DecidedBy) {
        let recorder = self
            .recorder
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        if let Some(recorder) = recorder {
            recorder(&record_of(approval, decision, by));
        }
    }

    /// Approves or denies, without saying from where. False when the
    /// approval is gone (decided already, or its run was cancelled).
    pub fn decide(&self, approval_id: &str, approve: bool) -> bool {
        self.decide_by(
            approval_id,
            approve,
            DecidedBy::from(DecisionSource::Unknown),
        )
    }

    /// Approves or denies, and records who answered, from where and how.
    pub fn decide_by(&self, approval_id: &str, approve: bool, by: DecidedBy) -> bool {
        let decision = if approve {
            Decision::Approved
        } else {
            Decision::Denied
        };
        self.settle(approval_id, approve, decision, by)
    }

    /// Ends an approval with `decision`: a waiter gets `answer`.
    pub(super) fn settle(
        &self,
        approval_id: &str,
        answer: bool,
        decision: Decision,
        by: DecidedBy,
    ) -> bool {
        let found = {
            let mut inner = self.lock();
            let found = take(&mut inner, approval_id, decision);
            if found.is_some() {
                inner.decisions.insert(approval_id.to_string(), answer);
            }
            found
        };
        let Some(approval) = found else {
            return false;
        };
        self.decided.notify_all();
        self.record(&approval, decision, by);
        (self.on_change)();
        true
    }

    /// Records a question a newer one replaced; the caller holds the lock
    /// and has taken it out of the list.
    pub(super) fn withdrawn(&self, approval: &Approval) {
        self.record(
            approval,
            Decision::Cancelled,
            DecidedBy::from(DecisionSource::QuestionChanged),
        );
    }

    /// Everything still waiting when the app quits, recorded as expired.
    pub fn expire_all(&self) {
        let gone: Vec<Approval> = {
            let mut inner = self.lock();
            let ids: Vec<String> = inner.approvals.iter().map(|a| a.id.clone()).collect();
            ids.iter()
                .filter_map(|id| take(&mut inner, id, Decision::Expired))
                .collect()
        };
        for approval in &gone {
            self.record(
                approval,
                Decision::Expired,
                DecidedBy::from(DecisionSource::Quit),
            );
        }
        if !gone.is_empty() {
            (self.on_change)();
        }
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
                let taken = take(&mut inner, approval_id, Decision::Cancelled);
                drop(inner);
                if let Some(approval) = taken {
                    let source = if approval.kind == ApprovalKind::Ship {
                        DecisionSource::RunCancelled
                    } else {
                        DecisionSource::JobsCancelled
                    };
                    self.record(&approval, Decision::Cancelled, DecidedBy::from(source));
                }
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
