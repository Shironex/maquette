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
    /// How often a tool told the windows that a pipeline file changed.
    pipeline_changes: std::sync::atomic::AtomicUsize,
    /// How often a tool told the windows that a scene layout changed.
    layout_changes: std::sync::atomic::AtomicUsize,
    /// How often a tool told the windows that material tweaks changed.
    materials_changes: std::sync::atomic::AtomicUsize,
    /// How often a tool told the windows that real sizes or fix-ups changed.
    prepare_changes: std::sync::atomic::AtomicUsize,
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
    fn pipeline_changed(&self, _project: &str) {
        self.pipeline_changes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    fn layout_changed(&self, _project: &str) {
        self.layout_changes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    fn materials_changed(&self, _project: &str) {
        self.materials_changes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    fn prepare_changed(&self, _project: &str) {
        self.prepare_changes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        for name in ["Stone lantern", "Desk lamp"] {
            assets::add(
                m,
                NewAsset {
                    name: name.into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Refs,
                },
            )?;
        }
        Ok(())
    })
    .expect("asset");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    let refs = project_dir.join("refs");
    std::fs::create_dir_all(&refs).expect("refs");
    for name in ["stone_lantern_front.png", "desk_lamp.png"] {
        std::fs::write(refs.join(name), b"\x89PNG\r\n\x1a\nfake").expect("ref");
    }
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

async fn raw_call(
    client: &RunningService<RoleClient, ClientConfig>,
    tool: &str,
    args: Value,
) -> CallToolResult {
    let mut params = CallToolRequestParams::new(tool.to_string());
    if let Value::Object(map) = args {
        params = params.with_arguments(map);
    }
    client.call_tool(params).await.expect("call")
}

async fn call(client: &RunningService<RoleClient, ClientConfig>, tool: &str, args: Value) -> Value {
    let result = raw_call(client, tool, args).await;
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
        pipeline_changes: std::sync::atomic::AtomicUsize::new(0),
        layout_changes: std::sync::atomic::AtomicUsize::new(0),
        materials_changes: std::sync::atomic::AtomicUsize::new(0),
        prepare_changes: std::sync::atomic::AtomicUsize::new(0),
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
            "add_built_asset",
            "add_fixups",
            "cancel_run",
            "copy_asset",
            "get_bake_settings",
            "get_concepts",
            "get_credits",
            "get_inbox_checks",
            "get_layout",
            "get_material_tweaks",
            "get_motion",
            "get_pipeline",
            "get_project",
            "get_provider_job",
            "get_real_size",
            "get_run",
            "get_starters",
            "list_assets",
            "list_built_assets",
            "list_fixups",
            "list_handoffs",
            "list_inbox",
            "list_projects",
            "make_object_refs",
            "pick_concepts",
            "preview_fixups",
            "rebuild_built_asset",
            "redo_inbox_model",
            "redo_inbox_models",
            "remove_fixups",
            "render_thumbnail",
            "retry_concept_jobs",
            "run_pipeline",
            "send_batch_to_3d",
            "send_to_3d",
            "set_bake_settings",
            "set_compression_presets",
            "set_layout",
            "set_material_tweaks",
            "set_pad_edges",
            "set_real_size",
            "set_skip_bake",
            "star_concepts",
            "start_concepts",
            "update_starters",
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

    // set_compression_presets: partial edits, locks, validation, live notice.
    let mut def = pipeline::load(&project_dir).expect("def").expect("file");
    let row = |model: &str, lock: Option<&str>| pipeline::CompressPreset {
        model: model.into(),
        label: model.into(),
        texture_size: 2048,
        data_texture_size: None,
        codec: maquette_core::TextureCodec::Etc1s,
        map_codecs: None,
        meshopt: maquette_core::MeshoptLevel::High,
        lock: lock.map(str::to_string),
    };
    def.presets = vec![
        row("stone_lantern", None),
        row("desk_lamp", None),
        row("hero", Some("High spins")),
    ];
    pipeline::save(&project_dir, &def).expect("presets");
    let set = call(
        &client,
        "set_compression_presets",
        json!({ "project": project_id, "presets": [
            { "model": "stone_lantern", "dataTextureSize": 1024, "codec": "UASTC" },
            { "model": "desk_lamp", "textureSize": 512 },
        ] }),
    )
    .await;
    let rows = set["presets"].as_array().expect("presets");
    let by = |m: &str| rows.iter().find(|p| p["model"] == json!(m)).expect(m);
    assert_eq!(by("stone_lantern")["dataTextureSize"], json!(1024));
    assert_eq!(by("stone_lantern")["codec"], json!("uastc"));
    assert_eq!(by("stone_lantern")["textureSize"], json!(2048), "kept");
    assert_eq!(by("hero")["lock"], json!("High spins"), "untouched");
    let after = pipeline::load(&project_dir).expect("def").expect("file");
    let lamp = after
        .presets
        .iter()
        .find(|p| p.model == "desk_lamp")
        .expect("lamp");
    assert_eq!(lamp.texture_size, 512);
    assert_eq!(lamp.codec, maquette_core::TextureCodec::Etc1s, "kept");
    assert_eq!(
        host.pipeline_changes
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    let cleared = call(
        &client,
        "set_compression_presets",
        json!({ "project": project_id, "presets": [
            { "model": "stone_lantern", "dataTextureSize": null },
        ] }),
    )
    .await;
    let lantern = cleared["presets"]
        .as_array()
        .and_then(|p| p.iter().find(|p| p["model"] == json!("stone_lantern")))
        .expect("row");
    assert_eq!(
        lantern["dataTextureSize"],
        json!(null),
        "null clears the split"
    );
    assert_eq!(lantern["codec"], json!("uastc"), "other fields stay");
    for (entry, needle) in [
        (json!({ "model": "hero", "textureSize": 512 }), "High spins"),
        (json!({ "model": "nope" }), "stone_lantern"),
        (
            json!({ "model": "desk_lamp", "textureSize": 999 }),
            "512, 1024",
        ),
        (
            json!({ "model": "desk_lamp", "codec": "png" }),
            "etc1s, uastc",
        ),
        (
            json!({ "model": "desk_lamp", "meshopt": "max" }),
            "low, medium, high",
        ),
    ] {
        let mut params = CallToolRequestParams::new("set_compression_presets".to_string());
        if let Value::Object(map) = json!({ "project": project_id, "presets": [entry] }) {
            params = params.with_arguments(map);
        }
        let refused = client.call_tool(params).await.expect("call");
        assert_eq!(refused.is_error, Some(true), "{entry}");
        assert!(text(&refused).contains(needle), "{}", text(&refused));
    }
    assert_eq!(
        pipeline::load(&project_dir)
            .expect("def")
            .expect("file")
            .presets
            .len(),
        3,
        "a refused edit saves nothing"
    );

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
    assert_eq!(lab["failed"], json!([]));
    // Nothing failed, so Retry has nothing to run, and an unknown job is refused.
    let none = raw_call(
        &client,
        "retry_concept_jobs",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(none.is_error, Some(true));
    assert!(text(&none).contains("no failed Lab job"), "{}", text(&none));
    let unknown = raw_call(
        &client,
        "retry_concept_jobs",
        json!({ "project": project_id, "jobs": ["nope-01"] }),
    )
    .await;
    assert_eq!(unknown.is_error, Some(true));
    assert!(text(&unknown).contains("no Lab job"), "{}", text(&unknown));

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

    // A batch to Tripo waits for one approval that lists every asset.
    let batch = call(
        &client,
        "send_batch_to_3d",
        json!({
            "project": project_id,
            "provider": "tripo",
            "items": [{ "asset": "stone-lantern" }, { "asset": "desk-lamp", "variant": "character" }],
        }),
    )
    .await;
    assert_eq!(batch["status"], json!("waiting for approval"), "{batch}");
    let batch_jobs = batch["jobs"].as_array().expect("jobs").clone();
    assert_eq!(batch_jobs.len(), 2);
    assert_eq!(batch_jobs[1]["images"], json!(["refs/desk_lamp.png"]));
    let approvals = hub.approvals();
    assert_eq!(approvals.len(), 1, "one prompt for the whole batch");
    let names: Vec<&str> = approvals[0]
        .items
        .iter()
        .map(|i| i.asset_name.as_str())
        .collect();
    assert_eq!(names, vec!["Stone lantern", "Desk lamp"]);
    assert_eq!(
        approvals[0].detail.as_deref(),
        Some("2 models in total, about 105 credits")
    );
    assert_eq!(
        approvals[0].estimate,
        Some(105.0),
        "a prop and a rigged character"
    );
    assert!(approvals[0].items[1].detail.contains("rig"), "a character");
    // Denied once, both are dropped without spending anything.
    assert!(hub.decide(&approvals[0].id, false));
    let deadline = Instant::now() + Duration::from_secs(5);
    for job in &batch_jobs {
        let id = job["id"].as_str().expect("id");
        let denied = loop {
            let job = call(&client, "get_provider_job", json!({ "id": id })).await;
            if job["status"] == json!("cancelled") || Instant::now() > deadline {
                break job;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        assert_eq!(denied["status"], json!("cancelled"), "{denied}");
    }
    assert!(hub.approvals().is_empty());

    // A by hand batch writes one folder per asset straight away.
    let by_hand = call(
        &client,
        "send_batch_to_3d",
        json!({
            "project": project_id,
            "provider": "by-hand",
            "overrides": { "faceLimit": 10000 },
            "items": [
                { "asset": "stone-lantern" },
                { "asset": "desk-lamp", "overrides": { "faceLimit": 2000 } },
            ],
        }),
    )
    .await;
    assert_eq!(by_hand["status"], json!("started"), "{by_hand}");
    let cards: Vec<String> = by_hand["jobs"]
        .as_array()
        .expect("jobs")
        .iter()
        .map(|j| {
            let folder = std::path::PathBuf::from(j["handoff"].as_str().expect("handoff"));
            std::fs::read_to_string(folder.join("settings.md")).expect("card")
        })
        .collect();
    assert!(cards[0].contains("10,000"), "{}", cards[0]);
    assert!(
        cards[1].contains("2,000"),
        "own overrides win: {}",
        cards[1]
    );
    for job in by_hand["jobs"].as_array().expect("jobs") {
        assert_eq!(job["status"], json!("waiting"));
        let folder = std::path::PathBuf::from(job["handoff"].as_str().expect("handoff"));
        assert!(folder.join("settings.md").is_file());
    }
    assert!(hub.approvals().is_empty());

    // Object refs need Codex, which this host does not have.
    let mut params = CallToolRequestParams::new("make_object_refs".to_string());
    if let Value::Object(map) = json!({
        "project": project_id,
        "source": "refs/desk_lamp.png",
        "objects": ["desk lamp"],
    }) {
        params = params.with_arguments(map);
    }
    let no_codex = client.call_tool(params).await.expect("call");
    assert_eq!(no_codex.is_error, Some(true));
    assert!(text(&no_codex).contains("Codex CLI"), "{}", text(&no_codex));

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_lay_out_the_scene_and_copy_assets() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, project_dir) = fixture(dir.path());
    let (root, config) = session.workspace().expect("ws");
    std::fs::create_dir_all(project_dir.join("models")).expect("models");
    std::fs::write(project_dir.join("models/desk_lamp_v1.glb"), b"glb").expect("glb");
    project::modify(&root, &config.layout, &project_id, |m| {
        assets::add_version(m, "desk-lamp", "models/desk_lamp_v1.glb".into(), None).map(|_| ())
    })
    .expect("version");
    let landing = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Maquette landing".into(),
            template: ProjectTemplate::FloatingDiorama,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("landing");
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        studio: studio(&session),
        session,
        runner: PipelineRunner::new(Arc::new(HubSink(Arc::clone(&hub)))),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        pipeline_changes: std::sync::atomic::AtomicUsize::new(0),
        layout_changes: std::sync::atomic::AtomicUsize::new(0),
        materials_changes: std::sync::atomic::AtomicUsize::new(0),
        prepare_changes: std::sync::atomic::AtomicUsize::new(0),
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

    let empty = call(&client, "get_layout", json!({ "project": project_id })).await;
    assert_eq!(empty["saved"], json!(false));
    assert_eq!(empty["models"][0]["asset"], json!("desk-lamp"));
    assert_eq!(empty["firstView"]["targetMb"], json!(3.0));

    let set = call(
        &client,
        "set_layout",
        json!({
            "project": project_id,
            "models": [{ "asset": "desk-lamp", "position": [0.5, 0.0, -0.2], "rotation": [0.0, 90.0, 0.0] }],
            "stops": [{ "name": "Concept", "position": [0.0, 0.5, 1.4], "target": [0.0, 0.1, 0.0] }],
            "desk": { "width": 1.6, "depth": 0.9 },
        }),
    )
    .await;
    assert_eq!(set["saved"], json!(true));
    assert_eq!(
        set["layout"]["models"]["desk-lamp"]["position"],
        json!([0.5, 0.0, -0.2])
    );
    assert_eq!(set["layout"]["stops"][0]["id"], json!("concept"));
    assert_eq!(set["layout"]["desk"]["size"], json!([1.6, 0.9]));
    assert_eq!(
        host.layout_changes
            .load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the app hears about it"
    );

    // Only what a change names moves; the rest stays.
    let scaled = call(
        &client,
        "set_layout",
        json!({
            "project": project_id,
            "models": [{ "asset": "desk-lamp", "scale": 0.5 }],
            "stops": [{ "id": "concept", "fov": 30.0 }],
            "desk": null,
        }),
    )
    .await;
    assert_eq!(
        scaled["layout"]["models"]["desk-lamp"]["position"],
        json!([0.5, 0.0, -0.2])
    );
    assert_eq!(scaled["layout"]["models"]["desk-lamp"]["scale"], json!(0.5));
    assert_eq!(scaled["layout"]["stops"][0]["fov"], json!(30.0));
    assert_eq!(scaled["layout"]["desk"], Value::Null);

    // Object states and clip choices go into the same layout file.
    let moving = call(
        &client,
        "set_layout",
        json!({
            "project": project_id,
            "states": [{
                "asset": "desk-lamp",
                "states": [{ "name": "Down" }, { "name": "Raised", "parts": { "Arm": { "rotation": [0.0, 0.0, 40.0] } } }],
                "transitions": [{ "from": "Down", "to": "Raised", "duration": 0.8, "easing": "ease-out" }],
                "triggers": [{ "kind": "stop", "to": "Raised", "stop": "concept" }],
            }],
            "clips": [{ "asset": "desk-lamp", "clip": "Flicker", "ship": false }],
        }),
    )
    .await;
    let lamp = &moving["layout"]["states"]["desk-lamp"];
    assert_eq!(lamp["states"][1]["id"], json!("raised"));
    assert_eq!(lamp["transitions"][0]["easing"], json!("ease-out"));
    assert_eq!(lamp["triggers"][0]["stop"], json!("concept"));
    assert_eq!(
        moving["layout"]["clips"]["desk-lamp"]["Flicker"]["ship"],
        json!(false)
    );
    assert_eq!(
        moving["layout"]["models"]["desk-lamp"]["scale"],
        json!(0.5),
        "placements stay"
    );
    let motion = call(&client, "get_motion", json!({ "project": project_id })).await;
    assert_eq!(motion["models"][0]["asset"], json!("desk-lamp"));
    assert_eq!(motion["clipsOff"]["desk-lamp"], json!(["Flicker"]));
    assert_eq!(
        motion["states"]["desk-lamp"]["states"][0]["name"],
        json!("Down")
    );
    let cleared = call(
        &client,
        "set_layout",
        json!({
            "project": project_id,
            "states": [{ "asset": "desk-lamp", "remove": true }],
            "clips": [{ "asset": "desk-lamp", "clip": "Flicker", "ship": true }],
        }),
    )
    .await;
    assert_eq!(cleared["layout"]["states"], json!({}));
    assert_eq!(cleared["layout"]["clips"], json!({}));

    // An unknown asset is refused with the valid ones, and nothing changes.
    let mut params = CallToolRequestParams::new("set_layout".to_string());
    params = params.with_arguments(
        json!({ "project": project_id, "models": [{ "asset": "stone-lantern", "scale": 2.0 }] })
            .as_object()
            .expect("object")
            .clone(),
    );
    let refused = client.call_tool(params).await.expect("call");
    assert_eq!(refused.is_error, Some(true));
    assert!(text(&refused).contains("desk-lamp"), "{}", text(&refused));
    let after = call(&client, "get_layout", json!({ "project": project_id })).await;
    assert_eq!(after["layout"]["models"]["desk-lamp"]["scale"], json!(0.5));
    assert_eq!(
        host.layout_changes
            .load(std::sync::atomic::Ordering::SeqCst),
        4
    );

    let copied = call(
        &client,
        "copy_asset",
        json!({ "project": project_id, "asset": "desk-lamp", "to": landing.id }),
    )
    .await;
    assert_eq!(copied["project"], json!(landing.id));
    assert_eq!(copied["asset"]["id"], json!("desk-lamp"));
    assert_eq!(copied["asset"]["versions"], json!(1));
    assert_eq!(copied["refs"], json!(["refs/desk_lamp.png"]));
    let landing_dir = project::dir(&root, &config.layout, &landing.id).expect("dir");
    assert!(landing_dir.join("models/desk_lamp_v1.glb").is_file());
    let source = project::load(&root, &config.layout, &project_id).expect("source");
    assert!(
        source.assets.iter().any(|a| a.id == "desk-lamp"),
        "the source stays"
    );

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_tweak_materials_and_pick_codecs_per_map() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, project_dir) = fixture(dir.path());
    let (root, config) = session.workspace().expect("ws");
    std::fs::create_dir_all(project_dir.join("models")).expect("models");
    // A real GLB, so the tool can list its one material, "Crate".
    std::fs::write(
        project_dir.join("models/desk_lamp_v1.glb"),
        maquette_core::inbox::fixture::clean_prop(),
    )
    .expect("glb");
    project::modify(&root, &config.layout, &project_id, |m| {
        assets::add_version(m, "desk-lamp", "models/desk_lamp_v1.glb".into(), None).map(|_| ())
    })
    .expect("version");
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        studio: studio(&session),
        session,
        runner: PipelineRunner::new(Arc::new(HubSink(Arc::clone(&hub)))),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        pipeline_changes: std::sync::atomic::AtomicUsize::new(0),
        layout_changes: std::sync::atomic::AtomicUsize::new(0),
        materials_changes: std::sync::atomic::AtomicUsize::new(0),
        prepare_changes: std::sync::atomic::AtomicUsize::new(0),
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

    let empty = call(
        &client,
        "get_material_tweaks",
        json!({ "project": project_id, "asset": "Desk lamp" }),
    )
    .await;
    assert_eq!(empty["asset"], json!("desk-lamp"));
    assert_eq!(empty["materials"], json!(["Crate"]));
    assert_eq!(empty["tweaks"]["all"]["tint"], Value::Null);
    assert_eq!(empty["compression"]["model"], json!("desk_lamp_v1"));
    assert_eq!(empty["compression"]["codecs"]["normal"], json!("uastc"));

    let set = call(
        &client,
        "set_material_tweaks",
        json!({
            "project": project_id,
            "asset": "desk-lamp",
            "all": { "roughness": { "mode": "override", "value": 0.62 }, "metalness": 0.25 },
            "materials": { "Crate": { "tint": "#3A3936" } },
            "codecs": { "color": "uastc", "normal": "etc1s" },
        }),
    )
    .await;
    assert_eq!(set["tweaks"]["all"]["roughness"]["value"], json!(0.62));
    assert_eq!(
        set["tweaks"]["materials"]["Crate"]["tint"],
        json!("#3a3936")
    );
    assert_eq!(set["compression"]["codecs"]["color"], json!("uastc"));
    assert_eq!(set["compression"]["codecs"]["normal"], json!("etc1s"));
    let stored = std::fs::read_to_string(project_dir.join("materials.json")).expect("file");
    assert!(stored.contains("\"desk-lamp\""), "{stored}");
    let def = pipeline::load(&project_dir)
        .expect("load")
        .expect("pipeline");
    let row = def
        .presets
        .iter()
        .find(|p| p.model == "desk_lamp_v1")
        .expect("row");
    assert_eq!(
        row.map_codecs.and_then(|c| c.normal),
        Some(maquette_core::TextureCodec::Etc1s)
    );
    let order = std::sync::atomic::Ordering::SeqCst;
    assert_eq!(
        host.materials_changes.load(order),
        1,
        "the app hears about it"
    );
    assert_eq!(host.pipeline_changes.load(order), 1);

    // Only what a change names moves; null clears a field or a group.
    let cleared = call(
        &client,
        "set_material_tweaks",
        json!({
            "project": project_id,
            "asset": "desk-lamp",
            "all": { "metalness": null },
            "codecs": { "normal": null },
        }),
    )
    .await;
    assert_eq!(cleared["tweaks"]["all"]["metalness"], Value::Null);
    assert_eq!(cleared["tweaks"]["all"]["roughness"]["value"], json!(0.62));
    assert_eq!(cleared["compression"]["codecs"]["normal"], json!("uastc"));
    assert_eq!(cleared["compression"]["codecs"]["color"], json!("uastc"));

    // A material the model does not have is refused, and nothing changes.
    let mut params = CallToolRequestParams::new("set_material_tweaks".to_string());
    params = params.with_arguments(
        json!({
            "project": project_id,
            "asset": "desk-lamp",
            "materials": { "Lid": { "metalness": 1.0 } },
            "codecs": { "other": "uastc" },
        })
        .as_object()
        .expect("object")
        .clone(),
    );
    let refused = client.call_tool(params).await.expect("call");
    assert_eq!(refused.is_error, Some(true));
    assert!(text(&refused).contains("Crate"), "{}", text(&refused));
    let after = call(
        &client,
        "get_material_tweaks",
        json!({ "project": project_id, "asset": "desk-lamp" }),
    )
    .await;
    assert_eq!(after["compression"]["codecs"]["other"], json!("etc1s"));
    assert_eq!(
        after["tweaks"]["materials"]["Crate"]["tint"],
        json!("#3a3936")
    );

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_set_real_sizes_and_fixups_without_partial_writes() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, project_dir) = fixture(dir.path());
    let (root, config) = session.workspace().expect("ws");
    std::fs::create_dir_all(project_dir.join("models")).expect("models");
    std::fs::write(
        project_dir.join("models/desk_lamp_v1.glb"),
        maquette_core::inbox::fixture::messy_prop(),
    )
    .expect("glb");
    project::modify(&root, &config.layout, &project_id, |m| {
        assets::add_version(m, "desk-lamp", "models/desk_lamp_v1.glb".into(), None).map(|_| ())
    })
    .expect("version");
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        studio: studio(&session),
        session,
        runner: PipelineRunner::new(Arc::new(HubSink(Arc::clone(&hub)))),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        pipeline_changes: std::sync::atomic::AtomicUsize::new(0),
        layout_changes: std::sync::atomic::AtomicUsize::new(0),
        materials_changes: std::sync::atomic::AtomicUsize::new(0),
        prepare_changes: std::sync::atomic::AtomicUsize::new(0),
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

    let empty = call(
        &client,
        "get_real_size",
        json!({ "project": project_id, "asset": "Desk lamp" }),
    )
    .await;
    assert_eq!(empty["asset"], json!("desk-lamp"));
    assert_eq!(empty["size"], Value::Null);
    assert_eq!(empty["suggestion"]["size"]["value"], json!(45.0));
    assert!(empty["measured"][0].as_f64().is_some_and(|w| w > 1.2));

    let sized = call(
        &client,
        "set_real_size",
        json!({ "project": project_id, "asset": "desk-lamp",
                "size": { "axis": "Height", "value": 45, "unit": "cm" } }),
    )
    .await;
    assert_eq!(sized["sizeText"], json!("45 cm high"));
    assert!(sized["scale"].as_f64().is_some_and(|s| s > 0.0));

    let fixed = call(
        &client,
        "add_fixups",
        json!({ "project": project_id, "asset": "desk-lamp",
                "fixups": [{ "op": "origin-bottom" }, { "op": "decimate", "triangles": 5000 }] }),
    )
    .await;
    assert_eq!(
        fixed["fixupText"],
        json!(["Decimate to 5k", "Origin to bottom centre"]),
        "kept in the order they apply"
    );
    let stored = std::fs::read_to_string(project_dir.join("prepare.json")).expect("file");
    assert!(stored.contains("\"desk-lamp\""), "{stored}");

    // One bad fix-up refuses the whole call, and nothing changes.
    let mut params = CallToolRequestParams::new("add_fixups".to_string());
    params = params.with_arguments(
        json!({ "project": project_id, "asset": "desk-lamp",
                "fixups": [{ "op": "merge" }, { "op": "decimate", "triangles": 3 }] })
        .as_object()
        .expect("object")
        .clone(),
    );
    let refused = client.call_tool(params).await.expect("call");
    assert_eq!(refused.is_error, Some(true));
    let mut params = CallToolRequestParams::new("remove_fixups".to_string());
    params = params.with_arguments(
        json!({ "project": project_id, "asset": "desk-lamp", "ops": ["smooth"] })
            .as_object()
            .expect("object")
            .clone(),
    );
    let unknown = client.call_tool(params).await.expect("call");
    assert!(
        text(&unknown).contains("origin-bottom"),
        "{}",
        text(&unknown)
    );
    let listed = call(
        &client,
        "list_fixups",
        json!({ "project": project_id, "asset": "desk-lamp" }),
    )
    .await;
    assert_eq!(listed["fixups"].as_array().map(Vec::len), Some(2));

    let removed = call(
        &client,
        "remove_fixups",
        json!({ "project": project_id, "asset": "desk-lamp", "ops": ["decimate"] }),
    )
    .await;
    assert_eq!(removed["fixupText"], json!(["Origin to bottom centre"]));
    let cleared = call(
        &client,
        "set_real_size",
        json!({ "project": project_id, "asset": "desk-lamp", "size": null }),
    )
    .await;
    assert_eq!(cleared["size"], Value::Null);
    let order = std::sync::atomic::Ordering::SeqCst;
    assert_eq!(
        host.prepare_changes.load(order),
        4,
        "the app hears each save"
    );

    client.cancel().await.expect("close");
    server.stop();
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
        pipeline_changes: std::sync::atomic::AtomicUsize::new(0),
        layout_changes: std::sync::atomic::AtomicUsize::new(0),
        materials_changes: std::sync::atomic::AtomicUsize::new(0),
        prepare_changes: std::sync::atomic::AtomicUsize::new(0),
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_redo_inbox_models_and_read_the_credits() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, _) = fixture(dir.path());
    let (root, config) = session.workspace().expect("ws");
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        studio: studio(&session),
        session,
        runner: PipelineRunner::new(Arc::new(HubSink(Arc::clone(&hub)))),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        pipeline_changes: std::sync::atomic::AtomicUsize::new(0),
        layout_changes: std::sync::atomic::AtomicUsize::new(0),
        materials_changes: std::sync::atomic::AtomicUsize::new(0),
        prepare_changes: std::sync::atomic::AtomicUsize::new(0),
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

    // Two by hand sends whose models came back into the inbox.
    let inbox = root.join(&config.layout.inbox);
    std::fs::create_dir_all(&inbox).expect("inbox");
    for asset in ["stone-lantern", "desk-lamp"] {
        let sent = call(
            &client,
            "send_to_3d",
            json!({
                "project": project_id,
                "asset": asset,
                "provider": "by-hand",
                "overrides": { "faceLimit": 12000 },
            }),
        )
        .await;
        let id = sent["id"].as_str().expect("id");
        std::fs::write(inbox.join(format!("{id}.glb")), b"glTF not really").expect("model");
    }

    // A model that came from no send has nothing to repeat.
    std::fs::write(inbox.join("loose.glb"), b"glTF").expect("loose");
    let mut params = CallToolRequestParams::new("redo_inbox_model".to_string());
    if let Value::Object(map) = json!({ "id": "loose.glb" }) {
        params = params.with_arguments(map);
    }
    let refused = client.call_tool(params).await.expect("call");
    assert_eq!(refused.is_error, Some(true));
    assert!(
        text(&refused).contains("did not come from a send"),
        "{}",
        text(&refused)
    );

    // A paid redo keeps the earlier settings, lays the overrides on top and
    // waits for approval with its expected cost.
    let redo = call(
        &client,
        "redo_inbox_model",
        json!({
            "id": "stone-lantern-1.glb",
            "provider": "tripo",
            "overrides": { "pbr": false },
        }),
    )
    .await;
    assert_eq!(redo["status"], json!("waiting for approval"), "{redo}");
    assert_eq!(redo["redoes"], json!("stone-lantern-1"));
    assert_eq!(redo["redoOf"], json!("stone-lantern-1"));
    assert_eq!(redo["id"], json!("stone-lantern-2"));
    assert_eq!(redo["settings"], json!("Tripo H3.1, 1 image, 12,000 faces"));
    let job = host.studio.get("stone-lantern-2").expect("job");
    assert_eq!(job.tripo.as_ref().map(|t| t.pbr), Some(false));
    let approvals = hub.approvals();
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].estimate, Some(30.0));
    assert!(hub.decide(&approvals[0].id, false));

    // By hand by default: a new handoff folder, the old model untouched.
    let deadline = Instant::now() + Duration::from_secs(5);
    while host.studio.get("stone-lantern-2").map(|j| j.state) == Some(JobState::Approval) {
        assert!(Instant::now() < deadline, "the denial never landed");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let many = call(
        &client,
        "redo_inbox_models",
        json!({ "ids": ["stone-lantern-1.glb", "desk-lamp-1.glb"] }),
    )
    .await;
    assert_eq!(many["status"], json!("started"), "{many}");
    let jobs = many["jobs"].as_array().expect("jobs");
    assert_eq!(jobs.len(), 2);
    for job in jobs {
        assert_eq!(job["status"], json!("waiting"));
        assert!(job["redoOf"].as_str().is_some_and(|r| r.ends_with("-1")));
        let folder = std::path::PathBuf::from(job["handoff"].as_str().expect("handoff"));
        let card = std::fs::read_to_string(folder.join("settings.md")).expect("card");
        assert!(card.contains("12,000"), "the earlier settings: {card}");
    }
    assert!(
        inbox.join("stone-lantern-1.glb").is_file(),
        "kept to compare"
    );

    // Nothing was paid for through an API: the ledger holds only the By
    // hand sends, as subscription spend. Tripo cannot be reached here and
    // Meshy has no key.
    let credits = call(&client, "get_credits", json!({})).await;
    let spends = credits["spends"].as_array().expect("spends");
    assert!(!spends.is_empty(), "{credits}");
    assert!(
        spends.iter().all(|s| s["source"] == json!("subscription")),
        "{credits}"
    );
    assert_eq!(credits["totals"].as_array().map(Vec::len), Some(6));
    assert_eq!(credits["totals"][2]["total"], json!(0.0), "no API spend");

    // The waiting hand-offs, with the name each waits for. This host runs
    // no Downloads watcher.
    let handoffs = call(&client, "list_handoffs", json!({})).await;
    let pending = handoffs["pending"].as_array().expect("pending");
    assert!(!pending.is_empty(), "{handoffs}");
    assert!(
        pending.iter().all(|h| h["expectedName"]
            .as_str()
            .is_some_and(|n| n.starts_with("maq_"))),
        "{handoffs}"
    );
    assert_eq!(handoffs["watcher"], Value::Null);
    assert_eq!(credits["balances"][0]["service"], json!("tripo"));
    assert!(credits["balances"][0]["error"].is_string(), "{credits}");
    assert_eq!(credits["balances"][1]["key"], json!(false));

    client.cancel().await.expect("close");
    server.stop();
}
