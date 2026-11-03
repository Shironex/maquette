//! The tool groups a person turns off in Settings > Agents: a real client
//! lists only the groups that are on, a call to an off group's tool is
//! refused and runs nothing, the core groups stay, and an older workspace
//! file's avatar and rig switch carries over.

use maquette_core::{AppState, WorkspaceConfig, workspace};
use maquette_desktop_lib::session::Session;
use serde_json::{Value, json};

use super::start::{serve, session_in};
use super::support::{call, raw_call, text};

fn off(ids: &[&str]) -> impl FnOnce(&mut WorkspaceConfig) {
    let ids: Vec<String> = ids.iter().map(ToString::to_string).collect();
    move |c| c.agents.rules.tool_groups_off = ids
}

async fn names(client: &super::support::Client) -> Vec<String> {
    client
        .list_all_tools()
        .await
        .expect("tools")
        .iter()
        .map(|t| t.name.to_string())
        .collect()
}

fn has(names: &[String], tool: &str) -> bool {
    names.iter().any(|n| n == tool)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_off_group_leaves_the_list_and_its_tools_run_nothing() {
    let dir = tempfile::tempdir().expect("temp");
    let session = session_in(dir.path(), true, off(&["plan", "avatar-rig"]));
    let (client, server, host) = serve(session).await;

    let listed = names(&client).await;
    for kept in ["get_started", "list_projects", "send_to_3d", "list_library"] {
        assert!(has(&listed, kept), "{kept}");
    }
    for gone in [
        "create_canvas",
        "list_canvases",
        "get_avatar",
        "run_rig_doctor",
    ] {
        assert!(!has(&listed, gone), "{gone}");
    }

    call(
        &client,
        "create_project",
        json!({ "name": "Moon island", "kind": "website" }),
    )
    .await;
    let refused = raw_call(
        &client,
        "create_canvas",
        json!({ "project": "moon-island", "name": "Hero ideas" }),
    )
    .await;
    assert_eq!(refused.is_error, Some(true));
    let body: Value = serde_json::from_str(&text(&refused)).expect("json");
    assert_eq!(body["code"], "group-off");
    assert_eq!(body["personOnly"], true);
    let message = body["error"].as_str().expect("message");
    assert!(message.contains("plan tool group"), "{message}");
    assert!(message.contains("Settings > Agents"), "{message}");
    // Nothing ran: the project has no planning canvas folder.
    let (root, _) = host.session.workspace().expect("ws");
    assert!(!root.join("projects/moon-island/plans").exists());

    let started = call(&client, "get_started", json!({})).await;
    let line = started["toolGroupsOff"].as_str().expect("toolGroupsOff");
    assert!(
        line.contains("plan (") && line.contains("avatar-rig (7)"),
        "{line}"
    );

    // On again: the same session lists and runs the group, no reconnect.
    host.session
        .update_config(|c| c.agents.rules.tool_groups_off.clear())
        .expect("save");
    assert!(has(&names(&client).await, "create_canvas"));
    let made = call(
        &client,
        "create_canvas",
        json!({ "project": "moon-island", "name": "Hero ideas" }),
    )
    .await;
    assert!(
        made.get("canvas").is_some() || made.get("id").is_some(),
        "{made}"
    );
    let started = call(&client, "get_started", json!({})).await;
    assert!(started.get("toolGroupsOff").is_none());

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_core_groups_stay_listed_whatever_the_file_says() {
    let dir = tempfile::tempdir().expect("temp");
    let session = session_in(dir.path(), true, off(&["start", "project", "nope"]));
    let (client, server, _host) = serve(session).await;
    let listed = names(&client).await;
    for tool in [
        "get_started",
        "create_project",
        "list_projects",
        "get_project",
    ] {
        assert!(has(&listed, tool), "{tool}");
    }
    assert_eq!(
        listed.len(),
        maquette_desktop_lib::mcp::tools::McpTools::tool_names().len()
    );
    let started = call(&client, "get_started", json!({})).await;
    assert!(started.get("toolGroupsOff").is_none());
    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_older_workspace_file_with_the_switch_off_keeps_the_avatar_tools_off() {
    let dir = tempfile::tempdir().expect("temp");
    let root = dir.path().join("ws");
    workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    // The file as the version before the groups wrote it.
    let file = root.join("maquette.json");
    let mut saved: Value =
        serde_json::from_slice(&std::fs::read(&file).expect("read")).expect("json");
    let rules = &mut saved["agents"]["rules"];
    rules
        .as_object_mut()
        .expect("rules")
        .remove("toolGroupsOff");
    rules["avatarRigTools"] = json!(false);
    std::fs::write(&file, serde_json::to_vec_pretty(&saved).expect("text")).expect("write");

    let config = workspace::open(&root).expect("open");
    assert_eq!(config.agents.rules.tool_groups_off, vec!["avatar-rig"]);
    let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
    session.set_workspace(root.clone(), config).expect("set");
    // A settings save writes the group list and drops the old switch.
    session.update_config(|_| {}).expect("save");
    let text = std::fs::read_to_string(&file).expect("read");
    assert!(!text.contains("avatarRigTools"), "{text}");
    assert!(text.contains("avatar-rig"), "{text}");

    let (client, server, _host) = serve(session).await;
    let listed = names(&client).await;
    assert!(!has(&listed, "build_avatar"));
    assert!(has(&listed, "get_started"));
    client.cancel().await.expect("close");
    server.stop();
}
