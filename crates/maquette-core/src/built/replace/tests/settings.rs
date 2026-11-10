//! The settings table: the real size carries over and never scales twice,
//! what repaired the scan is reset, and the built asset's own comes along.

use std::collections::BTreeMap;

use super::*;
use crate::materials::{MaterialTweak, ModelMaterials};
use crate::pipeline::CompressPreset;
use crate::prepare::{Fixup, ModelPrep, PartsPlan, RealSize, SizeAxis, SizeUnit};
use crate::workspace::{MeshoptLevel, TextureCodec};

fn thirty_cm() -> RealSize {
    RealSize {
        axis: SizeAxis::Longest,
        value: 30.0,
        unit: SizeUnit::Cm,
    }
}

fn longest(side: &SwapSide) -> f32 {
    let (min, max) = (side.min.expect("min"), side.max.expect("max"));
    (0..3).map(|k| max[k] - min[k]).fold(0.0, f32::max)
}

fn action_of(report: &ReplaceReport, what: &str) -> Option<SwapAction> {
    report
        .settings
        .iter()
        .find(|s| s.what.starts_with(what))
        .map(|s| s.action)
}

fn tint(color: &str) -> ModelMaterials {
    ModelMaterials {
        all: MaterialTweak {
            tint: Some(color.into()),
            ..MaterialTweak::default()
        },
        materials: BTreeMap::new(),
    }
}

#[test]
fn the_real_size_carries_and_the_model_is_not_scaled_twice() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    // The scan is half a metre in its file; its real size is 30 cm.
    let prep = ModelPrep {
        size: Some(thirty_cm()),
        ..ModelPrep::default()
    };
    prepare::save_entry(&w.dir, &planner.id, &prep).expect("prep");

    let report = w.replace(&planner.id, &desk.id).report;

    // The old model was scaled down to its real size; the built one, which
    // its script builds 30 cm long, is not scaled again.
    assert!(
        (report.old.scale - 0.6).abs() < 1e-4,
        "{}",
        report.old.scale
    );
    assert!(
        (report.new.scale - 1.0).abs() < 1e-4,
        "{}",
        report.new.scale
    );
    assert!((longest(&report.old) - 0.3).abs() < 1e-4);
    assert!((longest(&report.new) - 0.3).abs() < 1e-4);
    assert_eq!(
        action_of(&report, "Real size, 30 cm"),
        Some(SwapAction::Carried)
    );
    // The entry still says 30 cm: a measure, never a factor.
    let saved = prepare::load(&w.dir).expect("prepare");
    assert_eq!(saved.models["spiral-planner"].size, Some(thirty_cm()));
    // What Build scene will do with it: the same frame the layout measures.
    let model = w.dir.join("models/spiral_planner_v2.glb");
    let bounds = crate::inbox::analyse::bounds(&model).expect("bounds");
    let frame = prepare::frame_of(saved.models.get("spiral-planner"), Some(bounds));
    assert!((frame.scale - 1.0).abs() < 1e-4, "{}", frame.scale);
    assert!(
        !report
            .warnings
            .iter()
            .any(|w| w.contains("scales the built model")),
        "{:?}",
        report.warnings
    );
}

#[test]
fn a_model_not_built_to_the_real_size_is_scaled_to_it_and_said_so() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    // A longer built model than the real size says: five boxes, 50 cm.
    w.add_model(&desk.id, &fixture::split_parts(5));
    w.mark_built(&desk.id);
    let prep = ModelPrep {
        size: Some(thirty_cm()),
        ..ModelPrep::default()
    };
    prepare::save_entry(&w.dir, &planner.id, &prep).expect("prep");

    let report = plan(&w.env(), &planner.id, &desk.id).expect("plan");

    assert!(
        (report.new.scale - 0.6).abs() < 1e-4,
        "{}",
        report.new.scale
    );
    assert!(
        (longest(&report.new) - 0.3).abs() < 1e-4,
        "it still stands 30 cm"
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("scales the built model by 0.6")),
        "{:?}",
        report.warnings
    );
}

#[test]
fn without_a_real_size_the_report_says_the_sizes_differ() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");

    let report = plan(&w.env(), &planner.id, &desk.id).expect("plan");

    assert!((longest(&report.old) - 0.5).abs() < 1e-4);
    assert!((longest(&report.new) - 0.3).abs() < 1e-4);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("No real size is set for Spiral planner")
                && w.contains("30 cm")
                && w.contains("50 cm")),
        "{:?}",
        report.warnings
    );
    // The two models sit around different middles, which shows too.
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("build it around the same origin")),
        "{:?}",
        report.warnings
    );
}

#[test]
fn what_repaired_the_scan_is_reset_and_the_built_model_bakes() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    let old = ModelPrep {
        size: Some(thirty_cm()),
        fixups: vec![
            Fixup::Decimate { triangles: 60_000 },
            Fixup::OriginBottom,
            Fixup::Rotate {
                x: 0.0,
                y: 90.0,
                z: 0.0,
            },
        ],
        skip_bake: Some(true),
        pad_edges: Some(true),
        parts: Some(PartsPlan {
            names: BTreeMap::from([("Crate".to_string(), "box".to_string())]),
            merge: Vec::new(),
            hidden: vec!["Crate".to_string()],
        }),
    };
    prepare::save_entry(&w.dir, &planner.id, &old).expect("prep");
    materials::save_looks(
        &w.dir,
        &BTreeMap::from([(planner.id.clone(), tint("#3a3936"))]),
    )
    .expect("looks");
    let stored = prepare::load(&w.dir).expect("prepare").models[&planner.id].clone();

    let report = w.replace(&planner.id, &desk.id).report;

    // Only the real size is left of the scan's entry.
    let saved = prepare::load(&w.dir).expect("prepare");
    assert_eq!(
        saved.models["spiral-planner"],
        ModelPrep {
            size: Some(thirty_cm()),
            ..ModelPrep::default()
        }
    );
    assert!(
        materials::load(&w.dir)
            .expect("materials")
            .models
            .is_empty(),
        "the tint that corrected the scan is off"
    );
    assert!(report.bakes, "built pieces bake");
    for what in [
        "Decimate to 60k",
        "Origin to bottom centre",
        "Rotate y 90",
        "Skips the bake",
        "Pads texture edges",
        "Part name Crate as box",
        "Hidden part Crate",
        "Material tweak for every material (tint #3a3936)",
    ] {
        assert_eq!(
            action_of(&report, what),
            Some(SwapAction::Reset),
            "{what}: {:?}",
            report.settings
        );
    }
    let origin = report
        .settings
        .iter()
        .find(|s| s.what == "Origin to bottom centre")
        .expect("origin line");
    assert!(
        origin.why.contains("build it around that point"),
        "{}",
        origin.why
    );
    // Everything that was reset is kept for the undo.
    let record = load(&w.dir).expect("built").assets["spiral-planner"]
        .replaced
        .clone()
        .expect("record");
    assert_eq!(record.prep, Some(stored));
    assert_eq!(record.materials, Some(tint("#3a3936")));
}

#[test]
fn the_built_assets_own_settings_come_along() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    prepare::save_entry(
        &w.dir,
        &planner.id,
        &ModelPrep {
            skip_bake: Some(true),
            ..ModelPrep::default()
        },
    )
    .expect("scan prep");
    let own = ModelPrep {
        size: Some(thirty_cm()),
        fixups: vec![Fixup::OriginBottom],
        skip_bake: Some(true),
        pad_edges: None,
        parts: Some(PartsPlan {
            names: BTreeMap::from([("Part_0".to_string(), "cover".to_string())]),
            merge: Vec::new(),
            hidden: Vec::new(),
        }),
    };
    prepare::save_entry(&w.dir, &desk.id, &own).expect("built prep");
    materials::save_looks(
        &w.dir,
        &BTreeMap::from([(desk.id.clone(), tint("#808080"))]),
    )
    .expect("looks");

    let report = w.replace(&planner.id, &desk.id).report;

    let saved = prepare::load(&w.dir).expect("prepare");
    assert_eq!(
        saved.models.len(),
        1,
        "the built asset's entry left with it"
    );
    assert_eq!(saved.models["spiral-planner"], own);
    assert_eq!(
        materials::load(&w.dir).expect("materials").models["spiral-planner"],
        tint("#808080")
    );
    assert!(!report.bakes, "the built asset chose to skip the bake");
    assert_eq!(report.parts, ["cover", "Part_1", "Part_2"]);
    for what in [
        "Real size, 30 cm",
        "Origin to bottom centre",
        "Skips the bake",
        "Part names and joins of the built asset",
        "Material tweak for every material (tint #808080)",
    ] {
        assert_eq!(
            action_of(&report, what),
            Some(SwapAction::Brought),
            "{what}: {:?}",
            report.settings
        );
    }
}

#[test]
fn part_names_that_still_resolve_are_carried() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    let old = ModelPrep {
        parts: Some(PartsPlan {
            names: BTreeMap::from([
                ("Part_1".to_string(), "spine".to_string()),
                ("Crate".to_string(), "box".to_string()),
            ]),
            merge: Vec::new(),
            hidden: Vec::new(),
        }),
        ..ModelPrep::default()
    };
    prepare::save_entry(&w.dir, &planner.id, &old).expect("prep");

    let report = w.replace(&planner.id, &desk.id).report;

    assert_eq!(report.parts, ["Part_0", "spine", "Part_2"]);
    assert_eq!(
        action_of(&report, "Part name Part_1 as spine"),
        Some(SwapAction::Carried)
    );
    assert_eq!(
        action_of(&report, "Part name Crate as box"),
        Some(SwapAction::Reset)
    );
}

#[test]
fn compression_presets_do_not_follow_to_the_new_file() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    let row = |model: &str| CompressPreset {
        model: model.into(),
        label: model.into(),
        texture_size: 1024,
        data_texture_size: None,
        codec: TextureCodec::Etc1s,
        map_codecs: None,
        meshopt: MeshoptLevel::High,
        lock: None,
    };
    let manifest = w.manifest();
    crate::pipeline::ensure(&w.dir, &manifest).expect("pipeline");
    crate::pipeline::update(&w.dir, &manifest, |def| {
        def.presets = vec![row("spiral_planner_v1"), row("planner_built_v1")];
        Ok(())
    })
    .expect("presets");
    let before = w.read("pipeline.json");

    let report = w.replace(&planner.id, &desk.id).report;

    assert_eq!(
        w.read("pipeline.json"),
        before,
        "the rows stay where they are"
    );
    for what in [
        "Compression preset of spiral_planner_v1 (1024 px, etc1s, meshopt high)",
        "Compression preset of planner_built_v1 (1024 px, etc1s, meshopt high)",
    ] {
        let line = report
            .settings
            .iter()
            .find(|s| s.what == what)
            .unwrap_or_else(|| panic!("{what}: {:?}", report.settings));
        assert_eq!(line.action, SwapAction::Reset);
        assert!(line.why.contains("spiral_planner_v2"), "{}", line.why);
    }
}
