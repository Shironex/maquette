use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use maquette_core::{AppState, AssetKind, NewProject, ProjectTemplate, WorkspaceConfig, workspace};

struct World {
    dir: tempfile::TempDir,
    session: Session,
    project: String,
    project_dir: PathBuf,
}

/// A workspace with an empty project and a folder of piece scripts.
fn world() -> World {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
    let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Study room".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    let pieces = dir.path().join("pieces");
    std::fs::create_dir_all(&pieces).expect("pieces");
    std::fs::write(pieces.join("laptop.py"), "import common\n").expect("laptop");
    std::fs::write(pieces.join("common.py"), "PALETTE = 1\n").expect("common");
    World {
        dir,
        session,
        project: manifest.id,
        project_dir,
    }
}

fn laptop(w: &World) -> NewBuilt {
    let pieces = w.dir.path().join("pieces");
    NewBuilt {
        name: "Laptop".into(),
        kind: AssetKind::Prop,
        script: pieces.join("laptop.py").display().to_string(),
        helpers: vec![pieces.join("common.py").display().to_string()],
        root: None,
        replace: false,
    }
}

#[derive(Default)]
struct Count {
    projects: AtomicUsize,
    built: AtomicUsize,
}

impl Notify for Count {
    fn project(&self, _project: &ProjectManifest) {
        self.projects.fetch_add(1, Ordering::SeqCst);
    }
    fn built(&self, _project: &str) {
        self.built.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn an_added_asset_is_listed_as_new() {
    let w = world();
    let added = add(&w.session, &w.project, &laptop(&w)).expect("add");
    assert_eq!(added.asset.id, "laptop");
    assert!(added.project.assets.iter().any(|a| a.id == "laptop"));
    let all = list(&w.session, &w.project).expect("list");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].state, BuiltState::New);
}

#[test]
fn a_full_run_is_refused_while_a_built_asset_misses_a_file() {
    let w = world();
    add(&w.session, &w.project, &laptop(&w)).expect("add");
    std::fs::remove_file(w.project_dir.join("built/common.py")).expect("rm");
    let thumbs = Thumbnails::new(|_| {});
    let tools = Toolbox::default();
    let count = Count::default();
    let env = BuildEnv {
        session: &w.session,
        thumbs: &thumbs,
        tools: &tools,
        notify: &count,
    };
    let refused = before_run(&env, &w.project).expect_err("missing");
    assert!(
        refused
            .message
            .contains("Laptop is missing built/common.py"),
        "{}",
        refused.message
    );
    assert!(refused.message.starts_with("The run did not start."));
    assert_eq!(count.built.load(Ordering::SeqCst), 0, "nothing built");
}

#[test]
fn a_new_built_asset_needs_blender_before_a_run() {
    let w = world();
    add(&w.session, &w.project, &laptop(&w)).expect("add");
    let thumbs = Thumbnails::new(|_| {});
    let tools = Toolbox::default();
    let count = Count::default();
    let env = BuildEnv {
        session: &w.session,
        thumbs: &thumbs,
        tools: &tools,
        notify: &count,
    };
    let refused = before_run(&env, &w.project).expect_err("no blender");
    assert!(refused.message.contains("Blender"), "{}", refused.message);
}

#[test]
fn a_project_without_built_assets_runs_as_before() {
    let w = world();
    let thumbs = Thumbnails::new(|_| {});
    let tools = Toolbox::default();
    let count = Count::default();
    let env = BuildEnv {
        session: &w.session,
        thumbs: &thumbs,
        tools: &tools,
        notify: &count,
    };
    assert!(before_run(&env, &w.project).expect("run").is_empty());
}
