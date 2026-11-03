//! A real MCP client makes canvases from presets: blank by default, story
//! worded for the project's target, mechanics, each seed one undoable
//! change under the agent's name, and an unknown preset refused before
//! anything is written. Everything happens in a temp workspace.

use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::plans;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::Hub;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::plan_camera::fixture;
use super::support::{TestHost, call, call_error};

fn titles(canvas: &Value) -> Vec<String> {
    canvas["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|n| n["title"].as_str().unwrap_or_default().to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_starts_canvases_from_presets() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let session = fixture(dir.path());
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
    let agent = "Story agent";

    // The tool lists its presets.
    let tools = client.list_all_tools().await.expect("tools");
    let create = tools
        .iter()
        .find(|t| t.name == "create_canvas")
        .expect("tool");
    let about = create.description.as_deref().unwrap_or_default();
    for preset in ["brainstorm", "story", "mechanics"] {
        assert!(about.contains(preset), "{preset} in the description");
    }

    // No preset: a blank canvas, as before.
    let blank = call(
        &client,
        "create_canvas",
        json!({ "project": "moon-island", "name": "Ideas", "agent": agent }),
    )
    .await;
    assert_eq!(blank["preset"], json!("brainstorm"));
    assert_eq!(blank["nodes"], json!(0));
    assert!(
        blank.get("change").is_none(),
        "a blank canvas has nothing to undo, so no change id: {blank}"
    );
    let alias = call(
        &client,
        "create_canvas",
        json!({ "project": "moon-island", "name": "More ideas", "preset": "blank", "agent": agent }),
    )
    .await;
    assert_eq!(alias["preset"], json!("brainstorm"));

    // Story is worded for the target: a website's visitor, a game's player.
    let site = call(
        &client,
        "create_canvas",
        json!({ "project": "moon-island", "name": "Tour story", "preset": "story", "agent": agent }),
    )
    .await;
    assert_eq!(site["preset"], json!("story"));
    assert_eq!(
        (site["nodes"].clone(), site["links"].clone()),
        (json!(8), json!(7))
    );
    let read = call(
        &client,
        "get_canvas",
        json!({ "project": "moon-island", "canvas": "tour-story", "full": true }),
    )
    .await;
    assert!(titles(&read).contains(&"Beat 1: First view".to_string()));
    let game = call(
        &client,
        "create_canvas",
        json!({ "project": "frost-game", "name": "Act 1", "preset": "Story", "agent": agent }),
    )
    .await;
    let read = call(
        &client,
        "get_canvas",
        json!({ "project": "frost-game", "canvas": game["id"], "full": true }),
    )
    .await;
    assert!(titles(&read).contains(&"Beat 1: Opening".to_string()));
    assert!(
        read["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .all(|n| n["by"] == json!(agent))
    );

    // The seed is one change under the agent's name, and its own undo takes it back.
    let changes = call(
        &client,
        "get_changes",
        json!({ "project": "frost-game", "canvas": "act-1" }),
    )
    .await;
    let line = &changes["changes"][0];
    assert_eq!(
        line["summary"],
        json!("Made the canvas Act 1 from the Story preset")
    );
    assert_eq!(line["by"], json!(agent));
    assert_eq!(
        game["change"], line["id"],
        "the answer names the line to undo"
    );
    let undone = call(
        &client,
        "undo_run",
        json!({ "project": "frost-game", "canvas": "act-1", "changes": [game["change"]], "agent": agent }),
    )
    .await;
    assert!(undone.is_object());
    let frost = dir.path().join("ws/projects/frost-game");
    assert!(
        plans::get(&frost, "act-1")
            .expect("get")
            .plan
            .nodes
            .is_empty()
    );

    let mechanics = call(
        &client,
        "create_canvas",
        json!({ "project": "frost-game", "name": "Combat", "preset": "mechanics", "agent": agent }),
    )
    .await;
    assert_eq!(mechanics["preset"], json!("mechanics"));
    assert_eq!(mechanics["nodes"], json!(4));

    // An unknown preset is refused with the list, and nothing is written.
    let error = call_error(
        &client,
        "create_canvas",
        json!({ "project": "frost-game", "name": "Quests", "preset": "quests", "agent": agent }),
    )
    .await;
    assert!(error.contains("quests is not a preset"), "{error}");
    assert!(error.contains("brainstorm") && error.contains("story") && error.contains("mechanics"));
    assert!(!frost.join("plans/quests.plan.json").exists());

    client.cancel().await.expect("close");
    server.stop();
}
