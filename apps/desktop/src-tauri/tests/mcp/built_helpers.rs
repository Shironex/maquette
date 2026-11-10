//! A real MCP client finds the helper file Maquette ships for built
//! scripts, adds the example with that file listed, brings an older copy of
//! it up to date and removes the asset again. Everything happens in a temp
//! workspace; nothing reaches Codex, Tripo, Meshy or Blender.

use std::path::Path;
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::starter::{BUILT_EXAMPLE, BUILT_HELPERS};
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{AppState, WorkspaceConfig, workspace};
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::Hub;
use maquette_desktop_lib::session::Session;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::support::{TestHost, call};

/// Makes the helper file an untouched copy of an older one, as an older
/// Maquette would have written and recorded it.
fn age_helpers(project_dir: &Path) {
    use sha2::{Digest, Sha256};
    let old = "def box():\n    pass\n";
    std::fs::write(project_dir.join(BUILT_HELPERS.path), old).expect("old copy");
    let path = project_dir.join("tools/.starter.json");
    let mut record: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("record")).expect("json");
    record["scripts"][BUILT_HELPERS.path] =
        Value::String(hex::encode(Sha256::digest(old.as_bytes())));
    std::fs::write(&path, record.to_string()).expect("save record");
}

/// The state `get_starters` or `update_starters` gave for `path`.
fn state_of(answer: &Value, path: &str) -> Value {
    answer["starters"]
        .as_array()
        .expect("starters")
        .iter()
        .find(|s| s["path"] == path)
        .map(|s| s["state"].clone())
        .unwrap_or_else(|| panic!("{path} is not listed: {answer}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_find_the_shipped_helpers_and_use_them_for_a_built_asset() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
    session.set_workspace(root.clone(), config).expect("open");
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

    // A new project has both files where built scripts live, as it ships them.
    call(
        &client,
        "create_project",
        json!({ "name": "Moon Island", "target": "website", "template": "room" }),
    )
    .await;
    let project_dir = root.join("projects/moon-island");
    let starters = call(&client, "get_starters", json!({ "project": "moon-island" })).await;
    assert_eq!(state_of(&starters, BUILT_HELPERS.path), json!("current"));
    assert_eq!(state_of(&starters, BUILT_EXAMPLE.path), json!("current"));
    assert_eq!(
        std::fs::read_to_string(project_dir.join(BUILT_HELPERS.path)).expect("helpers"),
        BUILT_HELPERS.text
    );

    // The tools say where the helpers are, so a script needs none of its own.
    let tools = client.list_all_tools().await.expect("tools");
    let described = |name: &str| {
        let tool = tools.iter().find(|t| t.name == name).expect("the tool");
        format!(
            "{} {}",
            tool.description.as_deref().unwrap_or_default(),
            Value::Object((*tool.input_schema).clone())
        )
    };
    for name in ["get_starters", "add_built_asset"] {
        assert!(
            described(name).contains(BUILT_HELPERS.path),
            "{name} does not name the helper file"
        );
    }

    // The example, with the shipped helper file listed: nothing is copied,
    // both already are in built/.
    let added = call(
        &client,
        "add_built_asset",
        json!({
            "project": "moon-island",
            "script": BUILT_EXAMPLE.path,
            "helpers": [BUILT_HELPERS.path],
            "build": false,
        }),
    )
    .await;
    assert_eq!(added["asset"]["id"], json!("maquette-example"));
    assert_eq!(added["source"]["script"], json!(BUILT_EXAMPLE.path));
    assert_eq!(added["source"]["helpers"], json!([BUILT_HELPERS.path]));
    assert_eq!(added["source"]["state"], json!("new"));
    assert_eq!(added["source"]["missing"], json!([]));

    // An untouched older copy of the helper file updates like any starter.
    age_helpers(&project_dir);
    let aged = call(&client, "get_starters", json!({ "project": "moon-island" })).await;
    assert_eq!(state_of(&aged, BUILT_HELPERS.path), json!("outdated"));
    let updated = call(
        &client,
        "update_starters",
        json!({ "project": "moon-island" }),
    )
    .await;
    assert_eq!(state_of(&updated, BUILT_HELPERS.path), json!("current"));
    assert_eq!(
        std::fs::read_to_string(project_dir.join(BUILT_HELPERS.path)).expect("helpers"),
        BUILT_HELPERS.text
    );

    // Removing the asset leaves both files: they are the project's, and the
    // starters do not turn up as missing again.
    let removed = call(
        &client,
        "remove_assets",
        json!({ "project": "moon-island", "assets": ["maquette-example"] }),
    )
    .await;
    assert_eq!(removed["assets"][0]["state"], json!("removed"), "{removed}");
    assert_eq!(removed["assets"][0]["files"], json!(0));
    let after = call(&client, "get_starters", json!({ "project": "moon-island" })).await;
    assert_eq!(state_of(&after, BUILT_HELPERS.path), json!("current"));
    assert_eq!(state_of(&after, BUILT_EXAMPLE.path), json!("current"));

    // So it comes back as it was.
    let trash = removed["assets"][0]["trash"].as_str().expect("trash id");
    call(
        &client,
        "restore_asset",
        json!({ "project": "moon-island", "trash": trash }),
    )
    .await;
    let listed = call(
        &client,
        "list_built_assets",
        json!({ "project": "moon-island" }),
    )
    .await;
    assert_eq!(listed["assets"][0]["script"], json!(BUILT_EXAMPLE.path));
    assert_eq!(listed["assets"][0]["helpers"], json!([BUILT_HELPERS.path]));

    client.cancel().await.expect("close");
    server.stop();
}
