//! Tests for the fix-up preview: its Blender run, its cache key, its
//! numbers and the script it runs.

use std::fs;
use std::path::Path;

use serde_json::json;

use super::*;
use crate::prepare::{
    Fixup, ModelPrep, PartsPlan, RealSize, SCRIPT, SCRIPT_PATH, SizeAxis, SizeUnit, SplitBy,
};

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
fn the_preview_runs_headless_with_whole_path_args() {
    let input = Path::new("/ws/inbox/desk lamp.glb");
    let args = args(input);
    assert_eq!(&args[..2], ["-b", "--factory-startup"]);
    assert!(crate::pipeline::graph::blender_headless(&args));
    let at = args.iter().position(|a| a == "--input").expect("input");
    assert_eq!(args[at + 1], "/ws/inbox/desk lamp.glb", "one argument");
    assert!(args.windows(2).any(|w| w == ["--python", "{script}"]));
    let def = def(input);
    assert!(
        crate::pipeline::validate(&def).is_empty(),
        "{:?}",
        crate::pipeline::validate(&def)
    );
}

#[test]
fn the_preview_key_follows_the_model_plan_and_script() {
    let a = key("abc", &lamp(), SCRIPT);
    assert_eq!(a, key("abc", &lamp(), &SCRIPT.replace('\n', "\r\n")));
    assert_ne!(a, key("abd", &lamp(), SCRIPT));
    assert_ne!(a, key("abc", &ModelPrep::default(), SCRIPT));
    assert_ne!(a, key("abc", &lamp(), "print('mine')"));
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
    let report = read_report(dir.path(), &input, &lamp()).expect("report");
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
            draw_calls: None,
        }
    );
}

#[test]
fn a_project_copy_of_the_script_wins_over_the_shipped_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(script_for(Some(dir.path())), SCRIPT);
    assert_eq!(script_for(None), SCRIPT);
    fs::create_dir_all(dir.path().join("tools")).expect("tools");
    fs::write(dir.path().join(SCRIPT_PATH), "print('mine')\n").expect("mine");
    assert_eq!(script_for(Some(dir.path())), "print('mine')\n");
}

#[test]
fn skipping_the_bake_leaves_the_preview_key_alone() {
    let plan = lamp();
    let skipped = ModelPrep {
        skip_bake: Some(true),
        pad_edges: None,
        parts: None,
        ..lamp()
    };
    assert_eq!(key("abc", &plan, "script"), key("abc", &skipped, "script"));
}

#[test]
fn edge_padding_leaves_the_preview_key_alone() {
    // Padding changes no bounds, so a preview keeps its key.
    let padded = ModelPrep {
        pad_edges: Some(true),
        ..lamp()
    };
    assert_eq!(key("abc", &lamp(), "script"), key("abc", &padded, "script"));
}

#[test]
fn hiding_a_part_leaves_the_preview_key_alone_and_a_name_changes_it() {
    let base = ModelPrep {
        fixups: vec![Fixup::SplitParts {
            by: SplitBy::Loose,
            merge_under: 0.01,
        }],
        ..ModelPrep::default()
    };
    let key_of = |prep: &ModelPrep| key("abc", prep, SCRIPT);
    let hidden = ModelPrep {
        parts: Some(PartsPlan {
            hidden: vec!["part_1".into()],
            ..PartsPlan::default()
        }),
        ..base.clone()
    };
    assert_eq!(key_of(&base), key_of(&hidden));
    let named = ModelPrep {
        parts: Some(lantern_parts()),
        ..base.clone()
    };
    assert_ne!(key_of(&base), key_of(&named));
    let lamp_key = key("abc", &lamp(), SCRIPT);
    assert_eq!(
        lamp_key,
        key("abc", &lamp(), SCRIPT),
        "a plan without parts keeps its old key"
    );
}

#[test]
fn the_script_applies_merge_split_and_padding_in_plan_order() {
    assert!(
        SCRIPT.contains("\"merge\",\n    \"split-parts\",\n    \"pad-edges\""),
        "the script applies them in the order the plan keeps"
    );
}
