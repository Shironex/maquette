//! Kept jobs across a restart: a fresh service reads the file, shows the
//! jobs failed (or done when the image landed), and Retry runs them again
//! against a fake codex script or the fake OpenAI server.

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use serde_json::json;

use super::super::tests::{fake_codex, request, rig, toolbox};
use super::super::{ConceptJobState, Concepts, ImageApiClient, Start};
use crate::concepts::backend::{ApiPlan, ImageJobKind};
use crate::concepts::load_direction;
use crate::pipeline::tools::Toolbox;
use crate::providers::fake::{FakeServer, Reply};
use crate::providers::http::Timing;
use crate::providers::openai_images::ImageQuality;
use crate::providers::secrets::{MemoryStore, OPENAI_ACCOUNT, SecretStore};

fn kept_ids(path: &std::path::Path) -> Vec<String> {
    let file: serde_json::Value =
        serde_json::from_slice(&fs::read(path).expect("kept file")).expect("json");
    file["jobs"]
        .as_array()
        .expect("jobs")
        .iter()
        .map(|k| k["job"]["id"].as_str().expect("id").to_string())
        .collect()
}

#[test]
fn codex_jobs_come_back_failed_after_a_restart_and_retry_runs_them() {
    let r = rig();
    let slow = fake_codex(&r.root, &r.png, &r.home, "slow");
    let store = r.root.join("state/lab-jobs.json");
    let before = Concepts::new(Some(r.home.clone()), 1, |_| {});
    assert!(before.keep_jobs_in(store.clone()).is_empty());
    let (direction, jobs) = before
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &toolbox(&slow),
            request: request("Slow porch", 3),
            timeout: Duration::from_secs(60),
            api: None,
        })
        .expect("start");
    let ids: Vec<String> = jobs.iter().map(|j| j.id.clone()).collect();
    assert_eq!(kept_ids(&store), ids);
    // The app closes: the file as it was is what the next start reads.
    let closed = r.root.join("closed.json");
    fs::copy(&store, &closed).expect("copy");
    before.cancel_all("moon");
    before.wait();
    assert!(kept_ids(&store).is_empty(), "cancelled jobs are not kept");

    // The third image landed before the app closed.
    fs::write(
        r.project.join(&jobs[2].file),
        fs::read(&r.png).expect("png"),
    )
    .expect("landed");
    // The codex at the kept path now draws.
    let ok = fake_codex(&r.root, &r.png, &r.home, "ok");
    fs::copy(&ok, &slow).expect("swap codex");

    let after = Concepts::new(Some(r.home.clone()), 1, |_| {});
    let restored = after.keep_jobs_in(closed.clone());
    assert_eq!(restored.len(), 3);
    for job in &restored[..2] {
        assert_eq!(job.state, ConceptJobState::Failed, "{job:?}");
        let message = job.message.as_deref().unwrap_or_default();
        assert!(message.starts_with("Interrupted by a restart"), "{message}");
        assert!(message.contains("Retry runs it again with the same prompt and file."));
    }
    assert!(
        restored[0]
            .message
            .as_deref()
            .is_some_and(|m| m.contains("while it was running"))
    );
    assert!(
        restored[1]
            .message
            .as_deref()
            .is_some_and(|m| m.contains("while it was queued"))
    );
    assert_eq!(restored[2].state, ConceptJobState::Done);
    assert_eq!(
        kept_ids(&closed),
        ids[..2].to_vec(),
        "failed jobs stay kept"
    );

    let again = after.retry(&ids[0]).expect("retry");
    assert_ne!(again.state, ConceptJobState::Failed);
    after.wait();
    let job = after
        .jobs(Some("moon"))
        .into_iter()
        .find(|j| j.id == ids[0])
        .expect("job");
    assert_eq!(job.state, ConceptJobState::Done, "{job:?}");
    let landed = load_direction(&r.project, &direction.id)
        .expect("load")
        .expect("direction");
    let files: Vec<&str> = landed.images.iter().map(|i| i.file.as_str()).collect();
    assert!(files.contains(&jobs[0].file.as_str()), "{files:?}");
    assert!(files.contains(&jobs[2].file.as_str()), "{files:?}");
    assert_eq!(kept_ids(&closed), vec![ids[1].clone()]);
    // Clear drops the failed rows, and they are no longer kept.
    after.clear_finished("moon");
    assert!(kept_ids(&closed).is_empty());
}

fn answer() -> serde_json::Value {
    let png = crate::inbox::images::write_png(8, 4, &[90; 8 * 4 * 3]);
    json!({
        "created": 1,
        "data": [{ "b64_json": base64::engine::general_purpose::STANDARD.encode(png) }],
        "usage": { "input_tokens": 100, "output_tokens": 400 }
    })
}

fn image_api(concepts: &Concepts, server: &FakeServer) {
    let secrets = Arc::new(MemoryStore::default());
    secrets
        .set_account(OPENAI_ACCOUNT, "sk-test-key")
        .expect("key");
    concepts.set_image_api(ImageApiClient {
        secrets,
        base: server.url.clone(),
        timing: Timing::instant(),
    });
}

#[test]
fn image_api_jobs_never_resume_on_their_own_and_an_agents_retry_waits_for_approval() {
    let r = rig();
    let server = FakeServer::start(|_, _| Reply::ok(&answer()));
    let store = r.root.join("state/lab-jobs.json");
    let before = Concepts::new(None, 1, |_| {});
    before.keep_jobs_in(store.clone());
    let plan = ApiPlan {
        kind: ImageJobKind::Concepts,
        quality: ImageQuality::Low,
        monthly_cap_usd: 10,
        ledger_root: r.root.clone(),
        via_agent: true,
        hold: true,
        per_image_usd: None,
    };
    let (_, jobs) = before
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &Toolbox::default(),
            request: request("Paid porch", 2),
            timeout: Duration::from_secs(20),
            api: Some(plan),
        })
        .expect("start");
    assert!(jobs.iter().all(|j| j.state == ConceptJobState::Approval));
    assert_eq!(kept_ids(&store).len(), 2);

    let after = Concepts::new(None, 1, |_| {});
    image_api(&after, &server);
    let restored = after.keep_jobs_in(store.clone());
    assert_eq!(restored.len(), 2);
    for job in &restored {
        assert_eq!(job.state, ConceptJobState::Failed);
        assert!(
            job.message
                .as_deref()
                .is_some_and(|m| m.contains("waited for approval, so nothing was sent")),
            "{job:?}"
        );
    }
    std::thread::sleep(Duration::from_millis(100));
    assert!(server.requests().is_empty(), "nothing is sent on its own");

    // An agent's retry waits for approval again; the release sends it.
    let held = after.retry_with(&restored[0].id, true).expect("retry");
    assert_eq!(held.state, ConceptJobState::Approval);
    assert!(server.requests().is_empty());
    after.release(std::slice::from_ref(&held.id));
    for _ in 0..500 {
        if after
            .jobs(None)
            .iter()
            .any(|j| j.id == held.id && j.state == ConceptJobState::Done)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let done = after
        .jobs(None)
        .into_iter()
        .find(|j| j.id == held.id)
        .expect("job");
    assert_eq!(done.state, ConceptJobState::Done, "{done:?}");
    assert_eq!(server.requests().len(), 1);
    assert!(r.project.join(&done.file).is_file());
}

#[test]
fn a_missing_or_unreadable_file_restores_nothing() {
    let r = rig();
    let concepts = Concepts::new(None, 1, |_| {});
    assert!(concepts.keep_jobs_in(r.root.join("none.json")).is_empty());
    let bad = r.root.join("bad.json");
    fs::write(&bad, b"{ not json").expect("write");
    let concepts = Concepts::new(None, 1, |_| {});
    assert!(concepts.keep_jobs_in(bad.clone()).is_empty());
    assert!(kept_ids(&bad).is_empty(), "rewritten empty");
}
