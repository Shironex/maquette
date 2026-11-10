//! Pose key edits by bone and frame, and the size check that refuses an
//! edit list longer than a pose key change can hold.

use super::tests_pose_keys::plain_keys;
use super::*;

#[test]
fn key_edits_set_move_and_remove_by_bone_and_frame() {
    let layer = plain_keys().layers.remove(0);
    let set = |clip, frame, ease, pose: &[(&str, [f32; 3])]| KeySet {
        clip,
        frame,
        ease,
        pose: pose.iter().map(|(b, t)| ((*b).to_string(), *t)).collect(),
    };
    // A bone added to a key keeps its ease; a new key gets smooth.
    let edited = edit_keys(
        &layer,
        &KeyEdits {
            set: vec![
                set(0, 30, None, &[("head", [0.0, 10.0, 0.0])]),
                set(0, 5, None, &[("head", [0.0, 0.0, 0.0])]),
            ],
            ..KeyEdits::default()
        },
    )
    .expect("set");
    let at30 = edited.keys.iter().find(|k| k.frame == 30).expect("30");
    assert_eq!((at30.ease, at30.pose.len()), (KeyEase::Out, 2));
    assert_eq!(edited.keys[0].frame, 5, "sorted");
    assert_eq!(edited.keys[0].ease, KeyEase::Smooth);
    // Moving one bone off a key leaves the rest; moving the last bone takes
    // the key with it.
    let moved = edit_keys(
        &layer,
        &KeyEdits {
            moves: vec![
                KeyMove {
                    clip: 0,
                    frame: 10,
                    to_clip: 0,
                    to_frame: 12,
                    bones: Some(vec!["chest".into()]),
                },
                KeyMove {
                    clip: 0,
                    frame: 30,
                    to_clip: 0,
                    to_frame: 31,
                    bones: None,
                },
            ],
            ..KeyEdits::default()
        },
    )
    .expect("move");
    let frames: Vec<u32> = moved.keys.iter().map(|k| k.frame).collect();
    assert_eq!(frames, [10, 12, 31, 40, 45, 55]);
    assert_eq!(
        moved.keys[2].ease,
        KeyEase::Out,
        "a moved key keeps its ease"
    );
    let removed = edit_keys(
        &layer,
        &KeyEdits {
            remove: vec![KeyRemove {
                clip: 0,
                frame: 10,
                bones: Some(vec!["chest".into(), "rightUpperArm".into()]),
            }],
            ..KeyEdits::default()
        },
    )
    .expect("remove");
    assert_eq!(removed.keys.len(), 4, "an empty key goes");
    let refused = edit_keys(
        &layer,
        &KeyEdits {
            remove: vec![KeyRemove {
                clip: 0,
                frame: 11,
                bones: None,
            }],
            ..KeyEdits::default()
        },
    );
    assert!(refused.is_err());
    let not_pose = RecipeLayer::new("l-9", LayerKind::Nudge, "spine", LayerSettings::default());
    assert!(edit_keys(&not_pose, &KeyEdits::default()).is_err());
}

#[test]
fn key_edits_refuse_lists_longer_than_a_change_holds_before_scanning() {
    let layer = plain_keys().layers.remove(0);
    let remove = |bones: Option<Vec<String>>| KeyRemove {
        clip: 0,
        frame: 0,
        bones,
    };
    let refused = |edits: KeyEdits| {
        let err = edit_keys(&layer, &edits).expect_err("too long");
        assert!(err.to_string().contains("send at most"), "{err}");
    };
    refused(KeyEdits {
        remove: vec![remove(None); MAX_KEYS + 1],
        ..KeyEdits::default()
    });
    refused(KeyEdits {
        moves: vec![
            KeyMove {
                clip: 0,
                frame: 0,
                to_clip: 0,
                to_frame: 1,
                bones: None,
            };
            MAX_KEYS + 1
        ],
        ..KeyEdits::default()
    });
    refused(KeyEdits {
        set: vec![
            KeySet {
                clip: 0,
                frame: 0,
                ease: None,
                pose: BonePose::default(),
            };
            MAX_KEYS + 1
        ],
        ..KeyEdits::default()
    });
    let names = (0..=MAX_KEY_BONES).map(|i| format!("b{i}")).collect();
    refused(KeyEdits {
        remove: vec![remove(Some(names))],
        ..KeyEdits::default()
    });
}
