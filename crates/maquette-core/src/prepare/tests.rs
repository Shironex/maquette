use std::path::Path;

use serde_json::json;

use super::preview::{self, ModelNumbers};
use super::*;
use crate::assets::AssetKind;

fn size(axis: SizeAxis, value: f32, unit: SizeUnit) -> RealSize {
    RealSize { axis, value, unit }
}

fn lamp() -> ModelPrep {
    ModelPrep {
        skip_bake: None,
        pad_edges: None,
        size: Some(size(SizeAxis::Height, 45.0, SizeUnit::Cm)),
        fixups: vec![Fixup::OriginBottom, Fixup::Decimate { triangles: 60_000 }],
    }
}

#[test]
fn a_missing_file_is_nothing_prepared() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(load(dir.path()).expect("load"), ScenePrep::default());
}

#[test]
fn the_file_round_trips_in_fixup_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    let saved = save_entry(dir.path(), "desk-lamp", &lamp()).expect("save");
    let entry = &saved.models["desk-lamp"];
    assert_eq!(
        entry.fixups,
        vec![Fixup::Decimate { triangles: 60_000 }, Fixup::OriginBottom,],
        "kept in the order they apply"
    );
    assert_eq!(load(dir.path()).expect("load"), saved);
    let text = fs::read_to_string(path(dir.path())).expect("read");
    let value: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(value["version"], 1);
    assert_eq!(
        value["models"]["desk-lamp"]["size"],
        json!({ "axis": "height", "value": 45.0, "unit": "cm" })
    );
    assert_eq!(
        value["models"]["desk-lamp"]["fixups"][0],
        json!({ "op": "decimate", "triangles": 60000 })
    );
}

#[test]
fn saving_one_asset_keeps_the_others_and_an_empty_entry_goes() {
    let dir = tempfile::tempdir().expect("tempdir");
    save_entry(dir.path(), "desk-lamp", &lamp()).expect("lamp");
    let mug = ModelPrep {
        skip_bake: None,
        pad_edges: None,
        size: Some(size(SizeAxis::Height, 10.0, SizeUnit::Cm)),
        fixups: Vec::new(),
    };
    let both = save_entry(dir.path(), "mug", &mug).expect("mug");
    assert_eq!(both.models.len(), 2);
    let left = save_entry(dir.path(), "mug", &ModelPrep::default()).expect("clear");
    assert_eq!(left.models.keys().collect::<Vec<_>>(), vec!["desk-lamp"]);
}

#[test]
fn a_newer_or_broken_file_is_never_written_over() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = path(dir.path());
    fs::write(&file, r#"{ "version": 2, "models": {}, "later": true }"#).expect("newer");
    let err = save_entry(dir.path(), "mug", &lamp()).expect_err("newer");
    assert_eq!(err.code(), "unsupported-version");
    fs::write(&file, "{ not json").expect("broken");
    let err = save_entry(dir.path(), "mug", &lamp()).expect_err("broken");
    assert!(err.to_string().contains("nothing was saved"), "{err}");
    assert_eq!(fs::read_to_string(&file).expect("read"), "{ not json");
}

#[test]
fn validation_refuses_sizes_and_numbers_out_of_range() {
    let mut prep = ScenePrep::default();
    let mut entry = ModelPrep {
        skip_bake: None,
        pad_edges: None,
        size: Some(size(SizeAxis::Width, 0.05, SizeUnit::Cm)),
        fixups: Vec::new(),
    };
    prep.models.insert("mug".into(), entry.clone());
    assert!(validate(&prep).is_err(), "half a millimetre");
    entry.size = Some(size(SizeAxis::Width, 1001.0, SizeUnit::M));
    prep.models.insert("mug".into(), entry.clone());
    assert!(validate(&prep).is_err(), "over a kilometre");
    entry.size = Some(size(SizeAxis::Width, f32::NAN, SizeUnit::M));
    prep.models.insert("mug".into(), entry.clone());
    assert!(validate(&prep).is_err(), "not a number");
    entry.size = None;
    for bad in [
        Fixup::Decimate { triangles: 10 },
        Fixup::ShrinkTextures { max_size: 32 },
        Fixup::RemoveLoose { min_share: 0.0 },
        Fixup::RemoveLoose { min_share: 0.9 },
    ] {
        entry.fixups = vec![bad];
        prep.models.insert("mug".into(), entry.clone());
        assert!(validate(&prep).is_err(), "{bad:?}");
    }
    entry.fixups = vec![Fixup::OriginBottom, Fixup::OriginBottom];
    prep.models.insert("mug".into(), entry);
    let err = validate(&prep).expect_err("twice");
    assert!(err.to_string().contains("twice"), "{err}");
}

#[test]
fn a_patch_adds_replaces_removes_and_clears() {
    let mut current = ScenePrep::default();
    current.models.insert("desk-lamp".into(), lamp());
    let patch: PrepPatch = serde_json::from_value(json!({
        "add": [{ "op": "decimate", "triangles": 20000 }, { "op": "merge", "byMaterial": true }],
        "remove": ["origin-bottom"],
    }))
    .expect("patch");
    let next = apply_patch(&current, "desk-lamp", &patch).expect("apply");
    let entry = &next.models["desk-lamp"];
    assert_eq!(
        entry.fixups,
        vec![
            Fixup::Merge { by_material: true },
            Fixup::Decimate { triangles: 20_000 },
        ]
    );
    assert_eq!(entry.size, lamp().size, "a size left out stays");

    let clear: PrepPatch =
        serde_json::from_value(json!({ "size": null, "clearFixups": true })).expect("clear");
    let cleared = apply_patch(&next, "desk-lamp", &clear).expect("apply");
    assert!(cleared.models.is_empty(), "an empty entry is dropped");

    let twice: PrepPatch = serde_json::from_value(json!({
        "add": [{ "op": "origin-bottom" }, { "op": "origin-bottom" }],
    }))
    .expect("twice");
    assert!(apply_patch(&current, "desk-lamp", &twice).is_err());
    let bad: PrepPatch = serde_json::from_value(json!({
        "size": { "axis": "height", "value": -1, "unit": "cm" },
    }))
    .expect("bad");
    assert!(apply_patch(&current, "desk-lamp", &bad).is_err());
}

#[test]
fn the_real_size_scale_measures_the_chosen_side() {
    // A prop that came back about a metre high, 0.4 wide, 0.5 deep.
    let extent = [0.4, 1.0, 0.5];
    let near = |a: Option<f32>, b: f32| a.is_some_and(|a| (a - b).abs() < 1e-5);
    assert!(near(
        scale_for(&size(SizeAxis::Height, 45.0, SizeUnit::Cm), extent),
        0.45
    ));
    assert!(near(
        scale_for(&size(SizeAxis::Width, 32.0, SizeUnit::Cm), extent),
        0.8
    ));
    assert!(near(
        scale_for(&size(SizeAxis::Depth, 1.0, SizeUnit::M), extent),
        2.0
    ));
    assert!(near(
        scale_for(&size(SizeAxis::Longest, 2.0, SizeUnit::M), extent),
        2.0
    ));
    assert_eq!(
        scale_for(&size(SizeAxis::Height, 45.0, SizeUnit::Cm), [1.0, 0.0, 1.0]),
        None,
        "a flat model has no height to scale"
    );
}

#[test]
fn the_frame_moves_the_bottom_centre_to_the_origin_then_scales() {
    let bounds = ([0.05, 0.5, -0.45], [0.55, 1.2, 0.05]);
    let frame = frame_of(Some(&lamp()), Some(bounds));
    assert!((frame.scale - 0.45 / 0.7).abs() < 1e-5, "{frame:?}");
    let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-6);
    assert!(near(frame.offset, [-0.3, -0.5, 0.2]), "{frame:?}");
    let (min, max) = frame.apply(bounds);
    assert!(min[1].abs() < 1e-6, "on the floor");
    assert!((max[1] - 0.45).abs() < 1e-5, "45 cm high");
    assert!((min[0] + max[0]).abs() < 1e-6, "centred");

    let only_size = ModelPrep {
        skip_bake: None,
        pad_edges: None,
        size: lamp().size,
        fixups: Vec::new(),
    };
    let frame = frame_of(Some(&only_size), Some(bounds));
    assert_eq!(frame.offset, [0.0; 3], "no origin fix-up, no shift");
    assert!(frame_of(None, Some(bounds)).is_identity());
    assert!(frame_of(Some(&lamp()), None).is_identity());
}

#[test]
fn suggestions_come_from_the_name_and_kind() {
    let got = |name: &str, kind| suggest(name, kind).map(|s| (s.matched, s.size.describe()));
    assert_eq!(
        got("Desk lamp", AssetKind::Prop),
        Some(("desk lamp".into(), "45 cm high".into()))
    );
    assert_eq!(
        got("Old laptop", AssetKind::Prop),
        Some(("laptop".into(), "32 cm wide".into()))
    );
    assert_eq!(
        got("Coffee mugs", AssetKind::Prop),
        Some(("mug".into(), "10 cm high".into()))
    );
    assert_eq!(
        got("Wooden bench", AssetKind::Prop),
        Some(("bench".into(), "1.2 m wide".into()))
    );
    assert_eq!(
        got("Lantern keeper", AssetKind::Character),
        Some(("character".into(), "1.7 m high".into()))
    );
    assert_eq!(
        got("Smugglers den", AssetKind::Prop),
        None,
        "mug inside a word"
    );
    assert_eq!(got("Desk lamp", AssetKind::Scene), None);
}

#[test]
fn fixups_describe_themselves_for_the_buttons() {
    assert_eq!(
        Fixup::Decimate { triangles: 60_000 }.describe(),
        "Decimate to 60k"
    );
    assert_eq!(
        Fixup::ShrinkTextures { max_size: 2048 }.describe(),
        "Shrink textures to 2048 px"
    );
    assert_eq!(
        Fixup::RemoveLoose { min_share: 0.02 }.describe(),
        "Remove loose parts under 2%"
    );
    assert_eq!(Fixup::OriginBottom.describe(), "Origin to bottom centre");
}

#[test]
fn the_preview_runs_headless_with_whole_path_args() {
    let input = Path::new("/ws/inbox/desk lamp.glb");
    let args = preview::args(input);
    assert_eq!(&args[..2], ["-b", "--factory-startup"]);
    assert!(crate::pipeline::graph::blender_headless(&args));
    let at = args.iter().position(|a| a == "--input").expect("input");
    assert_eq!(args[at + 1], "/ws/inbox/desk lamp.glb", "one argument");
    assert!(args.windows(2).any(|w| w == ["--python", "{script}"]));
    let def = preview::def(input);
    assert!(
        crate::pipeline::validate(&def).is_empty(),
        "{:?}",
        crate::pipeline::validate(&def)
    );
}

#[test]
fn the_preview_key_follows_the_model_plan_and_script() {
    let a = preview::key("abc", &lamp(), SCRIPT);
    assert_eq!(
        a,
        preview::key("abc", &lamp(), &SCRIPT.replace('\n', "\r\n"))
    );
    assert_ne!(a, preview::key("abd", &lamp(), SCRIPT));
    assert_ne!(a, preview::key("abc", &ModelPrep::default(), SCRIPT));
    assert_ne!(a, preview::key("abc", &lamp(), "print('mine')"));
}

#[test]
fn a_finished_preview_reads_its_numbers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("lamp.glb");
    fs::write(&input, b"glTF....").expect("model");
    fs::write(dir.path().join("fixed.glb"), b"glTF").expect("fixed");
    let numbers = |triangles: u32| json!({ "triangles": triangles, "objects": 1, "maxTexture": 256, "min": [0, 0, 0], "max": [1, 1, 1] });
    let stats = json!({
        "before": numbers(19224),
        "after": numbers(4999),
        "steps": [
            { "op": "decimate", "before": numbers(19212), "after": numbers(4999), "note": "ratio 0.260" },
            { "op": "from-a-later-version", "before": numbers(1), "after": numbers(1), "note": "" },
        ],
        "scale": 0.64,
    });
    fs::write(dir.path().join("stats.json"), stats.to_string()).expect("stats");
    let report = preview::read_report(dir.path(), &input, &lamp()).expect("report");
    assert_eq!(report.before.triangles, 19_224);
    assert_eq!(report.after.triangles, 4_999);
    assert_eq!(report.steps.len(), 1, "an unknown kind is left out");
    assert_eq!(report.steps[0].op, FixupKind::Decimate);
    assert_eq!(report.bytes_before, Some(8.0));
    assert_eq!(report.bytes_after, Some(4.0));
    assert_eq!(
        report.after,
        ModelNumbers {
            triangles: 4999,
            objects: 1,
            max_texture: 256,
            min: Some([0.0; 3]),
            max: Some([1.0; 3]),
        }
    );
}

#[test]
fn a_project_copy_of_the_script_wins_over_the_shipped_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(preview::script_for(Some(dir.path())), SCRIPT);
    assert_eq!(preview::script_for(None), SCRIPT);
    fs::create_dir_all(dir.path().join("tools")).expect("tools");
    fs::write(dir.path().join(SCRIPT_PATH), "print('mine')\n").expect("mine");
    assert_eq!(preview::script_for(Some(dir.path())), "print('mine')\n");
}

#[test]
fn skipping_the_bake_is_kept_on_its_own_and_a_null_forgets_it() {
    let empty = ScenePrep::default();
    let skip: PrepPatch = serde_json::from_str(r#"{ "skipBake": true }"#).expect("patch");
    let next = apply_patch(&empty, "desk-globe", &skip).expect("skip");
    let entry = &next.models["desk-globe"];
    assert!(entry.skips_bake());
    assert!(!entry.is_empty(), "a choice alone is worth keeping");
    // Left out keeps the choice.
    let size: PrepPatch =
        serde_json::from_str(r#"{ "add": [{ "op": "origin-bottom" }] }"#).expect("patch");
    let kept = apply_patch(&next, "desk-globe", &size).expect("fixup");
    assert!(kept.models["desk-globe"].skips_bake());
    // Null forgets it, and the then empty entry goes.
    let forget: PrepPatch = serde_json::from_str(r#"{ "skipBake": null }"#).expect("patch");
    let gone = apply_patch(&next, "desk-globe", &forget).expect("forget");
    assert!(!gone.models.contains_key("desk-globe"));
    // Older files without the field still load, as "nobody chose".
    let old: ModelPrep = serde_json::from_str(r#"{ "size": null, "fixups": [] }"#).expect("old");
    assert_eq!(old.skip_bake, None);
    assert!(!old.skips_bake());
}

#[test]
fn skipping_the_bake_leaves_the_preview_key_alone() {
    let plan = lamp();
    let skipped = ModelPrep {
        skip_bake: Some(true),
        pad_edges: None,
        ..lamp()
    };
    assert_eq!(
        preview::key("abc", &plan, "script"),
        preview::key("abc", &skipped, "script")
    );
}

#[test]
fn edge_padding_is_a_choice_per_asset_that_a_patch_sets_and_forgets() {
    let on: PrepPatch = serde_json::from_str(r#"{ "padEdges": true }"#).expect("patch");
    let next = apply_patch(&ScenePrep::default(), "planner", &on).expect("on");
    assert!(next.models["planner"].pads_edges());
    let text = serde_json::to_string(&next.models["planner"]).expect("json");
    assert!(text.contains(r#""padEdges":true"#), "{text}");
    let off: PrepPatch = serde_json::from_str(r#"{ "padEdges": false }"#).expect("patch");
    let next = apply_patch(&next, "planner", &off).expect("off");
    assert_eq!(next.models["planner"].pad_edges, Some(false));
    let forget: PrepPatch = serde_json::from_str(r#"{ "padEdges": null }"#).expect("patch");
    let gone = apply_patch(&next, "planner", &forget).expect("forget");
    assert!(
        !gone.models.contains_key("planner"),
        "an entry with no choice left goes"
    );
    let old: ModelPrep = serde_json::from_str(r#"{ "size": null, "fixups": [] }"#).expect("old");
    assert!(!old.pads_edges(), "older files do not pad");
    // Padding changes no bounds, so a preview keeps its key.
    let padded = ModelPrep {
        pad_edges: Some(true),
        ..lamp()
    };
    assert_eq!(
        preview::key("abc", &lamp(), "script"),
        preview::key("abc", &padded, "script")
    );
}
