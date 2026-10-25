//! A real MCP client reads and changes a project's bake settings and sets
//! assets to skip the bake. Nothing runs Blender.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{
    AppState, AssetKind, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, assets,
    pipeline, prepare, project, workspace,
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
    pipeline_changes: AtomicUsize,
    prepare_changes: AtomicUsize,
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
    fn pipeline_changed(&self, _project: &str) {
        self.pipeline_changes.fetch_add(1, Ordering::SeqCst);
    }
    fn prepare_changed(&self, _project: &str) {
        self.prepare_changes.fetch_add(1, Ordering::SeqCst);
    }
}

/// A workspace with one Room project (its pipeline and starters written)
/// and two assets.
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
            name: "Study room".into(),
            template: ProjectTemplate::Room,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let manifest = project::modify(&root, &config.layout, &manifest.id, |m| {
        for name in ["Desk globe", "Desk"] {
            assets::add(
                m,
                NewAsset {
                    name: name.into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Blender,
                },
            )?;
        }
        Ok(())
    })
    .expect("assets");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    pipeline::ensure(&project_dir, &manifest).expect("pipeline");
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

fn bake_args(project_dir: &Path) -> Vec<String> {
    let def = pipeline::load(project_dir)
        .expect("load")
        .expect("pipeline");
    pipeline::bake::step(&def).expect("bake step").args.clone()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_change_the_bake_settings_and_skip_assets() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, project_dir) = fixture(dir.path());
    let studio = ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {});
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(maquette_core::pipeline::runner::NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio,
        pipeline_changes: AtomicUsize::new(0),
        prepare_changes: AtomicUsize::new(0),
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
    let tools = client.list_all_tools().await.expect("tools");
    let read = tools
        .iter()
        .find(|t| t.name == "get_bake_settings")
        .expect("get_bake_settings");
    assert_eq!(
        read.annotations.as_ref().and_then(|a| a.read_only_hint),
        Some(true)
    );

    // A new project bakes with the defaults.
    let first = call(
        &client,
        "get_bake_settings",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(first["size"], json!(1024));
    assert_eq!(first["samples"], json!(32));
    assert_eq!(first["denoise"], json!(true));
    assert_eq!(first["canDenoise"], json!(true));
    assert_eq!(first["script"], json!("current"));
    assert_eq!(first["skipped"], json!([]));

    // Only the given fields change, in the step's args.
    let set = call(
        &client,
        "set_bake_settings",
        json!({ "project": project_id, "samples": 128 }),
    )
    .await;
    assert_eq!(set["samples"], json!(128));
    assert_eq!(set["size"], json!(1024));
    let args = bake_args(&project_dir);
    let at = args.iter().position(|a| a == "--samples").expect("samples");
    assert_eq!(args[at + 1], "128");
    assert_eq!(host.pipeline_changes.load(Ordering::SeqCst), 1);

    // A size that is not offered is refused and nothing is written.
    let refused = raw_call(
        &client,
        "set_bake_settings",
        json!({ "project": project_id, "size": 1000 }),
    )
    .await;
    assert_eq!(refused.is_error, Some(true));
    assert!(text(&refused).contains("1000"), "{}", text(&refused));
    assert_eq!(bake_args(&project_dir), args);

    // Skip the bake for one asset by name; an unknown one writes nothing.
    let unknown = raw_call(
        &client,
        "set_skip_bake",
        json!({ "project": project_id, "assets": ["Desk globe", "Nowhere"], "skip": true }),
    )
    .await;
    assert_eq!(unknown.is_error, Some(true));
    assert!(
        prepare::load(&project_dir)
            .expect("prepare")
            .models
            .is_empty()
    );
    let skipped = call(
        &client,
        "set_skip_bake",
        json!({ "project": project_id, "assets": ["Desk globe"], "skip": true }),
    )
    .await;
    assert_eq!(skipped["assets"], json!(["desk-globe"]));
    assert_eq!(skipped["bake"]["skipped"], json!(["desk-globe"]));
    assert_eq!(host.prepare_changes.load(Ordering::SeqCst), 1);
    let real = call(
        &client,
        "get_real_size",
        json!({ "project": project_id, "asset": "desk-globe" }),
    )
    .await;
    assert_eq!(real["skipBake"], json!(true));

    // And back.
    call(
        &client,
        "set_skip_bake",
        json!({ "project": project_id, "assets": ["desk-globe"], "skip": false }),
    )
    .await;
    let after = call(
        &client,
        "get_bake_settings",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(after["skipped"], json!([]));
    let saved = prepare::load(&project_dir).expect("prepare");
    assert_eq!(saved.models["desk-globe"].skip_bake, Some(false));

    client.cancel().await.expect("close");
    server.stop();
}
