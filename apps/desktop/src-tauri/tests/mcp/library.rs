//! A real MCP client adds a model to the library, uses it in a second
//! project, adds a newer model and updates the copy. Everything happens in
//! a temp workspace; nothing reaches Codex, Tripo, Meshy or Blender.

use std::path::Path;
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{
    AppState, AssetKind, ModelStats, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig,
    assets, project, workspace,
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

fn stats(triangles: u32) -> ModelStats {
    ModelStats {
        triangles,
        bytes: 2048.0,
        textures: 0,
        max_texture: 0,
        joints: 0,
        animations: 0,
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
        assets::add_version(
            m,
            &lamp.id,
            "models/desk_lamp_v1.glb".into(),
            Some(stats(9000)),
        )?;
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
async fn agents_add_use_and_update_a_library_model() {
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

    let empty = call(&client, "list_library", json!({})).await;
    assert_eq!(empty["entries"], json!([]));

    let added = call(
        &client,
        "add_to_library",
        json!({ "project": "moon-island", "asset": "Desk lamp", "tags": ["Props", "desk"] }),
    )
    .await;
    assert_eq!(added["entry"], json!("desk-lamp"));
    assert_eq!(added["version"], json!(1));
    assert_eq!(added["tags"], json!(["props", "desk"]));
    assert!(
        root.join("library/desk-lamp/models/desk_lamp_v1.glb")
            .is_file()
    );

    let used = call(
        &client,
        "use_from_library",
        json!({ "entry": "desk-lamp", "project": "study-room" }),
    )
    .await;
    assert_eq!(used["asset"]["id"], json!("desk-lamp"));
    assert_eq!(
        used["origin"],
        json!({ "entry": "desk-lamp", "version": 1 })
    );

    // A lighter model arrives in Moon Island and goes to the library.
    let moon = root.join("projects/moon-island");
    std::fs::write(moon.join("models/desk_lamp_v2.glb"), "lamp 2").expect("v2");
    project::modify(&root, &Default::default(), "moon-island", |m| {
        assets::add_version(
            m,
            "desk-lamp",
            "models/desk_lamp_v2.glb".into(),
            Some(stats(4000)),
        )
        .map(|_| ())
    })
    .expect("v2");
    let newer = call(
        &client,
        "add_to_library",
        json!({ "project": "moon-island", "asset": "desk-lamp" }),
    )
    .await;
    assert_eq!(newer["version"], json!(2));
    assert_eq!(newer["added"], json!(true));

    let listed = call(
        &client,
        "list_library",
        json!({ "tag": "desk", "project": "study-room" }),
    )
    .await;
    assert_eq!(listed["entries"][0]["latest"], json!(2));
    assert_eq!(
        listed["updates"],
        json!([{ "asset": "desk-lamp", "entry": "desk-lamp", "on": 1, "newest": 2 }])
    );

    let plan = call(
        &client,
        "update_from_library",
        json!({ "project": "study-room", "asset": "desk-lamp", "dryRun": true }),
    )
    .await;
    assert_eq!(plan["plan"]["becomesVersion"], json!(2));
    let room = root.join("projects/study-room");
    assert!(
        !room.join("models/desk_lamp_v2.glb").exists(),
        "a dry run copies nothing"
    );

    let updated = call(
        &client,
        "update_from_library",
        json!({ "project": "study-room", "asset": "desk-lamp" }),
    )
    .await;
    assert_eq!(updated["updated"], json!(true));
    assert_eq!(
        std::fs::read_to_string(room.join("models/desk_lamp_v2.glb")).expect("copied"),
        "lamp 2"
    );
    let after = call(&client, "list_library", json!({ "project": "study-room" })).await;
    assert_eq!(after["updates"], json!([]));

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_library_pages_and_is_compact_by_default() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let session = fixture(dir.path());
    let root = dir.path().join("ws");
    let moon = root.join("projects/moon-island");
    project::modify(&root, &Default::default(), "moon-island", |m| {
        for name in ["Brass key", "Clay cup", "Dusty book"] {
            let id = assets::add(
                m,
                NewAsset {
                    name: name.into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Blender,
                },
            )?
            .id;
            let file = format!("models/{id}_v1.glb");
            std::fs::write(moon.join(&file), name).expect("model");
            assets::add_version(m, &id, file, Some(stats(1000)))?;
        }
        Ok(())
    })
    .expect("assets");
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
    for asset in ["Desk lamp", "Brass key", "Clay cup", "Dusty book"] {
        call(
            &client,
            "add_to_library",
            json!({ "project": "moon-island", "asset": asset, "tags": ["Props"] }),
        )
        .await;
    }

    let first = call(&client, "list_library", json!({ "limit": 3 })).await;
    assert_eq!(first["total"], json!(4));
    assert_eq!(first["entries"].as_array().expect("entries").len(), 3);
    let cursor = first["nextCursor"]
        .as_str()
        .expect("more pages")
        .to_string();
    let compact = &first["entries"][0];
    assert_eq!(compact["versions"], json!(1));
    assert_eq!(compact["uses"], json!(1), "the project it was added from");
    assert_eq!(compact["latest"], json!(1));
    for long in ["usedIn", "bytes", "triangles", "creditsSpent", "source"] {
        assert!(compact.get(long).is_none(), "{long} is for full");
    }

    let second = call(
        &client,
        "list_library",
        json!({ "limit": 3, "cursor": cursor }),
    )
    .await;
    assert_eq!(second["entries"].as_array().expect("entries").len(), 1);
    assert_eq!(second["nextCursor"], Value::Null);
    let mut ids: Vec<&str> = first["entries"]
        .as_array()
        .into_iter()
        .chain(second["entries"].as_array())
        .flatten()
        .map(|e| e["id"].as_str().expect("id"))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 4, "every entry once");

    // limit is capped, a filter narrows total, full brings the long form back.
    let capped = call(&client, "list_library", json!({ "limit": 5000 })).await;
    assert_eq!(capped["entries"].as_array().expect("entries").len(), 4);
    let one = call(
        &client,
        "list_library",
        json!({ "query": "clay", "full": true }),
    )
    .await;
    assert_eq!(one["total"], json!(1));
    assert_eq!(one["entries"][0]["triangles"], json!(1000));
    assert!(one["entries"][0]["usedIn"].is_array());

    let mut params = CallToolRequestParams::new("list_library".to_string());
    params = params.with_arguments(
        json!({ "cursor": "gone" })
            .as_object()
            .cloned()
            .expect("object"),
    );
    let stale = client.call_tool(params).await.expect("call");
    assert_eq!(stale.is_error, Some(true));

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_many_to_library_reports_each_asset_and_goes_on_after_a_failure() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let session = fixture(dir.path());
    let root = dir.path().join("ws");
    let moon = root.join("projects/moon-island");
    project::modify(&root, &Default::default(), "moon-island", |m| {
        let id = assets::add(
            m,
            NewAsset {
                name: "Brass key".into(),
                kind: AssetKind::Prop,
                stage: Stage::Blender,
            },
        )?
        .id;
        let file = format!("models/{id}_v1.glb");
        std::fs::write(moon.join(&file), "key").expect("model");
        assets::add_version(m, &id, file, Some(stats(1000)))?;
        Ok(())
    })
    .expect("assets");
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

    let done = call(
        &client,
        "add_many_to_library",
        json!({
            "project": "moon-island",
            "tags": ["Props", "batch"],
            "assets": [
                { "asset": "Desk lamp" },
                { "asset": "No such asset" },
                { "asset": "Brass key", "tags": ["keys"] },
            ],
        }),
    )
    .await;
    assert_eq!(done["added"], json!(2));
    assert_eq!(done["failed"], json!(1));
    let results = done["results"].as_array().expect("results");
    assert_eq!(results[0]["ok"], json!(true));
    assert_eq!(results[0]["tags"], json!(["props", "batch"]));
    assert_eq!(results[1]["ok"], json!(false));
    assert!(results[1]["error"].is_string());
    assert_eq!(
        results[2]["ok"],
        json!(true),
        "a failure does not stop the rest"
    );
    assert_eq!(results[2]["tags"], json!(["keys"]));
    let listed = call(&client, "list_library", json!({})).await;
    assert_eq!(listed["total"], json!(2));

    client.cancel().await.expect("close");
    server.stop();
}
