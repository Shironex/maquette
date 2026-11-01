//! Page preview, ask first: a real MCP client on a repo an agent set. The
//! dev command is a tiny Node server in a temp folder; a person's answer goes
//! through the hub, as the app's card does.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::site::preview::{SitePreview, answers};
use maquette_core::thumbs::Thumbnails;
use maquette_core::toolchain::find_on_path;
use maquette_core::{ProbeEnv, app_state, project::repos};
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::{ApprovalKind, Hub};
use maquette_desktop_lib::mcp::server::Running;
use maquette_desktop_lib::session::Session;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::page::{TestHost, fixture, set_command};
use super::support::{Client, call, raw_call, text};

fn make_git(site: &Path) {
    std::fs::create_dir_all(site.join(".git")).expect("git dir");
    std::fs::write(site.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("head");
}

/// The tools over `session`, with the hub a person answers through.
async fn serve(session: Session) -> (Arc<Hub>, Arc<TestHost>, Running, Client) {
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
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
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(format!(
            "http://127.0.0.1:{}/mcp",
            server.port()
        ))
        .auth_header(token),
    );
    let client: Client = ClientConfig::default()
        .serve(transport)
        .await
        .expect("initialize");
    (hub, host, server, client)
}

async fn poll_state(client: &Client, project: &str, want: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let read = call(client, "get_page_preview", json!({ "project": project })).await;
        if read["server"]["state"] == json!(want) {
            return read;
        }
        assert!(Instant::now() < deadline, "never {want}: {read}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agents_repo_waits_for_a_person_before_its_dev_command_runs() {
    if find_on_path(&["node"], &ProbeEnv::from_system()).is_none() {
        return;
    }
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, root, id, project_dir, site) = fixture(dir.path());
    make_git(&site);
    let (hub, host, server, client) = serve(session).await;

    // The agent sets the repo; the mark is on this computer's state.
    let set = call(
        &client,
        "set_ship_settings",
        json!({ "project": id, "siteRepo": site.to_string_lossy() }),
    )
    .await;
    assert!(set.get("error").is_none(), "{set}");
    assert!(
        host.session
            .app_state()
            .repo_set_by_agent(&repos::root_key(&root), &id)
    );
    set_command(&root, &project_dir, "node server.js");

    // Waiting, in the shape of the other approvals, with nothing running.
    let waiting = call(&client, "start_page_preview", json!({ "project": id })).await;
    assert_eq!(
        waiting["status"],
        json!("waiting for approval"),
        "{waiting}"
    );
    assert_eq!(waiting["pollEverySeconds"], json!(5));
    assert!(
        waiting["whereToApprove"]
            .as_str()
            .is_some_and(|w| w.contains("approval prompt"))
    );
    assert_eq!(waiting["approval"]["command"], json!("node server.js"));
    assert_eq!(waiting["server"]["state"], json!("idle"));
    let card = waiting["approval"]["id"].as_str().expect("id").to_string();
    let cards = hub.approvals();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].kind, ApprovalKind::PagePreview);
    let ask = cards[0].preview.clone().expect("ask");
    assert_eq!(ask.command, "node server.js");

    // Polling either tool neither runs it nor raises a second card.
    let again = call(&client, "start_page_preview", json!({ "project": id })).await;
    assert_eq!(again["approval"]["id"], json!(card));
    let read = call(&client, "get_page_preview", json!({ "project": id })).await;
    assert_eq!(read["status"], json!("waiting for approval"));
    assert_eq!(read["pollEverySeconds"], json!(5));
    assert_eq!(read["server"]["state"], json!("idle"));
    assert_eq!(hub.approvals().len(), 1);

    // A yes starts it, and the agent's server stays the agent's.
    assert!(hub.decide(&card, true));
    let running = poll_state(&client, &id, "running").await;
    assert_eq!(running["server"]["owner"], json!("agent"));
    assert!(running.get("status").is_none(), "{running}");
    assert!(answers(running["server"]["url"].as_str().expect("url")));
    call(&client, "stop_page_preview", json!({ "project": id })).await;

    // Remembered on disk, so a restart asks nothing, and this start does not.
    let saved = app_state::load(&dir.path().join("cfg"));
    assert!(saved.preview_approved(&ask.repo, &ask.key()));
    assert!(saved.repo_set_by_agent(&repos::root_key(&root), &id));
    let second = call(&client, "start_page_preview", json!({ "project": id })).await;
    assert_eq!(second["server"]["state"], json!("running"), "{second}");
    host.preview.stop();

    // A changed command asks again, and a no leaves it stopped with an error.
    set_command(&root, &project_dir, "node server.js --changed");
    let changed = call(&client, "start_page_preview", json!({ "project": id })).await;
    assert_eq!(changed["status"], json!("waiting for approval"));
    let card = changed["approval"]["id"].as_str().expect("id").to_string();
    assert!(hub.decide(&card, false));
    let deadline = Instant::now() + Duration::from_secs(5);
    let refused = loop {
        let answer = raw_call(&client, "start_page_preview", json!({ "project": id })).await;
        if answer.is_error == Some(true) {
            break answer;
        }
        // The answer may land before the waiting thread has noted it.
        let body: Value = serde_json::from_str(&text(&answer)).expect("json");
        if let Some(raised) = body["approval"]["id"].as_str() {
            assert_eq!(raised, card, "no new card for a question already answered");
        }
        assert!(Instant::now() < deadline, "never refused");
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let body: Value = serde_json::from_str(&text(&refused)).expect("json error");
    assert_eq!(body["code"], json!("preview-refused"), "{body}");
    assert_eq!(body["personOnly"], json!(true));
    assert!(body["fix"].as_str().is_some_and(|f| !f.is_empty()));
    assert!(
        body["whereToApprove"]
            .as_str()
            .is_some_and(|w| w.contains("approval prompt"))
    );
    let read = call(&client, "get_page_preview", json!({ "project": id })).await;
    assert_eq!(read["server"]["state"], json!("idle"));
    assert_eq!(read["approval"]["refused"], json!(true));
    assert!(hub.approvals().is_empty());

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_project_and_set_game_settings_mark_their_repo_as_an_agents() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, root, _, _, site) = fixture(dir.path());
    make_git(&site);
    let (_, host, server, client) = serve(session).await;
    let key = repos::root_key(&root);
    let marked = |id: &str| host.session.app_state().repo_set_by_agent(&key, id);

    let site_text = site.to_string_lossy().to_string();
    let web = call(
        &client,
        "create_project",
        json!({ "name": "With repo", "kind": "website", "siteRepo": site_text }),
    )
    .await;
    let web_id = web["project"]["id"].as_str().expect("id").to_string();
    assert!(marked(&web_id), "{web}");

    let bare = call(
        &client,
        "create_project",
        json!({ "name": "No repo", "kind": "website" }),
    )
    .await;
    assert!(!marked(bare["project"]["id"].as_str().expect("id")));

    let game = call(
        &client,
        "create_project",
        json!({ "name": "A game", "kind": "game" }),
    )
    .await;
    let game_id = game["project"]["id"].as_str().expect("id").to_string();
    assert!(!marked(&game_id));
    let set = call(
        &client,
        "set_game_settings",
        json!({ "project": game_id, "repo": site_text }),
    )
    .await;
    assert!(set.get("error").is_none(), "{set}");
    assert!(marked(&game_id));

    client.cancel().await.expect("close");
    server.stop();
}
