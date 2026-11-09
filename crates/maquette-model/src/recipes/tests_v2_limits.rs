//! Version 2 of the recipe file: the limits and checks of joined clips,
//! trims, blends, time edits, pose keys, pins and grips.

use super::tests_v2::*;
use super::*;

/// Every VRM finger bone of both hands.
fn fingers() -> Vec<String> {
    let mut out = Vec::new();
    for side in ["left", "right"] {
        for (finger, segments) in [
            ("Thumb", ["Metacarpal", "Proximal", "Distal"]),
            ("Index", ["Proximal", "Intermediate", "Distal"]),
            ("Middle", ["Proximal", "Intermediate", "Distal"]),
            ("Ring", ["Proximal", "Intermediate", "Distal"]),
            ("Little", ["Proximal", "Intermediate", "Distal"]),
        ] {
            for segment in segments {
                out.push(format!("{side}{finger}{segment}"));
            }
        }
    }
    out
}

fn refused(recipe: &RecipeFile, says: &str) {
    match validate(&normalized(recipe)) {
        Err(CoreError::Invalid(message)) => {
            assert!(message.contains(says), "{message:?} does not say {says:?}");
        }
        other => panic!("expected a refusal saying {says:?}, got {other:?}"),
    }
}

#[test]
fn the_whole_timeline_keeps_to_its_length() {
    let long = |name: &str| RecipeBase {
        frames: MAX_FRAMES,
        length: 3_333.333,
        ..clip(name, 60)
    };
    // One clip as long as a recipe takes plays whole.
    let one = RecipeFile::new("Long", long("A"));
    assert!(validate(&normalized(&one)).is_ok());
    // Each clip within its limit, the whole far over.
    let mut many = one.clone();
    many.joined = (1..MAX_CLIPS)
        .map(|i| RecipeBase {
            blend: Some(BlendWindow::default()),
            ..long(&format!("Clip {i}"))
        })
        .collect();
    refused(&many, "runs over 100000 frames");
    // A slow retime stretches one clip past it too.
    let mut slow = RecipeFile::new("Slow", clip("A", 30_000));
    slow.time = vec![TimeEdit::retime(0, 0, 30_000, MIN_SPEED)];
    refused(&slow, "runs over");
    slow.time = vec![TimeEdit::retime(0, 0, 20_000, MIN_SPEED)];
    assert!(validate(&normalized(&slow)).is_ok(), "90000 frames");
}

#[test]
fn clips_trims_and_blends_keep_to_their_limits() {
    let good = slash();
    let mut r = good.clone();
    r.joined = (0..MAX_CLIPS)
        .map(|i| clip(&format!("Clip {i}"), 60))
        .collect();
    refused(&r, "at most 8 clips");
    r.joined.pop();
    validate(&normalized(&r)).expect("8 clips fit");

    let mut r = good.clone();
    r.base.trim = Some([40, 20]);
    refused(&r, "clip 1 keeps frames 40 to 20");
    r.base.trim = Some([0, 73]);
    refused(&r, "within its 72 frames");

    let mut r = good.clone();
    r.joined[0].blend = Some(BlendWindow {
        frames: 27,
        ..BlendWindow::default()
    });
    refused(&r, "at most 26");
    r.joined[0].blend = Some(BlendWindow {
        frames: 0,
        ..BlendWindow::default()
    });
    validate(&normalized(&r)).expect("0 cuts straight");

    let mut r = good.clone();
    r.joined[1] = RecipeBase::rest_pose(60);
    refused(&r, "Only the base can be the rest pose");

    let mut r = RecipeFile::new("Rest", RecipeBase::rest_pose(60));
    r.joined = vec![clip("Sword Idle", 90)];
    refused(&r, "no clips to join");
    r.joined.clear();
    r.base.trim = Some([0, 10]);
    refused(&r, "no clip to trim");
}

#[test]
fn time_edits_sit_on_a_clip_and_within_its_frames() {
    let good = slash();
    let with = |edit: TimeEdit| {
        let mut r = good.clone();
        r.time.push(edit);
        r
    };
    refused(
        &with(TimeEdit::retime(3, 0, 10, 1.0)),
        "clip 4, which the recipe does not have",
    );
    refused(
        &with(TimeEdit::retime(2, 10, 91, 1.0)),
        "within its 90 frames",
    );
    refused(&with(TimeEdit::retime(2, 10, 20, 5.0)), "0.25x to 4x");
    refused(
        &with(TimeEdit::retime(0, 30, 40, 1.0)),
        "Two retime ranges on clip 1 overlap",
    );
    validate(&normalized(&with(TimeEdit::hold(
        TimeKind::Freeze,
        0,
        25,
        6,
    ))))
    .expect("a freeze inside a retime keeps its frames");
    refused(
        &with(TimeEdit::hold(TimeKind::Freeze, 1, 56, 4)),
        "Two holds on clip 2 overlap",
    );
    refused(
        &with(TimeEdit::hold(TimeKind::Freeze, 2, 10, 0)),
        "1 to 600 frames",
    );
    refused(
        &with(TimeEdit::hold(TimeKind::HoldOver, 2, 80, 20)),
        "keeps the length",
    );
    refused(
        &with(TimeEdit::hold(TimeKind::Freeze, 2, 90, 4)),
        "past its 90 frames",
    );
    let mut breathe = TimeEdit::hold(TimeKind::HoldOver, 2, 10, 20);
    breathe.breathe = Some(1.5);
    refused(&with(breathe), "breathe has to be 0 to 1");
    let mut missing = TimeEdit::retime(2, 10, 20, 1.0);
    missing.speed = None;
    refused(&with(missing), "needs from, to and speed");
    let mut r = good.clone();
    r.time = (0..=u32::try_from(MAX_TIME_EDITS).expect("max"))
        .map(|i| TimeEdit::hold(TimeKind::Freeze, 2, i, 1))
        .collect();
    refused(&r, "at most 32 time edits");
}

#[test]
fn pose_keys_keep_to_their_limits() {
    let good = slash();
    let with = |layer: RecipeLayer| {
        let mut r = good.clone();
        r.layers = vec![layer];
        r
    };
    let many: Vec<PoseKey> = (0..=u32::try_from(MAX_KEYS).expect("max"))
        .map(|f| key(2, f, &[("chest", [0.0, 1.0, 0.0])]))
        .collect();
    refused(&with(pose_layer("l-2", many.clone())), "at most 64 keys");
    validate(&normalized(&with(pose_layer(
        "l-2",
        many[..MAX_KEYS].to_vec(),
    ))))
    .expect("64 keys fit");
    let bones: Vec<(String, [f32; 3])> = fingers()
        .into_iter()
        .chain(["hips", "spine", "chest", "head"].map(String::from))
        .take(MAX_KEY_BONES + 1)
        .map(|b| (b, [1.0, 0.0, 0.0]))
        .collect();
    assert_eq!(bones.len(), MAX_KEY_BONES + 1);
    let wide = PoseKey {
        pose: bones.into_iter().collect(),
        ..key(0, 1, &[])
    };
    refused(&with(pose_layer("l-2", vec![wide])), "at most 32 bones");
    refused(
        &with(pose_layer("l-2", vec![key(5, 1, &[("chest", [0.0; 3])])])),
        "clip 6, which the recipe does not have",
    );
    refused(
        &with(pose_layer("l-2", vec![key(0, 72, &[("chest", [0.0; 3])])])),
        "past the 72 frames of clip 1",
    );
    refused(
        &with(pose_layer(
            "l-2",
            vec![
                key(0, 7, &[("chest", [0.0; 3])]),
                key(0, 7, &[("head", [0.0; 3])]),
            ],
        )),
        "two keys on frame 7 of clip 1",
    );
    refused(
        &with(pose_layer("l-2", vec![key(0, 7, &[("tail", [0.0; 3])])])),
        "not a humanoid bone",
    );
    refused(
        &with(pose_layer(
            "l-2",
            vec![key(0, 7, &[("chest", [0.0, 200.0, 0.0])])],
        )),
        "more than 180 degrees",
    );
    refused(
        &with(pose_layer("l-2", vec![key(0, 7, &[])])),
        "turns no bone",
    );
}

#[test]
fn keys_on_the_rest_pose_use_its_own_frames() {
    let mut r = RecipeFile::new("Hold a pose", RecipeBase::rest_pose(60));
    r.layers = vec![pose_layer(
        "l-1",
        vec![key(0, 59, &[("chest", [0.0, 10.0, 0.0])])],
    )];
    validate(&normalized(&r)).expect("frame 59 of 60");
    r.layers = vec![pose_layer(
        "l-1",
        vec![key(1, 5, &[("chest", [0.0, 10.0, 0.0])])],
    )];
    refused(&r, "clip 2, which the recipe does not have");
}

#[test]
fn pins_and_grips_keep_to_their_bones() {
    let good = slash();
    let with = |layer: RecipeLayer| {
        let mut r = good.clone();
        r.layers = vec![layer];
        r
    };
    refused(
        &with(pin("l-1", "head")),
        "A pin moves leftHand, rightHand, leftFoot, rightFoot",
    );
    let mut flat_hand = pin("l-1", "leftHand");
    flat_hand.settings.flat = Some(PinFlat::Heel);
    refused(&with(flat_hand), "needs a foot");
    let mut aimed = pin("l-1", "leftHand");
    aimed.settings.target = Some("wall".into());
    refused(
        &with(aimed),
        "the blank wall, which the recipe does not have",
    );
    let mut flat_foot = pin("l-1", "leftFoot");
    flat_foot.settings.flat = Some(PinFlat::ToeAndHeel);
    validate(&normalized(&with(flat_foot))).expect("a foot stays flat");

    refused(
        &with(RecipeLayer {
            pose: None,
            ..grip("l-3")
        }),
        "needs its pose",
    );
    refused(
        &with(RecipeLayer {
            bone: "head".into(),
            ..grip("l-3")
        }),
        "A grip moves leftHand, rightHand",
    );
    refused(
        &with(RecipeLayer {
            from: Some(GripFrom {
                entry: "longsword".into(),
                revision: 0,
            }),
            ..grip("l-3")
        }),
        "cannot be one",
    );
}
