//! Look-alikes on real PNG pictures, merging two entries and undoing the
//! merge with restore.

use super::tests::{add_lamp, fixture, second_model};
use super::tests_batch::{BLUE, RED, bar, block, glb, model, to_library};
use super::*;
use crate::error::CoreError;
use crate::project;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(ToString::to_string).collect()
}

#[test]
fn look_alikes_pair_same_looks_and_same_names_and_hide_decided_pairs() {
    let f = fixture();
    add_lamp(&f, &[]);
    let crate_id = model(&f, "moon-island", "Crate", &glb("crate"), Some(&block(RED)));
    to_library(&f, "moon-island", &crate_id);
    let copy = model(
        &f,
        "study-room",
        "Crate copy",
        &glb("crate b"),
        Some(&block(RED)),
    );
    to_library(&f, "study-room", &copy);
    let stool = model(&f, "study-room", "Stool", &glb("stool"), Some(&bar(BLUE)));
    to_library(&f, "study-room", &stool);
    let other = model(
        &f,
        "moon-island",
        "Stool 2",
        &glb("stool b"),
        Some(&block(BLUE)),
    );
    to_library(&f, "moon-island", &other);

    let found = look_alikes(f.root()).expect("scan");
    assert_eq!(found.unseen, ["desk-lamp"], "its render is not a PNG");
    assert_eq!(found.pairs.len(), 2, "{:?}", found.pairs);
    let looks = &found.pairs[0];
    assert_eq!(looks.kind, LookAlikeKind::SameLook);
    assert_eq!(
        (looks.a.as_str(), looks.b.as_str()),
        ("crate", "crate-copy")
    );
    assert!(looks.score.is_some_and(|s| s >= 0.9));
    assert!(looks.shape.is_some() && looks.colour.is_some() && looks.ratio.is_some());
    let names = &found.pairs[1];
    assert_eq!(names.kind, LookAlikeKind::SameName);
    assert_eq!((names.a.as_str(), names.b.as_str()), ("stool", "stool-2"));
    assert!(names.score.is_some_and(|s| s < 0.9));
    assert_eq!(found.settings, LookAlikeSettings::default());
    assert!(found.merges.is_empty());

    decide(f.root(), "crate-copy", "crate", LookAlikeVerdict::NotAlike).expect("decide");
    decide(f.root(), "stool", "stool-2", LookAlikeVerdict::KeepNames).expect("decide");
    let after = look_alikes(f.root()).expect("scan");
    assert!(after.pairs.is_empty());
    assert!(decide(f.root(), "crate", "crate", LookAlikeVerdict::KeepBoth).is_err());
}

#[test]
fn settings_keep_the_threshold_in_range() {
    let f = fixture();
    let low = LookAlikeSettings {
        threshold: 0.1,
        check_on_add: false,
        copy_tags: false,
    };
    let saved = save_settings(f.root(), &low).expect("save");
    assert!((saved.threshold - 0.5).abs() < f64::EPSILON);
    let high = LookAlikeSettings {
        threshold: 2.0,
        ..low
    };
    let saved = save_settings(f.root(), &high).expect("save");
    assert!((saved.threshold - 0.99).abs() < f64::EPSILON);
    assert_eq!(look_alikes(f.root()).expect("scan").settings, saved);
}

#[test]
fn a_merge_carries_versions_tags_and_origins_and_restore_undoes_it() {
    let f = fixture();
    add_lamp(&f, &["props"]);
    use_in_project(f.root(), &f.layout, "desk-lamp", None, "study-room").expect("use");
    // A second entry of the same lamp: v1 is new, v2 is the desk lamp's v1.
    let lamp = model(&f, "study-room", "Lamp", &glb("lamp b"), None);
    to_library(&f, "study-room", &lamp);
    model(&f, "study-room", "Lamp", b"lamp 1", None);
    to_library(&f, "study-room", &lamp);
    tag(
        f.root(),
        &TagEntries {
            entries: strings(&["lamp"]),
            add: strings(&["night"]),
            remove: Vec::new(),
        },
    )
    .expect("tag");
    use_in_project(f.root(), &f.layout, "lamp", Some(1), "moon-island").expect("use v1");
    let origin = |project: &str, asset: &str| {
        let m = project::load(f.root(), &f.layout, project).expect("project");
        let found = m.assets.iter().find(|a| a.id == asset).expect("asset");
        found.origin.clone().map(|o| (o.entry, o.version))
    };
    let request = MergeEntries {
        keep: "desk-lamp".into(),
        merge: "lamp".into(),
        copy_tags: true,
    };

    let merged = merge(f.root(), &f.layout, &request).expect("merge");
    assert_eq!(merged.merge.versions, [2, 1]);
    assert_eq!(merged.entry.versions.len(), 2);
    assert_eq!(merged.entry.tags, ["props", "night"]);
    assert_eq!(merged.projects.len(), 2);
    assert_eq!(origin("study-room", "lamp"), Some(("desk-lamp".into(), 1)));
    assert_eq!(origin("moon-island", "lamp"), Some(("desk-lamp".into(), 2)));
    assert_eq!(
        origin("study-room", "desk-lamp"),
        Some(("desk-lamp".into(), 1))
    );
    let folder = dir(f.root()).join("desk-lamp");
    assert!(folder.join("models/desk_lamp_v2.glb").is_file());
    assert_eq!(list(f.root()).expect("list").len(), 1);
    let recent = look_alikes(f.root()).expect("scan").merges;
    assert_eq!(recent.len(), 1);
    assert_eq!(
        (recent[0].merged.as_str(), recent[0].into_name.as_str()),
        ("lamp", "Desk lamp")
    );

    let back = restore(f.root(), &f.layout, &strings(&["lamp"])).expect("undo");
    assert_eq!(back.entries, ["lamp"]);
    assert_eq!(back.projects.len(), 2);
    let keep = load(f.root(), "desk-lamp").expect("keep");
    assert_eq!(keep.versions.len(), 1);
    assert_eq!(keep.tags, ["props"]);
    assert!(!folder.join("models/desk_lamp_v2.glb").exists());
    assert_eq!(load(f.root(), "lamp").expect("lamp").versions.len(), 2);
    assert_eq!(origin("study-room", "lamp"), Some(("lamp".into(), 2)));
    assert_eq!(origin("moon-island", "lamp"), Some(("lamp".into(), 1)));
    assert!(!dir(f.root()).join("lamp").join("merged.json").exists());
    assert!(look_alikes(f.root()).expect("scan").merges.is_empty());

    // Once the kept entry has a newer version, the merge stays.
    merge(f.root(), &f.layout, &request).expect("merge again");
    second_model(&f);
    add_lamp(&f, &[]);
    let refused = restore(f.root(), &f.layout, &strings(&["lamp"]));
    assert!(matches!(refused, Err(CoreError::Invalid(_))), "{refused:?}");
    assert!(load(f.root(), "lamp").is_err(), "still merged");
}

#[test]
fn a_merge_refuses_itself_unknown_entries_and_too_many_tags() {
    let f = fixture();
    add_lamp(&f, &["a"]);
    let lamp = model(&f, "study-room", "Lamp", &glb("lamp b"), None);
    to_library(&f, "study-room", &lamp);
    let ask = |keep: &str, gone: &str, copy_tags: bool| {
        merge(
            f.root(),
            &f.layout,
            &MergeEntries {
                keep: keep.into(),
                merge: gone.into(),
                copy_tags,
            },
        )
    };
    assert!(matches!(
        ask("lamp", "lamp", true),
        Err(CoreError::Invalid(_))
    ));
    assert!(matches!(
        ask("lamp", "nope", true),
        Err(CoreError::NotFound(_))
    ));
    let many: Vec<String> = (0..MAX_TAGS).map(|n| format!("t{n}")).collect();
    tag(
        f.root(),
        &TagEntries {
            entries: strings(&["lamp"]),
            add: many,
            remove: Vec::new(),
        },
    )
    .expect("tags");
    assert!(matches!(
        ask("desk-lamp", "lamp", true),
        Err(CoreError::Invalid(_))
    ));
    assert_eq!(list(f.root()).expect("list").len(), 2, "nothing merged");
    let done = ask("desk-lamp", "lamp", false).expect("without tags");
    assert_eq!(done.entry.tags, ["a"]);
}
