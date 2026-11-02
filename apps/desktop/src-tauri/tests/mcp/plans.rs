//! A real MCP client plans on a canvas: makes one, adds nodes and links
//! beside a person's note, edits and tidies them, reads the change log,
//! is refused when removing the person's note and undoes its own run,
//! newest first where its runs rewrote the same note.
//! Everything happens in a temp workspace.

use std::path::Path;
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::plans::{self, NodeKind, PlanNode, Writer, ops::PlanOp};
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
use serde_json::{Value, json};

use super::support::{TestHost, call, call_error};

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
            name: "Moon Island".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    session
}

fn ids(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|v| v.as_str().expect("id").to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_plans_beside_a_person_and_undoes_its_run() {
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
    let agent = "Research agent";
    let canvas_args = |extra: Value| {
        let mut args = json!({ "project": "moon-island", "canvas": "hero-ideas", "agent": agent });
        if let (Some(args), Value::Object(extra)) = (args.as_object_mut(), extra) {
            args.extend(extra);
        }
        args
    };

    let made = call(
        &client,
        "create_canvas",
        json!({ "project": "moon-island", "name": "Hero ideas", "agent": agent }),
    )
    .await;
    assert_eq!(made["id"], json!("hero-ideas"));
    assert_eq!(made["file"], json!("plans/hero-ideas.plan.json"));

    // The person writes a note in the app.
    let porch = PlanNode {
        id: "n-porch".into(),
        kind: NodeKind::Note,
        x: 0,
        y: 0,
        width: None,
        title: "Porch at dusk".into(),
        text: String::new(),
        image: None,
        url: None,
        asset: None,
        direction: None,
        by: None,
        person_wrote: false,
    };
    plans::change(&moon, "hero-ideas", &Writer::PERSON, |_| {
        Ok(vec![PlanOp::AddNode { node: porch }])
    })
    .expect("person");

    let listed = call(
        &client,
        "list_canvases",
        json!({ "project": "moon-island" }),
    )
    .await;
    assert_eq!(listed["canvases"][0]["nodes"], json!(1));

    let added = call(
        &client,
        "add_nodes",
        canvas_args(json!({
            "near": "n-porch",
            "run": "r-1",
            "nodes": [
                { "text": "Warm key from the lanterns, cool fill from the moon" },
                { "kind": "link", "title": "Lighting small scenes", "url": "https://example.com/night" }
            ],
            "links": [{ "from": "n-porch", "to": "$0" }, { "from": "$0", "to": "$1", "kind": "source" }]
        })),
    )
    .await;
    let new = ids(&added["added"]);
    assert_eq!(new.len(), 2);
    assert_eq!(added["placed"], json!(2));
    assert_eq!(ids(&added["links"]).len(), 2);
    assert!(added["change"].as_str().expect("change").starts_with("c-"));

    let read = call(&client, "get_canvas", canvas_args(json!({}))).await;
    let nodes = read["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 3);
    let research = nodes
        .iter()
        .find(|n| n["id"] == json!(new[0]))
        .expect("note");
    assert_eq!(research["by"], json!(agent));
    assert!(
        research["x"].as_i64().expect("x") > 220,
        "beside the porch: {research}"
    );
    let person = nodes
        .iter()
        .find(|n| n["id"] == json!("n-porch"))
        .expect("porch");
    assert!(person.get("by").is_none(), "a person's node has no by");
    assert_eq!(read["links"].as_array().expect("links").len(), 2);

    let refused = call_error(
        &client,
        "remove_nodes",
        canvas_args(json!({ "nodes": ["n-porch"] })),
    )
    .await;
    assert!(refused.contains("ask the person"), "{refused}");

    let edited = call(
        &client,
        "update_nodes",
        canvas_args(
            json!({ "run": "r-1", "nodes": [{ "id": "n-porch", "text": "Open on the porch." }] }),
        ),
    )
    .await;
    assert_eq!(edited["updated"], json!(1));
    let tidy = call(&client, "layout_canvas", canvas_args(json!({}))).await;
    assert_eq!(tidy["moved"], json!(0), "placed nodes are clear already");
    assert!(tidy["change"].is_null());

    let changes = call(&client, "get_changes", canvas_args(json!({ "by": agent }))).await;
    let lines = changes["changes"].as_array().expect("changes");
    assert_eq!(lines.len(), 3, "made, added, edited: {changes}");
    assert_eq!(lines[0]["summary"], json!("Edited note: Porch at dusk"));
    let person_lines = call(
        &client,
        "get_changes",
        canvas_args(json!({ "by": "person" })),
    )
    .await;
    assert_eq!(person_lines["changes"].as_array().expect("lines").len(), 1);

    let undone = call(&client, "undo_run", canvas_args(json!({ "run": "r-1" }))).await;
    assert_eq!(undone["undone"], 2);
    assert_eq!(undone["kept"], json!(0));
    assert_eq!(undone["nothingChanged"], json!(false));
    let after = call(&client, "get_canvas", canvas_args(json!({ "full": true }))).await;
    let nodes = after["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 1, "{after}");
    assert!(
        nodes[0].get("text").is_none(),
        "the person's note is back as it was"
    );
    let again = call_error(&client, "undo_run", canvas_args(json!({ "run": "r-1" }))).await;
    assert!(again.contains("Nothing of yours"), "{again}");

    // A person links their note to an agent's node: undoing the run keeps both.
    let second = call(
        &client,
        "add_nodes",
        canvas_args(json!({ "run": "r-2", "nodes": [{ "text": "Fireflies over the bay" }] })),
    )
    .await;
    let firefly = ids(&second["added"]).remove(0);
    plans::change(&moon, "hero-ideas", &Writer::PERSON, |_| {
        Ok(vec![PlanOp::AddEdge {
            edge: plans::PlanEdge {
                id: "e-person".into(),
                from: "n-porch".into(),
                to: firefly.clone(),
                from_side: None,
                to_side: None,
                kind: plans::EdgeKind::Relates,
                label: None,
                by: None,
            },
        }])
    })
    .expect("person link");
    let kept = call(&client, "undo_run", canvas_args(json!({ "run": "r-2" }))).await;
    assert_eq!(kept["kept"], json!(1), "{kept}");
    let after = call(&client, "get_canvas", canvas_args(json!({ "full": true }))).await;
    assert_eq!(
        after["nodes"].as_array().expect("nodes").len(),
        2,
        "{after}"
    );
    let links = after["links"].as_array().expect("links");
    assert!(
        links.iter().any(|l| l["id"] == json!("e-person")),
        "{after}"
    );

    // Two runs rewrite the person's note in turn: the older can not be undone
    // under the newer, and newest first brings the person's words back.
    let porch_text = |canvas: &Value| {
        canvas["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .find(|n| n["id"] == json!("n-porch"))
            .map(|n| n.get("text").cloned().unwrap_or(Value::Null))
            .expect("porch")
    };
    let first_look = call(&client, "get_canvas", canvas_args(json!({ "full": true }))).await;
    let original = porch_text(&first_look);
    for (run, text) in [("r-3", "a"), ("r-4", "b")] {
        call(
            &client,
            "update_nodes",
            canvas_args(json!({ "run": run, "nodes": [{ "id": "n-porch", "text": text }] })),
        )
        .await;
    }
    let waits = call_error(&client, "undo_run", canvas_args(json!({ "run": "r-3" }))).await;
    assert!(waits.contains("Undo the later change first"), "{waits}");
    let held = call(&client, "get_canvas", canvas_args(json!({ "full": true }))).await;
    assert_eq!(porch_text(&held), json!("b"), "nothing undone");
    call(&client, "undo_run", canvas_args(json!({ "run": "r-4" }))).await;
    call(&client, "undo_run", canvas_args(json!({ "run": "r-3" }))).await;
    let back = call(&client, "get_canvas", canvas_args(json!({ "full": true }))).await;
    assert_eq!(porch_text(&back), original, "{back}");
    // Both rewrites in one run undo together, newest first.
    for text in ["c", "d"] {
        call(
            &client,
            "update_nodes",
            canvas_args(json!({ "run": "r-5", "nodes": [{ "id": "n-porch", "text": text }] })),
        )
        .await;
    }
    let together = call(&client, "undo_run", canvas_args(json!({ "run": "r-5" }))).await;
    assert_eq!(together["undone"], json!(2), "{together}");
    let back = call(&client, "get_canvas", canvas_args(json!({ "full": true }))).await;
    assert_eq!(porch_text(&back), original, "{back}");

    // An undo can not be undone, by the agent or the person: a redo would
    // put the agent's words back with the person's only in plain lines.
    let held =
        || plans::log::keeper_bytes(&plans::log::entries(&moon, "hero-ideas").expect("entries"));
    let undo_line = together["change"].as_str().expect("undo line").to_string();
    let redo = call_error(
        &client,
        "undo_run",
        canvas_args(json!({ "changes": [undo_line.clone()] })),
    )
    .await;
    assert!(redo.contains("an undo can not be undone"), "{redo}");
    for (run, text) in [("r-6", "x"), ("r-7", "y")] {
        call(
            &client,
            "update_nodes",
            canvas_args(json!({ "run": run, "nodes": [{ "id": "n-porch", "text": text }] })),
        )
        .await;
    }
    let newer = call(&client, "undo_run", canvas_args(json!({ "run": "r-7" }))).await;
    let newer_undo = newer["change"].as_str().expect("undo line").to_string();
    let hold = held();
    let redo = call_error(
        &client,
        "undo_run",
        canvas_args(json!({ "changes": [newer_undo.clone()] })),
    )
    .await;
    assert!(redo.contains("an undo can not be undone"), "{redo}");
    let refused = plans::undo(&moon, "hero-ideas", &[newer_undo], None, None)
        .expect_err("the person's redo of it too");
    assert!(
        refused.to_string().contains("an undo can not be undone"),
        "{refused}"
    );
    assert_eq!(held(), hold, "the hold is unchanged");
    let now = call(&client, "get_canvas", canvas_args(json!({ "full": true }))).await;
    assert_eq!(porch_text(&now), json!("x"));
    call(&client, "undo_run", canvas_args(json!({ "run": "r-6" }))).await;
    let back = call(&client, "get_canvas", canvas_args(json!({ "full": true }))).await;
    assert_eq!(porch_text(&back), original, "{back}");
    assert_eq!(held(), 0);

    let missing = call_error(
        &client,
        "get_canvas",
        json!({ "project": "moon-island", "canvas": "nope" }),
    )
    .await;
    assert!(missing.contains("Could not find the canvas"), "{missing}");

    client.cancel().await.expect("close");
    server.stop();
}
