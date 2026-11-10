//! A real MCP client sends the same asset from two projects. Job ids are
//! numbered per project, so both sends are `lantern-1`: the tools that take
//! a job id reach the send of the project they are given, an id two projects
//! have is refused without one (with what to pass), and an approval or a
//! cancel in one project leaves the other's send alone. Everything happens
//! in a temp workspace; both providers point at a closed local port and no
//! send is ever approved.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::providers::{
    Endpoints, JobState, MemoryStore, Options, ProviderId, ProviderJobs, SecretStore as _,
};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{
    AppState, AssetKind, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, assets,
    project, workspace,
};
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::{ApprovalKind, Hub};
use maquette_desktop_lib::session::Session;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::support::{TestHost, call, raw_call, text};

const HARBOUR: &str = "harbour";
const LIGHTHOUSE: &str = "lighthouse";

/// A workspace with two projects, each with an asset named Lantern and a
/// front ref.
fn fixture(dir: &Path) -> Session {
    let root = dir.join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let session = Session::with_state(dir.join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    for name in ["Harbour", "Lighthouse"] {
        let manifest = project::create(
            &root,
            &config.layout,
            NewProject {
                name: name.into(),
                template: ProjectTemplate::FloatingDiorama,
                target: Default::default(),
                game: None,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        let refs = project::dir(&root, &config.layout, &manifest.id)
            .expect("dir")
            .join("refs");
        std::fs::create_dir_all(&refs).expect("refs");
        std::fs::write(refs.join("front.png"), b"\x89PNG\r\n\x1a\nfake").expect("ref");
        project::modify(&root, &config.layout, &manifest.id, |m| {
            let asset = assets::add(
                m,
                NewAsset {
                    name: "Lantern".into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Refs,
                },
            )?;
            assert_eq!(asset.id, "lantern");
            Ok(())
        })
        .expect("lantern");
    }
    session
}

/// Provider jobs that can never reach Tripo or Meshy.
fn studio(session: &Session) -> ProviderJobs {
    let secrets = Arc::new(MemoryStore::default());
    secrets
        .set(ProviderId::Tripo, "tsk_test_only_1234")
        .expect("key");
    let options = Options {
        endpoints: Endpoints {
            tripo: "http://127.0.0.1:9".into(),
            meshy: "http://127.0.0.1:9".into(),
        },
        ..Options::default()
    };
    let jobs = ProviderJobs::new(secrets, options, maquette_core::inbox::send_ports(), |_| {});
    let (root, config) = session.workspace().expect("workspace");
    jobs.open_workspace(&root, &config);
    jobs
}

fn send(project: &str, provider: &str) -> Value {
    json!({
        "project": project,
        "asset": "lantern",
        "provider": provider,
        "images": ["refs/front.png"],
    })
}

/// The body of a tool call that must fail.
async fn refused(client: &super::support::Client, tool: &str, args: Value) -> Value {
    let result = raw_call(client, tool, args).await;
    assert_eq!(result.is_error, Some(true), "{tool} should fail");
    serde_json::from_str(&text(&result)).expect("an error body")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_job_id_reaches_the_send_of_the_project_it_is_given() {
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
    let job = |project: &str, id: &str| json!({ "project": project, "id": id });
    let state = |project: &str, id: &str| host.studio.get(project, id).map(|j| j.state);

    // Each project's first paid send is its number one, under its own approval.
    let mine = call(&client, "send_to_3d", send(HARBOUR, "tripo")).await;
    let theirs = call(&client, "send_to_3d", send(LIGHTHOUSE, "tripo")).await;
    assert_eq!(mine["id"], json!("lantern-1"), "{mine}");
    assert_eq!(theirs["id"], json!("lantern-1"), "{theirs}");
    assert_eq!(mine["project"], json!(HARBOUR));
    assert_eq!(theirs["project"], json!(LIGHTHOUSE));
    for sent in [&mine, &theirs] {
        assert_eq!(sent["status"], json!("waiting for approval"), "{sent}");
    }
    let approvals = hub.approvals();
    assert_eq!(approvals.len(), 2, "one prompt per send");
    assert!(approvals.iter().all(|a| a.kind == ApprovalKind::SendTo3d));
    let approval_of = |project: &str| {
        approvals
            .iter()
            .find(|a| a.project == project)
            .expect("its approval")
            .id
            .clone()
    };
    assert_ne!(mine["approval"]["id"], theirs["approval"]["id"]);
    assert_eq!(mine["approval"]["id"], json!(approval_of(HARBOUR)));
    assert_eq!(theirs["approval"]["id"], json!(approval_of(LIGHTHOUSE)));

    // The id alone is two sends: refused, with the projects and what to pass.
    let both = refused(&client, "get_provider_job", json!({ "id": "lantern-1" })).await;
    assert_eq!(both["code"], json!("job-in-several-projects"), "{both}");
    let message = both["error"].as_str().expect("error");
    assert!(
        message.contains("harbour, lighthouse") && message.contains("Pass project"),
        "{message}"
    );
    assert!(
        both["fix"].as_str().is_some_and(|f| f.contains("project")),
        "{both}"
    );
    assert_eq!(both["tryTool"], json!("get_provider_job"));

    // With its project, each is its own send, waiting for its own approval.
    for (project, sent) in [(HARBOUR, &mine), (LIGHTHOUSE, &theirs)] {
        let read = call(&client, "get_provider_job", job(project, "lantern-1")).await;
        assert_eq!(read["project"], json!(project), "{read}");
        assert_eq!(read["status"], json!("waiting for approval"));
        assert_eq!(read["approval"]["id"], sent["approval"]["id"]);
    }
    let nowhere = refused(&client, "get_provider_job", job("moon", "lantern-1")).await;
    assert_eq!(
        nowhere["error"],
        json!("There is no send lantern-1 in moon.")
    );

    // An agent cannot cancel a send that waits for approval, in either project.
    let held = call(
        &client,
        "cancel_provider_jobs",
        json!({ "ids": ["lantern-1"], "project": HARBOUR }),
    )
    .await;
    assert_eq!(held["refused"], json!(1), "{held}");
    assert!(
        held["results"][0]["error"]
            .as_str()
            .is_some_and(|e| e.contains("waiting for approval")),
        "{held}"
    );

    // A no in the app to the first project's send leaves the second waiting.
    assert!(hub.decide(&approval_of(HARBOUR), false));
    let deadline = Instant::now() + Duration::from_secs(20);
    while state(HARBOUR, "lantern-1") == Some(JobState::Approval) {
        assert!(Instant::now() < deadline, "the denial never landed");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(state(HARBOUR, "lantern-1"), Some(JobState::Cancelled));
    assert_eq!(state(LIGHTHOUSE, "lantern-1"), Some(JobState::Approval));
    let left = hub.approvals();
    assert_eq!(left.len(), 1, "the other project's prompt is still up");
    assert_eq!(left[0].project, LIGHTHOUSE);
    let waiting = call(&client, "get_provider_job", job(LIGHTHOUSE, "lantern-1")).await;
    assert_eq!(
        waiting["status"],
        json!("waiting for approval"),
        "{waiting}"
    );
    let denied = call(&client, "get_provider_job", job(HARBOUR, "lantern-1")).await;
    assert_eq!(denied["status"], json!("cancelled"), "{denied}");
    assert_eq!(
        denied["message"],
        json!("Denied in Maquette. Nothing was spent.")
    );
    assert!(hub.decide(&left[0].id, false));
    let deadline = Instant::now() + Duration::from_secs(20);
    while state(LIGHTHOUSE, "lantern-1") == Some(JobState::Approval) {
        assert!(Instant::now() < deadline, "the denial never landed");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // By hand: each project's send is its number two, with a name of its own.
    let mine = call(&client, "send_to_3d", send(HARBOUR, "by-hand")).await;
    let theirs = call(&client, "send_to_3d", send(LIGHTHOUSE, "by-hand")).await;
    assert_eq!(mine["id"], json!("lantern-2"), "{mine}");
    assert_eq!(theirs["id"], json!("lantern-2"), "{theirs}");
    assert_eq!(mine["exportName"], json!("maq_harbour_lantern_v2_tripo"));
    assert_eq!(
        theirs["exportName"],
        json!("maq_lighthouse_lantern_v2_tripo")
    );
    for sent in [&mine, &theirs] {
        let name = sent["exportName"].as_str().expect("name");
        let note = sent["note"].as_str().expect("note");
        assert!(note.contains(&format!("{name}.glb")), "{note}");
        assert!(
            !note.contains("lantern-2.glb"),
            "the id alone names two sends: {note}"
        );
    }
    let handoffs = call(&client, "list_handoffs", json!({})).await;
    let mut pending: Vec<(String, String, String)> = handoffs["pending"]
        .as_array()
        .expect("pending")
        .iter()
        .map(|p| {
            let field = |key: &str| p[key].as_str().expect(key).to_string();
            (field("project"), field("job"), field("expectedName"))
        })
        .collect();
    pending.sort();
    assert_eq!(
        pending,
        [
            (
                HARBOUR.to_string(),
                "lantern-2".to_string(),
                "maq_harbour_lantern_v2_tripo".to_string()
            ),
            (
                LIGHTHOUSE.to_string(),
                "lantern-2".to_string(),
                "maq_lighthouse_lantern_v2_tripo".to_string()
            ),
        ]
    );

    // Stop waiting without a project: refused for the id two projects have,
    // and nothing is stopped.
    let unsure = call(
        &client,
        "cancel_provider_jobs",
        json!({ "ids": ["lantern-2"] }),
    )
    .await;
    assert_eq!(
        (&unsure["stopped"], &unsure["refused"]),
        (&json!(0), &json!(1))
    );
    assert_eq!(
        unsure["results"][0]["code"],
        json!("job-in-several-projects"),
        "{unsure}"
    );
    assert!(unsure["results"][0]["fix"].is_string(), "{unsure}");
    assert_eq!(state(HARBOUR, "lantern-2"), Some(JobState::Waiting));
    assert_eq!(state(LIGHTHOUSE, "lantern-2"), Some(JobState::Waiting));

    // With the project, that project's send stops and the other still waits.
    let stopped = call(
        &client,
        "cancel_provider_jobs",
        json!({ "ids": ["lantern-2"], "project": LIGHTHOUSE }),
    )
    .await;
    assert_eq!(stopped["stopped"], json!(1), "{stopped}");
    assert_eq!(stopped["results"][0]["project"], json!(LIGHTHOUSE));
    assert_eq!(state(LIGHTHOUSE, "lantern-2"), Some(JobState::Cancelled));
    assert_eq!(state(HARBOUR, "lantern-2"), Some(JobState::Waiting));

    // An id only one project has still needs no project.
    let third = call(&client, "send_to_3d", send(HARBOUR, "by-hand")).await;
    assert_eq!(third["id"], json!("lantern-3"), "{third}");
    let alone = call(&client, "get_provider_job", json!({ "id": "lantern-3" })).await;
    assert_eq!(alone["project"], json!(HARBOUR), "{alone}");
    let done = call(
        &client,
        "cancel_provider_jobs",
        json!({ "ids": ["lantern-3"] }),
    )
    .await;
    assert_eq!(done["stopped"], json!(1), "{done}");
    assert_eq!(done["results"][0]["project"], json!(HARBOUR));

    client.cancel().await.expect("close");
    server.stop();
}
