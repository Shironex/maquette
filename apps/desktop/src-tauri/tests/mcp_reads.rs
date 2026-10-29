//! A real MCP client reads the Lab with a direction filter. Codex is a
//! fake script that always fails, so the failed jobs of two directions
//! exist without any real service. The fake is a shell script, so the
//! test runs on unix only.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::{ResolvedTool, Toolbox};
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{
    AppState, HostOs, NewProject, ProjectTemplate, ToolId, WorkspaceConfig, project, workspace,
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
    codex: std::path::PathBuf,
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
        Toolbox {
            tools: BTreeMap::from([(
                ToolId::Codex,
                ResolvedTool {
                    program: self.codex.clone(),
                    prefix: Vec::new(),
                    version: "0.0.0-fake".into(),
                },
            )]),
            path_env: std::env::var_os("PATH").unwrap_or_default(),
            os: Some(HostOs::current()),
        }
    }
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

async fn call(client: &RunningService<RoleClient, ClientConfig>, tool: &str, args: Value) -> Value {
    let mut params = CallToolRequestParams::new(tool.to_string());
    if let Value::Object(map) = args {
        params = params.with_arguments(map);
    }
    let result = client.call_tool(params).await.expect("call");
    assert_ne!(
        result.is_error,
        Some(true),
        "{tool} failed: {}",
        text(&result)
    );
    serde_json::from_str(&text(&result)).expect("json")
}

fn fake_codex(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("codex");
    std::fs::write(
        &path,
        "#!/bin/sh\ncat >/dev/null\necho 'error: not signed in' >&2\nexit 2\n",
    )
    .expect("script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("mode");
    path
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_concepts_limits_directions_and_jobs_to_the_direction() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Moon Island".into(),
            template: ProjectTemplate::Room,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(maquette_core::pipeline::runner::NullSink)),
        concepts: Concepts::new(Some(dir.path().join("codex-home")), 2, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio: ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {}),
        codex: fake_codex(dir.path()),
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

    for name in ["Frozen lake", "Stone well"] {
        call(
            &client,
            "start_concepts",
            json!({ "project": manifest.id, "direction": name, "prompt": "A scene", "count": 1 }),
        )
        .await;
    }
    host.concepts.wait();

    let all = call(&client, "get_concepts", json!({ "project": manifest.id })).await;
    assert_eq!(all["directions"].as_array().expect("directions").len(), 2);
    assert_eq!(all["failed"].as_array().expect("failed").len(), 2, "{all}");

    // By name, in any case, and by id: one direction, and only its jobs.
    for wanted in ["stone well", "Stone well", "stone-well"] {
        let one = call(
            &client,
            "get_concepts",
            json!({ "project": manifest.id, "direction": wanted }),
        )
        .await;
        assert_eq!(one["directions"].as_array().expect("directions").len(), 1);
        assert_eq!(one["directions"][0]["id"], json!("stone-well"));
        let failed = one["failed"].as_array().expect("failed");
        assert_eq!(failed.len(), 1, "{wanted}: {one}");
        assert_eq!(failed[0]["direction"], json!("Stone well"));
        assert!(one["running"].as_array().expect("running").is_empty());
    }

    let mut params = CallToolRequestParams::new("get_concepts".to_string());
    params = params.with_arguments(
        json!({ "project": manifest.id, "direction": "No such" })
            .as_object()
            .cloned()
            .expect("object"),
    );
    let missing = client.call_tool(params).await.expect("call");
    assert_eq!(missing.is_error, Some(true));

    client.cancel().await.expect("close");
    server.stop();
}
