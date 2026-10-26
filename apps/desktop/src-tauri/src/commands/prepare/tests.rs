use super::*;
use maquette_core::prepare::{Fixup, RealSize, SizeAxis, SizeUnit};
use maquette_core::{
    AppState, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, workspace,
};

struct World {
    _dir: tempfile::TempDir,
    session: Session,
    project: String,
}

/// A workspace with a project whose "Desk lamp" has the messy fixture as v1.
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
            name: "Workbench".into(),
            template: ProjectTemplate::FloatingDiorama,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let dir_of = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    std::fs::create_dir_all(dir_of.join("models")).expect("models");
    std::fs::write(
        dir_of.join("models/desk_lamp_v1.glb"),
        inbox::fixture::messy_prop(),
    )
    .expect("glb");
    project::modify(&root, &config.layout, &manifest.id, |m| {
        let lamp = assets::add(
            m,
            NewAsset {
                name: "Desk lamp".into(),
                kind: AssetKind::Prop,
                stage: Stage::Blender,
            },
        )?;
        assets::add_version(m, &lamp.id, "models/desk_lamp_v1.glb".into(), None).map(|_| ())
    })
    .expect("lamp");
    World {
        _dir: dir,
        session,
        project: manifest.id,
    }
}

#[test]
fn an_asset_shows_its_size_suggestion_and_defaults_then_saves() {
    let w = world();
    let view = asset_view(&w.session, &w.project, "desk-lamp").expect("view");
    assert!(view.entry.is_empty());
    assert_eq!(view.version, Some(1));
    let measured = view.measured.expect("measured");
    assert!((measured[1] - 0.7025).abs() < 1e-3, "{measured:?}");
    assert_eq!(
        view.suggestion.map(|s| s.size.describe()),
        Some("45 cm high".to_string()),
        "a suggestion only"
    );
    assert_eq!(view.scale, None, "nothing applied until saved");
    assert_eq!(view.defaults.triangles, 60_000);

    let entry = ModelPrep {
        skip_bake: None,
        pad_edges: None,
        size: Some(RealSize {
            axis: SizeAxis::Height,
            value: 45.0,
            unit: SizeUnit::Cm,
        }),
        fixups: vec![Fixup::OriginBottom],
    };
    let saved = save(&w.session, &w.project, "desk-lamp", &entry).expect("save");
    assert_eq!(saved.entry, entry);
    let scale = saved.scale.expect("scale");
    assert!((scale - 0.45 / measured[1]).abs() < 1e-4);
    assert_eq!(
        get(&w.session, &w.project).expect("get").models["desk-lamp"],
        entry
    );
    let cleared = save(&w.session, &w.project, "desk-lamp", &ModelPrep::default()).expect("clear");
    assert!(cleared.entry.is_empty());
    assert!(get(&w.session, &w.project).expect("get").models.is_empty());
    assert!(save(&w.session, &w.project, "nope", &entry).is_err());
}

#[test]
fn a_preview_needs_a_model_a_valid_plan_and_blender() {
    let w = world();
    let plan = ModelPrep {
        skip_bake: None,
        pad_edges: None,
        size: None,
        fixups: vec![Fixup::Decimate { triangles: 5000 }],
    };
    let request = |asset: Option<&str>, plan: ModelPrep| PreviewRequest {
        project: Some(w.project.clone()),
        asset: asset.map(str::to_string),
        inbox: None,
        plan,
    };
    let job = preview_job(&w.session, &request(Some("desk-lamp"), plan.clone())).expect("job");
    assert!(job.input.ends_with("models/desk_lamp_v1.glb"));
    assert_eq!(
        job.script,
        maquette_core::prepare::SCRIPT,
        "the project's copy"
    );
    let err = job.run(&Toolbox::default()).expect_err("no blender");
    assert!(
        err.message.contains("Blender is not installed"),
        "{}",
        err.message
    );

    let bad = ModelPrep {
        skip_bake: None,
        pad_edges: None,
        size: None,
        fixups: vec![Fixup::Decimate { triangles: 1 }],
    };
    assert!(preview_job(&w.session, &request(Some("desk-lamp"), bad)).is_err());
    assert!(preview_job(&w.session, &request(None, plan)).is_err());
}

#[test]
fn suggestions_answer_for_a_name_and_kind() {
    assert_eq!(
        prepare_suggest("Laptop".into(), AssetKind::Prop).map(|s| s.matched),
        Some("laptop".into())
    );
    assert_eq!(prepare_suggest("Island".into(), AssetKind::Scene), None);
}
