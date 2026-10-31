//! A real MCP client plans an export pack of a library entry, reads the
//! presets, and is refused a pack outside the workspace, without Blender
//! and with settings that do not read. Everything happens in a temp
//! workspace; nothing reaches Codex, Tripo, Meshy or Blender.

use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::packs::Packs;
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::Hub;
use maquette_desktop_lib::mcp::server::Running;
use maquette_desktop_lib::session::Session;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::json;

use super::library::fixture;
use super::support::{Client, call, call_error};

/// A host with the export packs service, and no Blender.
struct PackHost {
    session: Session,
    runner: PipelineRunner,
    concepts: Concepts,
    thumbs: Thumbnails,
    studio: ProviderJobs,
    packs: Packs,
}

impl Host for PackHost {
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
    fn packs(&self) -> Option<&Packs> {
        Some(&self.packs)
    }
}

async fn connect(session: Session) -> (Client, Running) {
    let host = Arc::new(PackHost {
        session,
        runner: PipelineRunner::new(Arc::new(NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio: ProviderJobs::new(
            Arc::new(MemoryStore::default()),
            Options::default(),
            maquette_core::inbox::send_ports(),
            |_| {},
        ),
        packs: Packs::new(PipelineRunner::new(Arc::new(NullSink)), |_| {}),
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_plan_a_pack_and_are_kept_inside_the_workspace() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server) = connect(fixture(dir.path())).await;
    call(
        &client,
        "add_to_library",
        json!({ "project": "moon-island", "asset": "Desk lamp" }),
    )
    .await;

    let listed = call(&client, "list_packs", json!({})).await;
    assert_eq!(listed["packs"], json!([]));
    assert_eq!(listed["presets"][0]["id"], json!("plain-obj"));

    let plan = call(
        &client,
        "export_pack",
        json!({
            "entries": ["desk-lamp"],
            "settings": { "name": "lamps", "format": "both" },
            "dryRun": true,
        }),
    )
    .await;
    assert_eq!(plan["dryRun"], json!(true));
    assert_eq!(plan["settings"]["format"], json!("both"));
    assert_eq!(plan["plan"]["label"], json!("1 model from the library"));
    assert_eq!(plan["plan"]["models"][0]["key"], json!("desk_lamp"));
    assert_eq!(plan["plan"]["kinds"][0]["biggest"], json!(9000));

    let both = call_error(
        &client,
        "export_pack",
        json!({ "entries": ["desk-lamp"], "project": "moon-island", "dryRun": true }),
    )
    .await;
    assert!(
        both.contains("one of entries, collection or project"),
        "{both}"
    );

    let unread = call_error(
        &client,
        "export_pack",
        json!({ "entries": ["desk-lamp"], "settings": { "format": "fbx" } }),
    )
    .await;
    assert!(unread.contains("do not read"), "{unread}");

    let outside = dir.path().join("elsewhere");
    let refused = call_error(
        &client,
        "export_pack",
        json!({ "entries": ["desk-lamp"], "out": outside.to_string_lossy() }),
    )
    .await;
    assert!(refused.contains("outside the workspace"), "{refused}");
    assert!(!outside.exists());

    let no_blender = call_error(&client, "export_pack", json!({ "entries": ["desk-lamp"] })).await;
    assert!(
        no_blender.contains("Blender is not installed"),
        "{no_blender}"
    );

    let missing = call_error(&client, "get_pack", json!({ "id": "nope" })).await;
    assert!(!missing.is_empty());
    let no_key = call_error(
        &client,
        "decide_pack",
        json!({ "id": "nope", "choice": "raise", "triangles": 3000 }),
    )
    .await;
    assert!(no_key.contains("key"), "{no_key}");

    client.cancel().await.expect("close");
    server.stop();
}
