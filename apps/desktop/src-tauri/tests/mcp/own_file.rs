//! A real MCP client adds a model file someone fixed by hand as an asset's
//! next version: into the inbox aimed at the asset, or accepted at once.
//! Paths inside the workspace's versions are refused and the file itself is
//! never moved. Nothing runs Blender: the files are GLBs.

use std::path::Path;
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::pipeline::tools::Toolbox;
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

/// A workspace with one project holding a lantern asset at version 1.
fn fixture(dir: &Path) -> (Session, String, std::path::PathBuf) {
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
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    std::fs::create_dir_all(project_dir.join("models")).expect("models");
    let prop = maquette_core::inbox::fixture::clean_prop();
    std::fs::write(project_dir.join("models/lantern_v1.glb"), &prop).expect("v1");
    project::modify(&root, &config.layout, &manifest.id, |m| {
        let lantern = assets::add(
            m,
            NewAsset {
                name: "Lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Blender,
            },
        )?;
        assets::add_version(m, &lantern.id, "models/lantern_v1.glb".into(), None)?;
        Ok(())
    })
    .expect("lantern");
    (session, manifest.id, project_dir)
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

async fn raw_call(
    client: &RunningService<RoleClient, ClientConfig>,
    tool: &str,
    args: Value,
) -> CallToolResult {
    let mut params = CallToolRequestParams::new(tool.to_string());
    if let Value::Object(map) = args {
        params = params.with_arguments(map);
    }
    client.call_tool(params).await.expect("call")
}

async fn call(client: &RunningService<RoleClient, ClientConfig>, tool: &str, args: Value) -> Value {
    let result = raw_call(client, tool, args).await;
    assert_ne!(
        result.is_error,
        Some(true),
        "{tool} failed: {}",
        text(&result)
    );
    serde_json::from_str(&text(&result)).expect("json")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_add_a_fixed_model_file_as_the_next_version() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, project_dir) = fixture(dir.path());
    let studio = ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {});
    let (root, config) = session.workspace().expect("workspace");
    studio.open_workspace(&root, &config);
    let hub = Arc::new(Hub::new(|| {}));
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
        Arc::clone(&hub),
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

    let outside = dir.path().join("fixes");
    std::fs::create_dir_all(&outside).expect("fixes");
    let fixed = outside.join("lantern fixed.glb");
    let bytes = maquette_core::inbox::fixture::clean_prop();
    std::fs::write(&fixed, &bytes).expect("fixed");

    // Into the inbox, aimed at the asset, and the file stays put.
    let added = call(
        &client,
        "add_model_file",
        json!({ "project": project_id, "asset": "Lantern", "path": fixed.display().to_string() }),
    )
    .await;
    assert_eq!(added["asset"]["id"], "lantern", "{added}");
    assert_eq!(added["job"], Value::Null);
    assert_eq!(added["accepted"], Value::Null);
    let inbox_file = added["inboxFile"].as_str().expect("inbox file").to_string();
    assert!(root.join(&config.layout.inbox).join(&inbox_file).is_file());
    assert_eq!(std::fs::read(&fixed).expect("still there"), bytes);

    // Accepted at once, it is the asset's version 2.
    let accepted = call(
        &client,
        "add_model_file",
        json!({ "project": project_id, "asset": "lantern",
                "path": fixed.display().to_string(), "accept": true }),
    )
    .await;
    assert_eq!(accepted["accepted"]["version"], 2, "{accepted}");
    assert!(fixed.is_file(), "copied, never moved");
    let manifest = project::load(&root, &config.layout, &project_id).expect("load");
    let lantern = manifest
        .assets
        .iter()
        .find(|a| a.id == "lantern")
        .expect("lantern");
    assert_eq!(lantern.versions.len(), 2);

    // A version already in the project, a relative path or another file
    // type is refused, and nothing lands.
    let version = project_dir.join("models/lantern_v1.glb");
    for (path, says) in [
        (version.display().to_string(), "version already"),
        ("fixes/lantern.glb".to_string(), "absolute"),
        (
            outside.join("notes.txt").display().to_string(),
            "not a model file",
        ),
    ] {
        let refused = raw_call(
            &client,
            "add_model_file",
            json!({ "project": project_id, "asset": "lantern", "path": path }),
        )
        .await;
        assert_eq!(refused.is_error, Some(true), "{path}");
        assert!(text(&refused).contains(says), "{path}: {}", text(&refused));
    }
    let unknown = raw_call(
        &client,
        "add_model_file",
        json!({ "project": project_id, "asset": "no-such-thing",
                "path": fixed.display().to_string() }),
    )
    .await;
    assert_eq!(unknown.is_error, Some(true));

    client.cancel().await.expect("close");
    server.stop();
}
