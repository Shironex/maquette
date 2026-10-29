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
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::support::{call, raw_call, text};

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
            target: Default::default(),
            game: None,
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

/// Makes tools/bake_light.py an untouched copy of an older starter, as an
/// older Maquette would have written and recorded it.
fn age_bake_script(project_dir: &Path) {
    use sha2::{Digest, Sha256};
    let old = "print('an older bake')\n";
    std::fs::write(project_dir.join("tools/bake_light.py"), old).expect("old copy");
    let path = project_dir.join("tools/.starter.json");
    let mut record: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("record")).expect("json");
    record["scripts"]["tools/bake_light.py"] =
        Value::String(hex::encode(Sha256::digest(old.as_bytes())));
    std::fs::write(&path, record.to_string()).expect("save record");
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
    let studio = ProviderJobs::new(
        Arc::new(MemoryStore::default()),
        Options::default(),
        maquette_core::inbox::send_ports(),
        |_| {},
    );
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
    assert_eq!(real["padEdges"], json!(false));

    // Pad the texture edges, then forget the choice.
    let padded = call(
        &client,
        "set_pad_edges",
        json!({ "project": project_id, "asset": "Desk globe", "pad": true }),
    )
    .await;
    assert_eq!(padded["padEdges"], json!(true));
    let saved = prepare::load(&project_dir).expect("prepare");
    assert_eq!(saved.models["desk-globe"].pad_edges, Some(true));
    call(
        &client,
        "set_pad_edges",
        json!({ "project": project_id, "asset": "desk-globe", "pad": null }),
    )
    .await;
    let saved = prepare::load(&project_dir).expect("prepare");
    assert_eq!(saved.models["desk-globe"].pad_edges, None);

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

    // An untouched older bake script: the agent updates it itself.
    age_bake_script(&project_dir);
    let starters = call(&client, "get_starters", json!({ "project": project_id })).await;
    let bake_state = |v: &Value| {
        v["starters"]
            .as_array()
            .expect("starters")
            .iter()
            .find(|s| s["path"] == "tools/bake_light.py")
            .map(|s| s["state"].clone())
            .expect("bake script")
    };
    assert_eq!(bake_state(&starters), json!("outdated"));
    let changes = host.pipeline_changes.load(Ordering::SeqCst);
    let updated = call(&client, "update_starters", json!({ "project": project_id })).await;
    assert_eq!(bake_state(&updated), json!("current"));
    assert_eq!(host.pipeline_changes.load(Ordering::SeqCst), changes + 1);
    let settings = call(
        &client,
        "get_bake_settings",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(settings["script"], json!("current"));
    // Nothing left to update says so instead of passing quietly.
    let nothing = raw_call(&client, "update_starters", json!({ "project": project_id })).await;
    assert_eq!(nothing.is_error, Some(true));
    let unknown = raw_call(
        &client,
        "update_starters",
        json!({ "project": project_id, "scripts": ["tools/bake.py"] }),
    )
    .await;
    assert_eq!(unknown.is_error, Some(true));
    assert!(
        text(&unknown).contains("tools/bake.py"),
        "{}",
        text(&unknown)
    );

    client.cancel().await.expect("close");
    server.stop();
}
