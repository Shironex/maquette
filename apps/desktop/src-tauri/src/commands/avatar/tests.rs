use std::fs;
use std::path::Path;

use maquette_core::assets::{self, AssetKind, NewAsset, Stage};
use maquette_core::avatar::export::AvatarBuild;
use maquette_core::base::ports::ScriptRunner;
use maquette_core::inbox::fixture::{self, HumanoidSpec};
use maquette_core::{
    AppState, CoreError, CoreResult, NewProject, ProjectTemplate, WorkspaceConfig, workspace,
};

use super::*;

/// Fails the test if a build reaches for Blender.
struct NoBlender;

impl ScriptRunner for NoBlender {
    fn run_script(&self, _: &Toolbox, _: &Path, _: (&str, &str), _: &[String]) -> CoreResult<()> {
        Err(CoreError::Invalid("Blender should not run".into()))
    }
}

#[test]
fn a_t_posed_character_saves_its_spec_and_writes_its_vrm() {
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
            name: "Pets".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    fs::create_dir_all(project_dir.join("models")).expect("models");
    fs::write(
        project_dir.join("models/hero_v1.glb"),
        fixture::humanoid(&HumanoidSpec::default()),
    )
    .expect("model");
    let mut asset = String::new();
    project::modify(&root, &config.layout, &manifest.id, |m| {
        let added = assets::add(
            m,
            NewAsset {
                name: "Hero".into(),
                kind: AssetKind::Character,
                stage: Stage::Blender,
            },
        )?;
        assets::add_version(m, &added.id, "models/hero_v1.glb".into(), None)?;
        asset = added.id;
        Ok(())
    })
    .expect("asset");
    let target = Target {
        project: &manifest.id,
        asset: &asset,
        version: None,
        elbows: Elbows::default(),
        reweight: ReweightMode::default(),
        fingers: Fingers::default(),
        springs: SpringsMode::default(),
    };

    let fresh = view(&session, &target).expect("view");
    assert!(!fresh.stored && !fresh.needs_normalize);
    assert!(build(&session, &NoBlender, &Toolbox::default(), &target).is_err());

    let mut spec = fresh.spec.clone();
    spec.meta.authors = vec!["Me".into()];
    let saved = save(&session, &target, spec).expect("save");
    assert!(saved.stored && saved.meta_problem.is_none());
    let planned = plan(&session, &target).expect("plan");
    assert!(!planned.normalize);

    let AvatarBuild::Written { export, .. } =
        build(&session, &NoBlender, &Toolbox::default(), &target).expect("build")
    else {
        panic!("written");
    };
    assert!(Path::new(&export.path).is_file());
    assert!(Path::new(&export.path).ends_with(Path::new("exports").join(format!("{asset}.vrm"))));
    assert_eq!(view(&session, &target).expect("view").export, Some(export));
}
