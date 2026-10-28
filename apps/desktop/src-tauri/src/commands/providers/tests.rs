use super::*;
use maquette_core::concepts::{self, REFS_DIR};
use maquette_core::providers::MemoryStore;
use maquette_core::{
    AppState, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, workspace,
};

#[test]
fn send_options_pick_the_assets_own_refs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Moon".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    project::modify(&root, &config.layout, &manifest.id, |m| {
        assets::add(
            m,
            NewAsset {
                name: "Fox spirit".into(),
                kind: AssetKind::Character,
                stage: Stage::Refs,
            },
        )
        .map(|_| ())
    })
    .expect("asset");
    let refs = project::dir(&root, &config.layout, &manifest.id)
        .expect("dir")
        .join(REFS_DIR);
    for name in ["fox_spirit_front.png", "fox_spirit_side.png", "lantern.png"] {
        std::fs::write(refs.join(name), b"png").expect("ref");
    }
    let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    let jobs = ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {});
    jobs.open_workspace(&root, &config);
    let options = send_options(&session, &jobs, &manifest.id, "fox-spirit").expect("options");
    assert_eq!(options.variant, PresetVariant::Character);
    assert_eq!(options.images.len(), 3);
    assert_eq!(
        options.suggested,
        vec!["refs/fox_spirit_front.png", "refs/fox_spirit_side.png"]
    );
    assert!(options.keys.iter().all(|k| !k.saved));
}

#[test]
fn batch_options_find_object_refs_by_title() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Landing".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    let request = concepts::objects::clean(&project_dir, &{
        let src = project_dir.join("refs/workbench.png");
        std::fs::write(&src, b"png").expect("src");
        concepts::NewObjectRefs {
            project: manifest.id.clone(),
            source: "refs/workbench.png".into(),
            source_name: "Workbench".into(),
            objects: vec!["Desk lamp".into(), "Clay figure".into()],
            notes: Default::default(),
            aspect: concepts::Aspect::Square,
            kind: AssetKind::Prop,
            use_style: true,
        }
    })
    .expect("clean");
    let (direction, planned) = concepts::objects::prepare(&project_dir, &request).expect("prepare");
    for p in &planned {
        std::fs::write(project_dir.join(&p.file), b"png").expect("png");
        concepts::add_image(
            &project_dir,
            &direction.id,
            p.index,
            &p.file,
            Some(&p.object),
        )
        .expect("add");
        concepts::mark(&project_dir, &direction.id, &p.file, None, Some(true)).expect("pick");
    }
    let sent =
        concepts::send_object_picks(&root, &config.layout, &manifest.id, &direction.id, None)
            .expect("send");
    // Make the refs/ name differ from the asset id, so only the title
    // can find it.
    std::fs::rename(
        project_dir.join("refs/clay_figure.png"),
        project_dir.join("refs/figure_from_lab.png"),
    )
    .expect("rename");
    concepts::modify_direction(&project_dir, &direction.id, |d| {
        d.images[1].ref_file = Some("refs/figure_from_lab.png".into());
        Ok(())
    })
    .expect("ref");
    let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    let jobs = ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {});
    jobs.open_workspace(&root, &config);
    let ids: Vec<String> = sent.assets.iter().map(|a| a.asset.id.clone()).collect();
    let mut asked = ids.clone();
    asked.push(ids[0].clone());
    let options = batch_options(&session, &jobs, &manifest.id, &asked).expect("options");
    assert_eq!(options.items.len(), 2, "each asset once");
    assert_eq!(options.items[0].suggested, vec!["refs/desk_lamp.png"]);
    assert_eq!(options.items[1].suggested, vec!["refs/figure_from_lab.png"]);
    assert!(
        options
            .items
            .iter()
            .all(|i| i.variant == PresetVariant::Prop)
    );
    assert!(batch_options(&session, &jobs, &manifest.id, &["nope".into()]).is_err());
}

#[test]
fn only_images_on_the_card_can_be_revealed() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("front.png"), b"png").expect("png");
    std::fs::write(dir.path().join("settings.md"), b"md").expect("md");
    let card = maquette_core::providers::HandoffCard {
        service: ProviderId::Tripo,
        site: handoff::site(ProviderId::Tripo).into(),
        mode: "Image to 3D".into(),
        rows: Vec::new(),
        images: vec!["front.png".into()],
        return_name: "maq_lamp_v1_tripo.glb".into(),
        export_name: "maq_lamp_v1_tripo".into(),
        credits: Some(30.0),
        credits_note: Some("textured image to 3D".into()),
        prompt: None,
    };
    let json = serde_json::json!({
        "id": "lamp-1", "project": "p", "asset": "lamp", "assetName": "Lamp",
        "provider": "by-hand", "service": "tripo", "variant": "prop",
        "images": ["refs/lamp.png"], "tripo": null, "meshy": null, "state": "waiting",
        "phase": null, "progress": 0.0, "remoteId": null, "credits": null,
        "createdAtMs": 0.0, "startedAtMs": null, "finishedAtMs": null, "message": null,
        "inboxFile": null, "handoff": dir.path().display().to_string(), "card": card,
        "version": null, "viaAgent": false, "cancel": "stop-waiting"
    });
    let job: ProviderJob = serde_json::from_value(json).expect("job");
    assert_eq!(
        handoff_image(&job, "front.png").expect("front"),
        dir.path().join("front.png")
    );
    for bad in ["settings.md", "../front.png", "left.png"] {
        assert!(handoff_image(&job, bad).is_err(), "{bad}");
    }
}
