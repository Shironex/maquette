//! A real MCP client copies an asset to a second project, removes the
//! original into the workspace trash, lists it and restores it. Everything
//! happens in a temp workspace; nothing reaches Codex, Tripo, Meshy or
//! Blender.

use std::path::Path;
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{
    AppState, AssetKind, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, assets,
    project, workspace,
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

/// A workspace with Moon Island (a desk lamp, v1) and an empty Study room.
fn fixture(dir: &Path) -> Session {
    let root = dir.join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let session = Session::with_state(dir.join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    for name in ["Moon Island", "Study room"] {
        project::create(
            &root,
            &config.layout,
            NewProject {
                name: name.into(),
                template: ProjectTemplate::FloatingDiorama,
                target: Default::default(),
                game: None,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
    }
    let moon = project::dir(&root, &config.layout, "moon-island").expect("dir");
    std::fs::create_dir_all(moon.join("models")).expect("models");
    std::fs::write(moon.join("models/desk_lamp_v1.glb"), "lamp 1").expect("v1");
    project::modify(&root, &config.layout, "moon-island", |m| {
        let lamp = assets::add(
            m,
            NewAsset {
                name: "Desk lamp".into(),
                kind: AssetKind::Prop,
                stage: Stage::Blender,
            },
        )?;
        assets::add_version(m, &lamp.id, "models/desk_lamp_v1.glb".into(), None)?;
        Ok(())
    })
    .expect("lamp");
    session
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_remove_copied_assets_and_restore_one() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let session = fixture(dir.path());
    let root = dir.path().join("ws");
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(maquette_core::pipeline::runner::NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio: ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {}),
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

    call(
        &client,
        "copy_asset",
        json!({ "project": "moon-island", "asset": "desk-lamp", "to": "study-room" }),
    )
    .await;
    call(
        &client,
        "set_layout",
        json!({ "project": "moon-island", "models": [{ "asset": "desk-lamp", "scale": 0.5 }] }),
    )
    .await;

    let removed = call(
        &client,
        "remove_assets",
        json!({ "project": "moon-island", "assets": ["desk-lamp", "nope"] }),
    )
    .await;
    assert_eq!(removed["left"], json!(0));
    let lamp = &removed["assets"][0];
    assert_eq!(lamp["state"], json!("removed"), "{lamp}");
    assert_eq!(lamp["files"], json!(1));
    assert_eq!(removed["assets"][1]["state"], json!("not-found"));
    let trash_id = lamp["trash"].as_str().expect("trash id").to_string();
    let trash_path = lamp["trashPath"].as_str().expect("trash path");
    assert!(
        root.join(trash_path)
            .join("files/models/desk_lamp_v1.glb")
            .is_file()
    );
    let moon = root.join("projects/moon-island");
    assert!(!moon.join("models/desk_lamp_v1.glb").exists());
    assert!(
        root.join("projects/study-room/models/desk_lamp_v1.glb")
            .is_file(),
        "the copy stays"
    );
    let layout = call(&client, "get_layout", json!({ "project": "moon-island" })).await;
    assert_eq!(layout["layout"]["models"], json!({}));

    // Removing it again changes nothing and still answers.
    let again = call(
        &client,
        "remove_assets",
        json!({ "project": "moon-island", "assets": ["desk-lamp"] }),
    )
    .await;
    assert_eq!(again["assets"][0]["state"], json!("not-found"));

    let listed = call(
        &client,
        "list_removed_assets",
        json!({ "project": "moon-island" }),
    )
    .await;
    assert_eq!(listed["removed"][0]["id"], json!(trash_id));
    assert_eq!(listed["removed"][0]["name"], json!("Desk lamp"));

    let restored = call(
        &client,
        "restore_asset",
        json!({ "project": "moon-island", "trash": trash_id }),
    )
    .await;
    assert_eq!(restored["asset"]["id"], json!("desk-lamp"));
    assert_eq!(restored["asset"]["versions"], json!(1));
    assert_eq!(
        std::fs::read_to_string(moon.join("models/desk_lamp_v1.glb")).expect("back"),
        "lamp 1"
    );
    let layout = call(&client, "get_layout", json!({ "project": "moon-island" })).await;
    assert_eq!(layout["layout"]["models"]["desk-lamp"]["scale"], json!(0.5));

    // Too many at once is refused whole.
    let many: Vec<String> = (0..51).map(|n| format!("a-{n}")).collect();
    let mut params = CallToolRequestParams::new("remove_assets".to_string());
    params = params.with_arguments(
        json!({ "project": "moon-island", "assets": many })
            .as_object()
            .expect("object")
            .clone(),
    );
    let refused = client.call_tool(params).await.expect("call");
    assert_eq!(refused.is_error, Some(true));
    assert!(text(&refused).contains("at most 50"), "{}", text(&refused));

    client.cancel().await.expect("close");
    server.stop();
}
