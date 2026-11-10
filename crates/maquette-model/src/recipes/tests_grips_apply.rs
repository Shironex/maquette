//! Grips on a recipe: which hands a recipe takes, the change that holds the
//! copy and the link, a newer revision taken in, what counts toward the
//! motion version, a grip made from pose keys, a grip change on the
//! timeline, and the recipe's caps.

use super::grips::{
    GRIP_EASE_FRAMES, GripHands, GripSide, amount, default_hands, grip_bones, grip_of,
    hands_of_pose, keyed_pose, layer_of, mirror_pose, pose_for, same_pose, spans, updated,
};
use super::tests_grips::{dagger, keyed_recipe, link, longsword, on, pose, span_cases};
use super::tests_v2::clip;
use super::timeline::timeline;
use super::*;

#[test]
fn a_recipe_takes_both_hands_one_or_the_mirror_of_one() {
    let sword = longsword();
    assert_eq!(default_hands(&sword), GripHands::Both);
    let (holds, both) = pose_for(&sword, None);
    assert_eq!((holds, &both), (GripSide::Right, &sword.pose));
    let (holds, right) = pose_for(&sword, Some(GripHands::Right));
    assert_eq!(holds, GripSide::Right);
    assert_eq!(right.len(), 4);
    assert!(right.keys().all(|b| b.starts_with("right")));
    let (holds, left) = pose_for(&sword, Some(GripHands::Left));
    assert_eq!((holds, left.len()), (GripSide::Left, 2));
    assert_eq!(
        left["leftHand"],
        [-2.0, 6.0, 9.0],
        "its own left, not a mirror"
    );

    let knife = dagger();
    assert_eq!(default_hands(&knife), GripHands::Right);
    let (holds, own) = pose_for(&knife, None);
    assert_eq!((holds, &own), (GripSide::Right, &knife.pose));
    let (holds, other) = pose_for(&knife, Some(GripHands::Left));
    assert_eq!(holds, GripSide::Left);
    assert_eq!(other, mirror_pose(&knife.pose));
    let (holds, two) = pose_for(&knife, Some(GripHands::Both));
    assert_eq!((holds, two.len()), (GripSide::Right, 4));
    assert_eq!(two["leftHand"], [0.0, -30.0, 0.0]);
    assert_eq!(hands_of_pose(&two), Some(GripHands::Both));
    assert_eq!(hands_of_pose(&pose(&[("chest", [0.0; 3])])), None);
}

#[test]
fn a_grip_change_keeps_a_copy_and_a_link_and_replaces_on_the_holding_hand() {
    let layer = layer_of("l-3", &longsword(), link(2), None);
    assert_eq!(
        (layer.kind, layer.bone.as_str()),
        (LayerKind::Grip, "rightHand")
    );
    assert_eq!(
        layer.mode,
        Some(LayerMode::Replace),
        "grips replace on the fingers"
    );
    assert_eq!(layer.pose.as_ref(), Some(&longsword().pose));
    assert_eq!(layer.from, Some(link(2)));
    assert_eq!(
        (
            layer.settings.start,
            layer.settings.end,
            layer.settings.weight
        ),
        (0.0, 1.0, 1.0)
    );
    assert_eq!(layer.settings.ease_in, Some(GRIP_EASE_FRAMES));
    let mut recipe = on("Held", clip("Sword Idle", 90));
    recipe.layers = vec![
        layer,
        layer_of("l-4", &dagger(), link(1), Some(GripHands::Left)),
    ];
    let recipe = normalized(&recipe);
    validate(&recipe).expect("a recipe takes it");
    assert_eq!(recipe.version, RECIPE_VERSION);
    assert_eq!(recipe.layers[1].bone, "leftHand");
}

#[test]
fn a_newer_revision_goes_into_the_change_and_only_the_pose_moves_the_motion() {
    let mut recipe = on("Held", clip("Sword Idle", 90));
    let mut layer = layer_of("l-3", &longsword(), link(2), Some(GripHands::Right));
    layer.settings.start = 0.2;
    layer.settings.end = 0.7;
    layer.settings.weight = 0.8;
    layer.settings.target = Some("hilt".into());
    recipe.layers = vec![layer.clone()];
    let motion = motion_version(&recipe);

    // The same pose under a newer link: nothing a bake reads moved.
    let relinked = updated(&layer, &longsword(), link(3), None);
    assert_eq!(relinked.from, Some(link(3)));
    assert_eq!(relinked.pose, layer.pose, "the hands it posed before");
    assert_eq!(relinked.settings, layer.settings);
    assert_eq!(
        (relinked.id.as_str(), relinked.bone.as_str()),
        ("l-3", "rightHand")
    );
    recipe.layers = vec![relinked];
    assert_eq!(
        motion_version(&recipe),
        motion,
        "a link and its revision stay out"
    );

    // A revision with another pose does.
    let mut newer = longsword();
    newer.revision = 3;
    newer
        .pose
        .insert("rightIndexProximal".into(), [0.0, 0.0, 75.0]);
    let taken = updated(&layer, &newer, link(3), None);
    assert_eq!(
        taken.pose.as_ref().map(BonePose::len),
        Some(4),
        "still the right hand"
    );
    recipe.layers = vec![taken.clone()];
    assert_ne!(motion_version(&recipe), motion, "the grip pose counts");

    // Told other hands, the change moves to the hand that holds them.
    let both = updated(&taken, &newer, link(3), Some(GripHands::Left));
    assert_eq!(both.bone, "leftHand");
    assert!(
        both.pose
            .expect("pose")
            .keys()
            .all(|b| b.starts_with("left"))
    );
}

#[test]
fn a_grip_is_made_from_the_hands_the_pose_keys_turn() {
    let recipe = keyed_recipe();
    let layer = &recipe.layers[0];
    let pose = keyed_pose(&recipe, layer, 40, None).expect("keyed");
    assert_eq!(pose.len(), 3, "the chest stays the recipe's: {pose:?}");
    assert_eq!(pose["rightIndexProximal"], [0.0, 0.0, 70.0]);
    // Halfway between the keys a smooth ease is halfway there.
    let mid = keyed_pose(&recipe, layer, 25, None).expect("keyed");
    assert_eq!(mid["rightIndexProximal"], [0.0, 0.0, 40.0]);
    assert_eq!(mid["leftHand"], [5.0, 10.0, 0.0]);

    let made = grip_of("  Hilt  ", "Longsword", None, None, &pose).expect("made");
    assert_eq!(
        (made.name.as_str(), made.weapon.as_str()),
        ("Hilt", "longsword")
    );
    assert_eq!(made.hands.holds, GripSide::Right);
    assert_eq!(made.hands.supports, Some(GripSide::Left));
    grips::validate(&made).expect("valid");
    let one = grip_of("Hilt", "longsword", Some(GripHands::Left), None, &pose).expect("made");
    assert_eq!(one.pose.len(), 1);
    assert_eq!(one.hands.supports, None);
    assert!(same_pose(&made, &made.clone()));
    assert!(!same_pose(&made, &one));

    let only_chest = vec!["chest".to_string()];
    for refused in [
        keyed_pose(&recipe, layer, 40, Some(&only_chest)),
        keyed_pose(
            &recipe,
            &layer_of("l-9", &dagger(), link(1), None),
            40,
            None,
        ),
    ] {
        assert!(matches!(refused, Err(CoreError::Invalid(_))), "{refused:?}");
    }
    let right_only = pose_for(&dagger(), None).1;
    let no_left = grip_of("Hilt", "dagger", Some(GripHands::Both), None, &right_only);
    assert!(
        no_left
            .expect_err("refused")
            .to_string()
            .contains("left hand"),
        "both hands need both sides"
    );
}

#[test]
fn a_grip_holds_on_the_frames_its_shares_fall_on_and_eases_over_its_own() {
    let cases = span_cases();
    let t = timeline(&cases[0]);
    let found = spans(&cases[0], &t);
    let g = &found[0];
    assert_eq!((g.first, g.last), (0, 60), "the whole clip");
    assert_eq!(
        (g.ease_in, g.ease_out),
        (GRIP_EASE_FRAMES, GRIP_EASE_FRAMES)
    );
    assert_eq!(g.mode, LayerMode::Replace);
    assert!(amount(g, 0.0) == 0.0 && amount(g, 60.0) == 0.0);
    assert!(
        (amount(g, 2.0) - 0.5).abs() < 1e-9,
        "halfway in at half its ease"
    );
    assert!((amount(g, 4.0) - 1.0).abs() < 1e-9 && (amount(g, 56.0) - 1.0).abs() < 1e-9);
    assert!(amount(g, 61.0) == 0.0);

    // Half weight, added, no ease in: it holds at once at half.
    let t = timeline(&cases[1]);
    let found = spans(&cases[1], &t);
    assert_eq!(found.len(), 2);
    let half = &found[0];
    assert_eq!(half.mode, LayerMode::Add);
    assert!((amount(half, f64::from(half.first)) - 0.5).abs() < 1e-9);
    assert_eq!(
        found[1].last, t.frames,
        "a grip to the end holds to the last frame"
    );

    // A freeze inside a grip stays gripped: the frames are the timeline's.
    let t = timeline(&cases[2]);
    let found = spans(&cases[2], &t);
    assert_eq!(t.frames, 50, "40 frames and a freeze of 10");
    assert_eq!((found[0].first, found[0].last), (16, 34));
    assert!(
        amount(&found[0], 25.0) < 1.0,
        "eases that overlap never hold whole"
    );
    let mut pinned = cases[2].clone();
    pinned.layers[0].kind = LayerKind::Pin;
    assert!(
        spans(&pinned, &t).iter().all(|g| g.layer != "l-1"),
        "only grips"
    );
}

#[test]
fn a_recipe_lists_its_grip_links_and_keeps_the_caps() {
    let dir = tempfile::tempdir().expect("temp");
    let manifest = super::tests_v2::project();
    let mut recipe = on("Held", clip("Sword Idle", 90));
    let mut loose = layer_of("l-2", &dagger(), link(1), None);
    loose.from = None;
    recipe.layers = vec![layer_of("l-1", &longsword(), link(2), None), loose];
    let (id, _, _) = create_whole(dir.path(), &normalized(&recipe), None).expect("write");
    let listed = list(dir.path(), &manifest).expect("list");
    assert_eq!(listed[0].id, id);
    assert_eq!(
        listed[0].grips.len(),
        1,
        "only the one that came from the library"
    );
    assert_eq!(listed[0].grips[0].layer, "l-1");
    assert_eq!(listed[0].grips[0].from, link(2));

    // 29 grips of 36 bones each turn 1044 bones, past the recipe's 1024.
    let mut wide = longsword();
    for bone in grip_bones(GripSide::Left)
        .into_iter()
        .chain(grip_bones(GripSide::Right))
    {
        wide.pose.insert(bone, [1.0, 2.0, 3.0]);
    }
    grips::validate(&wide).expect("36 bones is a grip");
    let mut many = on("Too many turns", clip("Sword Idle", 90));
    many.layers = (0..29)
        .map(|n| layer_of(&format!("l-{n}"), &wide, link(1), None))
        .collect();
    let err = validate(&normalized(&many)).expect_err("too many turns");
    assert!(err.to_string().contains("at most 1024"), "{err}");
    many.layers.truncate(28);
    validate(&normalized(&many)).expect("28 fit");
}
