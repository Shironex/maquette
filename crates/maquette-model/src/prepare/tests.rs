use serde_json::json;

use super::*;
use crate::assets::AssetKind;

fn size(axis: SizeAxis, value: f32, unit: SizeUnit) -> RealSize {
    RealSize { axis, value, unit }
}

fn lamp() -> ModelPrep {
    ModelPrep {
        skip_bake: None,
        pad_edges: None,
        parts: None,
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
        parts: None,
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
        parts: None,
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
        parts: None,
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
fn rotate_reads_its_turns_checks_its_range_and_runs_first() {
    let lean = Fixup::Rotate {
        x: -24.5,
        y: 0.0,
        z: 90.0,
    };
    assert_eq!(lean.describe(), "Rotate x -24.5°, z 90°");
    assert!(super::check_fixup("lamp", &lean).is_ok());
    for bad in [f32::NAN, 361.0, -400.0] {
        let turn = Fixup::Rotate {
            x: bad,
            y: 0.0,
            z: 0.0,
        };
        assert!(super::check_fixup("lamp", &turn).is_err(), "{bad}");
    }
    assert_eq!(FixupKind::ALL[0], FixupKind::Rotate);
    let parsed: Fixup =
        serde_json::from_str(r#"{ "op": "rotate", "x": 10, "y": 0, "z": 0 }"#).expect("rotate");
    assert_eq!(parsed.kind(), FixupKind::Rotate);
    let mut entry = ModelPrep {
        fixups: vec![Fixup::OriginBottom, lean],
        ..ModelPrep::default()
    };
    entry.fixups.sort_by_key(Fixup::kind);
    assert_eq!(entry.fixups[0], lean);
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
}

#[test]
fn a_split_keeps_its_way_and_threshold_and_describes_itself() {
    let split: Fixup =
        serde_json::from_value(json!({ "op": "split-parts", "mergeUnder": 0.01 })).expect("split");
    assert_eq!(
        split,
        Fixup::SplitParts {
            by: SplitBy::Loose,
            merge_under: 0.01
        },
        "loose pieces unless said otherwise"
    );
    assert_eq!(split.describe(), "Split into parts");
    let by_material = Fixup::SplitParts {
        by: SplitBy::Material,
        merge_under: 0.0,
    };
    assert_eq!(by_material.describe(), "Split into parts by material");
    assert_eq!(
        serde_json::to_value(by_material).expect("json"),
        json!({ "op": "split-parts", "by": "material", "mergeUnder": 0.0 })
    );
    let mut prep = ScenePrep::default();
    for (under, ok) in [(0.0, true), (0.2, true), (0.25, false), (f32::NAN, false)] {
        let entry = ModelPrep {
            fixups: vec![Fixup::SplitParts {
                by: SplitBy::Loose,
                merge_under: under,
            }],
            ..ModelPrep::default()
        };
        prep.models.insert("frost-lantern".into(), entry);
        assert_eq!(validate(&prep).is_ok(), ok, "{under}");
    }
}

#[test]
fn a_split_runs_after_merge_and_before_decimate() {
    let mut prep = ScenePrep::default();
    prep.models.insert(
        "frost-lantern".into(),
        ModelPrep {
            fixups: vec![
                Fixup::Decimate { triangles: 20_000 },
                Fixup::SplitParts {
                    by: SplitBy::Loose,
                    merge_under: 0.01,
                },
                Fixup::Merge { by_material: false },
            ],
            ..ModelPrep::default()
        },
    );
    let kinds: Vec<FixupKind> = normalized(&prep).models["frost-lantern"]
        .fixups
        .iter()
        .map(Fixup::kind)
        .collect();
    assert_eq!(
        kinds,
        [FixupKind::Merge, FixupKind::SplitParts, FixupKind::Decimate]
    );
}

fn lantern_parts() -> PartsPlan {
    PartsPlan {
        names: [("part_1", "lid"), ("part_2", "cage")]
            .into_iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect(),
        merge: vec![vec!["part_4".into(), "part_5".into()]],
        hidden: vec!["part_3".into()],
    }
}

#[test]
fn part_names_are_saved_per_asset_and_checked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = ModelPrep {
        parts: Some(lantern_parts()),
        ..ModelPrep::default()
    };
    assert!(!entry.is_empty(), "names alone keep the entry");
    save_entry(dir.path(), "frost-lantern", &entry).expect("save");
    let saved = load(dir.path()).expect("load");
    assert_eq!(saved.models["frost-lantern"].parts, Some(lantern_parts()));
    let text = fs::read_to_string(path(dir.path())).expect("read");
    assert!(text.contains("\"names\""), "{text}");

    let check = |parts: PartsPlan| {
        let mut prep = ScenePrep::default();
        prep.models.insert(
            "frost-lantern".into(),
            ModelPrep {
                parts: Some(parts),
                ..ModelPrep::default()
            },
        );
        validate(&prep)
    };
    let mut twice = lantern_parts();
    twice.names.insert("part_3".into(), "lid".into());
    let err = check(twice).expect_err("two lids");
    assert!(err.to_string().contains("both ship as lid"), "{err}");
    let mut empty = lantern_parts();
    empty.names.insert("part_3".into(), "  ".into());
    assert!(check(empty).is_err());
    let mut long = lantern_parts();
    long.names
        .insert("part_3".into(), "x".repeat(MAX_PART_NAME + 1));
    assert!(check(long).is_err());
    let mut alone = lantern_parts();
    alone.merge.push(vec!["part_3".into()]);
    assert!(check(alone).is_err(), "a join of one part");
    let mut again = lantern_parts();
    again.merge.push(vec!["part_5".into(), "part_3".into()]);
    assert!(check(again).is_err(), "a part in two joins");
}

#[test]
fn a_parts_patch_renames_joins_and_hides_by_model_name() {
    let mut current = ScenePrep::default();
    current.models.insert(
        "frost-lantern".into(),
        ModelPrep {
            parts: Some(lantern_parts()),
            ..ModelPrep::default()
        },
    );
    let patch: PrepPatch = serde_json::from_value(json!({
        "parts": {
            "rename": { "part_2": "part_2", "part_3": "crystal" },
            "merge": [["part_1", "part_4"]],
            "show": ["part_3"],
            "hide": ["part_5"],
        },
    }))
    .expect("patch");
    let next = apply_patch(&current, "frost-lantern", &patch).expect("apply");
    let parts = next.models["frost-lantern"].parts.clone().expect("parts");
    assert_eq!(parts.names.get("part_2"), None, "its own name forgets it");
    assert_eq!(parts.name_of("part_3"), "crystal");
    assert_eq!(parts.name_of("part_1"), "lid");
    assert_eq!(
        parts.merge,
        vec![vec!["part_1".to_string(), "part_4".to_string()]],
        "part_4 moved into the new join, the old one fell under two parts"
    );
    assert_eq!(parts.joined_into("part_4"), Some("part_1"));
    assert_eq!(parts.hidden, vec!["part_5".to_string()]);

    let undo: PrepPatch =
        serde_json::from_value(json!({ "parts": { "unmerge": ["part_4"], "show": ["part_5"] } }))
            .expect("undo");
    let back = apply_patch(&next, "frost-lantern", &undo).expect("apply");
    let parts = back.models["frost-lantern"].parts.clone().expect("parts");
    assert!(parts.merge.is_empty());
    assert!(parts.hidden.is_empty());
}
