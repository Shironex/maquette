use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::mcp::hub::{DecisionLog, Refused};
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
        repo_for_run: None,
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
    let pending = hub.pending_for_job("landing", "cup-1").expect("covered");
    assert_eq!(pending.id, id);
    assert!(
        hub.pending_for_job("studio", "cup-1").is_none(),
        "another project's cup-1 is not waiting"
    );
    assert_eq!(
        pending.jobs(),
        vec!["lamp-1".to_string(), "cup-1".to_string()]
    );
    assert_eq!(
        pending.detail.as_deref(),
        Some("2 models in total, one model per asset")
    );
    assert!(hub.decide(&id, true));
    assert!(hub.pending_for_job("landing", "lamp-1").is_none());
}

#[test]
fn a_pending_send_is_told_by_its_project_and_its_kind() {
    let hub = Hub::new(|| {});
    let mine = hub.request_send(&job("desk-lamp-1", "Desk lamp"));
    let mut other = job("desk-lamp-1", "Desk lamp");
    other.project = "studio".into();
    let theirs = hub.request_send(&other);
    assert_ne!(mine, theirs);
    let found = |project: &str| hub.pending_for_job(project, "desk-lamp-1").map(|a| a.id);
    assert_eq!(found("landing"), Some(mine.clone()));
    assert_eq!(found("studio"), Some(theirs.clone()));
    assert_eq!(found("moon"), None);
    // Answering one leaves the other waiting.
    assert!(hub.decide(&mine, false));
    assert_eq!(found("landing"), None);
    assert_eq!(found("studio"), Some(theirs));
}

/// Runs while a test log writes.
type Hook = Box<dyn Fn() + Send>;

/// A log that keeps what it is given: where (the place captured when the
/// question was raised) and what. It can be told to fail, and runs `hook`
/// as it writes.
#[derive(Clone, Default)]
struct TestLog {
    place: Arc<Mutex<PathBuf>>,
    seen: Arc<Mutex<Vec<(PathBuf, DecisionRecord)>>>,
    fail: Arc<AtomicBool>,
    hook: Arc<Mutex<Option<Hook>>>,
}

impl DecisionLog for TestLog {
    fn place(&self) -> Option<PathBuf> {
        Some(self.place.lock().expect("lock").clone())
    }
    fn write(&self, place: &Path, record: &DecisionRecord) -> Result<(), String> {
        if let Some(hook) = self.hook.lock().expect("lock").as_ref() {
            hook();
        }
        if self.fail.load(Ordering::SeqCst) {
            return Err("the disk is full".to_string());
        }
        self.seen
            .lock()
            .expect("lock")
            .push((place.to_path_buf(), record.clone()));
        Ok(())
    }
}

impl TestLog {
    fn records(&self) -> Vec<DecisionRecord> {
        self.seen
            .lock()
            .expect("lock")
            .iter()
            .map(|(_, r)| r.clone())
            .collect()
    }
}

/// A hub whose answers land in the returned log.
fn recorded() -> (Hub, TestLog) {
    let hub = Hub::new(|| {});
    let log = TestLog::default();
    *log.place.lock().expect("lock") = PathBuf::from("/work/one");
    hub.set_log(log.clone());
    (hub, log)
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
    assert_eq!(hub.decide_by(&id, true, by), Ok(()));
    assert_eq!(
        hub.decide_by(&id, false, by),
        Err(Refused::Gone),
        "answered once"
    );
    let seen = seen.records();
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
    let seen = seen.records();
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
    hub.expire_all(DecisionSource::Quit);
    assert!(hub.approvals().is_empty());
    let seen = seen.records();
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

#[test]
fn a_yes_that_cannot_be_written_releases_nothing_and_says_why() {
    let (hub, log) = recorded();
    let hub = Arc::new(hub);
    let id = hub.request_send(&job("lamp-1", "Desk lamp"));
    let waiter = {
        let (hub, id) = (Arc::clone(&hub), id.clone());
        std::thread::spawn(move || hub.wait_until(&id, || false))
    };
    log.fail.store(true, Ordering::SeqCst);
    let refused = hub.decide_by(&id, true, DecidedBy::from(DecisionSource::FloatingCard));
    let Err(Refused::Unrecorded(why)) = refused else {
        panic!("a yes without its record went through: {refused:?}");
    };
    assert!(why.contains("the disk is full"), "{why}");
    assert_eq!(hub.approvals().len(), 1, "it still waits for a person");
    assert_eq!(
        hub.unrecorded(&id).as_deref(),
        Some(why.as_str()),
        "the agent can read why"
    );
    std::thread::sleep(Duration::from_millis(150));
    assert!(!waiter.is_finished(), "nothing was released");
    // A no still applies, even unrecorded.
    assert!(hub.decide(&id, false));
    assert_eq!(waiter.join().expect("join"), Some(false));
    assert!(hub.unrecorded(&id).is_none());
}

#[test]
fn the_answer_is_written_before_the_waiter_wakes() {
    let (hub, log) = recorded();
    let hub = Arc::new(hub);
    let id = hub.request_send(&job("lamp-1", "Desk lamp"));
    let woke = Arc::new(AtomicBool::new(false));
    let early = Arc::new(AtomicBool::new(false));
    {
        let (woke, early) = (Arc::clone(&woke), Arc::clone(&early));
        *log.hook.lock().expect("lock") = Some(Box::new(move || {
            std::thread::sleep(Duration::from_millis(150));
            early.store(woke.load(Ordering::SeqCst), Ordering::SeqCst);
        }));
    }
    let waiter = {
        let (hub, id, woke) = (Arc::clone(&hub), id.clone(), Arc::clone(&woke));
        std::thread::spawn(move || {
            let answer = hub.wait_until(&id, || false);
            woke.store(true, Ordering::SeqCst);
            answer
        })
    };
    std::thread::sleep(Duration::from_millis(30));
    assert!(hub.decide(&id, true));
    assert_eq!(waiter.join().expect("join"), Some(true));
    assert!(
        !early.load(Ordering::SeqCst),
        "the waiter woke before the record was written"
    );
    let record = &log.records()[0];
    assert!(record.at_ms >= record.requested_at_ms);
}

#[test]
fn an_answer_goes_to_the_workspace_open_when_it_was_asked() {
    let (hub, log) = recorded();
    let id = hub.request_send(&job("lamp-1", "Desk lamp"));
    *log.place.lock().expect("lock") = PathBuf::from("/work/two");
    assert!(hub.decide(&id, true));
    let seen = log.seen.lock().expect("lock");
    assert_eq!(seen[0].0, PathBuf::from("/work/one"));
}

#[test]
fn a_workspace_change_expires_what_waits_and_releases_nothing() {
    let (hub, log) = recorded();
    let hub = Arc::new(hub);
    let id = hub.request_send(&job("lamp-1", "Desk lamp"));
    let waiter = {
        let (hub, id) = (Arc::clone(&hub), id.clone());
        std::thread::spawn(move || hub.wait_until(&id, || false))
    };
    std::thread::sleep(Duration::from_millis(30));
    hub.expire_all(DecisionSource::WorkspaceChanged);
    assert_eq!(
        waiter.join().expect("join"),
        Some(false),
        "told no, never yes"
    );
    assert!(hub.approvals().is_empty());
    let record = &log.records()[0];
    assert_eq!(record.decision, Decision::Expired);
    assert_eq!(record.by.source, DecisionSource::WorkspaceChanged);
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

/// The identifiers of `source`, line comments left out.
fn identifiers(source: &str) -> Vec<String> {
    source
        .lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .flat_map(|code| code.split(|c: char| !(c.is_alphanumeric() || c == '_')))
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// What the MCP server can reach: every file of the MCP module but the
/// hub's own answering code. None of it may answer an approval or call a
/// command that does; only the app's commands do.
#[test]
fn no_mcp_tool_can_answer_an_approval() {
    let mcp = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mcp");
    let mut files = Vec::new();
    rust_files(&mcp, &mut files);
    // The answering code itself, and the tests that call it.
    let own = [
        "hub/decisions.rs",
        "hub/rig.rs",
        "hub/recipe_bake.rs",
        "hub/recipe_save.rs",
        "hub/tests.rs",
    ];
    files.retain(|f| !own.iter().any(|o| f.ends_with(o)));
    assert!(files.len() > 40, "found the module: {}", files.len());
    let answers = [
        "decide",
        "decide_by",
        "settle",
        "expire_all",
        "close_rig_doctor",
        "close_recipe_bake",
        "recipe_bake_add",
        "claim_add",
        "add_as_next_version",
        "add_staged_bake",
        "close_recipe_save",
        "save_back",
        "recipe_library_save",
        // Only the person adds a recipe file to the library, copies a
        // starter there, or removes a library recipe or revision.
        "import_to_library",
        "recipe_import_to_library",
        "copy_to_library",
        "recipe_library_copy_starter",
        "remove_entry",
        "remove_revision",
        "recipe_library_remove",
        "recipe_library_remove_revision",
        // The same holds for the grip library: only the person saves a
        // grip there or removes a grip or a revision.
        "close_grip_save",
        "save_grip",
        "grip_library_save",
        "grip_library_remove",
        "grip_library_remove_revision",
        "from_card",
        "mcp_decide",
        "avatar_decide",
        "avatar_decide_reweight",
        "avatar_decide_springs",
        "decide_reweight",
        "decide_springs",
        "decide_joints",
        "rig_decide_joints",
        "decide_rigid",
        "rig_decide_rigid",
        "rig_land",
        "rig_discard",
        "rig_confirm_chains",
        "approve_preview",
    ];
    // A pack's retry choice is not an approval: decide_pack answers an
    // export that failed, and nothing paid waits on it.
    let not_approvals = [("tools/packs.rs", "decide")];
    for file in &files {
        let source = std::fs::read_to_string(file).expect("read");
        for word in identifiers(&source) {
            let allowed = not_approvals
                .iter()
                .any(|(name, w)| file.ends_with(name) && *w == word);
            assert!(
                allowed || !answers.contains(&word.as_str()),
                "{} answers an approval with {word}",
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
