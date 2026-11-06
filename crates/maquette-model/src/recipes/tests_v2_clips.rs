//! Version 2 of the recipe file: keys and time edits follow their clip when
//! the clips are replaced, and taking away a clip one sits on is refused.

use super::tests_v2::{clip, key, pose_layer, project};
use super::*;

/// A recipe on clips A, B and C with a key and a freeze on B and a key on C;
/// its id, its clips and its version.
fn on_three(dir: &std::path::Path) -> (String, Vec<RecipeBase>, String) {
    let (id, _) = create_joined(
        dir,
        "Three",
        vec![clip("A", 60), clip("B", 60), clip("C", 60)],
        false,
    )
    .expect("joined");
    let read = update(
        dir,
        &project(),
        &id,
        RecipeChange {
            layers: Some(vec![pose_layer(
                "k",
                vec![
                    key(1, 10, &[("chest", [0.0, 20.0, 0.0])]),
                    key(2, 30, &[("head", [5.0, 0.0, 0.0])]),
                ],
            )]),
            time: Some(vec![TimeEdit::hold(TimeKind::Freeze, 1, 5, 4)]),
            expected: Some(version(dir, &id).expect("version")),
            ..RecipeChange::default()
        },
    )
    .expect("keys");
    let clips = read.recipe.clips().cloned().collect();
    (id, clips, read.version)
}

fn clips_change(clips: Vec<RecipeBase>, kept: Option<Vec<Option<u32>>>, at: &str) -> RecipeChange {
    RecipeChange {
        clips: Some(clips),
        kept,
        expected: Some(at.to_string()),
        ..RecipeChange::default()
    }
}

fn key_clips(recipe: &RecipeFile) -> Vec<u32> {
    recipe.layers[0].keys.iter().map(|k| k.clip).collect()
}

#[test]
fn taking_out_a_middle_clip_something_sits_on_is_refused() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, c, at) = on_three(dir.path());
    for kept in [None, Some(vec![Some(0), Some(2)])] {
        let err = update(
            dir.path(),
            &project(),
            &id,
            clips_change(vec![c[0].clone(), c[2].clone()], kept, &at),
        )
        .expect_err("a key and a freeze sit on B")
        .to_string();
        assert!(err.starts_with(CLIP_IN_USE), "{err}");
        assert!(
            err.contains("clip 2 (B)") && err.contains("the pose keys k"),
            "{err}"
        );
    }
    assert_eq!(
        version(dir.path(), &id).expect("version"),
        at,
        "nothing written"
    );
}

#[test]
fn taking_out_a_middle_clip_moves_what_sits_on_the_clips_after_it() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, c, at) = on_three(dir.path());
    // Send the keys and the freeze on B away first, in the order as it is.
    let read = update(
        dir.path(),
        &project(),
        &id,
        RecipeChange {
            layers: Some(vec![pose_layer(
                "k",
                vec![key(2, 30, &[("head", [5.0, 0.0, 0.0])])],
            )]),
            time: Some(Vec::new()),
            expected: Some(at),
            ..RecipeChange::default()
        },
    )
    .expect("off B");
    let read = update(
        dir.path(),
        &project(),
        &id,
        clips_change(vec![c[0].clone(), c[2].clone()], None, &read.version),
    )
    .expect("B taken out");
    let names: Vec<&str> = read.recipe.clips().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["A", "C"]);
    assert_eq!(
        key_clips(&read.recipe),
        [1],
        "the key on C follows C to clip 2"
    );
}

#[test]
fn reordering_with_kept_moves_keys_and_time_edits_with_their_clips() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, c, at) = on_three(dir.path());
    let read = update(
        dir.path(),
        &project(),
        &id,
        clips_change(
            vec![c[2].clone(), c[0].clone(), c[1].clone()],
            Some(vec![Some(2), Some(0), Some(1)]),
            &at,
        ),
    )
    .expect("reordered");
    let names: Vec<&str> = read.recipe.clips().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["C", "A", "B"]);
    assert_eq!(key_clips(&read.recipe), [0, 2], "C's key at 0, B's at 2");
    assert_eq!(read.recipe.time[0].clip, 2, "the freeze stays on B");
    assert_eq!(read.recipe.layers[0].keys[1].frame, 10, "on B's own frame");
}

#[test]
fn replacing_a_clip_something_sits_on_is_refused() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, c, at) = on_three(dir.path());
    let swapped = vec![c[0].clone(), clip("D", 60), c[2].clone()];
    for kept in [None, Some(vec![Some(0), None, Some(2)])] {
        let err = update(
            dir.path(),
            &project(),
            &id,
            clips_change(swapped.clone(), kept, &at),
        )
        .expect_err("D takes B's place")
        .to_string();
        assert!(err.starts_with(CLIP_IN_USE), "{err}");
    }
    // Lists sent with the clips count clips in the new order and move nowhere.
    let read = update(
        dir.path(),
        &project(),
        &id,
        RecipeChange {
            layers: Some(vec![pose_layer(
                "k",
                vec![key(1, 3, &[("chest", [1.0, 0.0, 0.0])])],
            )]),
            time: Some(Vec::new()),
            ..clips_change(swapped, Some(vec![Some(0), None, Some(2)]), &at)
        },
    )
    .expect("D with its own key");
    assert_eq!(read.recipe.joined[0].name, "D");
    assert_eq!(key_clips(&read.recipe), [1]);
}

#[test]
fn kept_has_to_fit_the_clips_sent_and_the_recipe() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, c, at) = on_three(dir.path());
    for kept in [vec![Some(0)], vec![Some(0), Some(1), Some(9)]] {
        let err = update(
            dir.path(),
            &project(),
            &id,
            clips_change(c.clone(), Some(kept), &at),
        )
        .expect_err("kept does not fit");
        assert!(matches!(err, CoreError::Invalid(_)), "{err}");
    }
}

#[test]
fn kept_has_to_name_the_clip_sent_in_its_place() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, c, at) = on_three(dir.path());
    // Clip 2 sent is C, but kept says it keeps B.
    let err = update(
        dir.path(),
        &project(),
        &id,
        clips_change(
            vec![c[0].clone(), c[2].clone(), c[1].clone()],
            Some(vec![Some(0), Some(1), Some(2)]),
            &at,
        ),
    )
    .expect_err("kept names another clip");
    assert!(
        matches!(&err, CoreError::Invalid(m) if m.contains("keeps clip 2")),
        "{err}"
    );
    assert_eq!(
        version(dir.path(), &id).expect("version"),
        at,
        "not written"
    );
}
