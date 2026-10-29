//! A real MCP client reads and changes how a website project ships (style,
//! texture format, per asset choices and file names, the site's folders),
//! sets presets before the first export, reads the size report and runs a
//! build only pipeline. Nothing runs Blender.

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
    pipeline, project, workspace,
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
}

/// A workspace with one Room project with a site repo and two assets that
/// have a model each (the files are never exported).
fn fixture(dir: &Path) -> (Session, String, std::path::PathBuf, std::path::PathBuf) {
    let root = dir.join("ws");
    let site = dir.join("site");
    std::fs::create_dir_all(site.join("apps/landing/public/room/models")).expect("site");
    std::fs::write(
        site.join("maquette.site.json"),
        r#"{ "version": 1, "models": "public/models", "preview": { "command": ["pnpm", "dev"] } }"#,
    )
    .expect("config");
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
            site_repo: Some(site.display().to_string()),
            source_folder: None,
        },
    )
    .expect("project");
    let manifest = project::modify(&root, &config.layout, &manifest.id, |m| {
        for name in ["Reading armchair", "Desk"] {
            let id = assets::add(
                m,
                NewAsset {
                    name: name.into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Blender,
                },
            )?
            .id;
            let file = format!("models/{}_v1.glb", id.replace('-', "_"));
            assets::add_version(m, &id, file, None)?;
        }
        Ok(())
    })
    .expect("assets");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    pipeline::ensure(&project_dir, &manifest).expect("pipeline");
    (session, manifest.id, project_dir, site)
}

#[cfg(unix)]
fn shell_step(id: &str, name: &str, script: &str, deps: &[&str]) -> pipeline::StepDef {
    pipeline::StepDef {
        id: id.into(),
        name: name.into(),
        kind: pipeline::StepKind::Shell,
        command: Some("/bin/sh".into()),
        script: Some(script.into()),
        args: vec!["{script}".into()],
        inputs: Vec::new(),
        outputs: Vec::new(),
        depends_on: deps.iter().map(|d| (*d).to_string()).collect(),
        compress: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_set_up_a_hand_built_site_and_build_without_shipping() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, project_dir, site) = fixture(dir.path());
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

    // An older project ships as a scene with KTX2 into the site's folder.
    let first = call(
        &client,
        "get_ship_settings",
        json!({ "project": project_id }),
    )
    .await;
    assert_eq!(first["settings"]["style"], json!("scene"));
    assert_eq!(first["settings"]["textures"], json!("ktx2"));
    assert_eq!(first["target"]["models"], json!("public/models"));
    assert_eq!(
        first["scriptReady"],
        json!(true),
        "a new project has the current starters"
    );

    // Presets can be set before the first export.
    let presets = call(
        &client,
        "set_compression_presets",
        json!({ "project": project_id, "presets": [
            { "model": "Reading armchair", "textureSize": 1024, "dataTextureSize": 512 }
        ] }),
    )
    .await;
    assert_eq!(presets["presets"][0]["model"], json!("reading_armchair_v1"));

    // Wrong words are refused and nothing is written.
    for (args, needle) in [
        (json!({ "textures": "png" }), "ktx2, webp, avif"),
        (
            json!({ "assets": [{ "asset": "Desk", "file": "Writing Desk" }] }),
            "not a file name",
        ),
        (
            json!({ "assets": [{ "asset": "Nowhere", "choice": "never" }] }),
            "Nowhere",
        ),
        (json!({ "models": "../outside" }), "inside the site"),
    ] {
        let mut args = args;
        args["project"] = json!(project_id);
        let refused = raw_call(&client, "set_ship_settings", args).await;
        assert_eq!(refused.is_error, Some(true));
        assert!(text(&refused).contains(needle), "{}", text(&refused));
    }
    assert!(!project_dir.join("ship.json").exists());

    let set = call(
        &client,
        "set_ship_settings",
        json!({ "project": project_id, "style": "models", "textures": "webp", "quality": 75,
                "assets": [{ "asset": "Desk", "choice": "never", "file": "writing_desk" }],
                "models": "apps/landing/public/room/models", "posters": null, "data": null }),
    )
    .await;
    assert_eq!(set["settings"]["style"], json!("models"));
    assert_eq!(set["settings"]["textures"], json!("webp"));
    assert_eq!(set["settings"]["quality"], json!(75));
    assert_eq!(
        set["settings"]["assets"]["desk"],
        json!({ "choice": "never", "file": "writing_desk" })
    );
    assert_eq!(
        set["target"]["models"],
        json!("apps/landing/public/room/models")
    );
    assert_eq!(set["target"]["modelsExist"], json!(true));
    let config: Value = serde_json::from_str(
        &std::fs::read_to_string(site.join("maquette.site.json")).expect("config"),
    )
    .expect("json");
    assert_eq!(config["models"], json!("apps/landing/public/room/models"));
    assert_eq!(config["posters"], Value::Null);
    assert_eq!(
        config["preview"]["command"],
        json!(["pnpm", "dev"]),
        "other keys stay"
    );
    assert_eq!(host.pipeline_changes.load(Ordering::SeqCst), 2);

    // The plan is empty before Compress ran, the size report lists both.
    let plan = call(&client, "get_ship_plan", json!({ "project": project_id })).await;
    assert_eq!(plan["exported"], json!(false));
    assert_eq!(
        plan["target"]["models"],
        json!("apps/landing/public/room/models")
    );
    let report = call(&client, "get_size_report", json!({ "project": project_id })).await;
    let files: Vec<&str> = report["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .filter_map(|r| r["shippedFile"].as_str())
        .collect();
    assert_eq!(files, ["reading_armchair.glb", "writing_desk.glb"]);
    assert!(
        report["csv"]
            .as_str()
            .is_some_and(|c| c.starts_with("asset,name,version"))
    );

    // A build only run never reaches Ship. Its fixture steps are sh
    // scripts, which a Windows machine has no /bin/sh to run.
    #[cfg(unix)]
    {
        let mut def = pipeline::load(&project_dir).expect("def").expect("file");
        std::fs::write(project_dir.join("tools/build.sh"), "echo built\n").expect("build");
        std::fs::write(project_dir.join("tools/ship.sh"), "touch shipped\n").expect("ship");
        def.steps = vec![
            shell_step("build", "Build", "tools/build.sh", &[]),
            shell_step("ship", "Ship", "tools/ship.sh", &["build"]),
        ];
        pipeline::save(&project_dir, &def).expect("save");
        let started = call(
            &client,
            "run_pipeline",
            json!({ "project": project_id, "buildOnly": true, "waitSeconds": 1 }),
        )
        .await;
        assert_eq!(started["buildOnly"], json!(true));
        let mut run = started;
        for _ in 0..300 {
            run = call(
                &client,
                "get_run",
                json!({ "project": project_id, "number": 1 }),
            )
            .await;
            if run["status"] == json!("done") || run["status"] == json!("failed") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert_eq!(run["status"], json!("done"), "{run}");
        assert_eq!(run["buildOnly"], json!(true));
        assert_eq!(run["steps"][1]["state"], json!("skipped"));
        assert!(!project_dir.join("shipped").exists());
    }

    client.cancel().await.expect("close");
    server.stop();
}
