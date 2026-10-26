use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::assets::{AssetStatus, NewAsset, Stage, add_version};
use crate::game::{GameFormat, NewGame, ProjectTarget};
use crate::prepare::{Fixup, RealSize, ScenePrep, SizeAxis, SizeUnit};
use crate::project::{self, NewProject, ProjectTemplate};
use crate::workspace::FolderLayout;

struct Fixture {
    root: tempfile::TempDir,
    layout: FolderLayout,
    moon: PathBuf,
    room: PathBuf,
}

impl Fixture {
    fn root(&self) -> &Path {
        self.root.path()
    }
}

fn stats(triangles: u32) -> ModelStats {
    ModelStats {
        triangles,
        bytes: 1000.0,
        textures: 1,
        max_texture: 2048,
        joints: 0,
        animations: 0,
    }
}

fn new_project(root: &Path, layout: &FolderLayout, name: &str, game: bool) -> PathBuf {
    let m = project::create(
        root,
        layout,
        NewProject {
            name: name.into(),
            template: ProjectTemplate::FloatingDiorama,
            target: if game {
                ProjectTarget::Game
            } else {
                ProjectTarget::Website
            },
            game: game.then(|| NewGame {
                format: GameFormat::Obj,
                models_dir: "assets/models".into(),
            }),
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    project::dir(root, layout, &m.id).expect("dir")
}

fn write(dir: &Path, file: &str, bytes: &str) {
    let path = dir.join(file);
    fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    fs::write(path, bytes).expect("write");
}

/// Moon Island has a desk lamp (v1, 80k triangles) with a render, a ref,
/// a real size and two fix-ups; Study room is empty.
fn fixture() -> Fixture {
    let root = tempfile::tempdir().expect("tempdir");
    let layout = FolderLayout::default();
    let moon = new_project(root.path(), &layout, "Moon Island", false);
    let room = new_project(root.path(), &layout, "Study room", false);
    write(&moon, "models/desk_lamp_v1.glb", "lamp 1");
    write(&moon, "thumbs/desk_lamp_v1.png", "render 1");
    write(&moon, "refs/desk_lamp.png", "front");
    project::modify(root.path(), &layout, "moon-island", |m| {
        let lamp = crate::assets::add(
            m,
            NewAsset {
                name: "Desk lamp".into(),
                kind: AssetKind::Prop,
                stage: Stage::Blender,
            },
        )?;
        add_version(
            m,
            &lamp.id,
            "models/desk_lamp_v1.glb".into(),
            Some(stats(80_000)),
        )?;
        m.assets[0].thumbnail = Some("thumbs/desk_lamp_v1.png".into());
        m.assets[0].status = AssetStatus::Done;
        Ok(())
    })
    .expect("lamp");
    let mut prep = ScenePrep::default();
    prep.models.insert(
        "desk-lamp".into(),
        ModelPrep {
            size: Some(RealSize {
                axis: SizeAxis::Height,
                value: 45.0,
                unit: SizeUnit::Cm,
            }),
            fixups: vec![Fixup::Decimate { triangles: 60_000 }, Fixup::OriginBottom],
            skip_bake: Some(true),
            pad_edges: None,
        },
    );
    crate::prepare::update(&moon, |_| Ok(prep.clone())).expect("prep");
    Fixture {
        root,
        layout,
        moon,
        room,
    }
}

fn add_lamp(f: &Fixture, tags: &[&str]) -> LibraryAdded {
    add(
        f.root(),
        &f.layout,
        &AddToLibrary {
            project: "moon-island".into(),
            asset: "desk-lamp".into(),
            name: None,
            tags: Some(tags.iter().map(ToString::to_string).collect()),
        },
        &[],
    )
    .expect("add")
}

/// Accepts a second, lighter model of the desk lamp in Moon Island.
fn second_model(f: &Fixture) {
    write(&f.moon, "models/desk_lamp_v2.glb", "lamp 2");
    project::modify(f.root(), &f.layout, "moon-island", |m| {
        add_version(
            m,
            "desk-lamp",
            "models/desk_lamp_v2.glb".into(),
            Some(stats(20_000)),
        )?;
        Ok(())
    })
    .expect("v2");
}

#[test]
fn adding_copies_the_model_picture_refs_and_settings_and_sets_the_origin() {
    let f = fixture();
    let added = add_lamp(&f, &["Props", "desk", "props"]);
    assert!(added.added);
    assert_eq!(added.version, 1);
    let entry = &added.entry;
    assert_eq!(entry.id, "desk-lamp");
    assert_eq!(entry.tags, ["props", "desk"]);
    assert_eq!(entry.refs, ["refs/desk_lamp.png"]);
    let v1 = &entry.versions[0];
    assert_eq!(v1.file, "models/desk_lamp_v1.glb");
    assert_eq!(v1.thumbnail.as_deref(), Some("thumbs/desk_lamp_v1.png"));
    assert_eq!(v1.source, LibrarySource::Imported);
    assert_eq!(v1.from.project, "moon-island");
    assert_eq!(v1.prep.as_ref().map(|p| p.fixups.len()), Some(2));
    let folder = dir(f.root()).join("desk-lamp");
    assert_eq!(
        fs::read_to_string(folder.join("models/desk_lamp_v1.glb")).expect("model"),
        "lamp 1"
    );
    assert!(folder.join(ENTRY_FILE).is_file());
    let lamp = &added.project.assets[0];
    assert_eq!(
        lamp.origin,
        Some(crate::AssetOrigin {
            entry: "desk-lamp".into(),
            version: 1
        })
    );
}

#[test]
fn adding_the_same_model_again_adds_nothing_and_a_newer_one_adds_a_version() {
    let f = fixture();
    add_lamp(&f, &["props"]);
    let again = add_lamp(&f, &["props", "lamps"]);
    assert!(!again.added);
    assert_eq!(again.entry.versions.len(), 1);
    assert_eq!(again.entry.tags, ["props", "lamps"]);

    second_model(&f);
    let newer = add_lamp(&f, &["props"]);
    assert!(newer.added);
    assert_eq!(newer.version, 2);
    assert_eq!(newer.entry.id, "desk-lamp");
    assert_eq!(newer.entry.versions[1].file, "models/desk_lamp_v2.glb");
    assert_eq!(list(f.root()).expect("list").len(), 1);
}

#[test]
fn an_asset_without_a_model_is_refused() {
    let f = fixture();
    project::modify(f.root(), &f.layout, "moon-island", |m| {
        crate::assets::add(
            m,
            NewAsset {
                name: "Bench".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )
        .map(|_| ())
    })
    .expect("bench");
    let refused = add(
        f.root(),
        &f.layout,
        &AddToLibrary {
            project: "moon-island".into(),
            asset: "bench".into(),
            name: None,
            tags: None,
        },
        &[],
    );
    assert!(matches!(refused, Err(CoreError::Invalid(_))));
    assert!(list(f.root()).expect("list").is_empty());
}

#[test]
fn using_an_entry_copies_it_in_as_a_new_asset_with_its_origin() {
    let f = fixture();
    add_lamp(&f, &["props"]);
    let used = use_in_project(f.root(), &f.layout, "desk-lamp", None, "study-room").expect("use");
    let lamp = &used.asset;
    assert_eq!(lamp.id, "desk-lamp");
    assert_eq!(lamp.versions.len(), 1);
    assert_eq!(lamp.versions[0].file, "models/desk_lamp_v1.glb");
    assert_eq!(lamp.thumbnail.as_deref(), Some("thumbs/desk_lamp_v1.png"));
    assert_eq!(lamp.origin.as_ref().map(|o| o.version), Some(1));
    assert!(used.settings);
    assert!(used.note.is_none());
    assert!(f.room.join("models/desk_lamp_v1.glb").is_file());
    assert!(f.room.join("refs/desk_lamp.png").is_file());
    let prep = crate::prepare::load(&f.room).expect("prep");
    assert_eq!(prep.models["desk-lamp"].fixups.len(), 2);

    // A second use gets a free name; the first copy is the project's own.
    let again = use_in_project(f.root(), &f.layout, "desk-lamp", None, "study-room").expect("use");
    assert_eq!(again.asset.name, "Desk lamp 2");
    assert_eq!(again.asset.versions[0].file, "models/desk_lamp_2_v1.glb");
}

#[test]
fn a_newer_version_shows_as_an_update_until_the_asset_is_updated() {
    let f = fixture();
    add_lamp(&f, &[]);
    use_in_project(f.root(), &f.layout, "desk-lamp", None, "study-room").expect("use");
    let uses = |f: &Fixture| {
        view(f.root(), &f.layout).expect("view").items[0]
            .uses
            .clone()
    };
    assert!(uses(&f).iter().all(|u| !u.update));

    second_model(&f);
    add_lamp(&f, &[]);
    let found = uses(&f);
    let room = found
        .iter()
        .find(|u| u.project == "study-room")
        .expect("room");
    assert!(room.update);
    assert_eq!(room.version, 1);
    // The asset the new version came from is on it already.
    let moon = found
        .iter()
        .find(|u| u.project == "moon-island")
        .expect("moon");
    assert!(!moon.update);
    // Nothing changed the room on its own.
    let manifest = project::load(f.root(), &f.layout, "study-room").expect("room");
    assert_eq!(manifest.assets[0].versions.len(), 1);
}

#[test]
fn update_adds_the_new_model_as_the_next_version_and_keeps_the_fix_ups_that_apply() {
    let f = fixture();
    add_lamp(&f, &[]);
    use_in_project(f.root(), &f.layout, "desk-lamp", None, "study-room").expect("use");
    second_model(&f);
    add_lamp(&f, &[]);

    let plan = plan_update(f.root(), &f.layout, "study-room", "desk-lamp").expect("plan");
    assert_eq!((plan.from, plan.to, plan.next_version), (1, 2, 2));
    assert_eq!(plan.dropped.len(), 1);
    assert_eq!(plan.dropped[0].what, "Decimate to 60k");
    assert!(plan.dropped[0].why.contains("20k triangles"));
    assert!(plan.kept.contains(&"Origin to bottom centre".to_string()));
    assert!(plan.kept.contains(&"Real size, 45 cm high".to_string()));

    let done = update(f.root(), &f.layout, "study-room", "desk-lamp").expect("update");
    let lamp = &done.asset;
    assert_eq!(lamp.versions.len(), 2);
    assert_eq!(lamp.versions[1].file, "models/desk_lamp_v2.glb");
    assert_eq!(lamp.origin.as_ref().map(|o| o.version), Some(2));
    assert_eq!(
        fs::read_to_string(f.room.join("models/desk_lamp_v2.glb")).expect("v2"),
        "lamp 2"
    );
    let prep = crate::prepare::load(&f.room).expect("prep");
    let entry = &prep.models["desk-lamp"];
    assert_eq!(entry.fixups, [Fixup::OriginBottom]);
    assert!(entry.size.is_some() && entry.skips_bake());

    let twice = plan_update(f.root(), &f.layout, "study-room", "desk-lamp");
    assert!(matches!(twice, Err(CoreError::Invalid(_))));
}

#[test]
fn a_website_refuses_an_obj_and_a_game_says_what_its_next_run_does() {
    let f = fixture();
    add_lamp(&f, &[]);
    new_project(f.root(), &f.layout, "Crypt", true);
    let used = use_in_project(f.root(), &f.layout, "desk-lamp", None, "crypt").expect("game");
    let note = used.note.expect("note");
    assert!(note.contains("takes .obj"), "{note}");
    assert!(note.contains("Nothing is converted now"), "{note}");

    let folder = dir(f.root()).join("desk-lamp");
    let mut entry = load(f.root(), "desk-lamp").expect("entry");
    write(&folder, "models/crate_v2/crate.obj", "o crate");
    write(&folder, "models/crate_v2/crate.mtl", "newmtl wood");
    let mut v2 = entry.versions[0].clone();
    v2.version = 2;
    v2.file = "models/crate_v2/crate.obj".into();
    v2.hash = "other".into();
    entry.versions.push(v2);
    save(f.root(), &entry).expect("save");
    let refused = use_in_project(f.root(), &f.layout, "desk-lamp", None, "study-room");
    assert!(matches!(refused, Err(CoreError::Invalid(_))));
    let game = use_in_project(f.root(), &f.layout, "desk-lamp", None, "crypt").expect("obj");
    assert_eq!(
        game.asset.versions[0].file,
        "models/desk_lamp_2_v1/crate.obj"
    );
    let crypt = project::dir(f.root(), &f.layout, "crypt").expect("dir");
    assert!(crypt.join("models/desk_lamp_2_v1/crate.mtl").is_file());
}

#[test]
fn library_paths_cannot_leave_the_library() {
    let f = fixture();
    for id in ["..", "../moon-island", "a/b", ".hidden", ""] {
        assert!(entry_dir(f.root(), id).is_err(), "{id:?}");
        assert!(load(f.root(), id).is_err(), "{id:?}");
    }
    add_lamp(&f, &[]);
    let folder = dir(f.root()).join("desk-lamp");
    let text = fs::read_to_string(folder.join(ENTRY_FILE)).expect("entry");
    for bad in [
        "../../projects/moon-island/project.json",
        "/etc/passwd",
        "models\\\\x.glb",
    ] {
        let forged = text.replace("models/desk_lamp_v1.glb", bad);
        fs::write(folder.join(ENTRY_FILE), forged).expect("forge");
        assert!(
            matches!(load(f.root(), "desk-lamp"), Err(CoreError::Invalid(_))),
            "{bad}"
        );
        assert!(use_in_project(f.root(), &f.layout, "desk-lamp", None, "study-room").is_err());
    }
    // An unreadable entry is skipped in the list, never followed.
    assert!(list(f.root()).expect("list").is_empty());
    let room = project::load(f.root(), &f.layout, "study-room").expect("room");
    assert!(room.assets.is_empty());
}

#[test]
fn tags_are_tidied_and_limited() {
    let tags = |t: &[&str]| clean_tags(&t.iter().map(ToString::to_string).collect::<Vec<_>>());
    assert_eq!(tags(&[" Props ", "props", ""]).expect("ok"), ["props"]);
    assert!(tags(&["x".repeat(TAG_MAX_CHARS + 1).as_str()]).is_err());
    let many: Vec<String> = (0..=MAX_TAGS).map(|n| format!("t{n}")).collect();
    assert!(clean_tags(&many).is_err());
}
