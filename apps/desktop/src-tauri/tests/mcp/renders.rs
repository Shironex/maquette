//! Render styles over MCP: an agent plans a render, sees what is refused
//! without Blender, and reads renders back.

use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::renders::Renders;
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

/// A host with the render styles service, and no Blender.
struct RenderHost {
    session: Session,
    runner: PipelineRunner,
    concepts: Concepts,
    thumbs: Thumbnails,
    studio: ProviderJobs,
    renders: Renders,
}

impl Host for RenderHost {
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
    fn renders(&self) -> Option<&Renders> {
        Some(&self.renders)
    }
}

async fn connect(session: Session) -> (Client, Running) {
    let host = Arc::new(RenderHost {
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
        renders: Renders::new(PipelineRunner::new(Arc::new(NullSink)), |_| {}),
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
async fn agents_plan_renders_and_hear_what_stops_them() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server) = connect(fixture(dir.path())).await;

    let listed = call(&client, "list_renders", json!({})).await;
    assert_eq!(listed["renders"], json!([]));

    let plan = call(
        &client,
        "render_styles",
        json!({ "project": "moon-island", "dryRun": true }),
    )
    .await;
    assert_eq!(plan["dryRun"], json!(true));
    assert_eq!(plan["plan"]["targets"][0]["folder"], json!("desk-lamp"));
    let out = plan["plan"]["dir"].as_str().expect("dir");
    assert!(out.ends_with("renders"), "{out}");
    assert!(out.contains("moon-island"), "{out}");

    let both = call_error(
        &client,
        "render_styles",
        json!({ "project": "moon-island", "entries": ["x"], "dryRun": true }),
    )
    .await;
    assert!(both.contains("Give project or entries"), "{both}");

    let style = call_error(
        &client,
        "render_styles",
        json!({ "project": "moon-island", "styles": ["sketch"] }),
    )
    .await;
    assert!(style.contains("no style sketch"), "{style}");

    let bad = call_error(
        &client,
        "render_styles",
        json!({ "project": "moon-island", "settings": { "size": 7 } }),
    )
    .await;
    assert!(bad.contains("Pick a size"), "{bad}");

    let blender = call_error(
        &client,
        "render_styles",
        json!({ "project": "moon-island", "styles": ["clay", "turntable"] }),
    )
    .await;
    assert!(blender.contains("Blender was not found"), "{blender}");

    let missing = call_error(&client, "get_render", json!({ "id": "nope" })).await;
    assert!(missing.contains("nope"), "{missing}");

    client.cancel().await.expect("close");
    server.stop();
}
