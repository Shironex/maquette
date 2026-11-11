//! The combat starters: joined clips found by name, pose keys and holds laid
//! as shares of whatever clips they find, a grip on the weapon hand (or both)
//! and left handed copies that only mirror.

use super::*;
use crate::animations::recipes::BaseStatus;
use crate::animations::store;
use crate::animations::tests::library_clip;
use crate::recipes::grips::{GripHands, hands_of_pose};
use crate::recipes::{self, LayerKind, TimeKind};

fn checked(recipe: &RecipeFile) -> RecipeFile {
    let written = recipes::normalized(recipe);
    recipes::validate(&written).unwrap_or_else(|e| panic!("{}: {e}", recipe.name));
    written
}

#[test]
fn every_combo_is_a_valid_v2_recipe_on_clip_names_alone() {
    for combo in &combat::COMBOS {
        let r = checked(&combat::recipe_of(combo, &[]));
        assert_eq!(r.version, recipes::RECIPE_VERSION, "{}", combo.name);
        assert!(
            r.clips()
                .all(|c| c.hash.is_empty() && c.by_name && !c.mixamo)
        );
        assert_eq!(r.joined.len() + 1, combo.clips.len());
        assert_eq!(r.mirror, combo.mirror);
        let kinds: Vec<LayerKind> = r.layers.iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            vec![LayerKind::Pose, LayerKind::Grip],
            "{}",
            combo.name
        );
        assert!(r.from.is_none());
    }
}

#[test]
fn the_slash_combo_joins_its_cuts_and_holds_the_sword_in_one_hand() {
    let r = combat::recipe_of(&combat::COMBOS[0], &[]);
    let names: Vec<&str> = r.clips().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Sword And Shield Slash",
            "Sword And Shield Attack",
            "Sword And Shield Idle"
        ]
    );
    let grip = r.layers[1].pose.as_ref().expect("grip pose");
    assert_eq!(hands_of_pose(grip), Some(GripHands::Right));
    let keys = &r.layers[0].keys;
    assert_eq!(
        keys.iter().filter(|k| k.clip == 1).count(),
        2,
        "the second cut is keyed"
    );
}

#[test]
fn the_scythe_charge_holds_its_wind_up_with_both_hands_on_the_scythe() {
    let r = combat::recipe_of(&combat::COMBOS[2], &[]);
    let hold = r.time.first().expect("a hold");
    assert_eq!(hold.kind, TimeKind::HoldOver);
    let (at, frames) = (hold.at.expect("at"), hold.frames.expect("frames"));
    assert!(
        at + frames <= r.base.frames,
        "the hold ends within the swing"
    );
    let grip = r.layers[1].pose.as_ref().expect("grip pose");
    assert_eq!(hands_of_pose(grip), Some(GripHands::Both));
}

#[test]
fn a_combo_fits_the_clips_it_finds_and_a_left_copy_only_mirrors() {
    let tmp = tempfile::tempdir().expect("temp");
    let root = tmp.path();
    let mut swing = library_clip("swing", "two handed slash", "Singles", "h2");
    swing.frames = 18;
    store(root, &swing).expect("clip");
    let starter = get(root, "starter:scythe-charge").expect("starter");
    assert_eq!(starter.base.status, BaseStatus::Found);
    assert_eq!(starter.joined, vec!["Great Sword Idle".to_string()]);
    assert_eq!(starter.missing, starter.joined, "no idle of its names yet");
    assert_eq!(starter.time.len(), 1);
    let found = checked(&recipe(root, "starter:scythe-charge").expect("recipe"));
    assert_eq!(found.base.frames, 18);
    assert!(
        found.layers[0]
            .keys
            .iter()
            .all(|k| k.clip != 0 || k.frame < 18)
    );
    assert!(
        found.joined[0]
            .blend
            .as_ref()
            .is_some_and(|b| b.frames <= 9)
    );

    let left = recipe(root, "starter:scythe-charge-left").expect("left");
    assert!(left.mirror);
    assert_eq!(
        (left.layers.clone(), left.time.clone(), left.joined.clone()),
        (found.layers, found.time, found.joined),
        "the left copy is the same recipe, mirrored"
    );
}
