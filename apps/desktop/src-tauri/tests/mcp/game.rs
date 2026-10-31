//! A real MCP client reads a game project's settings, scans and imports
//! the game's models, changes the settings and reads the Ship plan. Nothing
//! runs Blender and nothing is written into the game repo.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use maquette_core::concepts::Concepts;
use maquette_core::game::{GameFormat, NewGame, ProjectTarget};
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::thumbs::Thumbnails;
use maquette_core::{
    AppState, NewProject, ProjectTemplate, WorkspaceConfig, pipeline, project, workspace,
};
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::Hub;
use maquette_desktop_lib::session::Session;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::json;

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

/// A game repo with two props and a hand made character, and a workspace
/// with one game project that ships to it.
fn fixture(dir: &Path) -> (Session, String, std::path::PathBuf) {
    let repo = dir.join("crypt-game");
    let models = repo.join("assets/models");
    for folder in ["props", "characters"] {
        std::fs::create_dir_all(models.join(folder)).expect("folder");
    }
    std::fs::write(
        repo.join("CMakeLists.txt"),
        "add_executable(crypt main.cpp)\n",
    )
    .expect("cmake");
    let prop = maquette_core::inbox::fixture::clean_prop();
    std::fs::write(models.join("props/brazier.glb"), &prop).expect("brazier");
    std::fs::write(models.join("props/coffin.glb"), &prop).expect("coffin");
    std::fs::write(models.join("characters/hero.glb"), &prop).expect("hero");

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
            name: "Crypt game".into(),
            template: ProjectTemplate::GameImport,
            target: ProjectTarget::Game,
            game: Some(NewGame {
                format: GameFormat::Glb,
                models_dir: "assets/models".into(),
                template: None,
            }),
            site_repo: Some(repo.display().to_string()),
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    pipeline::ensure(&project_dir, &manifest).expect("pipeline");
    (session, manifest.id, repo)
}

fn tree(dir: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(folder) = stack.pop() {
        for entry in std::fs::read_dir(&folder).expect("read dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let len = entry.metadata().expect("meta").len();
                out.push((path.display().to_string(), len));
            }
        }
    }
    out.sort();
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_read_scan_import_and_change_a_game_project() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, project_id, repo) = fixture(dir.path());
    let before = tree(&repo);
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

    let projects = call(&client, "list_projects", json!({})).await;
    assert_eq!(projects["projects"][0]["target"], "game");
    let game = call(&client, "get_game", json!({ "project": project_id })).await;
    assert_eq!(game["settings"]["format"], "glb");
    assert_eq!(game["settings"]["bake"], false);
    let folders: Vec<&str> = game["settings"]["budgets"]
        .as_array()
        .expect("budgets")
        .iter()
        .filter_map(|b| b["folder"].as_str())
        .collect();
    assert!(
        folders.contains(&"props") && folders.contains(&"characters"),
        "{folders:?}"
    );

    let scan = call(
        &client,
        "scan_game_models",
        json!({ "project": project_id }),
    )
    .await;
    let files: Vec<&str> = scan["models"]
        .as_array()
        .expect("models")
        .iter()
        .filter_map(|m| m["file"].as_str())
        .collect();
    assert_eq!(
        files,
        [
            "characters/hero.glb",
            "props/brazier.glb",
            "props/coffin.glb"
        ]
    );

    let imported = call(
        &client,
        "import_game_models",
        json!({ "project": project_id, "files": ["props/brazier.glb", "props/coffin.glb"] }),
    )
    .await;
    assert_eq!(imported["imported"], json!(["brazier", "coffin"]));
    assert!(host.pipeline_changes.load(Ordering::SeqCst) > 0);

    let refused = raw_call(
        &client,
        "set_game_settings",
        json!({ "project": project_id, "textureCap": 300 }),
    )
    .await;
    assert_eq!(refused.is_error, Some(true), "{}", text(&refused));
    let changed = call(
        &client,
        "set_game_settings",
        json!({ "project": project_id, "textures": "jpg", "textureCap": 512, "bake": true }),
    )
    .await;
    assert_eq!(changed["settings"]["export"]["textures"], "jpg");
    assert_eq!(changed["settings"]["export"]["textureCap"], 512);
    assert_eq!(changed["settings"]["bake"], true);
    assert_eq!(
        changed["settings"]["export"]["dropMaps"], false,
        "maps kept by default"
    );
    let dropped = call(
        &client,
        "set_game_settings",
        json!({ "project": project_id, "dropMaps": true }),
    )
    .await;
    assert_eq!(dropped["settings"]["export"]["dropMaps"], true);
    let moved = call(
        &client,
        "set_game_settings",
        json!({ "project": project_id, "assetFolders": { "brazier": "kit" } }),
    )
    .await;
    let brazier = &moved["assets"][0];
    assert_eq!(brazier["file"], "kit/brazier.glb");
    assert_eq!(brazier["folder"], "kit");
    let outside = raw_call(
        &client,
        "set_game_settings",
        json!({ "project": project_id, "assetFolders": { "brazier": "../up" } }),
    )
    .await;
    assert_eq!(outside.is_error, Some(true), "{}", text(&outside));
    call(
        &client,
        "set_game_settings",
        json!({ "project": project_id, "assetFolders": { "brazier": null } }),
    )
    .await;
    assert_eq!(
        dropped["settings"]["export"]["textureCap"], 512,
        "the other export settings stay"
    );
    let files: Vec<&str> = changed["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .filter_map(|a| a["file"].as_str())
        .collect();
    assert_eq!(files, ["props/brazier.glb", "props/coffin.glb"]);

    let plan = call(&client, "get_ship_plan", json!({ "project": project_id })).await;
    let hand: Vec<&str> = plan["handMade"]
        .as_array()
        .expect("hand made")
        .iter()
        .filter_map(|f| f["path"].as_str())
        .collect();
    assert_eq!(hand, ["characters/hero.glb"]);
    assert_eq!(
        tree(&repo),
        before,
        "nothing was written into the game repo"
    );

    // An agent's worktree of the game: a plan against it for one look, then
    // the project's repo on this computer.
    let worktree = dir.path().join("crypt-game-agent");
    std::fs::create_dir_all(worktree.join("assets/models")).expect("worktree");
    std::fs::write(
        worktree.join(".git"),
        "gitdir: /src/crypt-game/.git/worktrees/agent\n",
    )
    .expect("git file");
    let at = worktree.display().to_string();
    let plan = call(
        &client,
        "get_ship_plan",
        json!({ "project": project_id, "repo": at }),
    )
    .await;
    assert_eq!(plan["repo"], at.as_str(), "planned against the worktree");
    let game = call(&client, "get_game", json!({ "project": project_id })).await;
    assert_ne!(game["gameRepo"], at.as_str(), "a plan saves nothing");
    let plain = raw_call(
        &client,
        "set_game_settings",
        json!({ "project": project_id, "repo": repo.join("assets").display().to_string() }),
    )
    .await;
    assert_eq!(
        plain.is_error,
        Some(true),
        "not a git repo: {}",
        text(&plain)
    );
    let relative = raw_call(
        &client,
        "set_game_settings",
        json!({ "project": project_id, "repo": "crypt-game-agent" }),
    )
    .await;
    assert_eq!(relative.is_error, Some(true), "{}", text(&relative));
    let set = call(
        &client,
        "set_game_settings",
        json!({ "project": project_id, "repo": at }),
    )
    .await;
    assert_eq!(set["gameRepo"], at.as_str());

    client.cancel().await.expect("close");
    server.stop();
}
