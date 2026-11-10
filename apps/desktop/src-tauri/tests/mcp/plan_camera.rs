//! A real MCP client works on a website project's camera plan: reads it,
//! proposes a tour as its own line, edits one stop's settings, reads what
//! Apply would write (a moved stop shows), finds no tool that applies, and
//! is refused undoing the person's Apply. A game project has no camera
//! plan. Everything happens in a temp workspace.

use std::path::Path;
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::game::{GameFormat, NewGame, ProjectTarget};
use maquette_core::layout::{self, LayoutStop};
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::plans::camera::{self, CAMERA_PLAN_ID};
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{AppState, NewProject, ProjectTemplate, WorkspaceConfig, project, workspace};
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::Hub;
use maquette_desktop_lib::session::Session;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::json;

use super::support::{TestHost, call, call_error};

fn stop(id: &str, name: &str, x: f32) -> LayoutStop {
    LayoutStop {
        id: id.into(),
        name: name.into(),
        position: [x, 1.6, 6.0],
        target: [0.0, 1.0, 0.0],
        fov: 35.0,
        models: None,
    }
}

pub(super) fn fixture(dir: &Path) -> Session {
    let root = dir.join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let session = Session::with_state(dir.join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    let repo = dir.join("game-repo");
    std::fs::create_dir_all(&repo).expect("repo");
    for (name, game) in [
        ("Moon Island", None),
        (
            "Frost Game",
            Some(NewGame {
                format: GameFormat::Glb,
                models_dir: "assets/models".into(),
                template: None,
            }),
        ),
    ] {
        let is_game = game.is_some();
        project::create(
            &root,
            &config.layout,
            NewProject {
                name: name.into(),
                template: if is_game {
                    ProjectTemplate::GameImport
                } else {
                    ProjectTemplate::FloatingDiorama
                },
                target: if is_game {
                    ProjectTarget::Game
                } else {
                    ProjectTarget::Website
                },
                game,
                site_repo: is_game.then(|| repo.display().to_string()),
                source_folder: None,
            },
        )
        .expect("project");
    }
    let moon = root.join("projects/moon-island");
    layout::update(&moon, |l| {
        let mut next = l.clone();
        next.stops = vec![
            stop("porch", "Porch", 0.0),
            stop("bridge", "Bridge", 1.0),
            stop("moon", "Moon", 2.0),
        ];
        Ok(next)
    })
    .expect("layout");
    session
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_proposes_a_tour_and_only_the_person_applies() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let session = fixture(dir.path());
    let moon = dir.path().join("ws/projects/moon-island");
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(maquette_core::pipeline::runner::NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio: ProviderJobs::new(
            Arc::new(MemoryStore::default()),
            Options::default(),
            maquette_core::inbox::send_ports(),
            |_| {},
        ),
    });
    let token = ensure_token(&host.session).expect("token");
    let server = maquette_desktop_lib::mcp::server::start(
        0,
        token.clone(),
        host.clone() as Arc<dyn Host>,
        Arc::new(Hub::new(|| {})),
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
    let agent = "Camera agent";
    let moon_args = json!({ "project": "moon-island" });

    // No tool applies a plan: of the two named for Apply one reads, and the
    // other lays a library grip on an animation recipe.
    let tools = client.list_all_tools().await.expect("tools");
    let mut apply_tools: Vec<_> = tools
        .iter()
        .filter(|t| t.name.contains("apply"))
        .map(|t| t.name.to_string())
        .collect();
    apply_tools.sort();
    assert_eq!(apply_tools, ["apply_grip", "get_apply_preview"]);

    let read = call(&client, "get_camera_plan", moon_args.clone()).await;
    assert_eq!(read["canvas"], json!(CAMERA_PLAN_ID));
    assert_eq!(read["main"].as_array().map(Vec::len), Some(3));
    assert_eq!(read["main"][0]["id"], json!("porch"));
    assert!(read.get("stale").is_none());
    // A read never makes the plan: it answers layout.json's stops with a note.
    assert!(read["note"].is_string());
    assert!(!moon.join("plans/camera-plan.plan.json").exists());
    assert!(
        read["apply"]
            .as_str()
            .is_some_and(|s| s.contains("Only the person"))
    );

    let proposed = call(
        &client,
        "propose_camera_plan",
        json!({
            "project": "moon-island", "agent": agent,
            "stops": [
                { "id": "porch" }, { "id": "moon" }, { "id": "bridge", "fov": 28 },
                { "id": "lantern", "name": "Lantern", "position": [3, 1, 2], "target": [0, 1, 0] },
            ],
        }),
    )
    .await;
    assert_eq!(proposed["nodes"].as_array().map(Vec::len), Some(4));
    assert!(proposed["change"].is_string());
    assert_eq!(
        proposed["vsLayout"]["counts"],
        json!("1 stop added · 1 moved · 0 removed · 1 changed")
    );
    let moved = proposed["vsLayout"]["stops"]
        .as_array()
        .expect("stops")
        .iter()
        .any(|s| s["moved"] == json!(true) && s["from"].is_number() && s["to"].is_number());
    assert!(moved, "{proposed}");
    // layout.json and the main line stay as they were.
    let stops: Vec<String> = layout::load(&moon)
        .expect("layout")
        .stops
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(stops, ["porch", "bridge", "moon"]);
    let read = call(&client, "get_camera_plan", moon_args.clone()).await;
    assert_eq!(read["main"][1]["id"], json!("bridge"));
    assert_eq!(read["lines"][0]["agent"], json!(agent));

    // One stop's settings, on the person's main line, through update_nodes.
    let moon_node = read["main"][2]["node"].clone();
    call(
        &client,
        "update_nodes",
        json!({
            "project": "moon-island", "canvas": CAMERA_PLAN_ID, "agent": agent,
            "nodes": [{ "id": moon_node, "stop": { "fov": 50 } }],
        }),
    )
    .await;
    let preview = call(&client, "get_apply_preview", moon_args.clone()).await;
    assert_eq!(
        preview["counts"],
        json!("0 stops added · 0 moved · 0 removed · 1 changed")
    );
    assert_eq!(preview["stops"][0]["changed"], json!(["fov"]));

    // The person applies in the app.
    let main = camera::review(&moon, CAMERA_PLAN_ID).expect("review");
    let (_, entry) =
        camera::apply(&moon, CAMERA_PLAN_ID, &main.layout_key, &main.plan_key).expect("apply");
    assert_eq!(layout::load(&moon).expect("layout").stops[2].fov, 50.0);

    let refused = call_error(
        &client,
        "undo_run",
        json!({ "project": "moon-island", "canvas": CAMERA_PLAN_ID, "agent": agent, "changes": [entry.id] }),
    )
    .await;
    assert!(
        refused.contains("Only the person undoes an Apply"),
        "{refused}"
    );
    assert_eq!(layout::load(&moon).expect("layout").stops[2].fov, 50.0);

    let game = call_error(
        &client,
        "get_camera_plan",
        json!({ "project": "frost-game" }),
    )
    .await;
    assert_eq!(game, "Only a website project has a camera plan.");

    client.cancel().await.expect("close");
    server.stop();
}
