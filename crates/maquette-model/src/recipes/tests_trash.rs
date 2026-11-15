//! A recipe removed to the workspace trash with its draft marks, and put
//! back.

use super::id_of;
use crate::recipes::drafts::{self, DRAFTS_SUFFIX};
use crate::recipes::tests_v2::clip;
use crate::recipes::*;

fn marked(dir: &std::path::Path) -> String {
    let (id, made) = create(dir, "Swing", clip("Slash", 40)).expect("create");
    let mut after = made.clone();
    let mut layer = empty_pose_layer("keys", LayerMode::Add);
    layer.keys = vec![PoseKey {
        clip: 0,
        frame: 12,
        ease: KeyEase::default(),
        pose: std::collections::BTreeMap::from([("spine".to_string(), [20.0, 0.0, 0.0])]),
    }];
    after.layers = vec![layer];
    drafts::mark(dir, &id, Some(&made), &after, "Agent", "now")
        .expect("mark")
        .expect("marked");
    id
}

#[test]
fn a_removed_recipe_takes_its_marks_to_the_trash_and_comes_back_with_them() {
    let dir = tempfile::tempdir().expect("temp");
    let trash = tempfile::tempdir().expect("temp");
    let id = marked(dir.path());
    let marks = dir
        .path()
        .join(RECIPES_DIR)
        .join(format!("{id}{DRAFTS_SUFFIX}"));
    assert!(marks.is_file());

    let removed = remove(dir.path(), trash.path(), &id).expect("remove");
    assert_eq!(removed.id, id);
    assert_eq!(removed.name, "Swing");
    assert!(!path(dir.path(), &id).expect("path").exists());
    assert!(!marks.exists());
    assert!(trash.path().join(&removed.path).is_file());
    let parked = trash
        .path()
        .join(RECIPES_DIR)
        .join(format!("{}{DRAFTS_SUFFIX}", removed.trash));
    assert!(parked.is_file(), "the marks wait beside it");
    assert!(names(dir.path()).is_empty());

    let back = restore(dir.path(), trash.path(), &removed.trash).expect("restore");
    assert_eq!(back, id);
    assert!(path(dir.path(), &id).expect("path").is_file());
    assert!(marks.is_file(), "the marks came back");
    assert!(!trash.path().join(&removed.path).exists());
    assert!(matches!(
        restore(dir.path(), trash.path(), &removed.trash),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn a_restore_never_writes_over_a_recipe_with_the_same_id() {
    let dir = tempfile::tempdir().expect("temp");
    let trash = tempfile::tempdir().expect("temp");
    let id = marked(dir.path());
    let first = remove(dir.path(), trash.path(), &id).expect("remove");
    let again = marked(dir.path());
    assert_eq!(again, id, "the id is free again, so a new recipe takes it");
    assert!(matches!(
        restore(dir.path(), trash.path(), &first.trash),
        Err(CoreError::Invalid(_))
    ));
    assert!(
        trash.path().join(&first.path).is_file(),
        "it stays in the trash"
    );

    // Removed twice in a row, each keeps its own trash name.
    let second = remove(dir.path(), trash.path(), &id).expect("remove");
    assert_ne!(second.trash, first.trash);
    restore(dir.path(), trash.path(), &first.trash).expect("restore the first");
}

#[test]
fn only_a_name_remove_makes_is_a_trash_name() {
    assert_eq!(id_of("swing-20261115101530123"), Some("swing"));
    assert_eq!(id_of("reach-up-2026111510153012"), Some("reach-up"));
    for bad in ["swing", "swing-", "-2026", "swing-12a", "../x-1", "Swing-1"] {
        assert_eq!(id_of(bad), None, "{bad}");
    }
    let dir = tempfile::tempdir().expect("temp");
    assert!(matches!(
        restore(dir.path(), dir.path(), "../evil-1"),
        Err(CoreError::NotFound(_))
    ));
}
