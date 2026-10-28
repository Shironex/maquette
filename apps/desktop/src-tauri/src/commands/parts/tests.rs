use super::*;
use maquette_core::prepare::{Fixup, SplitBy};
use maquette_core::{
    AppState, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, inbox, workspace,
};

struct World {
    _dir: tempfile::TempDir,
    session: Session,
    project: String,
}

/// A Frostveil project whose "Frost lantern" has a one mesh v1 and a v2
/// in three parts.
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
            name: "Frostveil".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let dir_of = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    std::fs::create_dir_all(dir_of.join("models")).expect("models");
    for (file, bytes) in [
        ("frost_lantern_v1.glb", inbox::fixture::merged_islands()),
        ("frost_lantern_v2.glb", inbox::fixture::messy_prop()),
    ] {
        std::fs::write(dir_of.join("models").join(file), bytes).expect("glb");
    }
    project::modify(&root, &config.layout, &manifest.id, |m| {
        let lantern = assets::add(
            m,
            NewAsset {
                name: "Frost lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Blender,
            },
        )?;
        let id = lantern.id.clone();
        assets::add_version(m, &id, "models/frost_lantern_v1.glb".into(), None)?;
        assets::add_version(m, &id, "models/frost_lantern_v2.glb".into(), None).map(|_| ())
    })
    .expect("lantern");
    World {
        _dir: dir,
        session,
        project: manifest.id,
    }
}

#[test]
fn the_parts_of_the_newest_model_carry_their_saved_names() {
    let w = world();
    let found = get(&w.session, &w.project, "frost-lantern").expect("parts");
    assert_eq!(found.version, Some(2));
    assert!(!found.split && !found.needs_preview);
    let ids: Vec<&str> = found.rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["Shade", "Base", "Speck"]);

    let plan = PartsPlan {
        names: [("Shade".to_string(), "lid".to_string())].into(),
        merge: vec![vec!["Base".into(), "Speck".into()]],
        hidden: vec!["Base".into()],
    };
    let saved = save_plan(&w.session, &w.project, "frost-lantern", &plan).expect("save");
    assert_eq!(saved.plan, plan);
    let names: Vec<(&str, bool)> = saved
        .rows
        .iter()
        .map(|r| (r.name.as_str(), r.hidden))
        .collect();
    assert_eq!(names, [("lid", false), ("Base", true)]);
    let cleared = save_plan(
        &w.session,
        &w.project,
        "frost-lantern",
        &PartsPlan::default(),
    )
    .expect("clear");
    assert_eq!(cleared.rows.len(), 3);
}

#[test]
fn a_split_waits_for_its_preview_before_listing_parts() {
    let w = world();
    let mut entry = asset_view(&w.session, &w.project, "frost-lantern")
        .expect("view")
        .entry;
    entry.fixups.push(Fixup::SplitParts {
        by: SplitBy::Loose,
        merge_under: 0.01,
    });
    save(&w.session, &w.project, "frost-lantern", &entry).expect("save");
    let found = get(&w.session, &w.project, "frost-lantern").expect("parts");
    assert!(found.split && found.needs_preview);
    assert!(found.rows.is_empty() && found.file.is_none());
}

#[test]
fn keeping_the_older_version_makes_it_current() {
    let w = world();
    let (root, config) = w.session.workspace().expect("ws");
    project::modify(&root, &config.layout, &w.project, |m| {
        parts::keep_version(m, "frost-lantern", 1)
    })
    .expect("keep");
    let found = get(&w.session, &w.project, "frost-lantern").expect("parts");
    assert_eq!(found.version, Some(1));
    assert_eq!(found.rows.len(), 1);
}
