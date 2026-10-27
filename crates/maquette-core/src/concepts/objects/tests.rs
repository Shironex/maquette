//! Tests for object refs: one isolated image per object of a concept.

use super::*;
use crate::assets::Stage;
use crate::concepts::{add_image, mark};

fn request(objects: &[&str]) -> NewObjectRefs {
    NewObjectRefs {
        project: "landing".into(),
        source: "concepts/workbench-hero/workbench-hero_01.png".into(),
        source_name: "Workbench hero".into(),
        objects: objects.iter().map(|o| (*o).to_string()).collect(),
        notes: HashMap::new(),
        aspect: Aspect::Square,
        kind: AssetKind::Prop,
    }
}

#[test]
fn the_object_prompt_asks_for_one_isolated_object() {
    let out = Path::new("/w/landing/concepts/workbench-hero-objects/workbench-hero-objects_01.png");
    let text = object_prompt(
        "desk lamp",
        "",
        AssetKind::Prop,
        "Workbench hero",
        Aspect::Square,
        out,
    );
    assert!(text.starts_with("Generate an image.\n\n"));
    assert!(text.contains(
        "Make a clean reference image of one object for a 3D model: the \"desk lamp\" from the attached concept image (the direction \"Workbench hero\")."
    ));
    for wanted in [
        "Show only the desk lamp, isolated: centered,",
        "the whole object in frame",
        "front three-quarter view",
        "plain light grey background",
        "even soft light",
        "no cast shadow on the background",
        "No other objects, no text, no labels, no watermark.",
        "Match the source's materials, colours, proportions and style",
        "Use a 1:1 square composition.",
        "~/.codex/generated_images/",
    ] {
        assert!(text.contains(wanted), "missing {wanted:?} in\n{text}");
    }
    assert!(!text.contains("head to feet"));
    assert!(text.trim_end().ends_with(&out.display().to_string()));
    assert!(!text.contains('\u{2014}') && !text.contains('\u{2013}'));

    assert!(!text.contains("Extra instructions"));

    let figure = object_prompt(
        "clay figure",
        "",
        AssetKind::Character,
        "",
        Aspect::Portrait,
        out,
    );
    assert!(figure.contains("one character for a 3D model"));
    assert!(figure.contains("from the attached concept image."));
    assert!(figure.contains("the whole character in frame"));
    assert!(figure.contains("from head to feet"));
    assert!(figure.contains("3:4 portrait"));
}

#[test]
fn notes_go_into_the_prompt_after_the_look_and_never_into_the_name() {
    let out = Path::new("/w/landing/concepts/x/x_01.png");
    let text = object_prompt(
        "Island",
        "  Painted stone and moss,\nno wireframe lines. ",
        AssetKind::Prop,
        "Hero",
        Aspect::Square,
        out,
    );
    assert!(text.contains("the \"Island\" from"));
    assert!(text.contains(
        "as in the concept.\n\nExtra instructions for this object: Painted stone and moss,\nno wireframe lines.\n\nUse a 1:1"
    ));
    assert!(!text.contains('\u{2014}'));
}

#[test]
fn notes_are_matched_to_objects_and_cleaned() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("concepts/workbench-hero");
    fs::create_dir_all(&src).expect("mkdir");
    fs::write(src.join("workbench-hero_01.png"), b"png").expect("png");
    let mut req = request(&["Desk lamp", "Laptop"]);
    req.notes = HashMap::from([
        ("desk LAMP".to_string(), "  brass,   warm bulb ".to_string()),
        ("Laptop".to_string(), "   ".to_string()),
        ("Stranger".to_string(), "dropped".to_string()),
    ]);
    let ok = clean(dir.path(), &req).expect("ok");
    assert_eq!(
        ok.notes,
        HashMap::from([("Desk lamp".to_string(), "brass, warm bulb".to_string())])
    );
    req.notes = HashMap::from([("Laptop".to_string(), "n".repeat(401))]);
    assert!(clean(dir.path(), &req).is_err());
    let (_, planned) = prepare(dir.path(), &ok).expect("prep");
    assert_eq!(planned[0].notes, "brass, warm bulb");
    assert_eq!(planned[1].notes, "");
}

#[test]
fn object_lists_are_split_and_cleaned() {
    assert_eq!(
        split_objects("desk lamp\n  plaster   plinth with clay figure ,laptop;\nDesk lamp\n\n"),
        vec!["desk lamp", "plaster plinth with clay figure", "laptop"]
    );
    assert_eq!(
        direction_name("  Workbench hero "),
        "Workbench hero objects"
    );
    assert_eq!(direction_name(""), "Concept objects");
    let long = "x".repeat(80);
    assert_eq!(direction_name(&long).chars().count(), 60);
}

#[test]
fn requests_are_checked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("concepts/workbench-hero");
    fs::create_dir_all(&src).expect("mkdir");
    fs::write(src.join("workbench-hero_01.png"), b"png").expect("png");
    let ok = clean(dir.path(), &request(&["desk lamp", " laptop ", "Laptop"])).expect("ok");
    assert_eq!(ok.objects, vec!["desk lamp", "laptop"]);
    assert!(clean(dir.path(), &request(&[])).is_err());
    assert!(clean(dir.path(), &request(&[&"y".repeat(61)])).is_err());
    let many: Vec<String> = (0..13).map(|n| format!("thing {n}")).collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    assert!(clean(dir.path(), &request(&many)).is_err());
    let mut outside = request(&["lamp"]);
    outside.source = "../elsewhere.png".into();
    assert!(clean(dir.path(), &outside).is_err());
    outside.source = "/abs/elsewhere.png".into();
    assert!(clean(dir.path(), &outside).is_err());
    let mut missing = request(&["lamp"]);
    missing.source = "refs/missing.png".into();
    assert!(clean(dir.path(), &missing).is_err());
    let mut scene = request(&["lamp"]);
    scene.kind = AssetKind::Scene;
    assert!(clean(dir.path(), &scene).is_err());
}

#[test]
fn prepare_names_the_direction_after_the_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (direction, planned) =
        prepare(dir.path(), &request(&["desk lamp", "laptop"])).expect("prep");
    assert_eq!(direction.name, "Workbench hero objects");
    assert_eq!(direction.id, "workbench-hero-objects");
    assert_eq!(direction.aspect, Aspect::Square);
    assert_eq!(direction.kind, Some(AssetKind::Prop));
    assert!(direction.is_objects());
    assert_eq!(
        direction.refs,
        vec!["concepts/workbench-hero/workbench-hero_01.png".to_string()]
    );
    assert_eq!(
        planned,
        vec![
            PlannedObject {
                index: 1,
                file: "concepts/workbench-hero-objects/workbench-hero-objects_01.png".into(),
                object: "desk lamp".into(),
                notes: String::new(),
            },
            PlannedObject {
                index: 2,
                file: "concepts/workbench-hero-objects/workbench-hero-objects_02.png".into(),
                object: "laptop".into(),
                notes: String::new(),
            },
        ]
    );
}

#[test]
fn send_object_picks_makes_one_asset_per_object() {
    let root = tempfile::tempdir().expect("tempdir");
    let layout = FolderLayout::default();
    let manifest = project::create(
        root.path(),
        &layout,
        crate::project::NewProject {
            name: "Landing".into(),
            template: crate::project::ProjectTemplate::Room,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(root.path(), &layout, &manifest.id).expect("dir");
    let (direction, planned) = prepare(
        &project_dir,
        &request(&["desk lamp", "laptop", "brush cup"]),
    )
    .expect("prep");
    let png = crate::inbox::images::write_png(2, 2, &[90; 12]);
    for p in &planned {
        fs::write(project_dir.join(&p.file), &png).expect("png");
        add_image(
            &project_dir,
            &direction.id,
            p.index,
            &p.file,
            Some(&p.object),
        )
        .expect("add");
    }
    assert!(
        send_object_picks(root.path(), &layout, &manifest.id, &direction.id, None).is_err(),
        "nothing picked"
    );
    // A laptop asset made earlier keeps its name as it is.
    project::modify(root.path(), &layout, &manifest.id, |m| {
        assets::add(
            m,
            crate::assets::NewAsset {
                name: "laptop".into(),
                kind: AssetKind::Prop,
                stage: Stage::Concept,
            },
        )
        .map(|_| ())
    })
    .expect("earlier asset");
    // Someone already keeps a laptop.png in refs/.
    fs::write(project_dir.join("refs/laptop.png"), b"mine").expect("theirs");
    for p in &planned[..2] {
        mark(&project_dir, &direction.id, &p.file, None, Some(true)).expect("pick");
    }
    let sent =
        send_object_picks(root.path(), &layout, &manifest.id, &direction.id, None).expect("send");
    let names: Vec<&str> = sent.assets.iter().map(|a| a.asset.name.as_str()).collect();
    assert_eq!(names, vec!["Desk lamp", "laptop"]);
    assert_eq!(sent.assets[0].asset.id, "desk-lamp");
    assert_eq!(sent.assets[0].refs, vec!["refs/desk_lamp.png".to_string()]);
    assert_eq!(sent.assets[1].refs, vec!["refs/laptop_2.png".to_string()]);
    assert_eq!(
        fs::read(project_dir.join("refs/laptop.png")).expect("kept"),
        b"mine"
    );
    assert!(sent.assets.iter().all(|a| a.asset.stage == Stage::Refs
        && a.asset.kind == AssetKind::Prop
        && a.asset.thumbnail.as_deref() == a.refs.first().map(String::as_str)));
    assert_eq!(sent.copied.len(), 2);
    assert_eq!(
        sent.direction.images[0].ref_file.as_deref(),
        Some("refs/desk_lamp.png")
    );

    // Again, as characters: no new copies, the same assets.
    let again = send_object_picks(
        root.path(),
        &layout,
        &manifest.id,
        &direction.id,
        Some(AssetKind::Character),
    )
    .expect("again");
    assert!(again.copied.is_empty());
    assert_eq!(again.assets[0].asset.id, sent.assets[0].asset.id);
    let saved = project::load(root.path(), &layout, &manifest.id).expect("load");
    assert_eq!(saved.assets.len(), 2);
}
