//! Run next: queued jobs jump the line on the Codex lane and on the Image
//! API's own line; running and finished jobs stay where they are.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use serde_json::json;

use super::super::tests::{Rig, request, rig, toolbox};
use super::super::{ConceptJob, ConceptJobState, Concepts, ImageApiClient, Start};
use crate::concepts::backend::{ApiPlan, ImageJobKind};
use crate::pipeline::tools::Toolbox;
use crate::providers::fake::{FakeServer, Reply};
use crate::providers::http::Timing;
use crate::providers::openai_images::ImageQuality;
use crate::providers::secrets::{MemoryStore, OPENAI_ACCOUNT, SecretStore};

/// A codex that notes the file it was asked for, takes a moment, and draws.
fn ordered_codex(r: &Rig) -> PathBuf {
    let script = format!(
        "#!/bin/sh\nprompt=$(cat)\nout=$(printf '%s\\n' \"$prompt\" | grep -E '^/.*\\.png$' | tail -n 1)\necho \"$(basename \"$out\")\" >> \"{log}\"\nsleep 0.25\nmkdir -p \"$(dirname \"$out\")\"\ncp \"{png}\" \"$out\"\n",
        log = r.root.join("order.log").display(),
        png = r.png.display()
    );
    let path = r.root.join("codex-ordered");
    fs::write(&path, script).expect("script");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

fn start_codex(concepts: &Concepts, r: &Rig, codex: &Path, name: &str, n: u32) -> Vec<ConceptJob> {
    concepts
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &toolbox(codex),
            request: request(name, n),
            timeout: Duration::from_secs(20),
            api: None,
        })
        .expect("start")
        .1
}

fn ids(jobs: &[ConceptJob]) -> Vec<String> {
    jobs.iter().map(|j| j.id.clone()).collect()
}

#[test]
fn run_next_moves_queued_codex_jobs_ahead_of_the_others() {
    let r = rig();
    let codex = ordered_codex(&r);
    let concepts = Concepts::new(Some(r.home.clone()), 1, |_| {});
    let early = start_codex(&concepts, &r, &codex, "Early", 3);
    let urgent = start_codex(&concepts, &r, &codex, "Urgent", 2);
    // The running one does not move; the urgent ones do.
    let mut asked = ids(&urgent);
    asked.push(early[0].id.clone());
    let moved = concepts.run_next(&asked);
    assert_eq!(ids(&moved), ids(&urgent));
    concepts.wait();
    assert!(
        concepts
            .jobs(None)
            .iter()
            .all(|j| j.state == ConceptJobState::Done)
    );
    let log = fs::read_to_string(r.root.join("order.log")).expect("log");
    let order: Vec<&str> = log.lines().collect();
    assert_eq!(
        order,
        [
            "early_01.png",
            "urgent_01.png",
            "urgent_02.png",
            "early_02.png",
            "early_03.png"
        ]
    );
    assert!(
        concepts.run_next(&ids(&urgent)).is_empty(),
        "done jobs stay put"
    );
}

fn answer() -> serde_json::Value {
    let png = crate::inbox::images::write_png(8, 4, &[90; 8 * 4 * 3]);
    json!({
        "created": 1,
        "data": [{ "b64_json": base64::engine::general_purpose::STANDARD.encode(png) }],
        "usage": { "input_tokens": 100, "output_tokens": 400 }
    })
}

#[test]
fn run_next_project_moves_a_projects_image_api_jobs_to_the_front() {
    let r = rig();
    let prompts = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = Arc::clone(&prompts);
    let server = FakeServer::start(move |_, req| {
        let prompt = req.json()["prompt"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        seen.lock().expect("prompts").push(prompt);
        std::thread::sleep(Duration::from_millis(150));
        Reply::ok(&answer())
    });
    let concepts = Concepts::new(None, 1, |_| {});
    let secrets = Arc::new(MemoryStore::default());
    secrets
        .set_account(OPENAI_ACCOUNT, "sk-test-key")
        .expect("key");
    concepts.set_image_api(ImageApiClient {
        secrets,
        base: server.url.clone(),
        timing: Timing::instant(),
    });
    let plan = ApiPlan {
        kind: ImageJobKind::Concepts,
        quality: ImageQuality::Low,
        monthly_cap_usd: 10,
        ledger_root: r.root.clone(),
        via_agent: false,
        hold: false,
        per_image_usd: None,
    };
    let tools = Toolbox::default();
    for (project, prompt, n) in [("moon", "A moon porch.", 3), ("sun", "A sun gate.", 2)] {
        let mut brief = request(prompt, n);
        brief.project = project.into();
        brief.prompt = prompt.into();
        concepts
            .start(Start {
                project_dir: &r.project,
                cache_dir: &r.cache,
                tools: &tools,
                request: brief,
                timeout: Duration::from_secs(20),
                api: Some(plan.clone()),
            })
            .expect("start");
    }
    let moved = concepts.run_next_project("sun");
    assert_eq!(moved.len(), 2);
    assert!(moved.iter().all(|j| j.project == "sun"));
    concepts.wait();
    let order: Vec<bool> = prompts
        .lock()
        .expect("prompts")
        .iter()
        .map(|p| p.contains("A sun gate."))
        .collect();
    assert_eq!(order, [false, true, true, false, false]);
}
