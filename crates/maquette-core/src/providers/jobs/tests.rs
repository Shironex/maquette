//! Tests for matching inbox file names to job ids and for what a
//! cancelled job costs.

use serde_json::json;

use super::cancel::stopped;
use super::records::match_job;
use super::spend;
use super::{CancelMode, JobState, Stored, cancel_mode, resume_fixup};
use crate::providers::ledger::SpendOutcome;

#[test]
fn inbox_names_match_the_longest_job_id() {
    let ids = ["lantern-1", "lantern-1-2", "fox-3"];
    let find = |stem: &str| match_job(stem, ids.iter().copied());
    assert_eq!(find("lantern-1"), Some("lantern-1"));
    assert_eq!(find("Lantern_1"), Some("lantern-1"));
    assert_eq!(find("lantern-1-2"), Some("lantern-1-2"));
    assert_eq!(find("lantern-1-final"), Some("lantern-1"));
    assert_eq!(find("lantern-10"), None);
    assert_eq!(find("fox 3 v2"), Some("fox-3"));
    assert_eq!(find("wolf-1"), None);
}

/// A Meshy prop job, running with the chain given.
fn meshy_job(state: &str, chain: serde_json::Value) -> Stored {
    serde_json::from_value(json!({
        "job": {
            "id": "lantern-1", "project": "moon-island", "asset": "lantern",
            "assetName": "Stone lantern", "provider": "meshy", "service": "meshy",
            "variant": "prop", "images": ["refs/front.png"], "state": state,
            "progress": 0.4, "createdAtMs": 1.0, "startedAtMs": 2.0,
            "viaAgent": false, "cancel": "provider", "estimate": 20.0,
        },
        "chain": chain,
    }))
    .expect("stored job")
}

/// Cancels `stored` the way `stop_job` does and returns the ledger outcome.
fn cancel(stored: &Stored) -> (String, Option<SpendOutcome>, f64, bool) {
    let before = spend::before(stored);
    let (message, refunded) = stopped(cancel_mode(stored), false, stored);
    let mut after = stored.clone();
    after.job.state = JobState::Cancelled;
    after.chain.refunded = refunded;
    let line = spend::line(&before, &after, 3.0);
    let (credits, estimated) = line.as_ref().map_or((0.0, false), |l| {
        crate::providers::ledger::charged(l.outcome, l.credits, l.estimate)
    });
    (message, line.map(|l| l.outcome), credits, estimated)
}

#[test]
fn only_a_model_task_the_provider_called_off_is_refunded() {
    let pending = meshy_job("running", json!({ "modelTask": "m-model" }));
    assert_eq!(cancel_mode(&pending), CancelMode::Provider);
    let (message, outcome, credits, _) = cancel(&pending);
    assert!(message.contains("refunds"), "{message}");
    assert_eq!(outcome, Some(SpendOutcome::Refunded));
    assert_eq!(credits, 0.0);

    // At the rig step Meshy only calls off the rig; the model was charged.
    let mut rigging = meshy_job(
        "running",
        json!({ "modelTask": "m-model", "rigTask": "m-rig", "credits": { "m-model": 20.0 } }),
    );
    rigging.job.credits = Some(20.0);
    let (message, outcome, credits, estimated) = cancel(&rigging);
    assert!(message.contains("credits are spent"), "{message}");
    assert_eq!(outcome, Some(SpendOutcome::Cancelled));
    assert_eq!(credits, 20.0);
    assert!(!estimated);
}

#[test]
fn a_resumed_job_cancelled_while_queued_has_spent_its_credits() {
    // Maquette closed while the model task ran; it is queued again on start.
    let mut resumed = meshy_job("running", json!({ "modelTask": "m-model" }));
    assert!(resume_fixup(&mut resumed));
    assert_eq!(resumed.job.state, JobState::Queued);
    assert_eq!(cancel_mode(&resumed), CancelMode::Free);
    let (message, outcome, credits, estimated) = cancel(&resumed);
    assert!(!message.contains("Nothing was spent"), "{message}");
    assert_eq!(outcome, Some(SpendOutcome::Cancelled));
    assert_eq!(credits, 20.0, "the price list stands in");
    assert!(estimated);

    // Queued before any task: free, and nothing is written.
    let fresh = meshy_job("queued", json!({}));
    let (message, outcome, _, _) = cancel(&fresh);
    assert_eq!(message, "Cancelled. Nothing was spent.");
    assert_eq!(outcome, None);
}
