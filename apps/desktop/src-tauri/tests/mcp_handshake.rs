//! A real MCP client against the built in server: the handshake, the tool
//! list, a read tool, and run_pipeline on a fixture project, including the
//! pause before shipping until someone approves or denies it, and
//! send_to_3d, which waits for a person before it could spend credits.

use std::io::{Read as _, Write as _};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::{PipelineEvent, PipelineRunner, RunSink};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::pipeline::{self, StepDef, StepKind};
use maquette_core::providers::{
    Endpoints, JobState, MemoryStore, Options, ProviderId, ProviderJobs, SecretStore as _,
};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{
    AppState, AssetKind, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, assets,
    project, workspace,
};
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::ApprovalKind;
use maquette_desktop_lib::mcp::hub::Hub;
use maquette_desktop_lib::mcp::{Agents, ensure_token};
use maquette_desktop_lib::session::Session;
use rmcp::ServiceExt as _;
use rmcp::model::{CallToolRequestParams, CallToolResult, ClientConfig, ContentBlock};
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

struct TestHost {
    session: Session,
    runner: PipelineRunner,
    concepts: Concepts,
    thumbs: Thumbnails,
    studio: ProviderJobs,
}

/// Provider jobs that can never reach Tripo or Meshy: both point at a
/// closed local port, and the test never approves a send anyway.
fn studio(session: &Session) -> ProviderJobs {
    let secrets = Arc::new(MemoryStore::default());
    secrets
        .set(ProviderId::Tripo, "tsk_test_only_1234")
        .expect("key");
    let options = Options {
        endpoints: Endpoints {
            tripo: "http://127.0.0.1:9".into(),
            meshy: "http://127.0.0.1:9".into(),
        },
        ..Options::default()
    };
    let jobs = ProviderJobs::new(secrets, options, |_| {});
    if let Ok((root, config)) = session.workspace() {
        jobs.open_workspace(&root, &config);
    }
    jobs
}

impl Host for TestHost {
    fn session(&self) -> &Session {
        &self.session
    }
    fn runner(&self) -> &PipelineRunner {
        &self.runner
    }
    fn concepts(&self) -> &Concepts {
        &self.concepts
    }
    fn thumbs(&self) -> &Thumbnails {
        &self.thumbs
    }
    fn studio(&self) -> &ProviderJobs {
        &self.studio
    }
    fn toolbox(&self) -> Toolbox {
        Toolbox::default()
    }
}

/// Feeds runner events to the hub, the way the app's sink does.
struct HubSink(Arc<Hub>);

impl RunSink for HubSink {
    fn event(&self, event: PipelineEvent) {
        self.0.observe(&event);
    }
}

fn shell_step(id: &str, name: &str, script: &str, deps: &[&str]) -> StepDef {
    StepDef {
        id: id.into(),
        name: name.into(),
        kind: StepKind::Shell,
        // An absolute path, because the test toolbox has no PATH to search.
        command: Some("/bin/sh".into()),
        script: Some(script.into()),
        args: vec!["{script}".into()],
        inputs: Vec::new(),
        outputs: Vec::new(),
        depends_on: deps.iter().map(|d| (*d).to_string()).collect(),
        compress: None,
    }
}

/// A workspace with one project whose pipeline builds a file and then
/// "ships" by appending to `shipped.log`.
fn fixture(dir: &Path) -> (Session, String, std::path::PathBuf) {
    let root = dir.join("ws");
    let mut config = WorkspaceConfig::default();
    config.agents.mcp_enabled = true;
    config.agents.mcp_port = 4791;
    let config = workspace::create(&root, &config).expect("workspace");
    let session = Session::with_state(dir.join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Moon Island".into(),
            template: ProjectTemplate::Room,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    project::modify(&root, &config.layout, &manifest.id, |m| {
        assets::add(
            m,
            NewAsset {
                name: "Stone lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )
        .map(|_| ())
    })
    .expect("asset");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    let refs = project_dir.join("refs");
    std::fs::create_dir_all(&refs).expect("refs");
    std::fs::write(
        refs.join("stone_lantern_front.png"),
        b"\x89PNG\r\n\x1a\nfake",
    )
    .expect("ref");
    let mut def = pipeline::ensure(&project_dir, &manifest).expect("pipeline");
    def.steps = vec![
        shell_step("build", "Build", "tools/build.sh", &[]),
        shell_step("ship", "Ship", "tools/ship.sh", &["build"]),
    ];
    pipeline::save(&project_dir, &def).expect("save");
    let tools = project_dir.join("tools");
    std::fs::create_dir_all(&tools).expect("tools");
    std::fs::write(
        tools.join("build.sh"),
        "echo building\nmkdir -p out\necho b > out/build.txt\n",
    )
    .expect("build");
    std::fs::write(
        tools.join("ship.sh"),
        "echo shipping\necho shipped >> shipped.log\n",
    )
    .expect("ship");
    (session, manifest.id, project_dir)
}

fn text(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .find_map(|c| match c {
            ContentBlock::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

async fn call(client: &RunningService<RoleClient, ClientConfig>, tool: &str, args: Value) -> Value {
    let mut params = CallToolRequestParams::new(tool.to_string());
    if let Value::Object(map) = args {
        params = params.with_arguments(map);
    }
    let result = client.call_tool(params).await.expect("call");
    assert_ne!(
        result.is_error,
        Some(true),
        "{tool} failed: {}",
        text(&result)
    );
    serde_json::from_str(&text(&result)).expect("json")
}

/// Sends a bare HTTP request and returns the status line.
fn raw_status(port: u16, request: &str) -> String {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    stream.write_all(request.as_bytes()).expect("write");
    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).expect("read");
    String::from_utf8_lossy(&buf[..n])
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg_attr(windows, ignore = "the fixture pipeline runs sh scripts")]
async fn a_client_connects_lists_tools_reads_and_runs_with_an_approval_pause() {
    // The MCP client below shares the app's reqwest build.
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, project_dir) = fixture(dir.path());
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        studio: studio(&session),
        session,
        runner: PipelineRunner::new(Arc::new(HubSink(Arc::clone(&hub)))),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
    });
    // Port 0 so the test never collides with a running app; the app itself
    // listens on the configured 4791.
    let token = ensure_token(&host.session).expect("token");
    assert_eq!(token.len(), 64, "32 random bytes as hex");
    assert_eq!(
        host.session.app_state().mcp_token.as_deref(),
        Some(token.as_str()),
        "kept in the app state"
    );
    let server = maquette_desktop_lib::mcp::server::start(
        0,
        token.clone(),
        host.clone() as Arc<dyn Host>,
        Arc::clone(&hub),
    )
    .expect("start");
    let port = server.port();
    let url = format!("http://127.0.0.1:{port}/mcp");

    // Without the token, or off the /mcp path, the server refuses.
    let status = raw_status(
        port,
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    assert!(status.contains("401"), "{status}");
    let status = raw_status(
        port,
        &format!(
            "GET /other HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
        ),
    );
    assert!(status.contains("404"), "{status}");

    // Initialize.
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(url.clone()).auth_header(token.clone()),
    );
    let client = ClientConfig::default()
        .serve(transport)
        .await
        .expect("initialize");
    let info = client.peer_info().expect("server info");
    assert_eq!(
        info.server_info.as_ref().map(|s| s.name.as_str()),
        Some("maquette")
    );
    assert!(info.capabilities.tools.is_some());
    assert!(
        info.instructions
            .as_deref()
            .is_some_and(|i| i.contains("headless"))
    );

    // List tools: exactly these, each with a description and an object schema.
    let tools = client.list_all_tools().await.expect("tools");
    let mut names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "accept_inbox_model",
            "cancel_run",
            "get_concepts",
            "get_inbox_checks",
            "get_pipeline",
            "get_project",
            "get_provider_job",
            "get_run",
            "list_assets",
            "list_inbox",
            "list_projects",
            "render_thumbnail",
            "run_pipeline",
            "send_to_3d",
            "start_concepts",
        ]
    );
    for tool in &tools {
        assert!(
            tool.description.as_deref().is_some_and(|d| d.len() > 20),
            "{} needs a description",
            tool.name
        );
        assert_eq!(tool.input_schema.get("type"), Some(&json!("object")));
        for banned in ["screen", "capture", "gui", "open_in"] {
            assert!(!tool.name.contains(banned), "{} must not exist", tool.name);
        }
    }
    let run_tool = tools
        .iter()
        .find(|t| t.name == "run_pipeline")
        .expect("run");
    let props = run_tool.input_schema.get("properties").expect("properties");
    for field in ["project", "from", "force", "waitSeconds"] {
        assert!(props.get(field).is_some(), "run_pipeline takes {field}");
    }

    // A read tool.
    let projects = call(&client, "list_projects", json!({})).await;
    assert_eq!(projects["projects"][0]["id"], json!(project_id));
    assert_eq!(projects["projects"][0]["current"], json!(true));
    assert_eq!(projects["projects"][0]["activity"]["run"], json!(null));
    assert!(projects["projects"][0]["createdAt"].is_string());
    assert!(projects["projects"][0].get("thumb").is_some());
    // No tool moves the app to another project.
    assert!(
        tools
            .iter()
            .all(|t| !t.name.contains("switch") && !t.name.contains("open_project")),
        "no tool switches projects"
    );
    let pipeline = call(&client, "get_pipeline", json!({ "project": project_id })).await;
    assert_eq!(pipeline["def"]["steps"][1]["id"], json!("ship"));

    // run_pipeline pauses before Ship and says so.
    let started = Instant::now();
    let run = call(
        &client,
        "run_pipeline",
        json!({ "project": project_id, "waitSeconds": 30 }),
    )
    .await;
    assert_eq!(run["status"], json!("waiting for approval"), "{run}");
    assert!(
        started.elapsed() < Duration::from_secs(25),
        "answers at the pause"
    );
    let run_id = run["runId"].as_str().expect("run id").to_string();
    let approval = run["approval"]["id"]
        .as_str()
        .expect("approval")
        .to_string();
    assert_eq!(run["approval"]["step"], json!("Ship"));
    assert!(project_dir.join("out/build.txt").is_file(), "build ran");
    assert!(!project_dir.join("shipped.log").exists(), "ship waits");
    assert_eq!(hub.approvals().len(), 1, "the app shows the prompt");

    let polled = call(
        &client,
        "get_run",
        json!({ "project": project_id, "runId": run_id }),
    )
    .await;
    assert_eq!(polled["status"], json!("waiting for approval"));
    assert!(polled["log"].as_array().is_some_and(|l| {
        l.iter()
            .any(|line| line.as_str().is_some_and(|s| s.contains("building")))
    }));

    // Someone approves in the app; the run ships and finishes.
    assert!(hub.decide(&approval, true));
    let done = wait_for(&client, &project_id, &run_id).await;
    assert_eq!(done["status"], json!("done"), "{done}");
    assert_eq!(
        std::fs::read_to_string(project_dir.join("shipped.log")).expect("shipped"),
        "shipped\n"
    );

    // A second run, denied this time: it stops without shipping.
    let run = call(
        &client,
        "run_pipeline",
        json!({ "project": project_id, "force": true }),
    )
    .await;
    assert_eq!(run["status"], json!("waiting for approval"), "{run}");
    let run_id = run["runId"].as_str().expect("run id").to_string();
    let approval = run["approval"]["id"]
        .as_str()
        .expect("approval")
        .to_string();
    assert!(hub.decide(&approval, false));
    let denied = wait_for(&client, &project_id, &run_id).await;
    assert_eq!(denied["status"], json!("cancelled"));
    assert_eq!(denied["message"], json!("Shipping was denied in Maquette."));
    assert_eq!(
        std::fs::read_to_string(project_dir.join("shipped.log")).expect("shipped"),
        "shipped\n",
        "shipped once"
    );

    // The Lab reads back, empty in the fixture.
    let lab = call(&client, "get_concepts", json!({ "project": project_id })).await;
    assert_eq!(lab["directions"], json!([]));

    // A send to Tripo waits for a person before anything could be spent.
    let sent = call(
        &client,
        "send_to_3d",
        json!({
            "project": project_id,
            "asset": "stone-lantern",
            "provider": "tripo",
            "overrides": { "faceLimit": 20000 },
        }),
    )
    .await;
    assert_eq!(sent["status"], json!("waiting for approval"), "{sent}");
    assert_eq!(sent["images"], json!(["refs/stone_lantern_front.png"]));
    assert_eq!(sent["settings"], json!("Tripo H3.1, 1 image, 20,000 faces"));
    let job_id = sent["id"].as_str().expect("job").to_string();
    let approvals = hub.approvals();
    assert_eq!(approvals.len(), 1, "the app shows the prompt");
    assert_eq!(approvals[0].kind, ApprovalKind::SendTo3d);
    assert_eq!(approvals[0].job.as_deref(), Some(job_id.as_str()));
    assert_eq!(
        host.studio.get(&job_id).expect("job").state,
        JobState::Approval
    );
    // A preset setting that does not exist is refused before anything starts.
    let mut params = CallToolRequestParams::new("send_to_3d".to_string());
    if let Value::Object(map) = json!({
        "project": project_id,
        "asset": "stone-lantern",
        "provider": "meshy",
        "overrides": { "lowpoly": true },
    }) {
        params = params.with_arguments(map);
    }
    let refused = client.call_tool(params).await.expect("call");
    assert_eq!(refused.is_error, Some(true));
    assert!(text(&refused).contains("lowpoly"), "{}", text(&refused));

    // Denied in the app: nothing was sent, nothing spent.
    assert!(hub.decide(&approvals[0].id, false));
    let deadline = Instant::now() + Duration::from_secs(5);
    let denied = loop {
        let job = call(&client, "get_provider_job", json!({ "id": job_id })).await;
        if job["status"] == json!("cancelled") || Instant::now() > deadline {
            break job;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert_eq!(denied["status"], json!("cancelled"), "{denied}");
    assert_eq!(
        denied["message"],
        json!("Denied in Maquette. Nothing was spent.")
    );
    assert!(hub.approvals().is_empty());

    // By hand needs no approval: the handoff folder is there at once.
    let by_hand = call(
        &client,
        "send_to_3d",
        json!({ "project": project_id, "asset": "stone-lantern", "provider": "by-hand" }),
    )
    .await;
    assert_eq!(by_hand["status"], json!("waiting"), "{by_hand}");
    assert!(hub.approvals().is_empty());
    let handoff = std::path::PathBuf::from(by_hand["handoff"].as_str().expect("handoff"));
    assert!(handoff.join("settings.md").is_file());
    assert!(handoff.join("front.png").is_file());
    let listed = call(
        &client,
        "get_provider_job",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(listed["jobs"].as_array().map(Vec::len), Some(2));

    // Every call is in the activity list, newest first.
    let calls = hub.calls();
    let tools_called: Vec<&str> = calls.iter().map(|c| c.tool.as_str()).collect();
    assert!(tools_called.contains(&"run_pipeline"));
    assert!(
        tools_called.contains(&"connect"),
        "the refused request is listed"
    );
    assert!(
        calls
            .iter()
            .any(|c| c.tool == "run_pipeline"
                && c.message.as_deref() == Some("waiting for approval"))
    );

    client.cancel().await.expect("close");
    server.stop();
    assert!(
        std::net::TcpStream::connect(("127.0.0.1", port)).is_err(),
        "stopped"
    );
}

async fn wait_for(
    client: &RunningService<RoleClient, ClientConfig>,
    project: &str,
    run_id: &str,
) -> Value {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let run = call(
            client,
            "get_run",
            json!({ "project": project, "runId": run_id }),
        )
        .await;
        let status = run["status"].as_str().unwrap_or_default().to_string();
        if ["done", "failed", "cancelled"].contains(&status.as_str()) || Instant::now() > deadline {
            return run;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[test]
fn the_app_server_follows_the_workspace_setting() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, _, _) = fixture(dir.path());
    let hub = Arc::new(Hub::new(|| {}));
    let other = Session::with_state(dir.path().join("cfg2"), None, AppState::default());
    let host = Arc::new(TestHost {
        studio: studio(&other),
        session: other,
        runner: PipelineRunner::new(Arc::new(HubSink(Arc::clone(&hub)))),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
    });
    let agents = Agents::new(host, hub);
    // Off by default: nothing listens and there is no token yet.
    let fresh = Session::with_state(dir.path().join("cfg3"), None, AppState::default());
    fresh
        .set_workspace(dir.path().join("ws"), WorkspaceConfig::default())
        .expect("open");
    agents.sync(&fresh);
    let status = agents.status(&fresh);
    assert!(!status.enabled && !status.running);
    assert!(status.token.is_none());
    assert_eq!(status.url, "http://127.0.0.1:4791/mcp");

    // On with a free port picked for the test.
    let free = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = free.local_addr().expect("addr").port();
    drop(free);
    let (root, mut config) = session.workspace().expect("ws");
    config.agents.mcp_port = u32::from(port);
    session.set_config(config.clone());
    agents.sync(&session);
    let status = agents.status(&session);
    assert!(status.running, "{:?}", status.error);
    assert_eq!(status.port, u32::from(port));
    assert!(status.token.is_some());
    assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_ok());

    // Off again.
    config.agents.mcp_enabled = false;
    workspace::save(&root, &config).expect("save");
    session.set_config(config);
    agents.sync(&session);
    assert!(!agents.status(&session).running);
}
