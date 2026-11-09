//! A change's start and end follow the motion they were placed on when the
//! clips are joined, taken out or moved, checked against
//! `fixtures/clip_moves.json`, which the web app's copy of the rules reads
//! too, and through `update` as agents send clips.

use serde::Deserialize;

use super::clip_moves::{Stranded, move_share};
use super::tests_v2::{clip, project};
use super::*;

const FIXTURE: &str = "src/recipes/fixtures/clip_moves.json";

#[derive(Deserialize)]
struct Case {
    name: String,
    from: Vec<u32>,
    to: Vec<u32>,
    places: Vec<Option<u32>>,
    change: [f32; 2],
    want: Option<[f32; 2]>,
    dropped: Option<usize>,
    split: Option<usize>,
}

#[derive(Deserialize)]
struct Cases {
    cases: Vec<Case>,
}

#[test]
fn shares_move_with_their_clips_as_the_fixture_says() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let cases: Cases =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json");
    assert!(cases.cases.len() >= 10);
    for c in cases.cases {
        let got = move_share(&c.from, &c.to, &c.places, c.change[0], c.change[1]);
        let want = match (c.want, c.dropped, c.split) {
            (Some([a, b]), None, None) => Ok((a, b)),
            (None, Some(i), None) => Err(Stranded::Dropped(i)),
            (None, None, Some(i)) => Err(Stranded::Split(i)),
            _ => panic!("{}: say want, dropped or split", c.name),
        };
        assert_eq!(got, want, "{}", c.name);
    }
}

fn nudge(id: &str, start: f32, end: f32) -> RecipeLayer {
    RecipeLayer::new(
        id,
        LayerKind::Nudge,
        "rightHand",
        LayerSettings {
            start,
            end,
            rotation: Some([0.0, 0.0, 10.0]),
            ..LayerSettings::default()
        },
    )
}

/// A recipe on one 60 frame clip with a change over 0.5 to 0.8: its id
/// and version.
fn one_clip(dir: &std::path::Path) -> (String, String) {
    let (id, _) = create_joined(dir, "Reach", vec![clip("Idle", 60)], false).expect("create");
    let read = update(
        dir,
        &project(),
        &id,
        RecipeChange {
            layers: Some(vec![nudge("l-1", 0.5, 0.8)]),
            expected: Some(version(dir, &id).expect("version")),
            ..RecipeChange::default()
        },
    )
    .expect("change");
    (id, read.version)
}

fn span(recipe: &RecipeFile) -> (f32, f32) {
    let s = &recipe.layers[0].settings;
    (s.start, s.end)
}

/// The frames of the clip a change covers: the clip and its first and last.
fn frames_on(recipe: &RecipeFile) -> (usize, f32, f32) {
    let frames: Vec<f32> = recipe
        .clips()
        .map(|c| f32::from(u16::try_from(c.frames).expect("frames")))
        .collect();
    let all: f32 = frames.iter().sum();
    let (start, end) = span(recipe);
    let (a, b) = (start * all, end * all);
    let mut before = 0.0;
    for (i, n) in frames.iter().enumerate() {
        if a < before + n {
            return (i, a - before, b - before);
        }
        before += n;
    }
    panic!("no clip");
}

#[test]
fn joining_a_clip_keeps_a_change_on_the_frames_it_covered() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, at) = one_clip(dir.path());
    let before = get(dir.path(), &project(), &id).expect("read").recipe;
    assert_eq!(frames_on(&before), (0, 30.0, 48.0));
    let read = update(
        dir.path(),
        &project(),
        &id,
        RecipeChange {
            clips: Some(vec![clip("Idle", 60), clip("Wave", 60)]),
            kept: Some(vec![Some(0), None]),
            expected: Some(at),
            ..RecipeChange::default()
        },
    )
    .expect("join");
    assert_eq!(span(&read.recipe), (0.25, 0.4));
    assert_eq!(
        frames_on(&read.recipe),
        (0, 30.0, 48.0),
        "still clip 1 frames 30 to 48, not clip 2 frames 4 to 36"
    );
}

#[test]
fn moving_and_taking_out_clips_carry_the_change_and_back() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, at) = one_clip(dir.path());
    let send = |clips: Vec<RecipeBase>, kept: Vec<Option<u32>>, at: String| {
        update(
            dir.path(),
            &project(),
            &id,
            RecipeChange {
                clips: Some(clips),
                kept: Some(kept),
                expected: Some(at),
                ..RecipeChange::default()
            },
        )
    };
    let joined = send(
        vec![clip("Idle", 60), clip("Wave", 40)],
        vec![Some(0), None],
        at,
    )
    .expect("join");
    let front = send(
        vec![clip("Wave", 40), clip("Idle", 60)],
        vec![Some(1), Some(0)],
        joined.version,
    )
    .expect("move");
    assert_eq!(span(&front.recipe), (0.7, 0.88), "frames 30 to 48 after 40");
    assert_eq!(frames_on(&front.recipe).0, 1);
    let back = send(vec![clip("Idle", 60)], vec![Some(1)], front.version).expect("take out");
    assert_eq!(span(&back.recipe), (0.5, 0.8), "as it was placed");
}

#[test]
fn taking_out_the_clip_a_change_sits_on_is_refused() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, at) = one_clip(dir.path());
    let joined = update(
        dir.path(),
        &project(),
        &id,
        RecipeChange {
            clips: Some(vec![clip("Idle", 60), clip("Wave", 60)]),
            kept: Some(vec![Some(0), None]),
            expected: Some(at),
            ..RecipeChange::default()
        },
    )
    .expect("join");
    let err = update(
        dir.path(),
        &project(),
        &id,
        RecipeChange {
            clips: Some(vec![clip("Wave", 60)]),
            kept: Some(vec![Some(1)]),
            expected: Some(joined.version.clone()),
            ..RecipeChange::default()
        },
    )
    .expect_err("the change sits on Idle")
    .to_string();
    assert!(err.starts_with(CLIP_IN_USE), "{err}");
    assert!(
        err.contains("clip 1 (Idle)") && err.contains("the change l-1"),
        "{err}"
    );
    assert_eq!(
        version(dir.path(), &id).expect("version"),
        joined.version,
        "nothing written"
    );
}

#[test]
fn a_change_across_a_seam_keeps_its_clips_together() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, _) = create_joined(
        dir.path(),
        "Seam",
        vec![clip("A", 60), clip("B", 60)],
        false,
    )
    .expect("create");
    let read = update(
        dir.path(),
        &project(),
        &id,
        RecipeChange {
            layers: Some(vec![nudge("l-1", 0.4, 0.6)]),
            expected: Some(version(dir.path(), &id).expect("version")),
            ..RecipeChange::default()
        },
    )
    .expect("change");
    let err = update(
        dir.path(),
        &project(),
        &id,
        RecipeChange {
            clips: Some(vec![clip("B", 60), clip("A", 60)]),
            kept: Some(vec![Some(1), Some(0)]),
            expected: Some(read.version.clone()),
            ..RecipeChange::default()
        },
    )
    .expect_err("A and B swapped split the change")
    .to_string();
    assert!(err.starts_with(CLIP_IN_USE), "{err}");
    assert!(err.contains("the change l-1"), "{err}");
    // Changes sent with the clips already read on the new order.
    let moved = update(
        dir.path(),
        &project(),
        &id,
        RecipeChange {
            clips: Some(vec![clip("B", 60), clip("A", 60)]),
            kept: Some(vec![Some(1), Some(0)]),
            layers: Some(vec![nudge("l-1", 0.1, 0.2)]),
            expected: Some(read.version),
            ..RecipeChange::default()
        },
    )
    .expect("sent with the clips");
    assert_eq!(span(&moved.recipe), (0.1, 0.2));
}
