//! A real MCP client puts a built asset in the place of a scan, reads what
//! changed, sees the swap in the Ship plan and undoes it. The built asset is
//! made current by hand (its model, its version and a build record with the
//! key of its files), so no Blender runs. Everything happens in a temp
//! workspace; nothing reaches Codex, Tripo or Meshy.

use std::path::Path;
use std::sync::Arc;

use maquette_core::built::{self, BuildRecord, NewBuilt};
use maquette_core::concepts::Concepts;
use maquette_core::inbox::fixture;
use maquette_core::layout::{self, LayoutStop, Placement};
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::prepare::{self, ModelPrep, RealSize, SizeAxis, SizeUnit};
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{
    AppState, AssetKind, AssetStatus, NewAsset, NewProject, ProjectTemplate, Stage,
    WorkspaceConfig, assets, project, workspace,
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

use super::support::{TestHost, call, call_error};

const PROJECT: &str = "study-room";

/// Writes `bytes` as the next version of `id`, the way an accept or a build does.
fn add_model(root: &Path, config: &WorkspaceConfig, id: &str, bytes: &[u8]) {
    let dir = project::dir(root, &config.layout, PROJECT).expect("dir");
    let manifest = project::load(root, &config.layout, PROJECT).expect("manifest");
    let number = assets::find(&manifest, id).expect("asset").next_version();
    let file = format!("models/{}_v{number}.glb", id.replace('-', "_"));
    std::fs::create_dir_all(dir.join("models")).expect("models");
    std::fs::write(dir.join(&file), bytes).expect("model");
    let report = maquette_core::inbox::analyse(&dir.join(&file)).expect("analyse");
    project::modify(root, &config.layout, PROJECT, |m| {
        assets::add_version(m, id, file.clone(), Some(assets::stats_of(&report)))?;
        assets::set_status(m, id, AssetStatus::Done)
    })
    .expect("version");
}

/// Study room with a placed scan (a half metre crate whose real size is
/// 30 cm, skipping the bake) and a built asset that is current.
fn fixture_scene(dir: &Path) -> Session {
    let root = dir.join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let session = Session::with_state(dir.join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    project::create(
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
    let project_dir = project::dir(&root, &config.layout, PROJECT).expect("dir");
    project::modify(&root, &config.layout, PROJECT, |m| {
        assets::add(
            m,
            NewAsset {
                name: "Spiral planner".into(),
                kind: AssetKind::Prop,
                stage: Stage::Web,
            },
        )
        .map(|_| ())
    })
    .expect("scan");
    add_model(&root, &config, "spiral-planner", &fixture::clean_prop());
    let scan = ModelPrep {
        size: Some(RealSize {
            axis: SizeAxis::Longest,
            value: 30.0,
            unit: SizeUnit::Cm,
        }),
        skip_bake: Some(true),
        ..ModelPrep::default()
    };
    prepare::save_entry(&project_dir, "spiral-planner", &scan).expect("prep");
    layout::update(&project_dir, |current| {
        let mut next = current.clone();
        next.models.insert(
            "spiral-planner".into(),
            Placement {
                position: [0.4, 0.0, -0.1],
                rotation: [0.0, 35.0, 0.0],
                scale: 1.0,
            },
        );
        next.stops.push(LayoutStop {
            id: "desk".into(),
            name: "Desk".into(),
            position: [0.9, 0.5, 1.4],
            target: [0.0, 0.15, 0.0],
            fov: 35.0,
            models: Some(vec!["spiral-planner".into()]),
        });
        Ok(next)
    })
    .expect("layout");

    let pieces = dir.join("pieces");
    std::fs::create_dir_all(&pieces).expect("pieces");
    std::fs::write(pieces.join("planner.py"), "def build():\n    pass\n").expect("script");
    built::add(
        &root,
        &config.layout,
        PROJECT,
        &NewBuilt {
            name: "Planner built".into(),
            kind: AssetKind::Prop,
            script: pieces.join("planner.py").display().to_string(),
            helpers: Vec::new(),
            root: None,
            replace: false,
        },
    )
    .expect("built");
    add_model(&root, &config, "planner-built", &fixture::split_parts(3));
    let source = built::load(&project_dir).expect("built file").assets["planner-built"].clone();
    let key = built::key(&project_dir, &source).expect("key");
    built::update(&project_dir, |file| {
        file.assets.get_mut("planner-built").expect("entry").last = Some(BuildRecord {
            key,
            at: project::now_rfc3339(),
            version: Some(1),
            error: None,
            objects: vec!["Part_0".into(), "Part_1".into(), "Part_2".into()],
            materials: vec!["Grey".into()],
            triangles: 36,
            warnings: Vec::new(),
        });
        Ok(())
    })
    .expect("record");
    session
}

fn setting<'a>(report: &'a Value, what: &str) -> &'a Value {
    report["settings"]
        .as_array()
        .expect("settings")
        .iter()
        .find(|s| s["what"].as_str().is_some_and(|w| w.starts_with(what)))
        .unwrap_or_else(|| panic!("no setting {what} in {}", report["settings"]))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_replace_a_scan_with_a_built_asset_and_undo_it() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let session = fixture_scene(dir.path());
    let project_dir = dir.path().join("ws/projects").join(PROJECT);
    let host = Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(maquette_core::pipeline::runner::NullSink)),
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
    let info = client.peer_info().expect("server info");
    assert!(
        info.instructions
            .as_deref()
            .is_some_and(|i| i.contains("replace_with_built_asset"))
    );
    let layout_before = std::fs::read_to_string(project_dir.join("layout.json")).expect("layout");
    let prepare_before = std::fs::read_to_string(project_dir.join("prepare.json")).expect("prep");

    // A dry run reports and changes nothing; names work as well as ids.
    let planned = call(
        &client,
        "replace_with_built_asset",
        json!({ "project": PROJECT, "asset": "Spiral planner", "built": "Planner built", "dryRun": true }),
    )
    .await;
    assert_eq!(planned["status"], json!("would replace"));
    assert_eq!(planned["new"]["version"], json!(2));
    let assets = call(&client, "list_assets", json!({ "project": PROJECT })).await;
    assert_eq!(assets["assets"].as_array().map(Vec::len), Some(2));

    let done = call(
        &client,
        "replace_with_built_asset",
        json!({ "project": PROJECT, "asset": "spiral-planner", "built": "planner-built" }),
    )
    .await;
    assert_eq!(done["status"], json!("replaced"));
    assert_eq!(done["asset"], json!("spiral-planner"));
    assert_eq!(done["built"], json!("planner-built"));
    assert_eq!(done["script"], json!("built/planner.py"));
    // File names, triangles and bytes of both models.
    assert_eq!(done["old"]["file"], json!("models/spiral_planner_v1.glb"));
    assert_eq!(done["new"]["file"], json!("models/spiral_planner_v2.glb"));
    assert_eq!(done["old"]["shipsAs"], json!("spiral_planner_v1.glb"));
    assert_eq!(done["new"]["shipsAs"], json!("spiral_planner_v2.glb"));
    assert_eq!(done["old"]["triangles"], json!(12));
    assert_eq!(done["new"]["triangles"], json!(36));
    assert!(done["old"]["bytes"].as_f64().is_some_and(|b| b > 0.0));
    assert!(done["new"]["bytes"].as_f64().is_some_and(|b| b > 0.0));
    // The real size carried, as a measure: the scan was scaled, the built model is not.
    let scale = |side: &str| done[side]["scale"].as_f64().expect("scale");
    assert!((scale("old") - 0.6).abs() < 1e-4, "{}", scale("old"));
    assert!((scale("new") - 1.0).abs() < 1e-4, "{}", scale("new"));
    assert_eq!(
        setting(&done, "Real size, 30 cm")["action"],
        json!("carried")
    );
    assert_eq!(setting(&done, "Skips the bake")["action"], json!("reset"));
    assert_eq!(setting(&done, "Layout placement")["action"], json!("kept"));
    assert_eq!(done["bakes"], json!(true));
    assert!(
        done["note"]
            .as_str()
            .is_some_and(|n| n.contains("Nothing is aligned"))
    );
    assert!(
        done["undo"]
            .as_str()
            .is_some_and(|u| u.contains("undo: true"))
    );
    assert_eq!(done["next"][0]["tool"], json!("run_pipeline"));
    let trash = done["trash"].as_str().expect("trash id").to_string();

    // The asset kept its id and place; the built asset's card is in the trash.
    assert_eq!(
        std::fs::read_to_string(project_dir.join("layout.json")).expect("layout"),
        layout_before
    );
    let assets = call(&client, "list_assets", json!({ "project": PROJECT })).await;
    let listed = assets["assets"].as_array().expect("assets");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], json!("spiral-planner"));
    assert_eq!(
        listed[0]["latest"]["file"],
        json!("models/spiral_planner_v2.glb")
    );
    let removed = call(
        &client,
        "list_removed_assets",
        json!({ "project": PROJECT }),
    )
    .await;
    assert_eq!(removed["removed"][0]["id"], json!(trash));
    let built_now = call(&client, "list_built_assets", json!({ "project": PROJECT })).await;
    assert_eq!(built_now["assets"][0]["asset"], json!("spiral-planner"));
    assert_eq!(built_now["assets"][0]["state"], json!("current"));
    assert_eq!(
        built_now["assets"][0]["replaced"]["by"],
        json!("Planner built")
    );
    assert_eq!(built_now["assets"][0]["replaced"]["version"], json!(1));
    // It rebuilds like any built asset: nothing changed, so no Blender runs.
    let rebuilt = call(
        &client,
        "rebuild_built_asset",
        json!({ "project": PROJECT, "asset": "spiral-planner" }),
    )
    .await;
    assert_eq!(rebuilt["builds"][0]["result"], json!("up-to-date"));
    // The Ship plan lists the swap for whoever approves the next Ship.
    let plan = call(&client, "get_ship_plan", json!({ "project": PROJECT })).await;
    let swap = &plan["swaps"][0];
    assert_eq!(swap["asset"], json!("spiral-planner"), "{plan}");
    assert_eq!(swap["oldFile"], json!("spiral_planner_v1.glb"));
    assert_eq!(swap["newFile"], json!("spiral_planner_v2.glb"));
    assert_eq!(swap["oldTriangles"], json!(12));
    assert_eq!(swap["newTriangles"], json!(36));

    // An asset cannot take its own place, and a replace names the built asset.
    let again = call_error(
        &client,
        "replace_with_built_asset",
        json!({ "project": PROJECT, "asset": "spiral-planner", "built": "spiral-planner" }),
    )
    .await;
    assert!(again.contains("Pick two assets"), "{again}");
    let no_built = call_error(
        &client,
        "replace_with_built_asset",
        json!({ "project": PROJECT, "asset": "spiral-planner" }),
    )
    .await;
    assert!(no_built.contains("Name the built asset"), "{no_built}");

    // One call brings the scan back as it was.
    let undone = call(
        &client,
        "replace_with_built_asset",
        json!({ "project": PROJECT, "asset": "spiral-planner", "undo": true }),
    )
    .await;
    assert_eq!(undone["status"], json!("undone"));
    assert_eq!(
        undone["back"]["file"],
        json!("models/spiral_planner_v1.glb")
    );
    assert_eq!(
        undone["takenOff"][0]["file"],
        json!("models/spiral_planner_v2.glb")
    );
    assert_eq!(undone["restores"], json!("Planner built"));
    assert_eq!(
        std::fs::read_to_string(project_dir.join("prepare.json")).expect("prep"),
        prepare_before
    );
    let assets = call(&client, "list_assets", json!({ "project": PROJECT })).await;
    let listed = assets["assets"].as_array().expect("assets");
    assert_eq!(listed.len(), 2);
    assert_eq!(
        listed[0]["latest"]["file"],
        json!("models/spiral_planner_v1.glb")
    );
    let built_now = call(&client, "list_built_assets", json!({ "project": PROJECT })).await;
    assert_eq!(built_now["assets"][0]["asset"], json!("planner-built"));
    assert_eq!(built_now["assets"][0]["replaced"], Value::Null);
    let plan = call(&client, "get_ship_plan", json!({ "project": PROJECT })).await;
    assert_eq!(plan["swaps"], json!([]));
    let twice = call_error(
        &client,
        "replace_with_built_asset",
        json!({ "project": PROJECT, "asset": "spiral-planner", "undo": true }),
    )
    .await;
    assert!(twice.contains("is not a built asset"), "{twice}");

    client.cancel().await.expect("close");
    server.stop();
}
