//! Draft marks: what an agent's write changed, gathered over writes, pruned
//! of what is gone, kept beside the recipe and cleared when a person saves
//! or the recipe goes.

use std::collections::BTreeMap;

use super::super::tests_v2::{clip, project};
use super::super::*;
use super::{DraftKey, RecipeDrafts, diff};
use crate::recipes::drafts;

fn key(frame: u32, turn: f32) -> PoseKey {
    PoseKey {
        clip: 0,
        frame,
        ease: KeyEase::default(),
        pose: BTreeMap::from([("spine".to_string(), [turn, 0.0, 0.0])]),
    }
}

fn with_keys(keys: Vec<PoseKey>) -> RecipeFile {
    let mut r = RecipeFile::new("Swing", clip("Slash", 40));
    let mut layer = empty_pose_layer("keys", LayerMode::Add);
    layer.keys = keys;
    r.layers = vec![layer];
    r
}

#[test]
fn a_write_marks_only_the_keys_and_changes_it_touched() {
    let before = with_keys(vec![key(0, 5.0), key(20, 10.0)]);
    let after = with_keys(vec![key(0, 5.0), key(20, 30.0), key(30, 4.0)]);
    let d = diff(Some(&before), &after);
    assert_eq!(d.layers, vec!["keys".to_string()]);
    let frames: Vec<u32> = d.keys.iter().map(|k| k.frame).collect();
    assert_eq!(frames, vec![20, 30], "the key at 0 is as it was");
    assert!(!d.clips && !d.time && !d.blanks && !d.mirror && !d.made);
    assert!(diff(Some(&after), &after).is_empty(), "no change, no mark");
}

#[test]
fn a_recipe_an_agent_made_is_drafted_whole_but_its_clips() {
    let made = with_keys(vec![key(0, 5.0)]);
    let d = diff(None, &made);
    assert!(d.made);
    assert!(!d.clips);
    assert_eq!(d.keys.len(), 1);
}

#[test]
fn clips_time_and_mirror_each_mark() {
    let before = with_keys(Vec::new());
    let mut after = before.clone();
    after.joined = vec![clip("Thrust", 30)];
    after.mirror = true;
    let d = diff(Some(&before), &after);
    assert!(d.clips && d.mirror && !d.time);
}

#[test]
fn marks_gather_over_writes_and_drop_what_is_gone() {
    let one = RecipeDrafts {
        by: "Agent one".into(),
        keys: vec![DraftKey {
            layer: "keys".into(),
            clip: 0,
            frame: 20,
        }],
        ..RecipeDrafts::default()
    };
    let two = RecipeDrafts {
        by: "Agent two".into(),
        layers: vec!["keys".into()],
        ..RecipeDrafts::default()
    };
    let both = drafts::merged(Some(drafts::merged(None, one)), two);
    assert_eq!((both.by.as_str(), both.writes), ("Agent two", 2));
    assert_eq!((both.keys.len(), both.layers.len()), (1, 1));
    let gone = drafts::pruned(both.clone(), &with_keys(vec![key(0, 1.0)]));
    assert_eq!(gone.map(|d| d.keys.len()), Some(0), "the key at 20 went");
    assert!(drafts::pruned(both, &RecipeFile::new("Bare", clip("Slash", 40))).is_none());
}

#[test]
fn marks_live_beside_the_recipe_and_clear_on_a_save_or_a_remove() {
    let dir = tempfile::tempdir().expect("temp");
    let trash = tempfile::tempdir().expect("temp");
    let (id, made) = create(dir.path(), "Swing", clip("Slash", 40)).expect("create");
    let after = with_keys(vec![key(12, 20.0)]);
    let marked = drafts::mark(dir.path(), &id, Some(&made), &after, "Agent", "now")
        .expect("mark")
        .expect("marked");
    assert_eq!(marked.keys.len(), 1);
    let file = dir
        .path()
        .join(RECIPES_DIR)
        .join(format!("{id}.drafts.json"));
    assert!(file.is_file());
    assert_eq!(drafts::read(dir.path(), &id, &after), Some(marked));
    let raw = std::fs::read_to_string(path(dir.path(), &id).expect("path")).expect("read");
    assert!(!raw.contains("drafts"), "the recipe file never holds them");
    assert_eq!(names(dir.path()).len(), 1, "the marks file is no recipe");

    drafts::clear(dir.path(), &id).expect("clear");
    assert!(drafts::read(dir.path(), &id, &after).is_none());
    drafts::clear(dir.path(), &id).expect("clearing twice is fine");

    drafts::mark(dir.path(), &id, Some(&made), &after, "Agent", "now").expect("mark");
    remove(dir.path(), trash.path(), &id).expect("remove");
    assert!(!file.exists(), "a removed recipe takes its marks");
}

#[test]
fn a_persons_save_clears_the_marks_only_once_it_has_landed() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, made) = create(dir.path(), "Swing", clip("Slash", 40)).expect("create");
    let after = with_keys(vec![key(12, 20.0)]);
    drafts::mark(dir.path(), &id, Some(&made), &after, "Agent", "now").expect("mark");
    let named = |name: &str| RecipeChange {
        name: Some(name.into()),
        ..RecipeChange::default()
    };
    // An agent's update keeps them; a person's save clears them.
    update(dir.path(), &project(), &id, named("Kept")).expect("update");
    let marks = dir
        .path()
        .join(RECIPES_DIR)
        .join(format!("{id}.drafts.json"));
    assert!(marks.is_file());
    save(dir.path(), &project(), &id, named("Saved")).expect("save");
    assert!(!marks.exists());

    // A save that fails to write keeps the marks: they clear only once the
    // recipe has landed.
    drafts::mark(dir.path(), &id, Some(&made), &after, "Agent", "now").expect("mark");
    crate::recipes::change::FAIL_NEXT_WRITE.with(|f| f.set(true));
    assert!(save(dir.path(), &project(), &id, named("Lost")).is_err());
    assert!(marks.is_file(), "a failed save keeps the marks");
    let read = get(dir.path(), &project(), &id).expect("get");
    assert_eq!(read.recipe.name, "Saved");

    // Marks that cannot be cleared never fail a save that landed.
    std::fs::remove_file(&marks).expect("unmark");
    std::fs::create_dir_all(marks.join("stuck")).expect("stuck");
    save(dir.path(), &project(), &id, named("Landed")).expect("save");
    let read = get(dir.path(), &project(), &id).expect("get");
    assert_eq!(read.recipe.name, "Landed");
}
