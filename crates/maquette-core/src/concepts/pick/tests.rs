//! Tests for picking concepts one image at a time.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::*;
use crate::assets::{NewAsset, Stage};
use crate::concepts::objects::{self, NewObjectRefs};
use crate::concepts::{Aspect, NewConcepts, add_image, load_direction, prepare};
use crate::project::{NewProject, ProjectTemplate};

struct Lab {
    _root: tempfile::TempDir,
    root: PathBuf,
    layout: FolderLayout,
    project: String,
    dir: PathBuf,
}

fn lab() -> Lab {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    let layout = FolderLayout::default();
    let manifest = project::create(
        &root,
        &layout,
        NewProject {
            name: "Moon".into(),
            template: ProjectTemplate::Room,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let dir = project::dir(&root, &layout, &manifest.id).expect("dir");
    Lab {
        _root: temp,
        root,
        layout,
        project: manifest.id,
        dir,
    }
}

fn png() -> Vec<u8> {
    crate::inbox::images::write_png(2, 2, &[90; 12])
}

/// A direction "Stone lantern" with `count` images on disk.
fn concepts(lab: &Lab, count: u32) -> Vec<String> {
    let brief = NewConcepts {
        project: lab.project.clone(),
        direction: "Stone lantern".into(),
        prompt: "A stone lantern".into(),
        refs: Vec::new(),
        aspect: Aspect::Square,
        count,
        use_style: true,
    };
    let (_, files) = prepare(&lab.dir, &brief).expect("prep");
    for (index, file) in &files {
        fs::write(lab.dir.join(file), png()).expect("png");
        add_image(&lab.dir, "stone-lantern", *index, file, None).expect("add");
    }
    files.into_iter().map(|(_, f)| f).collect()
}

/// A direction "Workbench objects" of titled object refs.
fn object_refs(lab: &Lab, kind: AssetKind) -> String {
    let request = NewObjectRefs {
        project: lab.project.clone(),
        source: "refs/workbench.png".into(),
        source_name: "Workbench".into(),
        objects: vec!["desk lamp".into(), "brush cup".into()],
        notes: HashMap::new(),
        aspect: Aspect::Square,
        kind,
        use_style: true,
    };
    let (direction, planned) = objects::prepare(&lab.dir, &request).expect("prep");
    for p in &planned {
        fs::write(lab.dir.join(&p.file), png()).expect("png");
        add_image(&lab.dir, &direction.id, p.index, &p.file, Some(&p.object)).expect("add");
    }
    direction.id
}

fn by_file(file: &str) -> PickRequest {
    PickRequest {
        image: ImageRef::File(file.to_string()),
        star: None,
        name: None,
        kind: None,
    }
}

fn by_index(direction: &str, index: u32) -> PickRequest {
    PickRequest {
        image: ImageRef::Numbered {
            direction: direction.to_string(),
            index,
        },
        ..by_file("")
    }
}

fn pick(lab: &Lab, requests: &[PickRequest]) -> CoreResult<Picked> {
    pick_images(&lab.root, &lab.layout, &lab.project, requests)
}

#[test]
fn a_pick_makes_the_direction_asset_at_refs() {
    let lab = lab();
    let files = concepts(&lab, 2);
    let picked = pick(
        &lab,
        &[PickRequest {
            star: Some(true),
            ..by_file(&files[1])
        }],
    )
    .expect("pick");
    let one = &picked.images[0];
    assert_eq!(one.asset.name, "Stone lantern");
    assert_eq!(one.asset.stage, Stage::Refs);
    assert_eq!(one.asset.kind, AssetKind::Prop);
    assert_eq!(one.ref_file, "refs/stone-lantern_02.png");
    assert!(one.copied);
    assert_eq!(one.index, 2);
    assert_eq!(
        one.asset.thumbnail.as_deref(),
        Some("refs/stone-lantern_02.png")
    );
    assert!(lab.dir.join("refs/stone-lantern_02.png").is_file());
    let saved = load_direction(&lab.dir, "stone-lantern")
        .expect("load")
        .expect("direction");
    let image = &saved.images[1];
    assert!(image.starred);
    assert!(!image.picked, "cleared after the send, as the Lab does");
    assert_eq!(image.ref_file.as_deref(), Some("refs/stone-lantern_02.png"));
    assert_eq!(picked.directions, vec![saved]);
    assert_eq!(picked.project.assets.len(), 1);
}

#[test]
fn picking_again_returns_the_same_asset() {
    let lab = lab();
    let files = concepts(&lab, 2);
    let first = pick(&lab, &[by_file(&files[0])]).expect("first");
    // Picked again by number, and with another name that must not fork it.
    let again = pick(
        &lab,
        &[PickRequest {
            name: Some("Lantern copy".into()),
            ..by_index("Stone lantern", 1)
        }],
    )
    .expect("again");
    assert!(!again.images[0].copied);
    assert_eq!(again.images[0].ref_file, first.images[0].ref_file);
    assert_eq!(again.images[0].asset.id, first.images[0].asset.id);
    assert_eq!(again.project.assets.len(), 1);
    let refs: Vec<_> = fs::read_dir(lab.dir.join("refs")).expect("refs").collect();
    assert_eq!(refs.len(), 1, "no second copy");

    // The second image joins the same asset, and its card shows the newest
    // send, as Send picks to Refs does.
    let second = pick(&lab, &[by_index("stone-lantern", 2)]).expect("second");
    assert_eq!(second.images[0].asset.id, first.images[0].asset.id);
    assert_eq!(
        second.images[0].asset.thumbnail.as_deref(),
        Some("refs/stone-lantern_02.png")
    );
}

#[test]
fn object_refs_are_named_after_their_object_with_the_direction_kind() {
    let lab = lab();
    fs::create_dir_all(lab.dir.join("refs")).expect("refs");
    fs::write(lab.dir.join("refs/workbench.png"), png()).expect("source");
    let direction = object_refs(&lab, AssetKind::Character);
    let picked = pick(
        &lab,
        &[
            by_index(&direction, 1),
            PickRequest {
                kind: Some(AssetKind::Prop),
                ..by_index("Workbench objects", 2)
            },
        ],
    )
    .expect("pick");
    let names: Vec<&str> = picked
        .images
        .iter()
        .map(|i| i.asset.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["Desk lamp", "Brush cup"],
        "capitalised, ids as before"
    );
    assert_eq!(picked.images[0].asset.id, "desk-lamp");
    assert_eq!(picked.images[0].ref_file, "refs/desk_lamp.png");
    assert_eq!(picked.images[1].ref_file, "refs/brush_cup.png");
    assert_eq!(picked.images[0].asset.kind, AssetKind::Character);
    assert_eq!(
        picked.images[1].asset.kind,
        AssetKind::Prop,
        "kind overrides"
    );

    let scene = pick(
        &lab,
        &[PickRequest {
            kind: Some(AssetKind::Scene),
            ..by_index(&direction, 1)
        }],
    );
    assert!(scene.is_err(), "object refs are never scenes");
}

#[test]
fn an_own_name_names_the_ref_and_moves_an_existing_asset_up() {
    let lab = lab();
    let files = concepts(&lab, 1);
    project::modify(&lab.root, &lab.layout, &lab.project, |m| {
        assets::add(
            m,
            NewAsset {
                name: "Garden lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Concept,
            },
        )
        .map(|_| ())
    })
    .expect("asset");
    let picked = pick(
        &lab,
        &[PickRequest {
            name: Some("Garden lantern".into()),
            kind: Some(AssetKind::Scene),
            ..by_file(&files[0])
        }],
    )
    .expect("pick");
    let one = &picked.images[0];
    assert_eq!(one.asset.id, "garden-lantern");
    assert_eq!(one.asset.stage, Stage::Refs);
    assert_eq!(
        one.asset.kind,
        AssetKind::Prop,
        "an existing asset keeps its kind"
    );
    assert_eq!(one.ref_file, "refs/garden_lantern.png");
    assert_eq!(picked.project.assets.len(), 1);
}

#[test]
fn bad_images_are_refused_before_anything_is_written() {
    let lab = lab();
    let files = concepts(&lab, 2);
    fs::write(lab.dir.join("refs.png"), png()).expect("stray");
    let outside = lab.root.join("outside.png");
    fs::write(&outside, png()).expect("outside");
    for bad in [
        "../outside.png",
        "concepts/../../outside.png",
        outside.to_string_lossy().as_ref(),
        "refs.png",
        "concepts/stone-lantern/stone-lantern_09.png",
        "concepts/nowhere/nowhere_01.png",
        "",
    ] {
        let refused = pick(&lab, &[by_file(&files[0]), by_file(bad)]);
        assert!(refused.is_err(), "{bad:?} must be refused");
    }
    assert!(pick(&lab, &[by_index("Stone lantern", 7)]).is_err());
    assert!(pick(&lab, &[by_index("Nowhere", 1)]).is_err());
    assert!(pick(&lab, &[by_file(&files[0]), by_index("stone-lantern", 1)]).is_err());
    assert!(pick(&lab, &[]).is_err());
    let many: Vec<PickRequest> = (0..=MAX_PICKS).map(|_| by_file(&files[0])).collect();
    assert!(pick(&lab, &many).is_err());
    let long = PickRequest {
        name: Some("x".repeat(200)),
        ..by_file(&files[0])
    };
    assert!(pick(&lab, &[long]).is_err());
    // A concept whose file went missing is refused too.
    fs::remove_file(lab.dir.join(&files[1])).expect("remove");
    assert!(pick(&lab, &[by_file(&files[1])]).is_err());

    assert!(!lab.dir.join("refs").join("stone-lantern_01.png").exists());
    let saved = project::load(&lab.root, &lab.layout, &lab.project).expect("load");
    assert!(saved.assets.is_empty());
    let direction = load_direction(&lab.dir, "stone-lantern")
        .expect("load")
        .expect("direction");
    assert!(direction.images.iter().all(|i| i.ref_file.is_none()));

    // An absolute path inside the project is fine.
    let inside = lab.dir.join(&files[0]).to_string_lossy().into_owned();
    assert!(pick(&lab, &[by_file(&inside)]).is_ok());
}

#[test]
fn stars_change_only_the_star() {
    let lab = lab();
    let files = concepts(&lab, 2);
    let changed = star_images(
        &lab.dir,
        &[
            (ImageRef::File(files[0].clone()), true),
            (
                ImageRef::Numbered {
                    direction: "Stone lantern".into(),
                    index: 2,
                },
                true,
            ),
        ],
    )
    .expect("star");
    assert_eq!(changed.len(), 1);
    assert!(changed[0].images.iter().all(|i| i.starred && !i.picked));
    let off = star_images(&lab.dir, &[(ImageRef::File(files[0].clone()), false)]).expect("off");
    assert!(!off[0].images[0].starred);
    assert!(off[0].images[1].starred);
    assert!(star_images(&lab.dir, &[(ImageRef::File("refs/x.png".into()), true)]).is_err());
    let saved = project::load(&lab.root, &lab.layout, &lab.project).expect("load");
    assert!(saved.assets.is_empty(), "a star puts nothing on the Board");
}
