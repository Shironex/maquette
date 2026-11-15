//! A new session that calls only get_started sees what is under way: two
//! projects each send the same asset to Tripo, and get_started lists both
//! approvals and both projects' sends with the tool that follows each, while
//! next, for the project asked about, follows the send instead of starting
//! the work again. Both providers point at a closed local port and no send
//! is ever approved.

use std::sync::Arc;
use std::time::{Duration, Instant};

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::providers::JobState;
use maquette_core::thumbs::Thumbnails;
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::Hub;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::sends_projects::{fixture, send, studio};
use super::support::{TestHost, call, raw_call, text};

fn tools(steps: &Value) -> Vec<&str> {
    steps
        .as_array()
        .expect("steps")
        .iter()
        .filter_map(|s| s["tool"].as_str())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_started_lists_what_waits_and_next_follows_the_send() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let session = fixture(dir.path());
    let host = Arc::new(TestHost {
        studio: studio(&session),
        session,
        runner: PipelineRunner::new(Arc::new(NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
    });
    let hub = Arc::new(Hub::new(|| {}));
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

    // Before any send, next for the asked project leads to its concepts.
    let idle = call(&client, "get_started", json!({ "project": "harbour" })).await;
    assert_eq!(idle["waiting"], json!([]));
    assert_eq!(idle["nextFor"], "harbour");
    assert!(tools(&idle["next"]).contains(&"send_to_3d"), "{idle}");

    // A paid send answers with the step that follows it, from the table.
    let sent = call(&client, "send_to_3d", send("harbour", "tripo")).await;
    assert_eq!(tools(&sent["next"]), ["get_provider_job"], "{sent}");
    assert_eq!(
        sent["next"][0]["args"],
        json!({ "project": "harbour", "id": "lantern-1" })
    );
    call(&client, "send_to_3d", send("lighthouse", "tripo")).await;

    // The short answer lists both approvals and both projects' sends.
    let started = call(&client, "get_started", json!({ "project": "lighthouse" })).await;
    let waiting = started["waiting"].as_array().expect("waiting");
    let approvals: Vec<&Value> = waiting
        .iter()
        .filter(|w| w["kind"] == "send-to-3d")
        .collect();
    assert_eq!(approvals.len(), 2, "{waiting:?}");
    for project in ["harbour", "lighthouse"] {
        let approval = approvals
            .iter()
            .find(|w| w["project"] == project)
            .expect("its approval");
        assert_eq!(approval["poll"]["tool"], "get_provider_job");
        assert_eq!(
            approval["poll"]["args"],
            json!({ "project": project, "id": "lantern-1" })
        );
        assert!(approval["approval"].is_string() && approval["ageSeconds"].is_u64());
        let sends = waiting
            .iter()
            .find(|w| w["kind"] == "sends" && w["project"] == project)
            .expect("its sends");
        assert_eq!(sends["waitingForApproval"], 1);
    }
    // next is about the asked project, follows its send and names it.
    assert_eq!(started["nextFor"], "lighthouse");
    let next = tools(&started["next"]);
    assert_eq!(next[0], "get_provider_job", "{next:?}");
    assert!(!next.contains(&"get_concepts") && !next.contains(&"pick_concepts"));
    let follow = started["next"]
        .as_array()
        .expect("next")
        .iter()
        .find(|s| s["tool"] == "get_provider_job")
        .expect("follow");
    assert_eq!(follow["args"]["project"], "lighthouse");

    // An unknown project is refused.
    let unknown = raw_call(&client, "get_started", json!({ "project": "moon" })).await;
    assert_eq!(unknown.is_error, Some(true));
    assert!(text(&unknown).contains("list_projects"));

    // A denied send leads to sending again.
    for approval in hub.approvals() {
        assert!(hub.decide(&approval.id, false));
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    while ["harbour", "lighthouse"]
        .iter()
        .any(|p| host.studio.get(p, "lantern-1").map(|j| j.state) == Some(JobState::Approval))
    {
        assert!(Instant::now() < deadline, "the denial never landed");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let denied = call(
        &client,
        "get_provider_job",
        json!({ "project": "harbour", "id": "lantern-1" }),
    )
    .await;
    assert_eq!(denied["status"], "cancelled");
    assert_eq!(tools(&denied["next"]), ["send_to_3d"]);
    let after = call(&client, "get_started", json!({})).await;
    assert_eq!(
        after["waiting"],
        json!([]),
        "nothing waits once both are denied"
    );

    client.cancel().await.expect("close");
    server.stop();
}
