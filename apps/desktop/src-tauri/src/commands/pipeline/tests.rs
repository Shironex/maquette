use std::sync::Arc;

use super::*;
use maquette_core::pipeline::history::RunState;
use maquette_core::pipeline::runner::NullSink;
use maquette_core::{AppState, MeshoptLevel, NewProject, ProjectTemplate, workspace};

fn open() -> (tempfile::TempDir, Session, String) {
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
            name: "Moon Island".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    (dir, session, manifest.id)
}

#[test]
fn exported_models_carry_the_name_of_their_asset() {
    use maquette_core::{Asset, AssetKind, AssetStatus, AssetVersion, Stage};
    let (_dir, session, id) = open();
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let (root, config) = session.workspace().expect("ws");
    let dir = project::dir(&root, &config.layout, &id).expect("dir");
    std::fs::create_dir_all(dir.join("web/raw")).expect("raw");
    std::fs::write(dir.join("web/raw/clay_v1.glb"), b"glb").expect("export");
    std::fs::write(dir.join("web/raw/island.glb"), b"glb").expect("island");
    project::modify(&root, &config.layout, &id, |m| {
        m.assets.push(Asset {
            id: "clay".into(),
            name: "clay".into(),
            kind: AssetKind::Prop,
            stage: Stage::Blender,
            status: AssetStatus::Review,
            versions: vec![AssetVersion {
                version: 1,
                file: "models/clay_v1.glb".into(),
                created_at: "2026-10-24T19:14:19.000Z".into(),
                stats: None,
            }],
            thumbnail: None,
            created_at: "2026-10-24T19:00:00.000Z".into(),
            updated_at: "2026-10-24T19:14:19.000Z".into(),
            origin: None,
            refs: Vec::new(),
        });
        Ok(())
    })
    .expect("asset");
    let view = view(&session, &runner, &id).expect("view");
    let clay = view
        .models
        .iter()
        .find(|m| m.model == "clay_v1")
        .expect("clay");
    assert_eq!(clay.asset_name.as_deref(), Some("clay"));
    assert_eq!(clay.version, Some(1));
    let island = view
        .models
        .iter()
        .find(|m| m.model == "island")
        .expect("island");
    assert_eq!(island.asset_name, None);
}

#[test]
fn old_starter_scripts_show_and_update_from_the_view() {
    use maquette_core::pipeline::starter::StarterState;
    let (_dir, session, id) = open();
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let first = view(&session, &runner, &id).expect("view");
    assert!(!first.starters.is_empty());
    assert!(
        first
            .starters
            .iter()
            .all(|s| s.state == StarterState::Current)
    );
    let (root, config) = session.workspace().expect("ws");
    let dir = project::dir(&root, &config.layout, &id).expect("dir");
    std::fs::write(dir.join("tools/bake_light.py"), "print('mine')\n").expect("edit");
    let edited = view(&session, &runner, &id).expect("view");
    let bake = |v: &PipelineView| {
        v.starters
            .iter()
            .find(|s| s.path == "tools/bake_light.py")
            .expect("bake")
            .clone()
    };
    assert_eq!(bake(&edited).state, StarterState::Edited);
    let all = vec!["tools/bake_light.py".to_string()];
    let kept = update_starters(&session, &runner, &id, &all, false).expect("update");
    assert_eq!(bake(&kept).state, StarterState::Edited);
    let replaced = update_starters(&session, &runner, &id, &all, true).expect("replace");
    assert_eq!(bake(&replaced).state, StarterState::Current);
    assert_eq!(
        std::fs::read_to_string(dir.join("tools/bake_light.py.bak")).expect("bak"),
        "print('mine')\n"
    );
}

/// Makes three of a project's starters untouched copies of an older starter,
/// recorded in tools/.starter.json the way an older Maquette wrote them.
fn age_starters(dir: &std::path::Path) {
    let scripts = [
        "tools/bake_light.py",
        "tools/build_scene.py",
        "tools/export_web.py",
    ];
    let mut record: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("tools/.starter.json")).expect("record"),
    )
    .expect("json");
    for path in scripts {
        let old = format!("print('an older {path}')\n");
        std::fs::write(dir.join(path), &old).expect("old copy");
        let hash = sha256_hex(&old);
        record["scripts"][path] = serde_json::Value::String(hash);
    }
    std::fs::write(
        dir.join("tools/.starter.json"),
        serde_json::to_string_pretty(&record).expect("text"),
    )
    .expect("save record");
}

fn sha256_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(text.as_bytes()))
}

#[test]
fn updating_starters_changes_the_named_project_and_only_it() {
    use maquette_core::pipeline::starter::StarterState;
    let (_dir, session, open_id) = open();
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let (root, config) = session.workspace().expect("ws");
    let other = project::create(
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
    .expect("second project");
    let open_dir = project::dir(&root, &config.layout, &open_id).expect("dir");
    let other_dir = project::dir(&root, &config.layout, &other.id).expect("dir");
    // Both views write their pipeline and starters first.
    view(&session, &runner, &open_id).expect("view");
    view(&session, &runner, &other.id).expect("view");
    age_starters(&open_dir);
    age_starters(&other_dir);

    let before = view(&session, &runner, &other.id).expect("view");
    let outdated: Vec<String> = before
        .starters
        .iter()
        .filter(|s| s.state == StarterState::Outdated)
        .map(|s| s.path.clone())
        .collect();
    assert_eq!(outdated.len(), 3, "{:?}", before.starters);

    let after = update_starters(&session, &runner, &other.id, &outdated, false).expect("update");
    assert!(
        after
            .starters
            .iter()
            .all(|s| s.state == StarterState::Current),
        "{:?}",
        after.starters
    );
    assert_eq!(
        after.project, other.id,
        "the view that comes back is the named project's"
    );
    assert!(
        std::fs::read_to_string(other_dir.join("tools/bake_light.py"))
            .expect("read")
            .contains("bake"),
    );
    // The project open in the app is not touched by an update of another.
    let untouched = view(&session, &runner, &open_id).expect("view");
    assert_eq!(
        untouched
            .starters
            .iter()
            .filter(|s| s.state == StarterState::Outdated)
            .count(),
        3
    );
    // A second Update of the same list is a no op, not an error.
    update_starters(&session, &runner, &other.id, &outdated, false).expect("again");
    // A path that is no starter is refused, not silently skipped.
    let err = update_starters(
        &session,
        &runner,
        &other.id,
        &["tools/bake.py".to_string()],
        false,
    )
    .expect_err("unknown");
    assert!(err.message.contains("tools/bake.py"), "{}", err.message);
}

#[test]
fn the_view_writes_the_template_and_reports_no_runs() {
    let (_dir, session, id) = open();
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let view = view(&session, &runner, &id).expect("view");
    assert_eq!(view.def.steps.len(), 6);
    assert!(view.issues.iter().all(|i| !i.is_error()));
    assert!(view.runs.is_empty());
    assert_eq!(view.defaults.texture_size, 2048);
    let (root, config) = session.workspace().expect("ws");
    let dir = project::dir(&root, &config.layout, &id).expect("dir");
    assert!(dir.join("pipeline.json").is_file());
}

#[test]
fn locked_presets_refuse_edits() {
    use maquette_core::{Asset, AssetKind, AssetStatus, AssetVersion, Stage};
    let (_dir, session, id) = open();
    let runner = PipelineRunner::new(Arc::new(NullSink));
    // A scene without a character has no character row and no phantom model.
    let first = view(&session, &runner, &id).expect("view");
    assert!(first.def.presets.is_empty());
    assert!(first.models.iter().all(|m| m.model != "character"));
    let (root, config) = session.workspace().expect("ws");
    project::modify(&root, &config.layout, &id, |m| {
        m.assets.push(Asset {
            id: "keeper".into(),
            name: "Keeper".into(),
            kind: AssetKind::Character,
            stage: Stage::Blender,
            status: AssetStatus::Review,
            versions: vec![AssetVersion {
                version: 1,
                file: "models/keeper_v1.glb".into(),
                created_at: String::new(),
                stats: None,
            }],
            thumbnail: None,
            created_at: String::new(),
            updated_at: String::new(),
            origin: None,
            refs: Vec::new(),
        });
        Ok(())
    })
    .expect("character");
    let mut rows = view(&session, &runner, &id).expect("view").def.presets;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].model, "keeper_v1");
    assert_eq!(rows[0].label, "Keeper");
    let locked = rows
        .iter()
        .position(|p| p.lock.is_some())
        .expect("character row");
    rows[locked].meshopt = MeshoptLevel::High;
    let err = save_presets(&session, &runner, &id, rows.clone()).expect_err("locked");
    assert_eq!(err.code, "invalid");
    rows[locked].meshopt = MeshoptLevel::Medium;
    rows.push(CompressPreset {
        model: "bench".into(),
        label: "Bench".into(),
        texture_size: 1024,
        data_texture_size: None,
        codec: maquette_core::TextureCodec::Uastc,
        map_codecs: None,
        meshopt: MeshoptLevel::High,
        lock: None,
    });
    let saved = save_presets(&session, &runner, &id, rows).expect("save");
    assert!(saved.def.presets.iter().any(|p| p.model == "bench"));
}

#[test]
fn a_run_without_scripts_fails_with_a_clear_message() {
    let (_dir, session, id) = open();
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let setup = setup(
        &session,
        &id,
        RunAsk::default(),
        maquette_core::pipeline::tools::Toolbox::default(),
    )
    .expect("setup");
    let run = runner.run_blocking(setup).expect("run");
    assert_eq!(run.state, RunState::Failed);
    let view = view(&session, &runner, &id).expect("view");
    assert_eq!(view.runs.len(), 1);
    assert!(!view.log.is_empty());
}

#[cfg(unix)]
#[test]
fn a_saved_repo_whose_link_moved_into_the_workspace_is_refused() {
    let (dir, session, id) = open();
    let tools = maquette_core::pipeline::tools::Toolbox::default;
    let site = dir.path().join("site");
    std::fs::create_dir_all(site.join(".git")).expect("site");
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&site, &link).expect("link");
    let (root, config) = session.workspace().expect("workspace");
    let name = maquette_core::project::load(&root, &config.layout, &id)
        .expect("load")
        .name;
    let saved = link.display().to_string();
    maquette_core::project::update(&root, &config.layout, &id, name, Some(saved)).expect("save");
    let run = setup(&session, &id, RunAsk::default(), tools()).expect("setup");
    assert_eq!(
        run.vars["siteRepo"],
        maquette_core::fs::real_folder(&site).display().to_string()
    );
    std::fs::remove_file(&link).expect("unlink");
    std::os::unix::fs::symlink(&root, &link).expect("moved");
    // A person chose this repo, so it stays their call.
    assert!(setup(&session, &id, RunAsk::default(), tools()).is_ok());
    crate::commands::site::gate::mark_repo(
        &session,
        &root,
        &id,
        maquette_core::app_state::RepoSetBy::Agent,
    )
    .expect("mark");
    assert!(setup(&session, &id, RunAsk::default(), tools()).is_err());
}

#[test]
fn a_run_ships_into_a_repo_given_for_it_alone() {
    let (dir, session, id) = open();
    let tools = maquette_core::pipeline::tools::Toolbox::default;
    let worktree = dir.path().join("site-agent");
    std::fs::create_dir_all(&worktree).expect("worktree");
    std::fs::write(
        worktree.join(".git"),
        "gitdir: /src/site/.git/worktrees/agent\n",
    )
    .expect("git file");
    let ask = |repo: Option<String>| RunAsk {
        repo,
        ..RunAsk::default()
    };
    let at = maquette_core::fs::real_folder(&worktree)
        .display()
        .to_string();
    let run = setup(&session, &id, ask(Some(at.clone())), tools()).expect("setup");
    assert_eq!(run.vars["siteRepo"], at);
    let plain = dir.path().join("plain");
    std::fs::create_dir_all(&plain).expect("plain");
    let refused = setup(
        &session,
        &id,
        ask(Some(plain.display().to_string())),
        tools(),
    );
    assert!(refused.is_err(), "not a git repo or worktree");
    let own = setup(&session, &id, ask(None), tools()).expect("own");
    assert_eq!(
        own.vars["siteRepo"], "",
        "the project's own repo, none here"
    );
}
