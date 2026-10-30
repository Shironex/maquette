//! A real MCP client plans a batch add, adds into a collection, tags,
//! renames, finds a look-alike, merges it and undoes the merge, and
//! removes and restores an entry. Everything happens in a temp workspace;
//! nothing reaches Codex, Tripo, Meshy or Blender.

use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{AssetKind, NewAsset, Stage, assets, project};
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::Hub;
use maquette_desktop_lib::mcp::server::Running;
use maquette_desktop_lib::session::Session;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::library::{fixture, stats};
use super::support::{Client, TestHost, call, call_error};

/// A .glb header and `body`, its declared length the real one.
fn glb(body: &str) -> Vec<u8> {
    let len = u32::try_from(12 + body.len()).expect("small");
    let mut out = b"glTF".to_vec();
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(body.as_bytes());
    out
}

async fn connect(session: Session) -> (Client, Running) {
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
    (client, server)
}

fn states(plan: &Value) -> Vec<(String, String)> {
    plan["plan"]["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .map(|a| {
            let name = a["name"].as_str().expect("name").to_string();
            (name, a["state"].as_str().expect("state").to_string())
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_plan_collect_tag_merge_undo_and_restore() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let session = fixture(dir.path());
    let root = dir.path().join("ws");
    let moon = root.join("projects/moon-island");
    project::modify(&root, &Default::default(), "moon-island", |m| {
        for (name, body) in [("Brass key", "key"), ("Lamp", "lamp b")] {
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
            std::fs::write(moon.join(&file), glb(body)).expect("model");
            assets::add_version(m, &id, file, Some(stats(1000)))?;
        }
        Ok(())
    })
    .expect("assets");
    let (client, server) = connect(session).await;

    let plan = call(
        &client,
        "add_many_to_library",
        json!({ "project": "moon-island", "dryRun": true }),
    )
    .await;
    assert_eq!(plan["plan"]["tag"], json!("moon-island"));
    assert_eq!(
        states(&plan),
        [
            ("Desk lamp".to_string(), "unreadable".to_string()),
            ("Brass key".to_string(), "new".to_string()),
            ("Lamp".to_string(), "new".to_string()),
        ]
    );
    let one = call(
        &client,
        "add_many_to_library",
        json!({ "project": "moon-island", "dryRun": true, "assets": [{ "asset": "Lamp" }] }),
    )
    .await;
    assert_eq!(states(&one).len(), 1);
    assert!(
        !root.join("library/lamp").exists(),
        "a dry run adds nothing"
    );

    let made = call(
        &client,
        "set_library_collection",
        json!({ "op": "create", "name": "Desk things" }),
    )
    .await;
    assert_eq!(made["collections"][0]["id"], json!("desk-things"));
    let missing = call_error(
        &client,
        "add_many_to_library",
        json!({ "project": "moon-island", "collection": "nope", "assets": [{ "asset": "Lamp" }] }),
    )
    .await;
    assert!(missing.contains("no collection"), "{missing}");
    let added = call(
        &client,
        "add_many_to_library",
        json!({
            "project": "moon-island",
            "collection": "desk-things",
            "assets": [{ "asset": "Brass key" }, { "asset": "Lamp" }],
        }),
    )
    .await;
    assert_eq!(added["added"], json!(2));
    let listed = call(
        &client,
        "list_library",
        json!({ "collection": "desk-things" }),
    )
    .await;
    assert_eq!(listed["total"], json!(2));
    assert_eq!(
        listed["collections"],
        json!([{ "id": "desk-things", "name": "Desk things", "count": 2 }])
    );

    let tagged = call(
        &client,
        "tag_library_entries",
        json!({ "entries": ["brass-key", "lamp"], "add": ["Metal"] }),
    )
    .await;
    assert_eq!(tagged["entries"][1]["tags"], json!(["metal"]));
    let renamed = call(
        &client,
        "rename_library_entry",
        json!({ "entry": "lamp", "name": "Desk lamp 2" }),
    )
    .await;
    assert_eq!(renamed, json!({ "id": "lamp", "name": "Desk lamp 2" }));
    call(
        &client,
        "add_to_library",
        json!({ "project": "moon-island", "asset": "Desk lamp" }),
    )
    .await;

    let alikes = call(&client, "find_look_alikes", json!({})).await;
    let pairs = alikes["pairs"].as_array().expect("pairs");
    assert_eq!(pairs.len(), 1, "{pairs:?}");
    assert_eq!(pairs[0]["kind"], json!("same-name"));
    assert!(pairs[0]["aName"].is_string());
    assert_eq!(alikes["unseen"].as_array().map(Vec::len), Some(3));
    assert_eq!(alikes["settings"]["threshold"], json!(0.9));

    let merged = call(
        &client,
        "merge_library_entries",
        json!({ "keep": "desk-lamp", "merge": "lamp" }),
    )
    .await;
    assert_eq!(merged["versions"], json!(2));
    assert_eq!(
        merged["tags"],
        json!(["metal"]),
        "copyTags follows the setting"
    );
    assert_eq!(merged["projects"], json!(["moon-island"]));
    assert_eq!(merged["merge"]["versions"], json!([2]));
    let after = call(&client, "find_look_alikes", json!({})).await;
    assert_eq!(after["merges"][0]["merged"], json!("lamp"));
    let manifest = project::load(&root, &Default::default(), "moon-island").expect("moon");
    let lamp = manifest
        .assets
        .iter()
        .find(|a| a.id == "lamp")
        .expect("lamp");
    assert_eq!(
        lamp.origin.as_ref().map(|o| o.entry.as_str()),
        Some("desk-lamp")
    );

    let undone = call(
        &client,
        "restore_to_library",
        json!({ "entries": ["lamp"] }),
    )
    .await;
    assert_eq!(undone["restored"], json!(["lamp"]));
    assert_eq!(undone["projects"], json!(["moon-island"]));
    let manifest = project::load(&root, &Default::default(), "moon-island").expect("moon");
    let lamp = manifest
        .assets
        .iter()
        .find(|a| a.id == "lamp")
        .expect("lamp");
    assert_eq!(lamp.origin.as_ref().map(|o| o.entry.as_str()), Some("lamp"));

    let removed = call(
        &client,
        "remove_from_library",
        json!({ "entries": ["brass-key"] }),
    )
    .await;
    assert_eq!(removed["removed"], json!(["brass-key"]));
    assert!(root.join("library/.removed/brass-key/entry.json").is_file());
    assert!(
        moon.join("models/brass-key_v1.glb").is_file(),
        "the copy stays"
    );
    let fewer = call(&client, "list_library", json!({})).await;
    assert_eq!(fewer["total"], json!(2));
    assert_eq!(fewer["collections"][0]["count"], json!(1));
    call(
        &client,
        "restore_to_library",
        json!({ "entries": ["brass-key"] }),
    )
    .await;
    let back = call(
        &client,
        "list_library",
        json!({ "collection": "desk-things" }),
    )
    .await;
    assert_eq!(
        back["total"],
        json!(2),
        "a restore puts it back in its collection"
    );

    let bad = call_error(
        &client,
        "set_library_collection",
        json!({ "op": "bogus", "collection": "desk-things" }),
    )
    .await;
    assert!(bad.contains("not an op"), "{bad}");
    let deleted = call(
        &client,
        "set_library_collection",
        json!({ "op": "delete", "collection": "desk-things" }),
    )
    .await;
    assert_eq!(deleted["collections"], json!([]));
    let all = call(&client, "list_library", json!({})).await;
    assert_eq!(
        all["total"],
        json!(3),
        "deleting a collection keeps the models"
    );

    client.cancel().await.expect("close");
    server.stop();
}
