//! A real MCP client lists the animation library's clips and is refused an
//! import from outside the workspace, clips from a model with no rig and
//! library clips onto one, and reads a model's empty retarget review.
//! Everything happens in a temp workspace; no Blender runs.

use std::path::Path;
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
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
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::support::{TestHost, call, call_error};

/// A workspace with Frostveil, whose dummy has a model file but no rig, and
/// one library clip written the way an import leaves it.
fn fixture(dir: &Path) -> Session {
    let root = dir.join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let session = Session::with_state(dir.join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Frostveil".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let frostveil = project::dir(&root, &config.layout, "frostveil").expect("dir");
    std::fs::create_dir_all(frostveil.join("models")).expect("models");
    std::fs::write(frostveil.join("models/dummy_v1.glb"), "not a model").expect("v1");
    project::modify(&root, &config.layout, "frostveil", |m| {
        let dummy = assets::add(
            m,
            NewAsset {
                name: "Dummy".into(),
                kind: AssetKind::Character,
                stage: Stage::Blender,
            },
        )?;
        assets::add_version(m, &dummy.id, "models/dummy_v1.glb".into(), None)?;
        Ok(())
    })
    .expect("dummy");
    let clip = root.join("library/animations/standing-dodge-left");
    std::fs::create_dir_all(&clip).expect("clip dir");
    std::fs::write(clip.join("clip.glb"), "skeleton").expect("glb");
    let meta = json!({
        "version": 1, "id": "standing-dodge-left", "name": "Standing Dodge Left",
        "pack": "Singles", "tags": ["combat", "dodge"],
        "rig": { "kind": "mixamo", "bones": [], "hipsHeight": 1.04 },
        "frames": 38, "fps": 30.0, "length": 1.27,
        "rootMotion": { "metres": 0.92, "direction": "left" }, "loops": false,
        "bytes": 46000, "file": "clip.glb",
        "origin": { "source": "import", "file": "Standing Dodge Left.fbx", "archive": null,
                    "kept": null, "project": null, "asset": null, "version": null },
        "hash": "abc", "revision": 1,
        "createdAt": "2026-10-26T10:00:00.000Z", "updatedAt": "2026-10-26T10:00:00.000Z",
    });
    std::fs::write(clip.join("clip.json"), meta.to_string()).expect("meta");
    session
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_list_clips_and_cannot_import_from_outside_the_workspace() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let session = fixture(dir.path());
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

    let all = call(&client, "list_animations", json!({})).await;
    assert_eq!(all["total"], 1);
    let clip = &all["clips"][0];
    assert_eq!(clip["name"], "Standing Dodge Left");
    assert_eq!(clip["rig"], "mixamo");
    assert_eq!(clip["rootMotion"]["metres"], 0.92);
    let none = call(&client, "list_animations", json!({ "tag": "magic" })).await;
    assert_eq!(none["clips"], json!([]));
    let by_pack = call(
        &client,
        "list_animations",
        json!({ "pack": "singles", "query": "dodge" }),
    )
    .await;
    assert_eq!(by_pack["clips"].as_array().map(Vec::len), Some(1));

    let outside = dir.path().join("Run To Rolling.fbx");
    std::fs::write(&outside, "fbx").expect("outside");
    let refused = call_error(
        &client,
        "import_animations",
        json!({ "paths": [outside.display().to_string()] }),
    )
    .await;
    assert!(refused.contains("outside the workspace"), "{refused}");
    let missing = call_error(
        &client,
        "import_animations",
        json!({ "paths": ["drop/gone.fbx"] }),
    )
    .await;
    assert!(missing.contains("is not there"), "{missing}");

    let no_rig = call_error(
        &client,
        "save_model_clips",
        json!({ "project": "frostveil", "asset": "dummy", "dryRun": true }),
    )
    .await;
    assert!(no_rig.contains("glTF"), "{no_rig}");

    let not_rigged = call_error(
        &client,
        "use_animations",
        json!({ "project": "frostveil", "asset": "dummy", "clips": [{ "clip": "standing-dodge-left" }] }),
    )
    .await;
    assert!(not_rigged.contains("glTF"), "{not_rigged}");
    let review = call(
        &client,
        "get_retarget_review",
        json!({ "project": "frostveil", "asset": "dummy" }),
    )
    .await;
    assert_eq!(review["version"], Value::Null);
    assert_eq!(review["baked"], json!([]));
    client.cancel().await.expect("close");
    server.stop();
}
