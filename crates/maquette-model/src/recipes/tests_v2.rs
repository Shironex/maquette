//! Version 2 of the recipe file: version 1 files keep their bytes and both
//! version numbers, the version a save writes, version 2 round trips and
//! normalising, plus the helpers the other version 2 tests share.

use std::path::{Path, PathBuf};

use super::*;
use crate::json::write_json;
use crate::project::ProjectManifest;

/// One change to a recipe, named in a table of them.
pub(super) type Edit = Box<dyn Fn(&mut RecipeFile)>;

/// A version 1 file as Maquette wrote it before version 2, with its file
/// version and motion version as that Maquette read them.
pub(super) const V1: &str = include_str!("fixtures/v1.recipe.json");
pub(super) const V1_VERSION: &str = "1d63824c469a38a3";
pub(super) const V1_MOTION: &str = "78b4eb7896dfd419";
/// The same for a recipe on the rest pose.
pub(super) const V1_REST: &str = include_str!("fixtures/v1-rest.recipe.json");
pub(super) const V1_REST_VERSION: &str = "7af43b79c37f4612";
pub(super) const V1_REST_MOTION: &str = "a7cabc22d18d5d21";

pub(super) fn project() -> ProjectManifest {
    serde_json::from_value(serde_json::json!({
        "version": 1, "id": "frostveil", "name": "Frostveil", "template": "floating-diorama",
        "createdAt": "2026-11-05T10:00:00.000Z", "assets": []
    }))
    .expect("manifest")
}

/// `text` written as the recipe `id` of a project in `dir`.
pub(super) fn placed(dir: &Path, id: &str, text: &str) -> PathBuf {
    let folder = dir.join(RECIPES_DIR);
    std::fs::create_dir_all(&folder).expect("dir");
    let file = folder.join(format!("{id}{RECIPE_SUFFIX}"));
    std::fs::write(&file, text).expect("write");
    file
}

pub(super) fn clip(name: &str, frames: u32) -> RecipeBase {
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

pub(super) fn key(clip: u32, frame: u32, bones: &[(&str, [f32; 3])]) -> PoseKey {
    PoseKey {
        clip,
        frame,
        ease: KeyEase::Smooth,
        pose: bones.iter().map(|(b, t)| ((*b).to_string(), *t)).collect(),
    }
}

pub(super) fn pose_layer(id: &str, keys: Vec<PoseKey>) -> RecipeLayer {
    RecipeLayer {
        keys,
        ..RecipeLayer::new(id, LayerKind::Pose, "", LayerSettings::default())
    }
}

pub(super) fn pin(id: &str, bone: &str) -> RecipeLayer {
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

pub(super) fn grip(id: &str) -> RecipeLayer {
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
pub(super) fn slash() -> RecipeFile {
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
        let layered = matches!(word, "pose" | "grip");
        assert_eq!(
            !bake_unsupported(&recipe).is_empty(),
            layered,
            "only pose keys and grips cannot bake yet ({word})"
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
    assert_eq!(bake_unsupported(&read), vec!["pose keys", "grips"]);
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
