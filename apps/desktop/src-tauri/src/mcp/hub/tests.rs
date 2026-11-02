use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use maquette_core::pipeline::history::RunState;
use maquette_core::providers::ledger::{
    DecidedBy, Decision, DecisionInput, DecisionRecord, DecisionSource,
};

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
    let id = hub.request_approval(&run("moon-run-3"), &ship(), None);
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

    let id = hub.request_approval(&run("moon-run-4"), &ship(), None);
    assert_eq!(hub.wait_decision(&id, &AtomicBool::new(true)), None);
    assert!(hub.approvals().is_empty());
}

fn job(id: &str, name: &str) -> ProviderJob {
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
}

#[test]
fn a_batch_approval_covers_every_job_of_it() {
    let hub = Hub::new(|| {});
    assert!(hub.request_batch(&[]).is_none());
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

/// A hub whose answers land in the returned list.
fn recorded() -> (Hub, Arc<Mutex<Vec<DecisionRecord>>>) {
    let hub = Hub::new(|| {});
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    hub.set_recorder(move |r| sink.lock().expect("lock").push(r.clone()));
    (hub, seen)
}

#[test]
fn an_answer_from_a_card_is_recorded_with_where_and_how() {
    let (hub, seen) = recorded();
    let id = hub
        .request_batch(&[job("lamp-1", "Desk lamp"), job("cup-1", "Brush cup")])
        .expect("approval");
    let by = DecidedBy {
        input: Some(DecisionInput::Keyboard),
        trusted: Some(true),
        page_focused: Some(true),
        page_visible: Some(true),
        window_focused: Some(false),
        window_visible: Some(true),
        window_minimized: Some(false),
        ..DecidedBy::from(DecisionSource::FloatingCard)
    };
    assert!(hub.decide_by(&id, true, by));
    assert!(!hub.decide_by(&id, false, by), "answered once");
    let seen = seen.lock().expect("lock");
    assert_eq!(seen.len(), 1, "{seen:?}");
    let record = &seen[0];
    assert_eq!(record.approval, id);
    assert_eq!(record.kind, "send-to-3d");
    assert_eq!(record.decision, Decision::Approved);
    assert_eq!(record.by, by);
    assert_eq!(record.project, "landing");
    assert_eq!(record.jobs, vec!["lamp-1".to_string(), "cup-1".to_string()]);
    assert_eq!(record.provider.as_deref(), Some("Tripo"));
    assert_eq!(record.asset.as_deref(), Some("Desk lamp, Brush cup"));
    assert!(record.at_ms >= record.requested_at_ms);
}

#[test]
fn an_answer_without_a_source_says_so() {
    let (hub, seen) = recorded();
    let id = hub.request_send(&job("lamp-1", "Desk lamp"));
    assert!(hub.decide(&id, false));
    let seen = seen.lock().expect("lock");
    assert_eq!(seen[0].decision, Decision::Denied);
    assert_eq!(seen[0].by.source, DecisionSource::Unknown);
    assert_eq!(seen[0].asset.as_deref(), Some("Desk lamp"));
}

#[test]
fn a_cancel_before_any_answer_and_a_quit_are_recorded() {
    let (hub, seen) = recorded();
    let ship_id = hub.request_approval(&run("moon-run-5"), &ship(), None);
    assert_eq!(hub.wait_decision(&ship_id, &AtomicBool::new(true)), None);
    let send_id = hub.request_send(&job("lamp-1", "Desk lamp"));
    assert_eq!(hub.wait_until(&send_id, || true), None);
    let left = hub.request_send(&job("cup-1", "Brush cup"));
    hub.expire_all();
    assert!(hub.approvals().is_empty());
    let seen = seen.lock().expect("lock");
    let summary: Vec<(&str, Decision, DecisionSource)> = seen
        .iter()
        .map(|r| (r.approval.as_str(), r.decision, r.by.source))
        .collect();
    assert_eq!(
        summary,
        vec![
            (
                ship_id.as_str(),
                Decision::Cancelled,
                DecisionSource::RunCancelled
            ),
            (
                send_id.as_str(),
                Decision::Cancelled,
                DecisionSource::JobsCancelled
            ),
            (left.as_str(), Decision::Expired, DecisionSource::Quit),
        ]
    );
    assert_eq!(seen[0].run_id.as_deref(), Some("moon-run-5"));
}

#[test]
fn the_call_that_raised_an_approval_stops_waiting_once_answered() {
    let hub = Hub::new(|| {});
    let id = hub.request_send(&job("lamp-1", "Desk lamp"));
    hub.log_waiting_call(
        "send_to_3d",
        "landing, lamp".into(),
        CallOutcome::Waiting,
        Some("waiting for approval".into()),
        3.0,
        Some(id.clone()),
    );
    hub.log_call(
        "get_provider_job",
        "lamp-1".into(),
        CallOutcome::Ok,
        None,
        1.0,
    );
    assert!(hub.decide(&id, true));
    let calls = hub.calls();
    let raised = calls.iter().find(|c| c.tool == "send_to_3d").expect("call");
    assert_eq!(raised.outcome, CallOutcome::Ok);
    assert_eq!(raised.message.as_deref(), Some("Approved in the app"));
    let other = calls
        .iter()
        .find(|c| c.tool == "get_provider_job")
        .expect("call");
    assert_eq!(other.message, None, "other calls stay as they were");
}

/// Every Rust file under `dir`.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// What the MCP server can reach: the tools, the router and the HTTP
/// server. None of it may answer an approval; only the app's commands do.
#[test]
fn no_mcp_tool_can_answer_an_approval() {
    let mcp = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mcp");
    let mut files = vec![mcp.join("tools.rs"), mcp.join("server.rs")];
    rust_files(&mcp.join("tools"), &mut files);
    assert!(files.len() > 30, "found the tools: {}", files.len());
    let answers = [
        "hub.decide",
        ".decide_by(",
        ".settle(",
        "expire_all(",
        "close_rig_doctor(",
        "export::decide",
        "decide_reweight(",
        "decide_springs(",
        "decide_joints(",
        "approve_preview(",
    ];
    for file in &files {
        let source = std::fs::read_to_string(file).expect("read");
        for answer in answers {
            assert!(
                !source.contains(answer),
                "{} answers an approval with {answer}",
                file.display()
            );
        }
    }
    // A send or an image round goes out only on the person's yes, which
    // the waiter thread reads from the hub.
    let releases = [("sending.rs", ".approve("), ("image_round.rs", ".release(")];
    for (name, call) in releases {
        let source = std::fs::read_to_string(mcp.join("tools").join(name)).expect("read");
        let lines: Vec<&str> = source.lines().collect();
        let at: Vec<usize> = (0..lines.len())
            .filter(|&i| lines[i].contains(call))
            .collect();
        assert_eq!(at.len(), 1, "{name} has one {call}");
        let near = lines[at[0].saturating_sub(1)..=at[0]].join(" ");
        assert!(
            near.contains("Some(true)"),
            "{name}: {call} only on a yes: {near}"
        );
    }
}
