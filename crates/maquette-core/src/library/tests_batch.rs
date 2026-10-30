//! Library at scale: bulk tags, rename, collections, remove and restore.
//! The helpers here draw real PNG pictures and write real .glb headers, and
//! the plan and merge tests use them too.

use std::fs;
use std::path::PathBuf;

use maquette_glb::images::write_png;

use super::tests::{Fixture, add_lamp, fixture, stats};
use super::*;
use crate::assets::{AssetStatus, NewAsset, Stage, add_version};
use crate::error::CoreError;
use crate::project;

const GROUND: [u8; 3] = [212, 212, 216];
pub(super) const RED: [u8; 3] = [196, 40, 36];
pub(super) const BLUE: [u8; 3] = [40, 70, 200];

/// A 200 px square of studio grey, `colour` where `inside` says.
fn picture(colour: [u8; 3], inside: impl Fn(f64, f64) -> bool) -> Vec<u8> {
    let mut rgb = Vec::new();
    for y in 0..200u32 {
        for x in 0..200u32 {
            let (fx, fy) = (f64::from(x) / 200.0, f64::from(y) / 200.0);
            rgb.extend_from_slice(&if inside(fx, fy) { colour } else { GROUND });
        }
    }
    write_png(200, 200, &rgb)
}

/// A tall block.
pub(super) fn block(colour: [u8; 3]) -> Vec<u8> {
    picture(colour, |x, y| {
        (0.3..0.7).contains(&x) && (0.15..0.85).contains(&y)
    })
}

/// A long flat bar.
pub(super) fn bar(colour: [u8; 3]) -> Vec<u8> {
    picture(colour, |x, y| {
        (0.1..0.9).contains(&x) && (0.45..0.55).contains(&y)
    })
}

/// A .glb header and `body`, its declared length the real one.
pub(super) fn glb(body: &str) -> Vec<u8> {
    let len = u32::try_from(12 + body.len()).expect("small");
    let mut out = b"glTF".to_vec();
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(body.as_bytes());
    out
}

pub(super) fn project_dir(f: &Fixture, project: &str) -> PathBuf {
    project::dir(f.root(), &f.layout, project).expect("dir")
}

/// Gives asset `name` in `project` a new model, made with `bytes`, and a
/// picture when `thumb` is given; the asset is made when it is new.
/// Answers its id.
pub(super) fn model(
    f: &Fixture,
    project: &str,
    name: &str,
    bytes: &[u8],
    thumb: Option<&[u8]>,
) -> String {
    let dir = project_dir(f, project);
    let mut id = String::new();
    project::modify(f.root(), &f.layout, project, |m| {
        let found = m
            .assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.id.clone());
        id = match found {
            Some(id) => id,
            None => {
                crate::assets::add(
                    m,
                    NewAsset {
                        name: name.into(),
                        kind: AssetKind::Prop,
                        stage: Stage::Blender,
                    },
                )?
                .id
            }
        };
        let asset = m.assets.iter_mut().find(|a| a.id == id).expect("asset");
        let n = asset.versions.len() + 1;
        let file = format!("models/{id}_v{n}.glb");
        fs::create_dir_all(dir.join("models")).expect("models");
        fs::write(dir.join(&file), bytes).expect("model");
        if let Some(png) = thumb {
            let pic = format!("thumbs/{id}_v{n}.png");
            fs::create_dir_all(dir.join("thumbs")).expect("thumbs");
            fs::write(dir.join(&pic), png).expect("thumb");
            asset.thumbnail = Some(pic);
        }
        asset.status = AssetStatus::Done;
        add_version(m, &id, file, Some(stats(1200)))?;
        Ok(())
    })
    .expect("model");
    id
}

/// Adds the asset's newest model to the library.
pub(super) fn to_library(f: &Fixture, project: &str, asset: &str) -> LibraryAdded {
    add(
        f.root(),
        &f.layout,
        &AddToLibrary {
            project: project.into(),
            asset: asset.into(),
            name: None,
            tags: None,
            into: None,
        },
        &[],
    )
    .expect("add")
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(ToString::to_string).collect()
}

#[test]
fn tags_go_on_and_off_many_entries_and_bad_requests_write_nothing() {
    let f = fixture();
    add_lamp(&f, &["props", "old"]);
    let key = model(&f, "moon-island", "Brass key", &glb("key"), None);
    to_library(&f, "moon-island", &key);

    let done = tag(
        f.root(),
        &TagEntries {
            entries: strings(&["brass-key", "desk-lamp"]),
            add: strings(&[" Metal ", "props"]),
            remove: strings(&["OLD"]),
        },
    )
    .expect("tag");
    assert_eq!(done[0].id, "brass-key");
    assert_eq!(done[0].tags, ["metal", "props"]);
    assert_eq!(done[1].tags, ["props", "metal"]);
    assert_eq!(
        load(f.root(), "desk-lamp").expect("lamp").tags,
        ["props", "metal"]
    );

    // Nothing to change leaves the entry and its date alone.
    let same = tag(
        f.root(),
        &TagEntries {
            entries: strings(&["brass-key"]),
            add: strings(&["metal"]),
            remove: Vec::new(),
        },
    )
    .expect("same");
    assert_eq!(same[0].updated_at, done[0].updated_at);

    let unknown = tag(
        f.root(),
        &TagEntries {
            entries: strings(&["desk-lamp", "nope"]),
            add: strings(&["new"]),
            remove: Vec::new(),
        },
    );
    assert!(matches!(unknown, Err(CoreError::NotFound(_))));
    let many: Vec<String> = (0..MAX_TAGS).map(|n| format!("t{n}")).collect();
    let too_many = tag(
        f.root(),
        &TagEntries {
            entries: strings(&["brass-key", "desk-lamp"]),
            add: many,
            remove: Vec::new(),
        },
    );
    assert!(matches!(too_many, Err(CoreError::Invalid(_))));
    assert_eq!(
        load(f.root(), "desk-lamp").expect("lamp").tags,
        ["props", "metal"]
    );
}

#[test]
fn renaming_keeps_the_id_and_refuses_an_empty_name() {
    let f = fixture();
    add_lamp(&f, &[]);
    let renamed = rename(f.root(), "desk-lamp", "  Reading lamp ").expect("rename");
    assert_eq!(
        (renamed.id.as_str(), renamed.name.as_str()),
        ("desk-lamp", "Reading lamp")
    );
    assert_eq!(
        load(f.root(), "desk-lamp").expect("lamp").name,
        "Reading lamp"
    );
    assert!(rename(f.root(), "desk-lamp", "  ").is_err());
    assert!(rename(f.root(), "nope", "Lamp").is_err());
}

#[test]
fn collections_are_made_filled_renamed_and_deleted_without_touching_models() {
    let f = fixture();
    add_lamp(&f, &[]);
    let key = model(&f, "moon-island", "Brass key", &glb("key"), None);
    to_library(&f, "moon-island", &key);
    let create = |name: &str, entries: &[&str]| {
        change_collection(
            f.root(),
            &CollectionChange::Create {
                name: name.into(),
                entries: strings(entries),
            },
        )
    };
    let made = create("Study props", &["desk-lamp"]).expect("create");
    assert_eq!(made[0].id, "study-props");
    assert!(matches!(
        create("study PROPS", &[]),
        Err(CoreError::Invalid(_))
    ));
    assert!(matches!(
        create("Other", &["nope"]),
        Err(CoreError::NotFound(_))
    ));
    assert!(create(" ", &[]).is_err());
    let all = create("Attic", &[]).expect("attic");
    assert_eq!(
        all.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["Attic", "Study props"]
    );

    let add = |entries: &[&str]| {
        change_collection(
            f.root(),
            &CollectionChange::Add {
                collection: "study-props".into(),
                entries: strings(entries),
            },
        )
    };
    let added = add(&["desk-lamp", "brass-key"]).expect("add");
    let props = added.iter().find(|c| c.id == "study-props").expect("props");
    assert_eq!(props.entries, ["desk-lamp", "brass-key"], "no lamp twice");
    assert!(add(&["nope"]).is_err());

    let renamed = change_collection(
        f.root(),
        &CollectionChange::Rename {
            collection: "study-props".into(),
            name: "Desk things".into(),
        },
    )
    .expect("rename");
    assert!(
        renamed
            .iter()
            .any(|c| c.id == "study-props" && c.name == "Desk things")
    );
    let taken = change_collection(
        f.root(),
        &CollectionChange::Rename {
            collection: "study-props".into(),
            name: "attic".into(),
        },
    );
    assert!(taken.is_err());

    let removed = change_collection(
        f.root(),
        &CollectionChange::Remove {
            collection: "study-props".into(),
            entries: strings(&["desk-lamp"]),
        },
    )
    .expect("remove");
    let props = removed
        .iter()
        .find(|c| c.id == "study-props")
        .expect("props");
    assert_eq!(props.entries, ["brass-key"]);

    let left = change_collection(
        f.root(),
        &CollectionChange::Delete {
            collection: "study-props".into(),
        },
    )
    .expect("delete");
    assert_eq!(left.len(), 1);
    assert_eq!(list(f.root()).expect("list").len(), 2, "models stay");
    assert_eq!(collections(f.root()).expect("read"), left);
}

#[test]
fn removing_and_restoring_round_trips_and_leaves_projects_alone() {
    let f = fixture();
    add_lamp(&f, &["props"]);
    use_in_project(f.root(), &f.layout, "desk-lamp", None, "study-room").expect("use");
    change_collection(
        f.root(),
        &CollectionChange::Create {
            name: "Lamps".into(),
            entries: strings(&["desk-lamp"]),
        },
    )
    .expect("collection");
    let room_before = project::load(f.root(), &f.layout, "study-room").expect("room");

    assert!(matches!(
        remove(f.root(), &strings(&["desk-lamp", "nope"])),
        Err(CoreError::NotFound(_))
    ));
    assert_eq!(list(f.root()).expect("list").len(), 1, "nothing moved");

    let gone = remove(f.root(), &strings(&["desk-lamp"])).expect("remove");
    assert_eq!(gone, ["desk-lamp"]);
    assert!(list(f.root()).expect("list").is_empty());
    assert!(
        dir(f.root())
            .join(REMOVED_DIR)
            .join("desk-lamp")
            .join(ENTRY_FILE)
            .is_file()
    );
    let room = project::load(f.root(), &f.layout, "study-room").expect("room");
    assert_eq!(room, room_before, "projects keep their copies");
    assert!(f.room.join("models/desk_lamp_v1.glb").is_file());
    let shown = view(f.root(), &f.layout).expect("view");
    assert!(shown.collections[0].entries.is_empty());

    let back = restore(f.root(), &f.layout, &strings(&["desk-lamp"])).expect("restore");
    assert_eq!(back.entries, ["desk-lamp"]);
    assert!(back.projects.is_empty());
    assert_eq!(load(f.root(), "desk-lamp").expect("lamp").tags, ["props"]);
    let shown = view(f.root(), &f.layout).expect("view");
    assert_eq!(shown.collections[0].entries, ["desk-lamp"]);
    assert_eq!(shown.items[0].uses.len(), 2);

    // A second remove replaces the older removed copy; a folder back in
    // the library blocks the restore.
    remove(f.root(), &strings(&["desk-lamp"])).expect("again");
    add_lamp(&f, &[]);
    let blocked = restore(f.root(), &f.layout, &strings(&["desk-lamp"]));
    assert!(matches!(blocked, Err(CoreError::Invalid(_))));
    assert!(restore(f.root(), &f.layout, &strings(&["nope"])).is_err());
}
