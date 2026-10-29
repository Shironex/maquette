//! A real MCP client controls Lab jobs as the Lab queue does: cancels them,
//! moves a project's queued jobs to the front (Run next) and retries the
//! failed or the held ones. The jobs run fake codex scripts; nothing
//! reaches Codex or OpenAI.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use maquette_core::concepts::jobs::Start;
use maquette_core::concepts::{Aspect, ConceptJob, ConceptJobState, Concepts, NewConcepts};
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::{ResolvedTool, Toolbox};
use maquette_core::providers::{Endpoints, MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::toolchain::{HostOs, ToolId};
use maquette_core::{AppState, NewProject, ProjectTemplate, WorkspaceConfig, project, workspace};
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::Hub;
use maquette_desktop_lib::session::Session;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::support::{Client, TestHost, call, raw_call, text};

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    project: String,
    project_dir: PathBuf,
}

fn fixture() -> (Fixture, Session) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Moon Island".into(),
            template: ProjectTemplate::Room,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    let fixture = Fixture {
        root: dir.path().to_path_buf(),
        _dir: dir,
        project: manifest.id,
        project_dir,
    };
    (fixture, session)
}

/// A fake codex that acts out `body` after reading the prompt.
fn codex(root: &Path, name: &str, body: &str) -> Toolbox {
    let path = root.join(format!("codex-{name}"));
    std::fs::write(&path, format!("#!/bin/sh\ncat > /dev/null\n{body}\n")).expect("script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    Toolbox {
        tools: BTreeMap::from([(
            ToolId::Codex,
            ResolvedTool {
                program: path,
                prefix: Vec::new(),
                version: "0.0.0-fake".into(),
            },
        )]),
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        os: Some(HostOs::current()),
    }
}

fn start(host: &TestHost, f: &Fixture, tools: &Toolbox, name: &str, count: u32) -> Vec<ConceptJob> {
    host.concepts
        .start(Start {
            project_dir: &f.project_dir,
            cache_dir: &f.root.join("cache/concepts"),
            tools,
            request: NewConcepts {
                project: f.project.clone(),
                direction: name.into(),
                prompt: "A stone lantern".into(),
                refs: Vec::new(),
                aspect: Aspect::Square,
                count,
                use_style: false,
            },
            timeout: Duration::from_secs(60),
            api: None,
        })
        .expect("start")
        .1
}

async fn connect(
    session: Session,
) -> (
    Arc<TestHost>,
    Client,
    maquette_desktop_lib::mcp::server::Running,
) {
    let options = Options {
        endpoints: Endpoints {
            tripo: "http://127.0.0.1:9".into(),
            meshy: "http://127.0.0.1:9".into(),
        },
        ..Options::default()
    };
    let studio = ProviderJobs::new(Arc::new(MemoryStore::default()), options, |_| {});
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(maquette_core::pipeline::runner::NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio,
    });
    let token = ensure_token(&host.session).expect("token");
    let server = maquette_desktop_lib::mcp::server::start(
        0,
        token.clone(),
        host.clone() as Arc<dyn Host>,
        Arc::clone(&hub),
    )
    .expect("start");
    let url = format!("http://127.0.0.1:{}/mcp", server.port());
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(url).auth_header(token),
    );
    let client = ClientConfig::default()
        .serve(transport)
        .await
        .expect("initialize");
    (host, client, server)
}

fn ids(value: &Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .expect(key)
        .iter()
        .map(|j| j["id"].as_str().expect("id").to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_cancel_and_run_lab_jobs_next() {
    maquette_core::providers::http::ensure_crypto();
    let (f, session) = fixture();
    let (host, client, server) = connect(session).await;
    let tools = client.list_all_tools().await.expect("tools");
    let cancel = tools
        .iter()
        .find(|t| t.name == "cancel_concept_jobs")
        .expect("cancel_concept_jobs");
    let hints = cancel.annotations.as_ref().expect("annotations");
    assert_eq!(hints.destructive_hint, Some(true));
    assert!(tools.iter().any(|t| t.name == "prioritize_concept_jobs"));
    let info = client.peer_info().expect("server info");
    assert!(info.instructions.as_deref().is_some_and(|i| {
        i.contains("cancel_concept_jobs") && i.contains("prioritize_concept_jobs")
    }));

    let slow = codex(&f.root, "slow", "sleep 30");
    let jobs = start(&host, &f, &slow, "Lanterns", 4);
    let job = |i: usize| jobs[i].id.clone();

    // Run next: the queued ones move; the running one and an unknown id are skipped.
    let next = call(
        &client,
        "prioritize_concept_jobs",
        json!({ "project": f.project, "jobs": [job(3), job(0), "nope"] }),
    )
    .await;
    assert_eq!(ids(&next, "moved"), vec![job(3)]);
    assert_eq!(ids(&next, "skipped"), vec![job(0), "nope".to_string()]);
    assert!(
        next["skipped"][0]["reason"]
            .as_str()
            .is_some_and(|r| r.contains("only a queued job moves"))
    );
    let all = call(
        &client,
        "prioritize_concept_jobs",
        json!({ "project": f.project }),
    )
    .await;
    assert_eq!(ids(&all, "moved"), vec![job(1), job(2), job(3)]);

    // Cancel one queued job and an unknown one, each answering on its own.
    let one = call(
        &client,
        "cancel_concept_jobs",
        json!({ "project": f.project, "jobs": [job(1), "nope"] }),
    )
    .await;
    assert_eq!(one["jobs"][0]["cancelled"], json!(true));
    assert_eq!(one["jobs"][0]["state"], json!("cancelled"));
    assert_eq!(one["jobs"][1]["cancelled"], json!(false));
    // Already cancelled: refused, not an error.
    let again = call(
        &client,
        "cancel_concept_jobs",
        json!({ "project": f.project, "jobs": [job(1)] }),
    )
    .await;
    assert_eq!(again["jobs"][0]["cancelled"], json!(false));

    // Without jobs: every unfinished one, the running one too.
    let rest = call(
        &client,
        "cancel_concept_jobs",
        json!({ "project": f.project }),
    )
    .await;
    assert_eq!(ids(&rest, "jobs"), vec![job(0), job(2), job(3)]);
    assert!(
        rest["jobs"]
            .as_array()
            .expect("jobs")
            .iter()
            .all(|j| j["cancelled"] == json!(true))
    );
    host.concepts.wait();
    assert!(
        host.concepts
            .jobs(None)
            .iter()
            .all(|j| j.state == ConceptJobState::Cancelled)
    );
    let none = raw_call(
        &client,
        "cancel_concept_jobs",
        json!({ "project": f.project }),
    )
    .await;
    assert_eq!(none.is_error, Some(true));
    let none = raw_call(
        &client,
        "prioritize_concept_jobs",
        json!({ "project": f.project }),
    )
    .await;
    assert_eq!(none.is_error, Some(true));

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retry_takes_the_failed_jobs_by_default_and_the_held_ones_on_request() {
    maquette_core::providers::http::ensure_crypto();
    let (f, session) = fixture();
    let (host, client, server) = connect(session).await;
    let failing = codex(&f.root, "fail", "echo 'error: not signed in' >&2; exit 2");
    let limited = codex(
        &f.root,
        "limited",
        "echo 'ERROR: exceeded retry limit, last status: 429 Too Many Requests' >&2; exit 1",
    );
    let failed = start(&host, &f, &failing, "Broken", 1);
    host.concepts.wait();
    let held = start(&host, &f, &limited, "Limited", 1);
    for _ in 0..500 {
        let states: Vec<_> = host.concepts.jobs(None).iter().map(|j| j.state).collect();
        if states == [ConceptJobState::Failed, ConceptJobState::RateLimited] {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let states: Vec<_> = host.concepts.jobs(None).iter().map(|j| j.state).collect();
    assert_eq!(
        states,
        [ConceptJobState::Failed, ConceptJobState::RateLimited]
    );

    let only_failed = call(
        &client,
        "retry_concept_jobs",
        json!({ "project": f.project }),
    )
    .await;
    assert_eq!(ids(&only_failed, "jobs"), vec![failed[0].id.clone()]);
    let only_held = call(
        &client,
        "retry_concept_jobs",
        json!({ "project": f.project, "include": "held" }),
    )
    .await;
    assert_eq!(ids(&only_held, "jobs"), vec![held[0].id.clone()]);

    // Both settle again: one fails, one waits out the rate limit again.
    for _ in 0..500 {
        let states: Vec<_> = host.concepts.jobs(None).iter().map(|j| j.state).collect();
        if states == [ConceptJobState::Failed, ConceptJobState::RateLimited] {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let both = call(
        &client,
        "retry_concept_jobs",
        json!({ "project": f.project, "include": "all" }),
    )
    .await;
    assert_eq!(
        ids(&both, "jobs"),
        vec![failed[0].id.clone(), held[0].id.clone()]
    );
    host.concepts.cancel_all(&f.project);
    host.concepts.wait();
    let nothing = raw_call(
        &client,
        "retry_concept_jobs",
        json!({ "project": f.project, "include": "held" }),
    )
    .await;
    assert_eq!(nothing.is_error, Some(true));
    assert!(text(&nothing).contains("no held Lab job"));

    client.cancel().await.expect("close");
    server.stop();
}
