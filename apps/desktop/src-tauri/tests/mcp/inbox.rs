//! A real MCP client reads the inbox two ways and discards a model. The
//! checks of a model made for a send must be the same in list_inbox and
//! get_inbox_checks, and a discard moves the file aside, closes the send
//! and leaves the rest of the inbox alone. Everything happens in a temp
//! workspace; nothing reaches Tripo, Meshy or Blender.

use std::path::Path;
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::providers::{
    MemoryStore, Options, PresetVariant, Provider, ProviderId, ProviderJobs, SendRequest,
};
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

use super::support::{TestHost, call, raw_call};

/// A workspace with one project and an asset named Lantern.
fn fixture(dir: &Path) -> (Session, String, String) {
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
            name: "Harbour".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let refs = project::dir(&root, &config.layout, &manifest.id)
        .expect("dir")
        .join("refs");
    std::fs::create_dir_all(&refs).expect("refs");
    std::fs::write(refs.join("front.png"), b"\x89PNG\r\n\x1a\nfake").expect("ref");
    let mut asset = String::new();
    project::modify(&root, &config.layout, &manifest.id, |m| {
        asset = assets::add(
            m,
            NewAsset {
                name: "Lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )?
        .id;
        Ok(())
    })
    .expect("lantern");
    (session, manifest.id, asset)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_inbox_tools_agree_on_the_checks_and_a_discard_moves_a_model_aside() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, asset) = fixture(dir.path());
    let (root, config) = session.workspace().expect("workspace");
    let studio = ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {});
    studio.open_workspace(&root, &config);
    // A By hand send made as a character, answered with a model that has no
    // rig: the send decides its kind, and the checks follow it.
    let job = studio
        .send(
            &SendRequest {
                project: project_id.clone(),
                asset: asset.clone(),
                provider: Provider::ByHand,
                service: ProviderId::Tripo,
                variant: PresetVariant::Character,
                images: vec!["refs/front.png".into()],
                tripo: None,
                meshy: None,
                prompt: None,
                redo_of: None,
                studio: None,
            },
            false,
            false,
        )
        .expect("send");
    let made = dir.path().join("made.glb");
    std::fs::write(&made, maquette_core::inbox::fixture::clean_prop()).expect("model");
    let back = studio.hand_back(&job.id, &made).expect("hand back");
    let sent_file = back.inbox_file.expect("inbox file");
    let inbox = root.join(&config.layout.inbox);
    std::fs::write(
        inbox.join("loose_crate.glb"),
        maquette_core::inbox::fixture::clean_prop(),
    )
    .expect("loose");

    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio,
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

    // Both tools read the model as its send says: a character, and a
    // character without a rig fails the rig check.
    let listed = call(&client, "list_inbox", json!({})).await;
    let item = listed["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|i| i["id"] == json!(sent_file))
        .expect("the sent model");
    assert_eq!(item["kind"], json!("character"));
    assert_eq!(item["worst"], json!("fail"));
    let checks = call(&client, "get_inbox_checks", json!({ "id": sent_file })).await;
    assert_eq!(checks["kind"], item["kind"]);
    assert_eq!(checks["worst"], item["worst"], "{checks}");
    assert_eq!(checks["sentFor"]["job"], json!(job.id));
    // A model nobody sent still reads as before.
    let loose = call(
        &client,
        "get_inbox_checks",
        json!({ "id": "loose_crate.glb" }),
    )
    .await;
    assert_eq!(loose["kind"], json!("prop"));
    assert_eq!(loose["worst"], json!("pass"));
    assert_eq!(loose["sentFor"], Value::Null);

    // Discard: moved aside, not deleted, and the send is closed.
    let done = call(&client, "discard_inbox_model", json!({ "id": sent_file })).await;
    assert_eq!(done["closedSend"], json!(job.id));
    let kept = inbox.join(".discarded").join(&sent_file);
    assert!(kept.is_file(), "moved to inbox/.discarded");
    assert!(!inbox.join(&sent_file).exists());
    assert_eq!(
        done["file"],
        json!(kept.display().to_string()),
        "answers where it went"
    );
    let after = call(&client, "list_inbox", json!({})).await;
    let ids: Vec<&str> = after["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter_map(|i| i["id"].as_str())
        .collect();
    assert_eq!(ids, ["loose_crate.glb"]);
    assert_eq!(
        host.studio.get(&job.id).expect("job").state,
        maquette_core::providers::JobState::Cancelled
    );

    // A model nobody sent: moved aside, no send to close. Unknown ids and
    // names outside the inbox are refused.
    let loose = call(
        &client,
        "discard_inbox_model",
        json!({ "id": "loose_crate.glb" }),
    )
    .await;
    assert_eq!(loose["closedSend"], Value::Null);
    for id in ["loose_crate.glb", "../escape.glb", "notes.txt"] {
        let refused = raw_call(&client, "discard_inbox_model", json!({ "id": id })).await;
        assert_eq!(refused.is_error, Some(true), "{id}");
    }

    client.cancel().await.expect("close");
    server.stop();
}
