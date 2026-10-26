//! A real MCP client starts, reads and stops a project's site dev server,
//! the one the Page view shows. The site is a temp folder whose dev command
//! is a tiny Node server; no real website is touched.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maquette_core::concepts::Concepts;
use maquette_core::page::{self, PageSettings};
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::site::preview::{SitePreview, answers};
use maquette_core::thumbs::Thumbnails;
use maquette_core::toolchain::find_on_path;
use maquette_core::{
    AppState, NewProject, ProbeEnv, ProjectTemplate, WorkspaceConfig, project, workspace,
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
    preview: Arc<SitePreview>,
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
    fn site_preview(&self) -> Option<Arc<SitePreview>> {
        Some(Arc::clone(&self.preview))
    }
}

/// A dev server that prints its address the way Vite does.
const SERVER: &str = "const http = require('http');\n\
http.createServer((q, r) => r.end('ok')).listen(+process.env.PORT, '127.0.0.1', () =>\n\
  console.log('  Local  http://127.0.0.1:' + process.env.PORT + '/'));\n";

/// A workspace with one project and a site folder beside it, not linked yet.
fn fixture(dir: &Path) -> (Session, PathBuf, String, PathBuf, PathBuf) {
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
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    let site = dir.join("site");
    std::fs::create_dir_all(&site).expect("site");
    std::fs::write(site.join("server.js"), SERVER).expect("server");
    std::fs::write(
        site.join("broken.js"),
        "console.error('Cannot find module astro'); process.exit(1);\n",
    )
    .expect("broken");
    (session, root, manifest.id, project_dir, site)
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

fn set_command(root: &Path, project_dir: &Path, command: &str) {
    page::save(
        project_dir,
        root,
        &PageSettings {
            dev_command: Some(command.into()),
            ..PageSettings::default()
        },
    )
    .expect("page settings");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_start_read_and_stop_the_page_preview_server() {
    if find_on_path(&["node"], &ProbeEnv::from_system()).is_none() {
        return;
    }
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, root, project_id, project_dir, site) = fixture(dir.path());
    let studio = ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {});
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(maquette_core::pipeline::runner::NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio,
        preview: Arc::new(SitePreview::default()),
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
        .find(|t| t.name == "get_page_preview")
        .expect("get_page_preview");
    assert_eq!(
        read.annotations.as_ref().and_then(|a| a.read_only_hint),
        Some(true)
    );

    // Without a site repo there is nothing to run, and it says where to pick one.
    let none = call(
        &client,
        "get_page_preview",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(none["server"]["state"], json!("idle"));
    assert!(
        none["commandError"]
            .as_str()
            .is_some_and(|e| e.contains("Settings > Project")),
        "{none}"
    );
    let mut params = CallToolRequestParams::new("start_page_preview");
    params = params.with_arguments(
        json!({ "project": project_id })
            .as_object()
            .cloned()
            .expect("args"),
    );
    let refused = client.call_tool(params).await.expect("call");
    assert_eq!(refused.is_error, Some(true));

    // Link the site and set the dev command in the project's settings.
    let site_text = site.to_string_lossy().into_owned();
    project::update(
        &root,
        &WorkspaceConfig::default().layout,
        &project_id,
        "Study room".into(),
        Some(site_text.clone()),
    )
    .expect("site repo");
    set_command(&root, &project_dir, "node server.js");
    let ready = call(
        &client,
        "get_page_preview",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(ready["command"]["line"], json!("node server.js"));
    assert_eq!(ready["command"]["source"], json!("settings"));
    assert_eq!(ready["siteRepo"], json!(site_text));

    let started = call(
        &client,
        "start_page_preview",
        json!({ "project": project_id, "waitSeconds": 30 }),
    )
    .await;
    assert_eq!(started["server"]["state"], json!("running"), "{started}");
    assert_eq!(started["server"]["owner"], json!("agent"));
    let address = started["server"]["url"].as_str().expect("url").to_string();
    assert!(address.starts_with("http://127.0.0.1:"), "{address}");
    assert!(answers(&address));
    assert!(
        started["server"]["log"].as_array().is_some_and(|l| l
            .iter()
            .any(|line| line.as_str().is_some_and(|s| s.contains("Local")))),
        "{started}"
    );

    // A second start reuses it.
    let again = call(
        &client,
        "start_page_preview",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(again["server"]["url"], json!(address));

    let stopped = call(
        &client,
        "stop_page_preview",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(stopped["server"]["state"], json!("idle"));
    let deadline = Instant::now() + Duration::from_secs(5);
    while answers(&address) && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!answers(&address), "the server stopped");

    // A dev command that fails says why, with its output.
    set_command(&root, &project_dir, "node broken.js");
    let failed = call(
        &client,
        "start_page_preview",
        json!({ "project": project_id, "waitSeconds": 30 }),
    )
    .await;
    assert_eq!(failed["server"]["state"], json!("failed"), "{failed}");
    assert!(
        failed["server"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("Cannot find module astro")),
        "{failed}"
    );
    host.preview.stop();
    client.cancel().await.expect("close");
    server.stop();
}
