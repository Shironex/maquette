//! Pose keys as they play, the joint limits and edits by bone and frame,
//! checked against `fixtures/pose_keys.json`, which the web app's copy of
//! the rules reads too. Run with `MAQUETTE_WRITE_POSE_KEYS=1` to write the
//! fixture again after a deliberate change to the rules.

use serde_json::{Value, json};

use super::joint_limits::{joint_limits, keyed_limits, over_limits};
use super::pose_keys::{clip_frame_at, ease, key_frame, key_share, pose_at};
use super::tests_v2::clip;
use super::timeline::timeline;
use super::*;

const FIXTURE: &str = "src/recipes/fixtures/pose_keys.json";

fn key(clip: u32, frame: u32, ease: KeyEase, pose: &[(&str, [f32; 3])]) -> PoseKey {
    PoseKey {
        clip,
        frame,
        ease,
        pose: pose.iter().map(|(b, t)| ((*b).to_string(), *t)).collect(),
    }
}

fn keys_layer(id: &str, mode: LayerMode, weight: f32, keys: Vec<PoseKey>) -> RecipeLayer {
    let mut layer = empty_pose_layer(id, mode);
    layer.settings.weight = weight;
    layer.keys = keys;
    layer
}

fn on(name: &str, base: RecipeBase, joined: Vec<RecipeBase>) -> RecipeFile {
    let mut r = RecipeFile::new(name, base);
    r.created = "2026-11-09T00:00:00.000Z".into();
    r.edited = r.created.clone();
    r.joined = joined;
    r
}

/// One clip played whole, a key of each ease.
fn plain_keys() -> RecipeFile {
    let mut r = on("Twist on idle", clip("Standing Idle", 60), Vec::new());
    r.layers = vec![keys_layer(
        "l-1",
        LayerMode::Add,
        1.0,
        vec![
            key(
                0,
                10,
                KeyEase::Smooth,
                &[
                    ("chest", [0.0, -18.0, 0.0]),
                    ("rightUpperArm", [-12.0, 0.0, 24.0]),
                ],
            ),
            key(0, 30, KeyEase::Out, &[("chest", [0.0, 6.0, 0.0])]),
            key(0, 40, KeyEase::In, &[("rightUpperArm", [0.0, 30.0, 0.0])]),
            key(0, 45, KeyEase::Step, &[("chest", [10.0, 0.0, 0.0])]),
            key(0, 55, KeyEase::Linear, &[("chest", [50.0, 0.0, 0.0])]),
        ],
    )];
    r
}

/// Two clips joined across a blend, trimmed, retimed and frozen, a key on
/// each and one in the freeze, laid on as replace at 80%.
fn joined_keys() -> RecipeFile {
    let mut slash = clip("Great Sword Slash", 72);
    slash.trim = Some([6, 58]);
    let mut thrust = clip("Great Sword Thrust", 64);
    thrust.trim = Some([4, 60]);
    thrust.blend = Some(BlendWindow {
        frames: 8,
        ..BlendWindow::default()
    });
    let mut r = on("Slash into thrust", slash, vec![thrust]);
    r.time = vec![
        TimeEdit::retime(0, 20, 34, 1.6),
        TimeEdit::hold(TimeKind::Freeze, 1, 30, 10),
    ];
    r.layers = vec![keys_layer(
        "l-2",
        LayerMode::Replace,
        0.8,
        vec![
            key(0, 30, KeyEase::Out, &[("chest", [0.0, -18.0, 0.0])]),
            key(1, 20, KeyEase::Smooth, &[("chest", [0.0, 52.0, 0.0])]),
            key(1, 30, KeyEase::Linear, &[("chest", [12.0, 0.0, 0.0])]),
            // Cut out by the trim: it still steers the blend toward it.
            key(1, 62, KeyEase::Smooth, &[("chest", [0.0, 0.0, 0.0])]),
        ],
    )];
    r
}

/// The rest pose, keys on its own frames.
fn rest_keys() -> RecipeFile {
    let mut r = on("Pose from nothing", RecipeBase::rest_pose(60), Vec::new());
    r.layers = vec![keys_layer(
        "l-1",
        LayerMode::Replace,
        1.0,
        vec![
            key(
                0,
                0,
                KeyEase::Smooth,
                &[("leftUpperArm", [0.0, 0.0, -70.0])],
            ),
            key(
                0,
                40,
                KeyEase::Smooth,
                &[("leftUpperArm", [0.0, -40.0, -20.0])],
            ),
        ],
    )];
    r
}

fn cases() -> Vec<RecipeFile> {
    vec![plain_keys(), joined_keys(), rest_keys()]
}

fn rounded(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

fn case(recipe: &RecipeFile) -> Value {
    let t = timeline(recipe);
    let mut frames: Vec<u32> = (0..=10).map(|i| i * t.frames / 10).collect();
    for layer in &recipe.layers {
        for k in &layer.keys {
            if let Some(f) = key_frame(&t, k.clip, k.frame) {
                frames.extend([f.saturating_sub(1), f, f + 1]);
            }
        }
    }
    frames.retain(|f| *f <= t.frames);
    frames.sort_unstable();
    frames.dedup();
    let probes: Vec<Value> = frames
        .into_iter()
        .map(|f| {
            let p = pose_at(recipe, f);
            json!({
                "frame": p.frame, "share": rounded(p.share), "clip": p.clip, "at": p.at,
                "turns": p.turns.iter().map(|k| json!({
                    "layer": k.layer, "bone": k.bone,
                    "turn": k.turn.map(rounded),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let keys: Vec<Value> = recipe
        .layers
        .iter()
        .flat_map(|l| l.keys.iter())
        .map(|k| {
            json!({
                "clip": k.clip, "frame": k.frame,
                "share": rounded(key_share(recipe, k.clip, k.frame)),
                "bakeFrame": key_frame(&t, k.clip, k.frame),
            })
        })
        .collect();
    json!({
        "name": recipe.name,
        "recipe": serde_json::from_str::<Value>(&serde_json::to_string(recipe).expect("recipe"))
            .expect("json"),
        "keys": keys,
        "probes": probes,
        "clipFrames": (0..=t.frames).step_by(7).map(|f| {
            let (clip, at) = clip_frame_at(&t, f);
            json!({ "frame": f, "clip": clip, "at": at })
        }).collect::<Vec<_>>(),
        "limits": keyed_limits(recipe),
    })
}

fn expected() -> Value {
    let eases = [
        KeyEase::Smooth,
        KeyEase::In,
        KeyEase::Out,
        KeyEase::Linear,
        KeyEase::Step,
    ];
    let bones = [
        "hips",
        "chest",
        "head",
        "leftUpperArm",
        "rightUpperArm",
        "rightLowerArm",
        "leftLowerLeg",
        "rightUpperLeg",
        "rightIndexProximal",
        "leftThumbDistal",
        "rightEye",
    ];
    json!({
        "eases": eases.iter().map(|e| json!({
            "ease": e,
            "at": ([0.0, 0.25, 0.5, 0.75, 1.0].map(|t| rounded(ease(*e, t)))),
        })).collect::<Vec<_>>(),
        "limits": bones.iter().map(|b| json!({
            "bone": b,
            "limits": joint_limits(b),
            "words": super::joint_limits::axis_words(b),
        })).collect::<Vec<_>>(),
        "cases": cases().iter().map(case).collect::<Vec<_>>(),
    })
}

#[test]
fn pose_keys_match_the_fixture_the_web_app_reads_too() {
    let want = expected();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    if std::env::var_os("MAQUETTE_WRITE_POSE_KEYS").is_some() {
        let text = serde_json::to_string_pretty(&want).expect("json");
        std::fs::write(&path, format!("{text}\n")).expect("write the fixture");
    }
    let have: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("fixture")).expect("json");
    assert_eq!(
        have, want,
        "the pose key rules moved: write the fixture again"
    );
}

fn turn_of(recipe: &RecipeFile, frame: u32, bone: &str) -> [f64; 3] {
    pose_at(recipe, frame)
        .turns
        .into_iter()
        .find(|t| t.bone == bone)
        .map(|t| t.turn)
        .unwrap_or_else(|| panic!("no turn for {bone} at {frame}"))
}

fn near(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6)
}

#[test]
fn a_bone_holds_its_first_and_last_key_and_blends_between_by_the_later_ease() {
    let r = plain_keys();
    assert!(
        near(turn_of(&r, 0, "chest"), [0.0, -18.0, 0.0]),
        "held before"
    );
    assert!(near(turn_of(&r, 10, "chest"), [0.0, -18.0, 0.0]));
    // Halfway to an out key: 1 - 0.5^2 of the way.
    assert!(near(
        turn_of(&r, 20, "chest"),
        [0.0, -18.0 + 24.0 * 0.75, 0.0]
    ));
    // A step holds the key before until its own frame.
    assert!(near(turn_of(&r, 44, "chest"), [0.0, 6.0, 0.0]));
    assert!(near(turn_of(&r, 45, "chest"), [10.0, 0.0, 0.0]));
    assert!(near(turn_of(&r, 50, "chest"), [30.0, 0.0, 0.0]), "linear");
    assert!(
        near(turn_of(&r, 60, "chest"), [50.0, 0.0, 0.0]),
        "held after"
    );
    // The arm blends only between its own keys, 10 and 40, eased in.
    let arm = turn_of(&r, 25, "rightUpperArm");
    assert!(near(
        arm,
        [-12.0 + 12.0 * 0.25, 30.0 * 0.25, 24.0 - 24.0 * 0.25]
    ));
}

#[test]
fn keys_ride_with_their_clip_through_trims_blends_retimes_and_freezes() {
    let r = joined_keys();
    let t = timeline(&r);
    let thrust_key = key_frame(&t, 1, 20).expect("shows");
    assert!(near(turn_of(&r, thrust_key, "chest"), [0.0, 52.0, 0.0]));
    // The freeze on clip 1 frame 30 holds that key over its added frames.
    let frozen = key_frame(&t, 1, 30).expect("shows");
    for f in frozen..frozen + 10 {
        assert!(near(turn_of(&r, f, "chest"), [12.0, 0.0, 0.0]), "frame {f}");
    }
    assert_eq!(key_frame(&t, 1, 62), None, "the trim cuts it out");
    // A key on the base still stands on the motion it sat on when a clip
    // is joined before it.
    let mut moved = r.clone();
    let mut first = clip("Draw", 20);
    first.blend = None;
    let mut slash = moved.base.clone();
    slash.blend = Some(BlendWindow::default());
    moved.base = first;
    moved.joined.insert(0, slash);
    for layer in &mut moved.layers {
        for k in &mut layer.keys {
            k.clip += 1;
        }
    }
    for time in &mut moved.time {
        time.clip += 1;
    }
    let t2 = timeline(&moved);
    let at = key_frame(&t2, 1, 30).expect("shows");
    assert_eq!(clip_frame_at(&t2, at), (1, 30));
}

#[test]
fn rest_pose_keys_sit_on_the_recipe_s_own_frames() {
    let r = rest_keys();
    let t = timeline(&r);
    assert_eq!(key_frame(&t, 0, 40), Some(40));
    assert!(near(turn_of(&r, 40, "leftUpperArm"), [0.0, -40.0, -20.0]));
    assert!(near(turn_of(&r, 20, "leftUpperArm"), [0.0, -20.0, -45.0]));
    assert_eq!(pose_at(&r, 33).at, 33);
}

#[test]
fn joint_limits_mirror_the_left_side_and_judge_keys_only() {
    let left = joint_limits("leftUpperArm").expect("left");
    let right = joint_limits("rightUpperArm").expect("right");
    assert_eq!(right.x, left.x);
    assert_eq!(right.y, [-left.y[1], -left.y[0]]);
    assert_eq!(right.z, [-left.z[1], -left.z[0]]);
    assert!(joint_limits("hips").is_none(), "the hips move the body");
    assert!(joint_limits("rightRingIntermediate").is_some());
    assert!(
        joint_limits("leftThumbIntermediate").is_none(),
        "no such bone"
    );
    let hits = over_limits("chest", [12.0, 52.0, 0.0]);
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].word.as_str(), hits[0].limit), ("twist", 45.0));
    let judged = keyed_limits(&joined_keys());
    assert_eq!(judged.judged, 4);
    assert_eq!(judged.hits.len(), 1, "{:?}", judged.hits);
    assert_eq!(judged.hits[0].clip, 1);
    assert_eq!(judged.hits[0].frame, 20);
    assert!(judged.hits[0].bake_frame.is_some());
    // The plain recipe keys the chest 50 degrees forward at its last key.
    let plain = keyed_limits(&plain_keys());
    assert_eq!(plain.hits[0].hit.degrees, 50.0);
    assert_eq!(plain.hits[0].bake_frame, Some(55));
}

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
