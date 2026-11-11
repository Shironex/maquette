//! Tests for a model's facts: what is read, where it comes from and how
//! the facing is said.

use std::fs;
use std::path::Path;

use super::facing::{blender_axis, front, placed};
use super::*;
use crate::assets::{Asset, AssetKind, AssetStatus, AssetVersion, Stage};
use crate::inbox::fixture::{self, HumanoidSpec};
use crate::inbox::vrm::Axis;
use crate::layout::Placement;

fn version(n: u32, file: &str) -> AssetVersion {
    AssetVersion {
        version: n,
        file: file.into(),
        created_at: "2026-11-10T09:30:00.000Z".into(),
        stats: None,
    }
}

fn asset(id: &str, versions: Vec<AssetVersion>) -> Asset {
    Asset {
        id: id.into(),
        name: id.replace('-', " "),
        kind: AssetKind::Prop,
        stage: Stage::Blender,
        status: AssetStatus::Review,
        versions,
        thumbnail: None,
        created_at: "2026-11-10T09:30:00.000Z".into(),
        updated_at: "2026-11-10T09:30:00.000Z".into(),
        origin: None,
        refs: Vec::new(),
    }
}

fn write(dir: &Path, name: &str, value: &serde_json::Value) {
    fs::write(
        dir.join(name),
        serde_json::to_vec_pretty(value).expect("json"),
    )
    .expect("write");
}

fn model(dir: &Path, name: &str, bytes: &[u8]) {
    fs::create_dir_all(dir.join("models")).expect("models");
    fs::write(dir.join("models").join(name), bytes).expect("model");
}

fn read(dir: &Path, asset: &Asset, n: usize, refs: &[AssetRef]) -> ModelFacts {
    facts(&FactsSource {
        project: "frostveil",
        project_dir: dir,
        asset,
        version: &asset.versions[n],
        previews: None,
        refs,
    })
    .expect("facts")
}

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < 1e-3)
}

#[test]
fn a_placed_prop_says_its_counts_its_real_size_and_its_layout_entry() {
    let dir = tempfile::tempdir().expect("dir");
    let dir = dir.path();
    model(dir, "signpost_v1.glb", &fixture::signpost());
    write(
        dir,
        "prepare.json",
        &serde_json::json!({
            "version": 1,
            "models": { "signpost": {
                "size": { "axis": "height", "value": 18, "unit": "cm" },
                "fixups": [{ "op": "origin-bottom" }],
            } },
        }),
    );
    write(
        dir,
        "layout.json",
        &serde_json::json!({
            "version": 1,
            "models": { "signpost": { "position": [0.4, 0.0, -0.1], "rotation": [0, 90, 0], "scale": 2.0 } },
            "stops": [],
        }),
    );
    let signpost = asset("signpost", vec![version(1, "models/signpost_v1.glb")]);
    let refs = [AssetRef {
        file: "refs/signpost.png".into(),
        view: RefView::Front,
    }];
    let found = read(dir, &signpost, 0, &refs);

    // The counts and the file's own box are the Inbox analysis'.
    assert_eq!(
        (
            found.counts.triangles,
            found.counts.meshes,
            found.counts.objects
        ),
        (48, 4, 4)
    );
    assert_eq!(found.counts.materials, 4);
    assert!(near(found.in_file.min, [-0.2, 0.0, -0.1]));
    assert!(near(found.in_file.size, [0.46, 0.36, 0.26]));
    // 0.36 m high in the file, 18 cm in the world: half size, bottom centre at the origin.
    assert_eq!(found.real_size.text.as_deref(), Some("18 cm high"));
    assert!((found.real_size.scale - 0.5).abs() < 1e-4);
    assert!(near(found.real_size.offset, [-0.03, 0.0, -0.03]));
    let real = found.at_real_size.expect("measured");
    assert!(near(real.size, [0.23, 0.18, 0.13]), "{:?}", real.size);
    assert!(near(real.min, [-0.115, 0.0, -0.065]), "{:?}", real.min);
    // The entry as layout.json has it, and the box it fills in the scene:
    // turned a quarter, width and depth swap, then twice the size.
    assert!(found.layout.placed);
    let entry = found.layout.entry.expect("entry");
    assert_eq!(
        (entry.position, entry.rotation, entry.scale),
        ([0.4, 0.0, -0.1], [0.0, 90.0, 0.0], 2.0)
    );
    let scene = found.layout.in_scene.expect("in the scene");
    assert!(near(scene.size, [0.26, 0.36, 0.46]), "{:?}", scene.size);
    // The ref comes with its whole path and its view.
    assert_eq!(found.refs.len(), 1);
    assert_eq!(found.refs[0].view, RefView::Front);
    assert!(Path::new(&found.refs[0].file).ends_with("refs/signpost.png"));
    assert!(Path::new(&found.refs[0].file).is_absolute());
    assert!(Path::new(&found.file).ends_with("models/signpost_v1.glb"));
    assert!(
        found.notes.iter().any(|n| n.contains("scales it by 0.5")),
        "{:?}",
        found.notes
    );
    assert!(
        found
            .notes
            .iter()
            .any(|n| n.contains("scales it by 2 on top")),
        "{:?}",
        found.notes
    );
    assert!(found.axes.contains("front is +z"));
}

#[test]
fn a_props_front_is_the_formats_and_the_answer_says_it_was_not_measured() {
    let dir = tempfile::tempdir().expect("dir");
    let dir = dir.path();
    model(dir, "crate_v1.glb", &fixture::clean_prop());
    let scan = asset("crate", vec![version(1, "models/crate_v1.glb")]);
    let found = read(dir, &scan, 0, &[]);
    assert_eq!(found.facing.in_file.axis, Axis::PlusZ);
    assert_eq!(found.facing.in_file.blender, "-y");
    assert_eq!(
        (found.facing.how, found.facing.sure),
        (FacingHow::Convention, Sure::Assumed)
    );
    assert!(
        found.facing.note.starts_with("Not measured"),
        "{}",
        found.facing.note
    );
    assert!(found.facing.note.contains("front picture"));
    assert_eq!(found.facing.in_scene, None);
    // Nothing is placed, sized or built, and the answer says each.
    assert!(!found.built && !found.layout.placed);
    assert_eq!(found.real_size.scale, 1.0);
    assert_eq!(found.at_real_size.map(|b| b.size), Some(found.in_file.size));
    assert_eq!(found.counts.objects, 1);
    for said in [
        "no real size",
        "one mesh",
        "No layout entry",
        "no ref image",
    ] {
        assert!(
            found.notes.iter().any(|n| n.contains(said)),
            "{said}: {:?}",
            found.notes
        );
    }

    // A built asset's note names the script as what decides.
    write(
        dir,
        "built.json",
        &serde_json::json!({
            "version": 1,
            "assets": { "crate": { "script": "built/crate.py", "helpers": [], "root": null, "last": null } },
        }),
    );
    let built = read(dir, &scan, 0, &[]);
    assert!(built.built);
    assert!(
        built
            .facing
            .note
            .contains("built script decides its own front")
    );
    assert_eq!(built.facing.sure, Sure::Assumed);
}

#[test]
fn a_rigged_models_front_is_read_from_its_skeleton() {
    let dir = tempfile::tempdir().expect("dir");
    let dir = dir.path();
    // A character built facing +X: a quarter turn from glTF's front.
    let turned = HumanoidSpec {
        yaw: 90.0,
        ..HumanoidSpec::default()
    };
    model(dir, "warden_v1.glb", &fixture::humanoid(&turned));
    model(
        dir,
        "warden_v2.glb",
        &fixture::humanoid(&HumanoidSpec::default()),
    );
    let warden = asset(
        "warden",
        vec![
            version(1, "models/warden_v1.glb"),
            version(2, "models/warden_v2.glb"),
        ],
    );
    write(
        dir,
        "layout.json",
        &serde_json::json!({
            "version": 1,
            "models": { "warden": { "position": [0, 0, 0], "rotation": [0, 90, 0], "scale": 1.0 } },
            "stops": [],
        }),
    );
    let old = read(dir, &warden, 0, &[]);
    assert_eq!(old.facing.in_file.axis, Axis::PlusX);
    assert!((old.facing.in_file.degrees - 90.0).abs() < 0.5);
    assert_eq!(old.facing.in_file.blender, "+x");
    assert_eq!(
        (old.facing.how, old.facing.sure),
        (FacingHow::Rig, Sure::Measured)
    );
    // The layout turns it a quarter more: it looks along -Z in the scene.
    assert_eq!(old.facing.in_scene.map(|f| f.axis), Some(Axis::MinusZ));
    assert!(old.notes.iter().any(|n| n.contains("not the newest (v2)")));

    let new = read(dir, &warden, 1, &[]);
    assert_eq!(new.facing.in_file.axis, Axis::PlusZ);
    assert_eq!(new.facing.in_scene.map(|f| f.axis), Some(Axis::PlusX));
    assert!(new.counts.joints > 10);
}

#[test]
fn the_axes_of_every_layer_agree_on_a_turn() {
    // glTF and three.js: +Z is the front. Blender: -Y. X stays X.
    for (axis, blender) in [
        (Axis::PlusZ, "-y"),
        (Axis::MinusZ, "+y"),
        (Axis::PlusX, "+x"),
        (Axis::MinusX, "-x"),
    ] {
        assert_eq!(blender_axis(axis), blender);
    }
    assert_eq!(front(0.0).axis, Axis::PlusZ);
    assert_eq!(front(90.0).axis, Axis::PlusX);
    assert_eq!(front(-90.0).axis, Axis::MinusX);
    assert_eq!(front(180.0).axis, Axis::MinusZ);
    assert_eq!(front(270.0).degrees, -90.0);
    assert_eq!(front(-180.0).degrees, 180.0);
    // A layout entry's Y rotation turns the front the same way: +Z towards +X.
    let turn = |y: f32| Placement {
        rotation: [0.0, y, 0.0],
        scale: 0.4,
        ..Placement::default()
    };
    assert_eq!(placed(0.0, &turn(90.0)).axis, Axis::PlusX);
    assert!((placed(0.0, &turn(35.0)).degrees - 35.0).abs() < 1e-3);
    assert_eq!(placed(90.0, &turn(-90.0)).axis, Axis::PlusZ);
    assert_eq!(placed(0.0, &turn(200.0)).axis, Axis::MinusZ);
    // A roll about Z turns nothing on the ground: the front stays where it was.
    let tipped = Placement {
        rotation: [0.0, 0.0, 90.0],
        ..Placement::default()
    };
    assert_eq!(placed(0.0, &tipped).axis, Axis::PlusZ);
}

#[test]
fn a_missing_model_or_an_unreadable_layout_is_said_plainly() {
    let dir = tempfile::tempdir().expect("dir");
    let dir = dir.path();
    let gone = asset("lamp", vec![version(1, "models/lamp_v1.glb")]);
    let source = FactsSource {
        project: "frostveil",
        project_dir: dir,
        asset: &gone,
        version: &gone.versions[0],
        previews: None,
        refs: &[],
    };
    assert_eq!(facts(&source).expect_err("gone").code(), "not-found");
    model(dir, "lamp_v1.glb", &fixture::clean_prop());
    fs::write(dir.join("layout.json"), b"{ not json").expect("layout");
    assert_eq!(facts(&source).expect_err("layout").code(), "json");
}
