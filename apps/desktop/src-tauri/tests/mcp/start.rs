//! A real MCP client starts from nothing: an empty temp workspace, then
//! get_started, create_project and the answers that lead on. Nothing reaches
//! Codex, Tripo, Meshy or Blender, and no credits exist to spend.

use std::path::Path;
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::providers::{MemoryStore, Options, ProviderId, ProviderJobs, SecretStore as _};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{
    AppState, ToolId, ToolReport, ToolStatus, WorkspaceConfig, project, workspace,
};
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

use super::support::{Client, TestHost, call, call_error, raw_call, text};

const SECRET: &str = "tsk_secret_value_never_shown_9876";

pub(super) fn session_in(
    dir: &Path,
    open: bool,
    rules: impl FnOnce(&mut WorkspaceConfig),
) -> Session {
    let session = Session::with_state(dir.join("cfg"), None, AppState::default());
    if open {
        let mut config = WorkspaceConfig::default();
        rules(&mut config);
        let root = dir.join("ws");
        let config = workspace::create(&root, &config).expect("workspace");
        session.set_workspace(root, config).expect("open");
    }
    session
}

pub(super) async fn serve(session: Session) -> (Client, Running, Arc<TestHost>) {
    maquette_core::providers::http::ensure_crypto();
    let secrets = Arc::new(MemoryStore::default());
    secrets.set(ProviderId::Tripo, SECRET).expect("key");
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(NoSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio: ProviderJobs::new(
            secrets,
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
    (client, server, host)
}

struct NoSink;
impl maquette_core::pipeline::runner::RunSink for NoSink {
    fn event(&self, _: maquette_core::pipeline::runner::PipelineEvent) {}
}

fn next_tools(answer: &Value) -> Vec<String> {
    answer["next"]
        .as_array()
        .expect("next")
        .iter()
        .filter_map(|s| s["tool"].as_str().map(str::to_string))
        .collect()
}

fn report(id: ToolId, status: ToolStatus) -> ToolReport {
    serde_json::from_value(json!({
        "id": id,
        "label": format!("{id:?}"),
        "required": true,
        "status": status,
        "source": null,
        "path": null,
        "version": null,
        "detail": null,
        "hint": "how to fix it",
        "warning": null,
    }))
    .expect("report")
}

#[tokio::test(flavor = "multi_thread")]
async fn with_no_workspace_get_started_asks_a_person_and_create_project_says_why() {
    let dir = tempfile::tempdir().expect("temp");
    let (client, server, _host) = serve(session_in(dir.path(), false, |_| {})).await;

    let started = call(&client, "get_started", json!({})).await;
    assert_eq!(started["workspace"], Value::Null);
    assert_eq!(started["next"][0]["person"], true);

    let refused = raw_call(
        &client,
        "create_project",
        json!({ "name": "Moon", "kind": "website" }),
    )
    .await;
    let body: Value = serde_json::from_str(&text(&refused)).expect("json");
    assert_eq!(body["code"], "invalid");
    assert_eq!(body["personOnly"], true);
    assert_eq!(body["tryTool"], "get_started");

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_starts_from_an_empty_workspace() {
    let dir = tempfile::tempdir().expect("temp");
    let site = dir.path().join("site");
    std::fs::create_dir_all(&site).expect("site");
    std::fs::create_dir_all(site.join(".git")).expect("git");
    let session = session_in(dir.path(), true, |_| {});
    session.set_tools(Some(vec![
        report(ToolId::Blender, ToolStatus::Found),
        report(ToolId::Codex, ToolStatus::Missing),
    ]));
    let (client, server, host) = serve(session).await;

    let started = call(&client, "get_started", json!({})).await;
    let raw = started.to_string();
    assert!(
        !raw.contains(SECRET) && !raw.contains("9876"),
        "a key leaked"
    );
    assert_eq!(started["keys"]["tripo"], true);
    assert_eq!(started["keys"]["meshy"], false);
    assert_eq!(started["keys"]["openai"], false);
    assert_eq!(started["askBeforeShipping"], true);
    assert_eq!(started["toolchain"][1]["status"], "missing");
    assert_eq!(started["toolchain"][1]["hint"], "how to fix it");
    assert_eq!(started["projects"], json!([]));
    assert_eq!(next_tools(&started), ["create_project"]);
    assert!(raw.contains("Install Codex CLI"), "{raw}");

    let listed = call(&client, "list_projects", json!({})).await;
    assert_eq!(next_tools(&listed), ["create_project"]);

    let made = call(
        &client,
        "create_project",
        json!({ "name": "Moon Island", "kind": "website", "template": "room" }),
    )
    .await;
    assert_eq!(made["project"]["id"], "moon-island");
    assert_eq!(made["project"]["template"], "room");
    assert_eq!(made["project"]["target"], "website");
    assert!(next_tools(&made).contains(&"set_ship_settings".to_string()));

    // The same core path as New project: the starters and the pipeline are there.
    let pipeline = call(&client, "get_pipeline", json!({ "project": "moon-island" })).await;
    assert!(
        pipeline["def"]["steps"]
            .as_array()
            .is_some_and(|s| !s.is_empty())
    );
    let (root, config) = host.session.workspace().expect("ws");
    let project_dir = project::dir(&root, &config.layout, "moon-island").expect("dir");
    assert!(project_dir.join("project.json").is_file());
    // It leaves the app's open project alone.
    assert_eq!(config.current_project, None);

    // A taken name is refused, by name and by id, and nothing is written.
    for name in ["Moon Island", "moon  island", "MOON-ISLAND"] {
        let refused = raw_call(
            &client,
            "create_project",
            json!({ "name": name, "kind": "website" }),
        )
        .await;
        assert_eq!(refused.is_error, Some(true), "{name}");
        let body: Value = serde_json::from_str(&text(&refused)).expect("json");
        assert_eq!(body["code"], "taken");
        assert_eq!(body["tryTool"], "list_projects");
    }
    assert_eq!(
        project::summaries(&root, &config.layout)
            .expect("list")
            .len(),
        1
    );

    // A missing repo and an unknown template are refused before anything is made.
    let missing = call_error(
        &client,
        "create_project",
        json!({ "name": "Sun", "kind": "website", "siteRepo": dir.path().join("nope") }),
    )
    .await;
    assert!(!missing.is_empty());
    let bad = call_error(
        &client,
        "create_project",
        json!({ "name": "Sun", "kind": "website", "template": "castle" }),
    )
    .await;
    assert!(bad.contains("not a website template"), "{bad}");
    assert_eq!(
        project::summaries(&root, &config.layout)
            .expect("list")
            .len(),
        1
    );

    // A game with its repo.
    let game = call(
        &client,
        "create_project",
        json!({ "name": "Frost", "kind": "game", "template": "prop-set", "siteRepo": site }),
    )
    .await;
    assert_eq!(game["project"]["target"], "game");
    assert_eq!(game["project"]["template"], "game-import");
    assert!(!next_tools(&game).contains(&"set_game_settings".to_string()));

    // get_started now names both, and leads on from the empty project.
    let started = call(&client, "get_started", json!({})).await;
    assert_eq!(started["projects"][1]["stage"], "empty");
    assert_eq!(started["projects"][1]["target"], "game");
    assert_eq!(
        next_tools(&started),
        ["add_built_asset", "set_ship_settings"]
    );
    assert_eq!(started["next"][0]["person"], true);

    // Empty answers lead on, and a run that never happened says how to start one.
    let assets = call(&client, "list_assets", json!({ "project": "moon-island" })).await;
    assert_eq!(next_tools(&assets), ["start_concepts", "add_built_asset"]);
    let inbox = call(&client, "list_inbox", json!({})).await;
    assert_eq!(next_tools(&inbox)[0], "list_handoffs");
    let lab = call(&client, "get_concepts", json!({ "project": "moon-island" })).await;
    assert_eq!(next_tools(&lab), ["start_concepts"]);
    let no_run = raw_call(&client, "get_run", json!({ "project": "moon-island" })).await;
    let body: Value = serde_json::from_str(&text(&no_run)).expect("json");
    assert_eq!(body["tryTool"], "run_pipeline");

    // The code of a core error comes through.
    let unknown = raw_call(&client, "get_project", json!({ "project": "nope" })).await;
    let body: Value = serde_json::from_str(&text(&unknown)).expect("json");
    assert!(
        body["code"].is_string() && body["code"] != "refused",
        "{body}"
    );

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_ship_settings_sets_and_clears_a_website_repo() {
    let dir = tempfile::tempdir().expect("temp");
    let site = dir.path().join("site");
    std::fs::create_dir_all(site.join("public")).expect("site");
    std::fs::create_dir_all(site.join(".git")).expect("git");
    let (client, server, host) = serve(session_in(dir.path(), true, |_| {})).await;
    call(
        &client,
        "create_project",
        json!({ "name": "Web", "kind": "website" }),
    )
    .await;
    call(
        &client,
        "create_project",
        json!({ "name": "Game", "kind": "game" }),
    )
    .await;
    let (root, config) = host.session.workspace().expect("ws");
    let repo = |id: &str| {
        project::load(&root, &config.layout, id)
            .expect("load")
            .site_repo
    };

    call(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": site }),
    )
    .await;
    assert_eq!(repo("web"), Some(site.display().to_string()));

    // A bad word changes nothing, the repo included.
    call_error(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": null, "textures": "png" }),
    )
    .await;
    assert_eq!(repo("web"), Some(site.display().to_string()));

    let missing = call_error(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": dir.path().join("nope") }),
    )
    .await;
    assert!(!missing.is_empty());
    assert_eq!(repo("web"), Some(site.display().to_string()));

    call(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": null }),
    )
    .await;
    assert_eq!(repo("web"), None);

    let game = call_error(
        &client,
        "set_ship_settings",
        json!({ "project": "game", "siteRepo": site }),
    )
    .await;
    assert!(!game.is_empty());
    assert_eq!(repo("game"), None);

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_avatar_and_rig_tools_leave_when_the_switch_is_off() {
    let dir = tempfile::tempdir().expect("temp");
    let session = session_in(dir.path(), true, |c| {
        c.agents.rules.avatar_rig_tools = false
    });
    let (client, server, _host) = serve(session).await;
    let names: Vec<String> = client
        .list_all_tools()
        .await
        .expect("tools")
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    assert!(names.iter().any(|n| n == "get_started"));
    for gone in ["get_avatar", "build_avatar", "start_rig_doctor"] {
        assert!(!names.iter().any(|n| n == gone), "{gone}");
    }
    client.cancel().await.expect("close");
    server.stop();
}

#[test]
fn an_older_workspace_file_keeps_the_avatar_and_rig_tools() {
    let rules: maquette_core::AgentRules = serde_json::from_value(json!({
        "blenderHeadlessOnly": true,
        "neverCaptureScreen": true,
        "askBeforeShipping": true,
    }))
    .expect("rules");
    assert!(rules.avatar_rig_tools);
}
