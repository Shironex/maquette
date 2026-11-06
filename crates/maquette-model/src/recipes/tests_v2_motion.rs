//! Version 2 of the recipe file: what the motion version covers, updates
//! that replace clips, and Mixamo marks across clips.

use super::tests_v2::*;
use super::*;

#[test]
fn the_motion_version_covers_what_moves_and_leaves_out_links() {
    let recipe = slash();
    let motion = motion_version(&recipe);
    let mut relinked = recipe.clone();
    relinked.layers[2].from = Some(GripFrom {
        entry: "longsword-two-hands".into(),
        revision: 3,
    });
    relinked.from = Some(RecipeFrom {
        entry: "slash-into-thrust".into(),
        uid: "0a1b2c3d4e5f6071".into(),
        revision: 4,
    });
    relinked.revision = 4;
    assert_eq!(
        motion_version(&relinked),
        motion,
        "a grip's link and revision stay out"
    );
    let moves: Vec<(&str, Edit)> = vec![
        ("trim", Box::new(|r| r.joined[0].trim = Some([4, 50]))),
        (
            "blend",
            Box::new(|r| {
                r.joined[0].blend = Some(BlendWindow {
                    frames: 12,
                    ..BlendWindow::default()
                });
            }),
        ),
        (
            "line up",
            Box::new(|r| {
                r.joined[1].blend = Some(BlendWindow {
                    frames: 6,
                    line_up: LineUp::Off,
                    ..BlendWindow::default()
                });
            }),
        ),
        ("clip hash", Box::new(|r| r.joined[1].hash = "new".into())),
        ("order", Box::new(|r| r.joined.swap(0, 1))),
        ("mirror", Box::new(|r| r.mirror = true)),
        ("retime", Box::new(|r| r.time[0].speed = Some(1.2))),
        ("freeze", Box::new(|r| r.time[1].frames = Some(12))),
        ("pin", Box::new(|r| r.layers[0].settings.end = 0.6)),
        ("key", Box::new(|r| r.layers[1].keys[0].frame = 31)),
        (
            "ease",
            Box::new(|r| r.layers[1].keys[0].ease = KeyEase::Step),
        ),
        (
            "mode",
            Box::new(|r| r.layers[1].mode = Some(LayerMode::Replace)),
        ),
        (
            "grip pose",
            Box::new(|r| {
                if let Some(pose) = &mut r.layers[2].pose {
                    pose.insert("rightThumbProximal".into(), [10.0, 0.0, 0.0]);
                }
            }),
        ),
    ];
    for (word, change) in moves {
        let mut moved = recipe.clone();
        change(&mut moved);
        assert_ne!(motion_version(&moved), motion, "{word} moves the model");
    }
}

#[test]
fn an_update_replaces_clips_and_refuses_keys_left_on_a_clip_taken_out() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, written, version) = create_whole(dir.path(), &slash(), None).expect("write");
    let two: Vec<RecipeBase> = written.clips().take(2).cloned().collect();
    let err = update(
        dir.path(),
        &project(),
        &id,
        RecipeChange {
            clips: Some(vec![two[0].clone()]),
            expected: Some(version.clone()),
            ..RecipeChange::default()
        },
    )
    .expect_err("a freeze sits on clip 2");
    assert!(err.to_string().contains("clip 2"), "{err}");
    let read = update(
        dir.path(),
        &project(),
        &id,
        RecipeChange {
            clips: Some(two),
            expected: Some(version.clone()),
            ..RecipeChange::default()
        },
    )
    .expect("two clips");
    assert_eq!(read.recipe.joined.len(), 1);
    assert!(matches!(
        update(
            dir.path(),
            &project(),
            &id,
            RecipeChange {
                mirror: Some(true),
                expected: Some(version),
                ..RecipeChange::default()
            },
        ),
        Err(CoreError::Moved(_))
    ));
    let joined = create_joined(
        dir.path(),
        "Joined",
        vec![clip("Great Sword Slash", 72), clip("Sword Idle", 90)],
        true,
    )
    .expect("joined")
    .1;
    assert_eq!(joined.version, RECIPE_VERSION);
    assert_eq!(joined.joined[0].blend, Some(BlendWindow::default()));
    assert!(create_joined(dir.path(), "None", Vec::new(), false).is_err());
}

#[test]
fn a_list_marks_mixamo_when_any_clip_is() {
    let dir = tempfile::tempdir().expect("temp");
    let mut first = clip("Own Slash", 72);
    first.mixamo = false;
    let (id, _) = create_joined(
        dir.path(),
        "Mixed",
        vec![first.clone(), clip("Sword Idle", 90)],
        false,
    )
    .expect("make");
    let listed = list(dir.path(), &project()).expect("list");
    assert!(listed.iter().find(|r| r.id == id).expect("listed").mixamo);
    let (own, _) = create(dir.path(), "Own", first).expect("own");
    let listed = list(dir.path(), &project()).expect("list");
    assert!(!listed.iter().find(|r| r.id == own).expect("listed").mixamo);
}
