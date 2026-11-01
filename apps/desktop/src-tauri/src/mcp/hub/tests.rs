use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use super::*;
use maquette_core::pipeline::history::RunState;

fn run(id: &str) -> RunRecord {
    RunRecord {
        id: id.into(),
        number: 3,
        project: "moon".into(),
        from: None,
        force: false,
        build_only: false,
        state: RunState::Running,
        queued_at_ms: 0.0,
        started_at_ms: None,
        finished_at_ms: None,
        steps: Vec::new(),
        message: None,
        not_shipped: None,
    }
}

fn ship() -> StepDef {
    StepDef {
        id: "ship".into(),
        name: "Ship".into(),
        kind: maquette_core::pipeline::StepKind::Shell,
        command: Some("true".into()),
        script: None,
        args: Vec::new(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        depends_on: Vec::new(),
        compress: None,
    }
}

#[test]
fn the_activity_list_keeps_the_newest_calls() {
    let changes = Arc::new(AtomicUsize::new(0));
    let counter = changes.clone();
    let hub = Hub::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    for i in 0..(MAX_CALLS + 5) {
        hub.log_call("list_projects", format!("{i}"), CallOutcome::Ok, None, 1.0);
    }
    let calls = hub.calls();
    assert_eq!(calls.len(), MAX_CALLS);
    assert_eq!(calls[0].detail, format!("{}", MAX_CALLS + 4));
    assert_eq!(changes.load(Ordering::SeqCst), MAX_CALLS + 5);
}

#[test]
fn an_approval_waits_for_a_decision_or_a_cancel() {
    let hub = Arc::new(Hub::new(|| {}));
    let id = hub.request_approval(&run("moon-run-3"), &ship());
    assert_eq!(hub.pending_for("moon-run-3").expect("pending").id, id);
    let waiter = {
        let hub = hub.clone();
        let id = id.clone();
        std::thread::spawn(move || hub.wait_decision(&id, &AtomicBool::new(false)))
    };
    std::thread::sleep(Duration::from_millis(50));
    assert!(hub.decide(&id, true));
    assert_eq!(waiter.join().expect("join"), Some(true));
    assert!(hub.pending_for("moon-run-3").is_none());
    assert!(!hub.decide(&id, false), "already decided");

    let id = hub.request_approval(&run("moon-run-4"), &ship());
    assert_eq!(hub.wait_decision(&id, &AtomicBool::new(true)), None);
    assert!(hub.approvals().is_empty());
}

#[test]
fn a_batch_approval_covers_every_job_of_it() {
    let hub = Hub::new(|| {});
    assert!(hub.request_batch(&[]).is_none());
    let job = |id: &str, name: &str| {
        let json = serde_json::json!({
            "id": id, "project": "landing", "asset": id, "assetName": name,
            "provider": "tripo", "service": "tripo", "variant": "prop",
            "images": ["refs/a.png"], "tripo": null, "meshy": null, "state": "approval",
            "phase": null, "progress": 0.0, "remoteId": null, "credits": null,
            "createdAtMs": 0.0, "startedAtMs": null, "finishedAtMs": null, "message": null,
            "inboxFile": null, "handoff": null, "card": null, "version": null,
            "viaAgent": true, "cancel": "free", "batch": "batch-1"
        });
        serde_json::from_value::<ProviderJob>(json).expect("job")
    };
    let id = hub
        .request_batch(&[job("lamp-1", "Desk lamp"), job("cup-1", "Brush cup")])
        .expect("approval");
    let pending = hub.pending_for_job("cup-1").expect("covered");
    assert_eq!(pending.id, id);
    assert_eq!(
        pending.jobs(),
        vec!["lamp-1".to_string(), "cup-1".to_string()]
    );
    assert_eq!(
        pending.detail.as_deref(),
        Some("2 models in total, one model per asset")
    );
    assert!(hub.decide(&id, true));
    assert!(hub.pending_for_job("lamp-1").is_none());
}
