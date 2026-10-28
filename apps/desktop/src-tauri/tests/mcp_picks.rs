//! A real MCP client stars and picks Lab concepts onto the Board, then
//! sends a picked object By hand with the refs the pick gave it. Nothing
//! reaches Codex, Tripo or Meshy.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use maquette_core::concepts::objects::{self, NewObjectRefs};
use maquette_core::concepts::{self, Aspect, Concepts, Direction, NewConcepts, StyleBoard};
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{Endpoints, MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{
    AppState, AssetKind, NewProject, ProjectManifest, ProjectTemplate, WorkspaceConfig, project,
    workspace,
};
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::Hub;
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
    /// How often a tool told the windows that a project changed.
    project_changes: AtomicUsize,
    /// How often a tool told the windows that a direction changed.
    concept_changes: AtomicUsize,
    /// How often a tool told the windows that the style board changed.
    style_changes: AtomicUsize,
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
    fn project_changed(&self, _project: &ProjectManifest) {
        self.project_changes.fetch_add(1, Ordering::SeqCst);
    }
    fn concepts_changed(&self, _project: &str, _direction: &Direction) {
        self.concept_changes.fetch_add(1, Ordering::SeqCst);
    }
    fn style_changed(&self, _project: &str, _style: &StyleBoard) {
        self.style_changes.fetch_add(1, Ordering::SeqCst);
    }
}

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

/// A workspace with one project, a direction "Stone lantern" of two
/// concepts and a direction "Workbench objects" of two object refs.
fn fixture(dir: &Path) -> (Session, String, std::path::PathBuf) {
    let root = dir.join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
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
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    let brief = NewConcepts {
        project: manifest.id.clone(),
        direction: "Stone lantern".into(),
        prompt: "A stone lantern".into(),
        refs: Vec::new(),
        aspect: Aspect::Square,
        count: 2,
        use_style: true,
    };
    let (_, files) = concepts::prepare(&project_dir, &brief).expect("prep");
    for (index, file) in &files {
        std::fs::write(project_dir.join(file), PNG).expect("png");
        concepts::add_image(&project_dir, "stone-lantern", *index, file, None).expect("add");
    }
    let request = NewObjectRefs {
        project: manifest.id.clone(),
        source: files[0].1.clone(),
        source_name: "Workbench".into(),
        objects: vec!["desk lamp".into(), "brush cup".into()],
        notes: HashMap::new(),
        aspect: Aspect::Square,
        kind: AssetKind::Prop,
        use_style: true,
    };
    let (direction, planned) = objects::prepare(&project_dir, &request).expect("objects");
    for p in &planned {
        std::fs::write(project_dir.join(&p.file), PNG).expect("png");
        concepts::add_image(
            &project_dir,
            &direction.id,
            p.index,
            &p.file,
            Some(&p.object),
        )
        .expect("add");
    }
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

type Client = RunningService<RoleClient, ClientConfig>;

/// The MCP server over a host for `session`, and a client signed in to it.
async fn connect(
    session: Session,
) -> (
    Arc<TestHost>,
    Client,
    maquette_desktop_lib::mcp::server::Running,
) {
    // Provider jobs that can never reach a service; By hand spends nothing.
    let options = Options {
        endpoints: Endpoints {
            tripo: "http://127.0.0.1:9".into(),
            meshy: "http://127.0.0.1:9".into(),
        },
        ..Options::default()
    };
    let studio = ProviderJobs::new(Arc::new(MemoryStore::default()), options, |_| {});
    if let Ok((root, config)) = session.workspace() {
        studio.open_workspace(&root, &config);
    }
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(maquette_core::pipeline::runner::NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio,
        project_changes: AtomicUsize::new(0),
        concept_changes: AtomicUsize::new(0),
        style_changes: AtomicUsize::new(0),
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_star_and_pick_concepts_onto_the_board() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, project_dir) = fixture(dir.path());
    let (host, client, server) = connect(session).await;
    let info = client.peer_info().expect("server info");
    assert!(
        info.instructions
            .as_deref()
            .is_some_and(|i| i.contains("pick_concepts") && i.contains("star_concepts"))
    );
    let tools = client.list_all_tools().await.expect("tools");
    for name in ["pick_concepts", "star_concepts"] {
        let tool = tools.iter().find(|t| t.name == name).expect(name);
        let hints = tool.annotations.as_ref().expect("annotations");
        assert_eq!(hints.idempotent_hint, Some(true), "{name}");
        assert_eq!(hints.read_only_hint, None, "{name} writes");
    }

    // Star a shortlist by direction and index.
    let starred = call(
        &client,
        "star_concepts",
        json!({ "project": project_id, "images": [{ "direction": "Stone lantern", "index": 2, "starred": true }] }),
    )
    .await;
    assert_eq!(
        starred["directions"][0]["starred"],
        json!(["concepts/stone-lantern/stone-lantern_02.png"])
    );
    assert_eq!(host.concept_changes.load(Ordering::SeqCst), 1);
    assert_eq!(host.project_changes.load(Ordering::SeqCst), 0);

    // Pick one concept by file and one object ref by number.
    let picked = call(
        &client,
        "pick_concepts",
        json!({
            "project": project_id,
            "images": [
                { "file": "concepts/stone-lantern/stone-lantern_02.png" },
                { "direction": "workbench-objects", "index": 1, "kind": "character", "star": true },
            ],
        }),
    )
    .await;
    assert_eq!(
        picked["images"][0]["refFile"],
        json!("refs/stone-lantern_02.png")
    );
    assert_eq!(picked["images"][0]["copied"], json!(true));
    assert_eq!(picked["assets"][0]["id"], json!("stone-lantern"));
    assert_eq!(picked["assets"][0]["stage"], json!("refs"));
    assert_eq!(picked["assets"][1]["name"], json!("Desk lamp"));
    assert_eq!(picked["assets"][1]["kind"], json!("character"));
    assert_eq!(picked["assets"][1]["refs"], json!(["refs/desk_lamp.png"]));
    assert!(project_dir.join("refs/desk_lamp.png").is_file());
    assert_eq!(host.project_changes.load(Ordering::SeqCst), 1);
    assert_eq!(host.concept_changes.load(Ordering::SeqCst), 3);

    // The Lab records the refs, and the picks are cleared as after a send.
    let lab = call(
        &client,
        "get_concepts",
        json!({ "project": project_id, "direction": "Workbench objects" }),
    )
    .await;
    let image = &lab["directions"][0]["images"][0];
    assert_eq!(image["refFile"], json!("refs/desk_lamp.png"));
    assert_eq!(image["starred"], json!(true));
    assert_eq!(image["picked"], json!(false));

    // Again: the same assets, no new copies.
    let again = call(
        &client,
        "pick_concepts",
        json!({ "project": project_id, "images": [{ "file": "concepts/workbench-objects/workbench-objects_01.png" }] }),
    )
    .await;
    assert_eq!(again["images"][0]["copied"], json!(false));
    assert_eq!(again["assets"][0]["id"], json!("desk-lamp"));
    let listed = call(&client, "list_assets", json!({ "project": project_id })).await;
    let text_listed = listed.to_string();
    assert_eq!(text_listed.matches("\"desk-lamp\"").count(), 1, "{listed}");

    // A path outside the project is refused and nothing changes.
    let before = host.project_changes.load(Ordering::SeqCst);
    for bad in ["../../outside.png", "refs/desk_lamp.png", "/etc/hosts"] {
        let refused = raw_call(
            &client,
            "pick_concepts",
            json!({ "project": project_id, "images": [{ "file": bad }] }),
        )
        .await;
        assert_eq!(refused.is_error, Some(true), "{bad}");
    }
    let unnamed = raw_call(
        &client,
        "pick_concepts",
        json!({ "project": project_id, "images": [{ "direction": "Stone lantern" }] }),
    )
    .await;
    assert_eq!(unnamed.is_error, Some(true));
    assert!(text(&unnamed).contains("direction and an index"));
    assert_eq!(host.project_changes.load(Ordering::SeqCst), before);

    // The picked object goes By hand with the ref the pick gave it.
    let sent = call(
        &client,
        "send_to_3d",
        json!({ "project": project_id, "asset": "desk-lamp", "provider": "by-hand" }),
    )
    .await;
    assert_eq!(sent["images"], json!(["refs/desk_lamp.png"]), "{sent}");

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_set_the_style_board_from_project_files_only() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, project_dir) = fixture(dir.path());
    // The PNG signature and header are all the board's image check reads.
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&[0, 0, 0, 4, 0, 0, 0, 4, 8, 2, 0, 0, 0]);
    std::fs::create_dir_all(project_dir.join("refs")).expect("refs");
    std::fs::write(project_dir.join("refs/frost_glass.png"), &png).expect("ref");
    let outside = dir.path().join("elsewhere.png");
    std::fs::write(&outside, &png).expect("outside");
    let (host, client, server) = connect(session).await;

    let empty = call(&client, "get_style_board", json!({ "project": project_id })).await;
    assert_eq!(empty["empty"], json!(true));
    assert_eq!(empty["board"]["revision"], json!(0));

    let set = call(
        &client,
        "set_style_board",
        json!({
            "project": project_id,
            "line": "frosted glass, cool night light",
            "palette": ["#9FC3FF", "1b2440"],
            "images": ["refs/frost_glass.png"],
        }),
    )
    .await;
    assert_eq!(set["board"]["revision"], json!(1), "{set}");
    assert_eq!(set["board"]["palette"], json!(["#9fc3ff", "#1b2440"]));
    assert_eq!(
        set["board"]["images"][0]["file"],
        json!("style/frost_glass.png")
    );
    assert!(project_dir.join("style/frost_glass.png").is_file());
    assert_eq!(host.style_changes.load(Ordering::SeqCst), 1);

    for bad in [
        json!(["../elsewhere.png"]),
        json!([outside.display().to_string()]),
    ] {
        let refused = raw_call(
            &client,
            "set_style_board",
            json!({ "project": project_id, "images": bad }),
        )
        .await;
        assert_eq!(refused.is_error, Some(true), "{bad}");
    }
    let read = call(&client, "get_style_board", json!({ "project": project_id })).await;
    assert_eq!(read["board"]["revision"], json!(1));
    assert_eq!(host.style_changes.load(Ordering::SeqCst), 1);

    // Starting without the board is accepted; it fails only for want of Codex.
    let no_codex = raw_call(
        &client,
        "start_concepts",
        json!({
            "project": project_id,
            "direction": "Frozen lake",
            "prompt": "A frozen lake at night",
            "useStyle": false,
        }),
    )
    .await;
    assert_eq!(no_codex.is_error, Some(true));
    assert!(text(&no_codex).contains("Codex CLI"), "{}", text(&no_codex));

    client.cancel().await.expect("close");
    server.stop();
}
