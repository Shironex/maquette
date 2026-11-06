//! Version 2 of the recipe file and the file cap: a recipe at every limit
//! fits under [`MAX_FILE_BYTES`], the recipe-wide key and turn caps refuse
//! what would not, and a write that would still go over is refused before
//! anything reaches the disk.

use super::tests_v2::{clip, key, pose_layer, project};
use super::*;

/// The longest humanoid bone names, so each turn takes the most bytes.
const LONG_BONES: [&str; 4] = [
    "rightLittleIntermediate",
    "leftLittleIntermediate",
    "rightMiddleIntermediate",
    "rightThumbMetacarpal",
];

/// The widest turn a pose key holds: every digit an f32 prints.
const WIDE: [f32; 3] = [-179.123_46, -123.456_79, -98.765_43];

fn long(prefix: &str, i: usize, len: usize) -> String {
    let head = format!("{prefix}-{i:02}-");
    format!("{head}{}", "x".repeat(len - head.len()))
}

/// A recipe at every limit at once: the longest names, labels and hashes,
/// every clip trimmed and blended, every time edit with all its fields,
/// every blank a point, every change used, and the recipe-wide keys and
/// turns spent on the widest turns.
fn at_every_limit() -> RecipeFile {
    let clips: Vec<RecipeBase> = (0..MAX_CLIPS)
        .map(|i| RecipeBase {
            name: long("clip", i, NAME_MAX_CHARS),
            hash: "f".repeat(HASH_MAX_CHARS),
            trim: Some([1, 1999]),
            blend: (i > 0).then_some(BlendWindow {
                frames: 999,
                curve: BlendCurve::Linear,
                line_up: LineUp::HipsAndFacing,
            }),
            ..clip("x", 2000)
        })
        .collect();
    let mut recipe = RecipeFile::new(&long("recipe", 0, NAME_MAX_CHARS), clips[0].clone());
    recipe.joined = clips[1..].to_vec();
    recipe.mirror = true;
    recipe.time = (0..MAX_TIME_EDITS)
        .map(|i| {
            let clip = u32::try_from(i % MAX_CLIPS).expect("clip");
            let at = u32::try_from(i / MAX_CLIPS).expect("slot") * 400 + 1;
            if i % 2 == 0 {
                TimeEdit {
                    ease: Some(99),
                    ..TimeEdit::retime(clip, at, at + 300, 1.234_567_9)
                }
            } else {
                TimeEdit {
                    breathe: Some(0.123_456_79),
                    ease_in: Some(599),
                    ease_out: Some(599),
                    ..TimeEdit::hold(TimeKind::Freeze, clip, at, 600)
                }
            }
        })
        .collect();
    recipe.blanks = (0..MAX_BLANKS)
        .map(|i| RecipeBlank {
            id: long("blank", i, ID_MAX_CHARS),
            label: long("label", i, LABEL_MAX_CHARS),
            kind: BlankKind::Point,
            prop: None,
            point: Some([-999.123_4, -999.123_4, -999.123_4]),
        })
        .collect();
    let per_layer = MAX_KEYS;
    let pose_layers = MAX_RECIPE_KEYS / per_layer;
    let bones_per_key = MAX_RECIPE_TURNS / MAX_RECIPE_KEYS;
    let mut layers: Vec<RecipeLayer> = (0..pose_layers)
        .map(|l| {
            let keys = (0..per_layer)
                .map(|k| {
                    let clip = u32::try_from(k % MAX_CLIPS).expect("clip");
                    let frame = u32::try_from(1000 + l * per_layer + k).expect("frame");
                    let bones: Vec<(&str, [f32; 3])> = LONG_BONES[..bones_per_key]
                        .iter()
                        .map(|b| (*b, WIDE))
                        .collect();
                    key(clip, frame, &bones)
                })
                .collect();
            pose_layer(&long("pose", l, ID_MAX_CHARS), keys)
        })
        .collect();
    for l in layers.len()..MAX_LAYERS {
        layers.push(RecipeLayer::new(
            &long("nudge", l, ID_MAX_CHARS),
            LayerKind::Nudge,
            "rightUpperArm",
            LayerSettings {
                weight: 0.123_456_79,
                start: 0.123_456_79,
                end: 0.987_654_3,
                rotation: Some(WIDE),
                offset: Some([-1.987_654_3, -1.987_654_3, -1.987_654_3]),
                ..LayerSettings::default()
            },
        ));
    }
    recipe.layers = layers;
    normalized(&recipe)
}

#[test]
fn a_recipe_at_every_limit_fits_in_a_file_and_opens_again() {
    let recipe = at_every_limit();
    validate(&recipe).expect("every limit is allowed");
    let keys: usize = recipe.layers.iter().map(|l| l.keys.len()).sum();
    assert_eq!(keys, MAX_RECIPE_KEYS);
    let text = serde_json::to_string_pretty(&recipe).expect("text") + "\n";
    let bytes = u64::try_from(text.len()).expect("len");
    assert!(
        bytes <= MAX_FILE_BYTES,
        "{bytes} bytes is over the {MAX_FILE_BYTES} byte cap"
    );

    let dir = tempfile::tempdir().expect("temp");
    let (id, _) = create(dir.path(), "Big", clip("Walk", 2000)).expect("create");
    let manifest = project();
    let read = update(
        dir.path(),
        &manifest,
        &id,
        RecipeChange {
            clips: Some(recipe.clips().cloned().collect()),
            time: Some(recipe.time.clone()),
            mirror: Some(true),
            blanks: Some(recipe.blanks.clone()),
            layers: Some(recipe.layers.clone()),
            ..RecipeChange::default()
        },
    )
    .expect("a recipe at every limit saves");
    assert_eq!(read.recipe.layers.len(), MAX_LAYERS);
    get(dir.path(), &manifest, &id).expect("and opens again");
}

#[test]
fn the_recipe_wide_caps_refuse_one_key_or_turn_more() {
    let mut recipe = at_every_limit();
    let extra = key(0, 7, &[("chest", [1.0, 0.0, 0.0])]);
    recipe.layers[0].keys.pop();
    recipe.layers.pop();
    recipe.layers.push(pose_layer(
        "more",
        vec![extra.clone(), {
            let mut k = extra.clone();
            k.frame = 8;
            k
        }],
    ));
    let err = validate(&recipe).expect_err("one key too many").to_string();
    assert!(err.contains(&format!("at most {MAX_RECIPE_KEYS}")), "{err}");

    let mut recipe = at_every_limit();
    recipe.layers.pop();
    recipe.layers.push(RecipeLayer {
        pose: Some([("rightHand".to_string(), [1.0, 0.0, 0.0])].into()),
        ..RecipeLayer::new(
            "hold",
            LayerKind::Grip,
            "rightHand",
            LayerSettings::default(),
        )
    });
    let err = validate(&recipe)
        .expect_err("one turn too many")
        .to_string();
    assert!(
        err.contains(&format!("at most {MAX_RECIPE_TURNS}")),
        "{err}"
    );
}

#[test]
fn a_write_over_the_cap_is_refused_before_it_reaches_the_disk() {
    let mut bones: Vec<String> = [
        "hips",
        "spine",
        "chest",
        "upperChest",
        "neck",
        "head",
        "leftShoulder",
        "rightShoulder",
    ]
    .iter()
    .map(|b| (*b).to_string())
    .collect();
    for side in ["left", "right"] {
        for finger in maquette_glb::vrm::FINGERS {
            for segment in ["Proximal", "Distal"] {
                bones.push(format!("{side}{finger}{segment}"));
            }
        }
    }
    bones.truncate(MAX_KEY_BONES);
    let mut recipe = RecipeFile::new("Too big", clip("Walk", 2000));
    recipe.layers = (0..4)
        .map(|l| {
            let keys = (0..MAX_KEYS)
                .map(|f| {
                    let turns: Vec<(&str, [f32; 3])> =
                        bones.iter().map(|b| (b.as_str(), WIDE)).collect();
                    key(0, u32::try_from(f).expect("frame"), &turns)
                })
                .collect();
            pose_layer(&format!("p-{l}"), keys)
        })
        .collect();
    let recipe = normalized(&recipe);
    assert!(validate(&recipe).is_err(), "the caps refuse it first");

    let dir = tempfile::tempdir().expect("temp");
    let file = dir.path().join("big.recipe.json");
    let err = files::write_recipe(&file, &recipe).expect_err("too big");
    assert!(err.to_string().starts_with(TOO_BIG), "{err}");
    assert!(!file.exists(), "nothing is written");
}
