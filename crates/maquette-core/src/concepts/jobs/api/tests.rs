//! Concept jobs on the Image API against a local fake server: the image
//! lands in its direction, the ledger gets the spend, the cap and a
//! missing key stop it, a kept 429 waits with the backoff, Retry and
//! Cancel work. No request reaches OpenAI.

use std::collections::BTreeMap;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use serde_json::json;

use super::super::{Concepts, Start};
use super::*;
use crate::concepts::backend::{ApiPlan, ImageBackend, ImageJobKind};
use crate::concepts::{Aspect, NewConcepts, load_direction};
use crate::pipeline::tools::Toolbox;
use crate::providers::fake::{FakeServer, Reply};
use crate::providers::openai_images::ImageQuality;
use crate::providers::secrets::MemoryStore;

fn png() -> Vec<u8> {
    crate::inbox::images::write_png(8, 4, &[90; 8 * 4 * 3])
}

fn answer() -> serde_json::Value {
    json!({
        "created": 1,
        "data": [{ "b64_json": base64::engine::general_purpose::STANDARD.encode(png()) }],
        "usage": {
            "input_tokens": 100,
            "output_tokens": 400,
            "input_tokens_details": { "text_tokens": 100, "image_tokens": 0 }
        }
    })
}

struct Rig {
    _dir: tempfile::TempDir,
    root: PathBuf,
    project: PathBuf,
    cache: PathBuf,
    concepts: Concepts,
    events: Arc<Mutex<Vec<ConceptEvent>>>,
}

fn rig(server: &FakeServer, key: bool, concurrency: u32) -> Rig {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    let project = root.join("moon");
    fs::create_dir_all(project.join("refs")).expect("project");
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let concepts = Concepts::new(None, concurrency, move |e| {
        sink.lock().expect("events").push(e);
    });
    let secrets = Arc::new(MemoryStore::default());
    if key {
        secrets
            .set_account(OPENAI_ACCOUNT, "sk-test-key")
            .expect("key");
    }
    concepts.set_image_api(ImageApiClient {
        secrets,
        base: server.url.clone(),
        timing: Timing::instant(),
    });
    Rig {
        _dir: dir,
        cache: root.join("cache/concepts"),
        root,
        project,
        concepts,
        events,
    }
}

fn plan(r: &Rig, cap: u32) -> ApiPlan {
    ApiPlan {
        kind: ImageJobKind::Concepts,
        quality: ImageQuality::Low,
        monthly_cap_usd: cap,
        ledger_root: r.root.clone(),
        via_agent: true,
        hold: false,
        per_image_usd: None,
    }
}

/// No Codex at all: the Image API must not need it.
fn no_tools() -> Toolbox {
    Toolbox {
        tools: BTreeMap::new(),
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        os: Some(crate::toolchain::HostOs::current()),
    }
}

fn request(direction: &str, count: u32, refs: Vec<String>) -> NewConcepts {
    NewConcepts {
        project: "moon".into(),
        direction: direction.into(),
        prompt: "A cosy porch at dusk.".into(),
        refs,
        aspect: Aspect::Wide,
        count,
        use_style: true,
    }
}

fn start(
    r: &Rig,
    request: NewConcepts,
    plan: ApiPlan,
) -> crate::CoreResult<(crate::concepts::Direction, Vec<ConceptJob>)> {
    let tools = no_tools();
    r.concepts.start(Start {
        project_dir: &r.project,
        cache_dir: &r.cache,
        tools: &tools,
        request,
        timeout: Duration::from_secs(20),
        api: Some(plan),
    })
}

fn wait_for(r: &Rig, id: &str, state: ConceptJobState) -> ConceptJob {
    for _ in 0..500 {
        if let Some(job) = r.concepts.jobs(None).into_iter().find(|j| j.id == id)
            && job.state == state
        {
            return job;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("{id} never became {state:?}: {:?}", r.concepts.jobs(None));
}

#[test]
fn a_round_lands_in_its_direction_and_each_image_goes_in_the_ledger() {
    let server = FakeServer::start(|_, _| Reply::ok(&answer()));
    let r = rig(&server, true, 2);
    let (direction, jobs) =
        start(&r, request("Porch", 2, Vec::new()), plan(&r, 10)).expect("start");
    assert_eq!(jobs.len(), 2);
    r.concepts.wait();
    let done = r.concepts.jobs(Some("moon"));
    for job in &done {
        assert_eq!(job.state, ConceptJobState::Done, "{job:?}");
        assert_eq!(job.backend, ImageBackend::OpenaiImages);
        assert!(r.project.join(&job.file).is_file());
        assert!(
            job.message
                .as_deref()
                .unwrap_or_default()
                .contains("OpenAI Images (low)")
        );
    }
    let landed = load_direction(&r.project, &direction.id)
        .expect("load")
        .expect("direction");
    assert_eq!(landed.images.len(), 2);
    let ledger = ledger::read(&r.root).expect("ledger");
    assert_eq!(ledger.images.len(), 2);
    assert_eq!(ledger.skipped, 0);
    let line = &ledger.images[0];
    assert_eq!(line.project, "moon");
    assert!(line.via_agent && !line.estimated);
    // 100 text tokens in at $5/M, 400 out at $30/M.
    assert!((line.usd - 0.0125).abs() < 1e-9, "{}", line.usd);
    assert!(
        r.events
            .lock()
            .expect("events")
            .iter()
            .any(|e| matches!(e, ConceptEvent::Direction { .. }))
    );
    let sent = server
        .find("POST", "/images/generations")
        .expect("sent")
        .json();
    let prompt = sent["prompt"].as_str().expect("prompt");
    assert!(prompt.starts_with("Generate an image."));
    assert!(prompt.contains("A cosy porch at dusk."));
    assert!(
        !prompt.contains("generated_images"),
        "no Codex save note: {prompt}"
    );
    assert_eq!(sent["size"], "1536x864");
}

#[test]
fn refs_go_as_an_edit_in_their_order() {
    let server = FakeServer::start(|_, _| Reply::ok(&answer()));
    let r = rig(&server, true, 1);
    fs::write(r.project.join("refs/a.png"), png()).expect("ref a");
    fs::write(r.project.join("refs/b.png"), b"second").expect("ref b");
    let refs = vec!["refs/a.png".to_string(), "refs/b.png".to_string()];
    start(&r, request("Edit", 1, refs), plan(&r, 10)).expect("start");
    r.concepts.wait();
    let sent = server.find("POST", "/images/edits").expect("edit").json();
    let images = sent["images"].as_array().expect("images");
    assert_eq!(images.len(), 2);
    let b64 = |b: &[u8]| base64::engine::general_purpose::STANDARD.encode(b);
    assert_eq!(
        images[0]["image_url"],
        format!("data:image/png;base64,{}", b64(&png()))
    );
    assert_eq!(
        images[1]["image_url"],
        format!("data:image/png;base64,{}", b64(b"second"))
    );
}

#[test]
fn a_round_past_the_cap_is_refused_before_anything_is_made() {
    let server = FakeServer::start(|_, _| Reply::ok(&answer()));
    let r = rig(&server, true, 1);
    let err = start(&r, request("Capped", 3, Vec::new()), plan(&r, 0)).expect_err("cap");
    assert!(
        err.to_string().contains("monthly OpenAI image cap"),
        "{err}"
    );
    assert!(r.concepts.jobs(None).is_empty());
    assert!(
        !r.project.join("concepts/capped").exists(),
        "no direction is made"
    );
    assert!(server.requests().is_empty());
}

#[test]
fn without_a_key_the_job_fails_and_says_where_to_add_one() {
    let server = FakeServer::start(|_, _| Reply::ok(&answer()));
    let r = rig(&server, false, 1);
    let (_, jobs) = start(&r, request("Keyless", 1, Vec::new()), plan(&r, 10)).expect("start");
    let job = wait_for(&r, &jobs[0].id, ConceptJobState::Failed);
    assert!(
        job.message
            .as_deref()
            .unwrap_or_default()
            .contains("Settings > Image generation"),
        "{job:?}"
    );
    assert!(server.requests().is_empty());
}

#[test]
fn a_kept_429_fails_with_openais_name_and_retry_runs_it_again() {
    let open = Arc::new(AtomicBool::new(false));
    let gate = Arc::clone(&open);
    let server = FakeServer::start(move |_, _| {
        if gate.load(Ordering::SeqCst) {
            Reply::ok(&answer())
        } else {
            Reply::json(
                429,
                &json!({ "error": { "message": "Rate limit reached" } }),
            )
            .with_header("Retry-After", "0")
        }
    });
    let r = rig(&server, true, 1);
    r.concepts.set_backoff(super::super::Backoff {
        first: Duration::from_millis(10),
        longest: Duration::from_millis(10),
        tries: 0,
    });
    let (_, jobs) = start(&r, request("Busy", 1, Vec::new()), plan(&r, 10)).expect("start");
    let failed = wait_for(&r, &jobs[0].id, ConceptJobState::Failed);
    let message = failed.message.unwrap_or_default();
    assert!(message.contains("Rate limited by OpenAI"), "{message}");
    assert!(ledger::read(&r.root).expect("ledger").images.is_empty());
    open.store(true, Ordering::SeqCst);
    r.concepts.retry(&jobs[0].id).expect("retry");
    let done = wait_for(&r, &jobs[0].id, ConceptJobState::Done);
    assert_eq!(done.file, jobs[0].file, "the same file, no new direction");
}

#[test]
fn a_waiting_job_cancels_without_a_request() {
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    let server = FakeServer::start(move |_, _| {
        counter.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(300));
        Reply::ok(&answer())
    });
    let r = rig(&server, true, 1);
    let (_, jobs) = start(&r, request("Queue", 2, Vec::new()), plan(&r, 10)).expect("start");
    r.concepts.cancel(&jobs[1].id).expect("cancel");
    let cancelled = wait_for(&r, &jobs[1].id, ConceptJobState::Cancelled);
    assert_eq!(cancelled.message.as_deref(), Some("Cancelled."));
    wait_for(&r, &jobs[0].id, ConceptJobState::Done);
    r.concepts.wait();
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn an_agents_round_waits_for_approval_and_sends_nothing_until_released() {
    let server = FakeServer::start(|_, _| Reply::ok(&answer()));
    let r = rig(&server, true, 2);
    let held = ApiPlan {
        hold: true,
        ..plan(&r, 10)
    };
    let (_, jobs) = start(&r, request("Held", 2, Vec::new()), held).expect("start");
    let ids: Vec<String> = jobs.iter().map(|j| j.id.clone()).collect();
    assert!(jobs.iter().all(|j| j.state == ConceptJobState::Approval));
    assert_eq!(r.concepts.awaiting_approval(&ids), ids);
    assert!(r.concepts.estimate_usd(&ids) > 0.0);
    assert!(
        r.concepts.api_pending_usd() > 0.0,
        "held jobs count against the cap"
    );
    std::thread::sleep(Duration::from_millis(50));
    assert!(
        server.requests().is_empty(),
        "nothing is sent before approval"
    );
    let err = r
        .concepts
        .retry(&ids[0])
        .expect_err("not retried while held");
    assert!(err.to_string().contains("waiting for approval"), "{err}");
    let released = r.concepts.release(&ids);
    assert_eq!(released.len(), 2);
    r.concepts.wait();
    assert!(
        r.concepts
            .jobs(None)
            .iter()
            .all(|j| j.state == ConceptJobState::Done)
    );
    assert_eq!(ledger::read(&r.root).expect("ledger").images.len(), 2);
}

#[test]
fn a_denied_or_cancelled_round_is_cancelled_without_a_request() {
    let server = FakeServer::start(|_, _| Reply::ok(&answer()));
    let r = rig(&server, true, 1);
    let held = ApiPlan {
        hold: true,
        ..plan(&r, 10)
    };
    let (_, jobs) = start(&r, request("Denied", 2, Vec::new()), held).expect("start");
    r.concepts.cancel(&jobs[1].id).expect("cancel");
    let ids: Vec<String> = jobs.iter().map(|j| j.id.clone()).collect();
    let denied = r.concepts.deny(&ids);
    assert_eq!(denied.len(), 1, "the cancelled one is not denied again");
    assert!(
        denied[0]
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("Denied in Maquette")
    );
    let cancelled = wait_for(&r, &jobs[1].id, ConceptJobState::Cancelled);
    assert!(
        cancelled
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("before it was approved")
    );
    assert!(r.concepts.awaiting_approval(&ids).is_empty());
    r.concepts.wait();
    assert!(server.requests().is_empty());
    // An agent's retry of a denied job waits for approval again.
    let again = r.concepts.retry_with(&ids[0], true).expect("retry");
    assert_eq!(again.state, ConceptJobState::Approval);
    assert!(server.requests().is_empty());
}

#[test]
fn a_safety_refusal_fails_as_refused_and_writes_nothing_to_the_ledger() {
    let server = FakeServer::start(|_, _| {
        Reply::json(
            400,
            &json!({ "error": {
                "message": "Your request was rejected by the safety system.",
                "type": "image_generation_user_error",
                "code": "moderation_blocked"
            } }),
        )
    });
    let r = rig(&server, true, 1);
    let (_, jobs) = start(&r, request("Refused", 1, Vec::new()), plan(&r, 10)).expect("start");
    let failed = wait_for(&r, &jobs[0].id, ConceptJobState::Failed);
    assert!(failed.refused);
    let message = failed.message.unwrap_or_default();
    assert!(
        message.contains("refused this image on safety grounds"),
        "{message}"
    );
    assert!(message.contains("Reword the prompt"), "{message}");
    assert!(ledger::read(&r.root).expect("ledger").images.is_empty());
}

#[test]
fn the_cap_uses_the_recent_average_when_the_plan_has_one() {
    let server = FakeServer::start(|_, _| Reply::ok(&answer()));
    let r = rig(&server, true, 1);
    // The price list's guess for medium is about $0.06 an image, so 20
    // images would pass a $1 cap; at a measured $0.04 they fit.
    let guessed = ApiPlan {
        quality: ImageQuality::Medium,
        ..plan(&r, 1)
    };
    assert!(guessed.estimate_usd(20, 0) > 1.0);
    let measured = ApiPlan {
        per_image_usd: Some(0.04),
        ..guessed.clone()
    };
    assert!((measured.estimate_usd(20, 3) - 0.8).abs() < 1e-9);
    r.concepts
        .check_api_cap(&measured, 20, 3)
        .expect("fits at the measured cost");
    assert!(r.concepts.check_api_cap(&guessed, 20, 3).is_err());
}
