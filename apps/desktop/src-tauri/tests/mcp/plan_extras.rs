//! A real MCP client uses a canvas's extras: it asks the person to remove
//! their note (and can neither remove it nor answer its own proposal), reads
//! a mechanics canvas's overlaps (marking one fine is the person's), starts
//! a canvas from a node, and has its edited overlapping nodes tidied by Auto layout
//! after agents until the person turns it off. Everything happens in a temp
//! workspace.

use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::plans::{self, NodeKind, PlanNode, Writer, ops::PlanOp, removal};
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

fn card(id: &str, x: i32, title: &str, text: &str) -> PlanNode {
    PlanNode {
        id: id.into(),
        kind: NodeKind::Note,
        x,
        y: 600,
        width: None,
        title: title.into(),
        text: text.into(),
        image: None,
        url: None,
        asset: None,
        direction: None,
        by: None,
        person_wrote: false,
        person_placed: false,
        stop: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_asks_reads_overlaps_starts_from_a_node_and_is_tidied() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let session = fixture(dir.path());
    let game = dir.path().join("ws/projects/frost-game");
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
    let agent = "Research agent";
    let on = |extra: Value| {
        let mut args = json!({ "project": "frost-game", "canvas": "combat", "agent": agent });
        if let (Some(args), Value::Object(extra)) = (args.as_object_mut(), extra) {
            args.extend(extra);
        }
        args
    };

    // No tool answers a removal proposal or marks an overlap fine.
    let tools = client.list_all_tools().await.expect("tools");
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.contains(&"propose_removal") && names.contains(&"start_from_node"));
    for name in &names {
        let answers = name.contains("proposal") || name.contains("dismiss_removal");
        let marks = name.contains("fine") || name.contains("overlap");
        let accepts = name.contains("removal") && *name != "propose_removal";
        assert!(!answers && !marks && !accepts, "{name} must not exist");
    }

    let made = call(
        &client,
        "create_canvas",
        json!({ "project": "frost-game", "name": "Combat", "preset": "mechanics", "agent": agent }),
    )
    .await;
    assert_eq!(made["id"], json!("combat"));
    // The person writes two systems that share a key.
    let dash = card("n-dash", 0, "System: Dash", "Verb: evade\nInput: tap Shift");
    let veil = card(
        "n-veil",
        400,
        "System: Frost veil",
        "Verb: slow time\nInput: hold Shift",
    );
    plans::change(&game, "combat", &Writer::PERSON, |_| {
        Ok(vec![
            PlanOp::AddNode { node: dash },
            PlanOp::AddNode { node: veil },
        ])
    })
    .expect("person");

    let read = call(&client, "get_canvas", on(json!({}))).await;
    assert_eq!(read["autoLayout"], json!(true));
    assert_eq!(
        read["overlaps"],
        json!([{ "a": "n-dash", "b": "n-veil", "part": "input", "word": "shift", "fine": false }])
    );
    assert!(read.get("proposals").is_none());

    // Asking leaves the note; the agent still cannot remove it or undo the ask.
    let asked = call(
        &client,
        "propose_removal",
        on(json!({ "nodes": ["n-dash"], "reason": "Frost veil covers evading.", "run": "r-1" })),
    )
    .await;
    assert_eq!(asked["summary"], json!("Asked to remove: System: Dash"));
    let refused = call_error(&client, "remove_nodes", on(json!({ "nodes": ["n-dash"] }))).await;
    assert!(refused.contains("never a person's"), "{refused}");
    let refused = call_error(
        &client,
        "undo_run",
        on(json!({ "changes": [asked["proposal"]] })),
    )
    .await;
    assert!(!refused.is_empty());
    let read = call(&client, "get_canvas", on(json!({}))).await;
    let waiting = &read["proposals"][0];
    assert_eq!(waiting["id"], asked["proposal"]);
    assert_eq!(waiting["reason"], json!("Frost veil covers evading."));
    assert_eq!(waiting["stale"], json!(false));
    assert_eq!(read["nodes"].as_array().expect("nodes").len(), 6);

    // The person marks the overlap fine and answers Keep it, in the app.
    plans::mark_fine(&game, "combat", "n-veil", "n-dash", None).expect("fine");
    let proposal = asked["proposal"].as_str().expect("id");
    removal::dismiss(&game, "combat", proposal, None).expect("keep it");
    let read = call(&client, "get_canvas", on(json!({}))).await;
    assert_eq!(read["overlaps"][0]["fine"], json!(true));
    assert!(read.get("proposals").is_none());

    // Starting from a node copies it and its links; the source stays.
    let started = call(
        &client,
        "start_from_node",
        on(json!({ "node": "n-veil", "name": "Frost veil ideas" })),
    )
    .await;
    assert_eq!(started["id"], json!("frost-veil-ideas"));
    assert_eq!(started["preset"], json!("mechanics"));
    assert_eq!(started["nodes"], json!(1));
    let undone = call(
        &client,
        "undo_run",
        json!({ "project": "frost-game", "canvas": "frost-veil-ideas", "changes": [started["change"]], "agent": agent }),
    )
    .await;
    assert!(undone.get("change").is_some(), "{undone}");
    let source = call(&client, "get_canvas", on(json!({}))).await;
    assert_eq!(source["nodes"].as_array().expect("nodes").len(), 6);

    // A position the agent gives is kept; Auto layout after agents tidies an
    // edited node it gave no x or y, inside the same change.
    let dropped = json!({ "nodes": [{ "text": "Blink", "x": 10, "y": 610 }] });
    let added = call(&client, "add_nodes", on(dropped)).await;
    assert!(added.get("autoLayout").is_none(), "{added}");
    let blink = added["added"][0].clone();
    let edit = |text: &str| json!({ "nodes": [{ "id": blink, "text": text }] });
    let edited = call(&client, "update_nodes", on(edit("Blink, longer"))).await;
    assert_eq!(edited["autoLayout"]["moved"], json!([blink]));
    let read = call(&client, "get_canvas", on(json!({ "nodes": ["n-dash"] }))).await;
    assert_eq!(
        (read["nodes"][0]["x"].clone(), read["nodes"][0]["y"].clone()),
        (json!(0), json!(600))
    );

    // Off: nothing is tidied, and layout_canvas is refused.
    plans::set_auto_layout(&game, "combat", false, None).expect("off");
    let back = json!({ "nodes": [{ "id": blink, "x": 10, "y": 610 }] });
    call(&client, "update_nodes", on(back)).await;
    let edited = call(&client, "update_nodes", on(edit("Blink again"))).await;
    assert!(edited.get("autoLayout").is_none(), "{edited}");
    let refused = call_error(&client, "layout_canvas", on(json!({}))).await;
    assert!(
        refused.contains("Auto layout after agents off"),
        "{refused}"
    );
    assert_eq!(
        call(&client, "get_canvas", on(json!({}))).await["autoLayout"],
        json!(false)
    );

    client.cancel().await.expect("close");
    server.stop();
}
