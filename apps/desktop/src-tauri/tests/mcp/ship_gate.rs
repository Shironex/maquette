//! Ship into a repo an agent chose waits for a person whatever "Ask before
//! shipping" says: a real MCP client on a workspace with that rule off. The
//! fixture's Ship is one shell step that notes the run in the repo it ships
//! into, so a run that waits never starts it. Temp folders only.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::{PipelineEvent, PipelineRunner, RunSink};
use maquette_core::pipeline::{self, StepDef, StepKind};
use maquette_core::project;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::{ApprovalKind, Hub};
use maquette_desktop_lib::mcp::server::Running;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::start::session_in;
use super::support::{Client, TestHost, call};

/// Feeds runner events to the hub, the way the app's sink does.
struct HubSink(Arc<Hub>);

impl RunSink for HubSink {
    fn event(&self, event: PipelineEvent) {
        self.0.observe(&event);
    }
}

struct World {
    dir: tempfile::TempDir,
    /// What a person answers through, as the app's card does.
    hub: Arc<Hub>,
    host: Arc<TestHost>,
    server: Running,
    client: Client,
}

fn git_repo(dir: &Path, name: &str) -> PathBuf {
    let repo = dir.join(name);
    std::fs::create_dir_all(repo.join(".git")).expect("repo");
    repo
}

/// The tools over a new workspace with "Ask before shipping" off, and a
/// website project `web` whose pipeline is the one Ship step.
async fn rule_off() -> World {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp");
    let session = session_in(dir.path(), true, |config| {
        config.agents.rules.ask_before_shipping = false;
    });
    let hub = Arc::new(Hub::new(|| {}));
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(HubSink(Arc::clone(&hub)))),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio: ProviderJobs::new(
            Arc::new(MemoryStore::default()),
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
        Arc::clone(&hub),
    )
    .expect("start");
    let url = format!("http://127.0.0.1:{}/mcp", server.port());
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(url).auth_header(token),
    );
    let client: Client = ClientConfig::default()
        .serve(transport)
        .await
        .expect("initialize");
    call(
        &client,
        "create_project",
        json!({ "name": "Web", "target": "website" }),
    )
    .await;

    let (root, config) = host.session.workspace().expect("ws");
    let web = project::dir(&root, &config.layout, "web").expect("dir");
    let manifest = project::load(&root, &config.layout, "web").expect("load");
    let mut def = pipeline::ensure(&web, &manifest).expect("pipeline");
    def.steps = vec![StepDef {
        id: "ship".into(),
        name: "Ship".into(),
        kind: StepKind::Shell,
        command: Some("/bin/sh".into()),
        script: Some("ship.sh".into()),
        args: vec!["{script}".into(), "{siteRepo}".into()],
        inputs: Vec::new(),
        outputs: Vec::new(),
        depends_on: Vec::new(),
        compress: None,
    }];
    pipeline::save(&web, &def).expect("save");
    std::fs::write(web.join("ship.sh"), "echo shipped >> \"$1/shipped.log\"\n").expect("script");
    World {
        dir,
        hub,
        host,
        server,
        client,
    }
}

impl World {
    async fn read(&self, run: &Value) -> Value {
        let args = json!({ "project": "web", "runId": run["runId"] });
        call(&self.client, "get_run", args).await
    }

    async fn finished(&self, run: &Value) -> Value {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let read = self.read(run).await;
            let status = read["status"].as_str().unwrap_or_default();
            if ["done", "failed", "cancelled"].contains(&status) {
                return read;
            }
            assert!(Instant::now() < deadline, "never finished: {read}");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// `run` stopped before Ship on a card that names `repo`, because an
    /// agent chose that repo. A person denies it, and nothing is written.
    async fn waits_and_is_denied(&self, run: &Value, repo: &Path) {
        assert_eq!(run["status"], json!("waiting for approval"), "{run}");
        let cards = self.hub.approvals();
        assert_eq!(cards.len(), 1, "the app shows the card");
        assert_eq!(cards[0].kind, ApprovalKind::Ship);
        let real = maquette_core::fs::real_folder(repo);
        assert_eq!(
            cards[0].detail,
            Some(format!("Writes to {}", real.display())),
            "the card says where Ship writes"
        );
        let waiting = self.read(run).await;
        let why = waiting["steps"][0]["message"].as_str().unwrap_or_default();
        assert!(why.contains("an agent chose the repo"), "{waiting}");

        assert!(self.hub.decide(&cards[0].id, false));
        let denied = self.finished(run).await;
        assert_eq!(denied["status"], json!("cancelled"), "{denied}");
        assert_eq!(denied["message"], json!("Shipping was denied in Maquette."));
        assert!(!repo.join("shipped.log").exists(), "nothing was written");
    }

    async fn close(self) {
        self.client.cancel().await.expect("close");
        self.server.stop();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_with_repo_waits_for_a_person_with_the_rule_off() {
    let world = rule_off().await;
    let worktree = git_repo(world.dir.path(), "worktree");
    let run = call(
        &world.client,
        "run_pipeline",
        json!({ "project": "web", "repo": worktree, "waitSeconds": 30 }),
    )
    .await;
    world.waits_and_is_denied(&run, &worktree).await;
    world.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_site_repo_an_agent_set_waits_for_a_person_with_the_rule_off() {
    let world = rule_off().await;
    let site = git_repo(world.dir.path(), "site");
    call(
        &world.client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": site }),
    )
    .await;
    let run = call(
        &world.client,
        "run_pipeline",
        json!({ "project": "web", "waitSeconds": 30 }),
    )
    .await;
    world.waits_and_is_denied(&run, &site).await;
    world.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg_attr(windows, ignore = "the fixture's Ship step is an sh script")]
async fn a_site_repo_a_person_set_ships_unasked_with_the_rule_off() {
    let world = rule_off().await;
    let site = git_repo(world.dir.path(), "site");
    // A person picked the repo in the app: it is saved with no agent's mark.
    let (root, config) = world.host.session.workspace().expect("ws");
    let repo = Some(site.display().to_string());
    project::update(&root, &config.layout, "web", "Web".into(), repo).expect("repo");

    let run = call(
        &world.client,
        "run_pipeline",
        json!({ "project": "web", "waitSeconds": 30 }),
    )
    .await;
    let done = world.finished(&run).await;
    assert_eq!(done["status"], json!("done"), "{done}");
    assert_eq!(done["shipped"], json!(true), "{done}");
    assert!(world.hub.approvals().is_empty(), "no card");
    assert_eq!(
        std::fs::read_to_string(site.join("shipped.log")).expect("shipped"),
        "shipped\n"
    );
    world.close().await;
}
