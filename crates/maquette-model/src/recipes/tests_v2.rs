//! Version 2 of the recipe file: version 1 files keep their bytes and both
//! version numbers, the version a save writes, version 2 round trips, the
//! limits and checks, and what the motion version covers.

use std::path::{Path, PathBuf};

use super::*;
use crate::json::write_json;
use crate::project::ProjectManifest;

/// One change to a recipe, named in a table of them.
type Edit = Box<dyn Fn(&mut RecipeFile)>;

/// A version 1 file as Maquette wrote it before version 2, with its file
/// version and motion version as that Maquette read them.
const V1: &str = include_str!("fixtures/v1.recipe.json");
const V1_VERSION: &str = "1d63824c469a38a3";
const V1_MOTION: &str = "78b4eb7896dfd419";
/// The same for a recipe on the rest pose.
const V1_REST: &str = include_str!("fixtures/v1-rest.recipe.json");
const V1_REST_VERSION: &str = "7af43b79c37f4612";
const V1_REST_MOTION: &str = "a7cabc22d18d5d21";

fn project() -> ProjectManifest {
    serde_json::from_value(serde_json::json!({
        "version": 1, "id": "frostveil", "name": "Frostveil", "template": "floating-diorama",
        "createdAt": "2026-11-05T10:00:00.000Z", "assets": []
    }))
    .expect("manifest")
}

/// `text` written as the recipe `id` of a project in `dir`.
fn placed(dir: &Path, id: &str, text: &str) -> PathBuf {
    let folder = dir.join(RECIPES_DIR);
    std::fs::create_dir_all(&folder).expect("dir");
    let file = folder.join(format!("{id}{RECIPE_SUFFIX}"));
    std::fs::write(&file, text).expect("write");
    file
}

fn clip(name: &str, frames: u32) -> RecipeBase {
    RecipeBase {
        name: name.into(),
        frames,
        length: f32::from(u16::try_from(frames).expect("frames")) / 30.0,
        hash: format!("{}-hash", name.to_lowercase().replace(' ', "-")),
        mixamo: true,
        rest: false,
        by_name: false,
        trim: None,
        blend: None,
    }
}

fn key(clip: u32, frame: u32, bones: &[(&str, [f32; 3])]) -> PoseKey {
    PoseKey {
        clip,
        frame,
        ease: KeyEase::Smooth,
        pose: bones.iter().map(|(b, t)| ((*b).to_string(), *t)).collect(),
    }
}

fn pose_layer(id: &str, keys: Vec<PoseKey>) -> RecipeLayer {
    RecipeLayer {
        keys,
        ..RecipeLayer::new(id, LayerKind::Pose, "", LayerSettings::default())
    }
}

fn pin(id: &str, bone: &str) -> RecipeLayer {
    RecipeLayer::new(
        id,
        LayerKind::Pin,
        bone,
        LayerSettings {
            start: 0.08,
            end: 0.54,
            ..LayerSettings::default()
        },
    )
}

fn grip(id: &str) -> RecipeLayer {
    RecipeLayer {
        pose: Some(
            [
                ("rightHand".to_string(), [4.0, 0.0, -8.0]),
                ("rightIndexProximal".to_string(), [62.0, 0.0, 0.0]),
            ]
            .into_iter()
            .collect(),
        ),
        from: Some(GripFrom {
            entry: "longsword-two-hands".into(),
            revision: 2,
        }),
        ..RecipeLayer::new(id, LayerKind::Grip, "rightHand", LayerSettings::default())
    }
}

/// The joined slash of the design: three clips with trims and blends, a
/// retime and a freeze, and a pin, pose keys and a grip.
fn slash() -> RecipeFile {
    let mut recipe = RecipeFile::new(
        "Slash into thrust",
        RecipeBase {
            trim: Some([6, 58]),
            ..clip("Great Sword Slash", 72)
        },
    );
    recipe.joined = vec![
        RecipeBase {
            trim: Some([4, 60]),
            blend: Some(BlendWindow::default()),
            ..clip("Great Sword Thrust", 64)
        },
        RecipeBase {
            trim: Some([0, 40]),
            blend: Some(BlendWindow {
                frames: 6,
                ..BlendWindow::default()
            }),
            ..clip("Sword Idle", 90)
        },
    ];
    recipe.time = vec![
        TimeEdit::retime(0, 20, 34, 1.6),
        TimeEdit::hold(TimeKind::Freeze, 1, 56, 18),
    ];
    recipe.layers = vec![
        pin("l-1", "leftFoot"),
        pose_layer(
            "l-2",
            vec![
                key(
                    0,
                    30,
                    &[
                        ("chest", [0.0, -18.0, 0.0]),
                        ("rightUpperArm", [-12.0, 0.0, 24.0]),
                    ],
                ),
                key(1, 20, &[("chest", [0.0, 6.0, 0.0])]),
            ],
        ),
        grip("l-3"),
    ];
    normalized(&recipe)
}

#[test]
fn a_version_1_file_keeps_its_bytes_and_both_versions() {
    for (text, version, motion) in [
        (V1, V1_VERSION, V1_MOTION),
        (V1_REST, V1_REST_VERSION, V1_REST_MOTION),
    ] {
        let dir = tempfile::tempdir().expect("temp");
        let file = placed(dir.path(), "old", text);
        let (recipe, read) = load_versioned(&file).expect("load");
        assert_eq!(read, version);
        assert_eq!(recipe.version, RECIPE_VERSION_1);
        assert_eq!(motion_version(&recipe), motion, "no bake of it goes stale");
        assert!(!uses_v2(&recipe));
        assert_eq!(recipe.clips().count(), 1, "reads as one clip");
        assert!(recipe.time.is_empty() && !recipe.mirror);
        write_json(&file, &recipe).expect("write");
        assert_eq!(std::fs::read_to_string(&file).expect("read"), text);
        assert_eq!(
            version_of(&std::fs::read_to_string(&file).expect("read")),
            version
        );
    }
}

#[test]
fn a_version_1_file_changed_without_anything_new_stays_version_1() {
    let dir = tempfile::tempdir().expect("temp");
    placed(dir.path(), "old", V1);
    let read = get(dir.path(), &project(), "old").expect("get");
    let mut layers = read.recipe.layers.clone();
    layers[3].settings.weight = 0.25;
    let change = RecipeChange {
        layers: Some(layers),
        expected: Some(read.version.clone()),
        ..RecipeChange::default()
    };
    let changed = update(dir.path(), &project(), "old", change).expect("update");
    assert_eq!(changed.recipe.version, RECIPE_VERSION_1);
    let text = std::fs::read_to_string(dir.path().join("recipes/old.recipe.json")).expect("read");
    assert!(text.contains("\"version\": 1,"), "{text}");
    for word in ["joined", "mirror", "time", "trim", "blend", "keys", "mode"] {
        assert!(!text.contains(&format!("\"{word}\"")), "{word} in {text}");
    }
}

#[test]
fn a_save_writes_version_2_only_while_something_new_is_used() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, recipe) = create(dir.path(), "Slash", clip("Great Sword Slash", 72)).expect("make");
    assert_eq!(recipe.version, RECIPE_VERSION_1);
    let send = |change: RecipeChange| {
        let version = super::version(dir.path(), &id).expect("version");
        let change = RecipeChange {
            expected: Some(version),
            ..change
        };
        update(dir.path(), &project(), &id, change).expect("update")
    };
    let on = send(RecipeChange {
        mirror: Some(true),
        ..RecipeChange::default()
    });
    assert_eq!(on.recipe.version, RECIPE_VERSION);
    let text = std::fs::read_to_string(dir.path().join(format!("recipes/{id}.recipe.json")))
        .expect("read");
    assert!(text.contains("\"version\": 2,") && text.contains("\"mirror\": true"));
    let off = send(RecipeChange {
        mirror: Some(false),
        ..RecipeChange::default()
    });
    assert_eq!(
        off.recipe.version, RECIPE_VERSION_1,
        "back to what an older Maquette opens"
    );
    assert_eq!(off.motion_version, motion_version(&recipe));
}

#[test]
fn each_new_thing_alone_needs_version_2() {
    let one = normalized(&RecipeFile::new("Slash", clip("Great Sword Slash", 72)));
    let cases: Vec<(&str, Edit)> = vec![
        ("trim", Box::new(|r| r.base.trim = Some([2, 40]))),
        (
            "joined",
            Box::new(|r| r.joined = vec![clip("Sword Idle", 90)]),
        ),
        ("mirror", Box::new(|r| r.mirror = true)),
        (
            "time",
            Box::new(|r| r.time = vec![TimeEdit::retime(0, 10, 20, 0.5)]),
        ),
        ("pin", Box::new(|r| r.layers = vec![pin("l-1", "leftFoot")])),
        (
            "pose",
            Box::new(|r| r.layers = vec![pose_layer("l-1", Vec::new())]),
        ),
        ("grip", Box::new(|r| r.layers = vec![grip("l-1")])),
    ];
    for (word, change) in cases {
        let mut recipe = one.clone();
        change(&mut recipe);
        let recipe = normalized(&recipe);
        validate(&recipe).unwrap_or_else(|e| panic!("{word}: {e}"));
        assert_eq!(recipe.version, RECIPE_VERSION, "{word}");
        assert_ne!(
            motion_version(&recipe),
            motion_version(&one),
            "{word} moves"
        );
        assert!(
            !bake_unsupported(&recipe).is_empty(),
            "{word} cannot bake yet"
        );
    }
    assert!(bake_unsupported(&one).is_empty());
}

#[test]
fn a_version_2_recipe_round_trips_whole() {
    let recipe = slash();
    validate(&recipe).expect("valid");
    assert_eq!(recipe.version, RECIPE_VERSION);
    let dir = tempfile::tempdir().expect("temp");
    let (id, written, version) = create_whole(dir.path(), &recipe, None).expect("write");
    let file = dir.path().join(format!("recipes/{id}.recipe.json"));
    let (read, read_version) = load_versioned(&file).expect("read");
    assert_eq!(read, written);
    assert_eq!(read_version, version);
    let text = std::fs::read_to_string(&file).expect("text");
    write_json(&file, &read).expect("again");
    assert_eq!(
        std::fs::read_to_string(&file).expect("text"),
        text,
        "same bytes"
    );
    assert_eq!(read.clips().count(), 3);
    assert_eq!(read.clip(2).map(|c| c.name.as_str()), Some("Sword Idle"));
    assert_eq!(read.layers[1].mode, Some(LayerMode::Add), "pose keys add");
    assert_eq!(
        read.layers[2].mode,
        Some(LayerMode::Replace),
        "grips replace"
    );
    assert_eq!(
        bake_unsupported(&read),
        vec![
            "joined clips",
            "trims",
            "time edits",
            "pins",
            "pose keys",
            "grips"
        ]
    );
}

#[test]
fn a_file_of_the_design_reads_as_written() {
    let text = serde_json::json!({
        "version": 2, "name": "Slash into thrust", "revision": 3,
        "created": "2026-11-05T10:00:00.000Z", "edited": "2026-11-05T10:12:00.000Z",
        "base": { "name": "Great Sword Slash", "frames": 72, "length": 2.4, "hash": "4be09c",
                  "mixamo": true, "trim": [6, 58] },
        "joined": [{ "name": "Great Sword Thrust", "frames": 64, "length": 2.133, "hash": "a91c27",
                     "mixamo": true, "trim": [4, 60], "blend": { "frames": 8 } }],
        "mirror": false,
        "time": [{ "kind": "freeze", "clip": 1, "at": 56, "frames": 18 }],
        "blanks": [],
        "layers": [{ "id": "l-2", "kind": "pose", "bone": "",
                     "settings": { "weight": 1, "start": 0, "end": 1, "target": null, "rotation": null,
                                   "offset": null, "cycles": null, "amount": null },
                     "keys": [{ "frame": 30, "pose": { "chest": [0, -18, 0] } }] }]
    })
    .to_string();
    let dir = tempfile::tempdir().expect("temp");
    let file = placed(dir.path(), "slash", &text);
    let recipe = load_file(&file).expect("load");
    let blend = recipe.joined[0].blend.as_ref().expect("blend");
    assert_eq!(
        blend.line_up,
        LineUp::HipsAndFacing,
        "hips and facing lined up"
    );
    assert_eq!(blend.curve, BlendCurve::Smooth);
    assert_eq!(recipe.layers[0].keys[0].clip, 0);
    assert_eq!(
        recipe.layers[0].keys[0].ease,
        KeyEase::Smooth,
        "keys ease smooth unless told"
    );
    assert_eq!(recipe.layers[0].mode, Some(LayerMode::Add));
}

#[test]
fn a_newer_file_than_version_2_is_left_alone() {
    let mut value = serde_json::to_value(slash()).expect("value");
    value["version"] = serde_json::json!(RECIPE_VERSION + 1);
    let dir = tempfile::tempdir().expect("temp");
    let file = placed(dir.path(), "later", &value.to_string());
    assert!(matches!(
        load_file(&file),
        Err(CoreError::UnsupportedVersion {
            found: 3,
            supported: 2
        })
    ));
}

#[test]
fn normalising_fills_blends_and_keeps_only_what_a_kind_reads() {
    let mut recipe = RecipeFile::new(
        "Slash",
        RecipeBase {
            blend: Some(BlendWindow::default()),
            ..clip("Great Sword Slash", 72)
        },
    );
    recipe.joined = vec![clip(" Sword Idle ", 90)];
    let mut stray = pin("l-1", "rightHand");
    stray.keys = vec![key(0, 1, &[("chest", [0.0, 1.0, 0.0])])];
    stray.mode = Some(LayerMode::Add);
    stray.settings.rotation = Some([1.0, 0.0, 0.0]);
    let mut keyed = pose_layer("l-2", vec![key(1, 5, &[]), key(0, 9, &[])]);
    keyed.bone = "chest".into();
    keyed.settings.start = 0.4;
    let mut freeze = TimeEdit::hold(TimeKind::Freeze, 0, 10, 4);
    freeze.speed = Some(2.0);
    recipe.time = vec![freeze];
    recipe.layers = vec![stray, keyed];
    let out = normalized(&recipe);
    assert_eq!(out.base.blend, None, "the base has no blend");
    assert_eq!(
        out.joined[0].blend,
        Some(BlendWindow::default()),
        "8 f smooth lined up"
    );
    assert_eq!(out.joined[0].name, "Sword Idle");
    assert!(out.layers[0].keys.is_empty() && out.layers[0].mode.is_none());
    assert_eq!(out.layers[0].settings.rotation, None);
    assert_eq!(out.layers[1].bone, "");
    assert_eq!(out.layers[1].settings.start, 0.0);
    let frames: Vec<(u32, u32)> = out.layers[1]
        .keys
        .iter()
        .map(|k| (k.clip, k.frame))
        .collect();
    assert_eq!(frames, vec![(0, 9), (1, 5)], "in clip and frame order");
    assert_eq!(out.time[0].speed, None);
}

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
