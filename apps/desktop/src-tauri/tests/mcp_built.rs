//! A real MCP client adds a built asset, builds it, edits its script and
//! builds it again, with a stand in for Blender that copies a fixture
//! model. Nothing reaches Codex, Tripo or Meshy, and no real Blender runs.
#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::{ResolvedTool, Toolbox};
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::toolchain::{HostOs, ToolId};
use maquette_core::{
    AppState, NewProject, ProjectManifest, ProjectTemplate, WorkspaceConfig, project, workspace,
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
    tools: Toolbox,
    built_changes: AtomicUsize,
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
        self.tools.clone()
    }
    fn project_changed(&self, _project: &ProjectManifest) {}
    fn built_changed(&self, _project: &str) {
        self.built_changes.fetch_add(1, Ordering::SeqCst);
    }
}

/// A Blender stand in: copies the clean fixture to `--output` (the messy
/// one when the staged script says "two") and fails on "boom".
fn fake_blender(dir: &Path) -> Toolbox {
    let fake = dir.join("fake");
    std::fs::create_dir_all(&fake).expect("fake");
    let one = fake.join("one.glb");
    let two = fake.join("two.glb");
    std::fs::write(&one, maquette_core::inbox::fixture::clean_prop()).expect("one");
    std::fs::write(&two, maquette_core::inbox::fixture::messy_prop()).expect("two");
    let script = format!(
        "#!/bin/sh\n\
         while [ $# -gt 0 ]; do case \"$1\" in --script) s=\"$2\"; shift;; --output) o=\"$2\"; shift;; --report) r=\"$2\"; shift;; esac; shift; done\n\
         if grep -q boom \"$s\"; then echo '[built] Error: NameError: boom'; exit 1; fi\n\
         if grep -q two \"$s\"; then cp '{two}' \"$o\"; else cp '{one}' \"$o\"; fi\n\
         echo '{{\"objects\": [\"laptop\", \"lid\", \"screen\"], \"materials\": [\"screen\"], \"triangles\": 12}}' > \"$r\"\n",
        one = one.display(),
        two = two.display(),
    );
    let program = fake.join("blender");
    std::fs::write(&program, script).expect("script");
    let mut perms = std::fs::metadata(&program).expect("meta").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&program, perms).expect("chmod");
    let mut tools = Toolbox {
        path_env: OsString::from("/bin:/usr/bin"),
        os: Some(HostOs::current()),
        ..Toolbox::default()
    };
    tools.tools.insert(
        ToolId::Blender,
        ResolvedTool {
            program,
            prefix: Vec::new(),
            version: "5.2.2".into(),
        },
    );
    tools
}

fn fixture(dir: &Path) -> (Session, String, PathBuf, PathBuf) {
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
    let pieces = dir.join("pieces");
    std::fs::create_dir_all(&pieces).expect("pieces");
    std::fs::write(
        pieces.join("laptop.py"),
        "import common\ndef build():\n    pass\n",
    )
    .expect("laptop");
    std::fs::write(pieces.join("common.py"), "PALETTE = 1\n").expect("common");
    (session, manifest.id, project_dir, pieces)
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
async fn agents_add_build_and_rebuild_a_built_asset() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, project_dir, pieces) = fixture(dir.path());
    let studio = ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {});
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(maquette_core::pipeline::runner::NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio,
        tools: fake_blender(dir.path()),
        built_changes: AtomicUsize::new(0),
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
    let info = client.peer_info().expect("server info");
    assert!(
        info.instructions
            .as_deref()
            .is_some_and(|i| i.contains("add_built_asset") && i.contains("rebuild_built_asset"))
    );
    let tools = client.list_all_tools().await.expect("tools");
    let listing = tools
        .iter()
        .find(|t| t.name == "list_built_assets")
        .expect("list tool");
    assert_eq!(
        listing.annotations.as_ref().and_then(|a| a.read_only_hint),
        Some(true)
    );

    // Add it: the files land in built/ and it builds into v1 at once.
    let added = call(
        &client,
        "add_built_asset",
        json!({
            "project": project_id,
            "script": pieces.join("laptop.py").display().to_string(),
            "helpers": [pieces.join("common.py").display().to_string()],
        }),
    )
    .await;
    assert_eq!(added["asset"]["id"], json!("laptop"));
    assert_eq!(added["asset"]["name"], json!("Laptop"));
    assert_eq!(added["build"]["result"], json!("built"));
    assert_eq!(added["build"]["version"], json!(1));
    assert_eq!(added["build"]["file"], json!("models/laptop_v1.glb"));
    assert_eq!(added["build"]["materials"], json!(["screen"]));
    assert_eq!(added["source"]["state"], json!("current"));
    assert_eq!(added["source"]["script"], json!("built/laptop.py"));
    assert!(project_dir.join("built/common.py").is_file());
    assert!(project_dir.join("models/laptop_v1.glb").is_file());
    assert!(host.built_changes.load(Ordering::SeqCst) >= 1);

    // Unchanged files answer at once.
    let again = call(
        &client,
        "rebuild_built_asset",
        json!({ "project": project_id, "asset": "Laptop" }),
    )
    .await;
    assert_eq!(again["builds"][0]["result"], json!("up-to-date"));

    // Edit the script in the project: it reads stale, then builds v2.
    std::fs::write(
        project_dir.join("built/laptop.py"),
        "import common\n# two\ndef build():\n    pass\n",
    )
    .expect("edit");
    let listed = call(
        &client,
        "list_built_assets",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(listed["assets"][0]["state"], json!("stale"));
    let rebuilt = call(
        &client,
        "rebuild_built_asset",
        json!({ "project": project_id, "all": true }),
    )
    .await;
    assert_eq!(rebuilt["builds"][0]["result"], json!("built"));
    assert_eq!(rebuilt["builds"][0]["version"], json!(2));

    // A failing script keeps v2 and says why.
    std::fs::write(project_dir.join("built/laptop.py"), "boom\n").expect("break");
    let failed = call(
        &client,
        "rebuild_built_asset",
        json!({ "project": project_id, "asset": "laptop" }),
    )
    .await;
    assert_eq!(failed["builds"][0]["result"], json!("failed"));
    assert_eq!(failed["builds"][0]["error"], json!("NameError: boom"));
    let assets: Value = call(&client, "list_assets", json!({ "project": project_id })).await;
    let listing_text = assets.to_string();
    assert!(listing_text.contains("laptop_v2.glb"), "{listing_text}");

    // A full run is refused while it fails.
    let refused = raw_call(&client, "run_pipeline", json!({ "project": project_id })).await;
    assert_eq!(refused.is_error, Some(true));
    assert!(
        text(&refused).contains("Laptop did not build"),
        "{}",
        text(&refused)
    );

    let missing = raw_call(
        &client,
        "rebuild_built_asset",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(missing.is_error, Some(true));

    client.cancel().await.expect("close");
    server.stop();
}
