use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::json;

use super::placement::transform;
use super::*;
use crate::assets::{AssetStatus, AssetVersion, Stage};
use crate::inbox::fixture;
use crate::pipeline::template::for_template;
use crate::project::{PROJECT_VERSION, ProjectTemplate};

fn stop(name: &str, position: [f32; 3], target: [f32; 3]) -> LayoutStop {
    LayoutStop {
        id: slugify(name),
        name: name.to_string(),
        position,
        target,
        fov: 35.0,
        models: None,
    }
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < 1e-4)
}

#[test]
fn the_file_round_trips_in_camel_case_and_a_missing_one_is_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(load(dir.path()).expect("missing"), SceneLayout::default());
    let mut layout = SceneLayout::default();
    layout.models.insert(
        "desk-lamp".into(),
        Placement {
            position: [0.4, 0.0, -0.1],
            rotation: [0.0, -35.0, 0.0],
            scale: 0.4,
        },
    );
    layout
        .stops
        .push(stop("Concept", [0.9, 0.5, 1.4], [0.0, 0.15, 0.0]));
    layout.desk = Some(Desk { size: [1.6, 0.8] });
    save(dir.path(), &layout).expect("save");
    let text = fs::read_to_string(path(dir.path())).expect("read");
    let value: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(value["version"], json!(1));
    assert_eq!(value["models"]["desk-lamp"]["scale"], json!(0.4));
    assert_eq!(value["stops"][0]["id"], json!("concept"));
    assert_eq!(value["stops"][0]["models"], json!(null));
    assert_eq!(value["desk"]["size"][0], json!(1.6));
    assert_eq!(load(dir.path()).expect("load"), layout);

    fs::write(
        path(dir.path()),
        r#"{ "version": 2, "models": {}, "stops": [], "desk": null }"#,
    )
    .expect("newer");
    assert_eq!(
        load(dir.path()).expect_err("newer").code(),
        "unsupported-version"
    );
}

#[test]
fn validation_refuses_what_blender_could_not_place() {
    let ok = SceneLayout {
        stops: vec![stop("Concept", [0.0, 1.0, 2.0], [0.0, 0.0, 0.0])],
        ..SceneLayout::default()
    };
    assert!(validate(&ok).is_ok());
    let refused = |change: &dyn Fn(&mut SceneLayout), needle: &str| {
        let mut layout = ok.clone();
        change(&mut layout);
        let err = validate(&layout).expect_err(needle).to_string();
        assert!(err.contains(needle), "{err}");
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(save(dir.path(), &layout).is_err());
        assert!(
            !path(dir.path()).exists(),
            "a refused layout writes nothing"
        );
    };
    refused(
        &|l| {
            l.models.insert(
                "a".into(),
                Placement {
                    scale: 0.0,
                    ..Placement::default()
                },
            );
        },
        "scale",
    );
    refused(
        &|l| {
            l.models.insert(
                "a".into(),
                Placement {
                    position: [f32::NAN, 0.0, 0.0],
                    ..Placement::default()
                },
            );
        },
        "not a number",
    );
    refused(&|l| l.stops[0].name = "  ".into(), "name");
    refused(&|l| l.stops[0].target = l.stops[0].position, "looks at");
    refused(&|l| l.stops[0].fov = 175.0, "field of view");
    refused(&|l| l.stops[0].id = "Not A Slug".into(), "id");
    refused(&|l| l.stops.push(l.stops[0].clone()), "share the id");
    refused(
        &|l| l.stops[0].models = Some(vec!["a".into(), "a".into()]),
        "twice",
    );
    refused(&|l| l.desk = Some(Desk { size: [0.0, 1.0] }), "desk");
}

#[test]
fn patches_change_only_what_they_name() {
    let known = vec!["lamp".to_string(), "bowl".to_string()];
    let mut layout = SceneLayout::default();
    layout.models.insert(
        "lamp".into(),
        Placement {
            position: [1.0, 0.0, 0.0],
            rotation: [0.0, 90.0, 0.0],
            scale: 0.5,
        },
    );
    layout
        .stops
        .push(stop("Concept", [0.0, 1.0, 2.0], [0.0, 0.0, 0.0]));
    let patch: LayoutPatch = serde_json::from_value(json!({
        "models": { "lamp": { "scale": 0.25 }, "bowl": { "position": [0.2, 0, 0.1] } },
        "stops": [
            { "id": "concept", "fov": 28 },
            { "name": "Refs", "position": [1, 1, 1], "target": [0, 0, 0] },
            { "name": "Concept", "position": [2, 1, 1], "target": [0, 0, 0], "models": ["bowl"] }
        ],
        "desk": { "size": [1.2, 0.6] }
    }))
    .expect("patch");
    let next = apply(&layout, &patch, &known).expect("apply");
    let lamp = next.models["lamp"];
    assert!((lamp.scale - 0.25).abs() < 1e-6);
    assert_eq!(lamp.rotation, [0.0, 90.0, 0.0], "kept");
    assert_eq!(
        next.models["bowl"].scale, 1.0,
        "a new placement starts at scale 1"
    );
    let ids: Vec<&str> = next.stops.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        ["concept", "refs", "concept-2"],
        "a taken id gets a number"
    );
    assert!((next.stops[0].fov - 28.0).abs() < 1e-6);
    assert!((next.stops[1].fov - DEFAULT_FOV).abs() < 1e-6);
    assert_eq!(next.stops[2].models, Some(vec!["bowl".to_string()]));
    assert_eq!(next.desk, Some(Desk { size: [1.2, 0.6] }));

    let patch: LayoutPatch = serde_json::from_value(json!({
        "models": { "lamp": null },
        "stops": [{ "id": "refs", "remove": true }, { "id": "concept-2", "models": null }],
        "stopOrder": ["concept-2", "concept"],
        "desk": null
    }))
    .expect("patch");
    let after = apply(&next, &patch, &known).expect("apply");
    assert!(
        !after.models.contains_key("lamp"),
        "null drops the placement"
    );
    let ids: Vec<&str> = after.stops.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["concept-2", "concept"]);
    assert_eq!(after.stops[0].models, None, "null measures the view again");
    assert_eq!(after.desk, None);

    for (patch, needle) in [
        (
            json!({ "models": { "ghost": { "scale": 2 } } }),
            "lamp, bowl",
        ),
        (json!({ "models": { "lamp": { "scale": -1 } } }), "scale"),
        (
            json!({ "stops": [{ "id": "nope", "remove": true }] }),
            "no camera stop",
        ),
        (
            json!({ "stops": [{ "name": "Ship" }] }),
            "position and a target",
        ),
        (
            json!({ "stops": [{ "position": [1, 1, 1], "target": [0, 0, 0] }] }),
            "name",
        ),
        (
            json!({ "stops": [{ "id": "concept", "models": ["ghost"] }] }),
            "ghost",
        ),
        (
            json!({ "stopOrder": ["concept", "concept"] }),
            "every stop once",
        ),
    ] {
        let patch: LayoutPatch = serde_json::from_value(patch.clone()).expect("patch");
        let err = apply(&layout, &patch, &known)
            .expect_err(needle)
            .to_string();
        assert!(err.contains(needle), "{err}");
    }
}

#[test]
fn placements_compose_like_three() {
    // A quarter turn about Y takes +X to -Z, as three.js turns an object.
    let turn = Placement {
        rotation: [0.0, 90.0, 0.0],
        ..Placement::default()
    };
    assert!(close(
        transform(&matrix(&turn), [1.0, 0.0, 0.0]),
        [0.0, 0.0, -1.0]
    ));
    // X then Y then Z: Rx * Ry * Rz applied to the point.
    let mixed = Placement {
        position: [1.0, 2.0, 3.0],
        rotation: [90.0, 0.0, 90.0],
        scale: 2.0,
    };
    // Rz takes +X to +Y, then Rx takes +Y to +Z; scaled by 2 and moved.
    assert!(close(
        transform(&matrix(&mixed), [1.0, 0.0, 0.0]),
        [1.0, 2.0, 5.0]
    ));
    let (min, max) = placed_bounds(([-0.5, 0.0, -0.25], [0.5, 1.0, 0.25]), &turn);
    assert!(close(min, [-0.25, 0.0, -0.5]) && close(max, [0.25, 1.0, 0.5]));
}

#[test]
fn unplaced_models_go_into_a_row_only_when_they_overlap() {
    let unit = ([-0.5, 0.0, -0.5], [0.5, 1.0, 0.5]);
    let wide = ([-1.0, 0.0, -0.5], [1.0, 1.0, 0.5]);
    let row = fallback_row(&[("a".into(), unit), ("b".into(), wide)]);
    // Widths 1 and 2, a gap of a quarter of the wide box's diagonal.
    let gap = 0.25 * (4.0f32 + 1.0 + 1.0).sqrt();
    let start = -(3.0 + gap) / 2.0;
    assert!(close(row["a"], [start + 0.5, 0.0, 0.0]));
    assert!(close(row["b"], [start + 1.0 + gap + 1.0, 0.0, 0.0]));
    let apart = ([4.0, 0.0, 0.0], [5.0, 1.0, 1.0]);
    let row = fallback_row(&[("a".into(), unit), ("b".into(), apart)]);
    assert_eq!(row["a"], [0.0; 3]);
    assert_eq!(row["b"], [0.0; 3]);
    assert_eq!(fallback_row(&[("a".into(), unit)])["a"], [0.0; 3]);
}

#[test]
fn the_view_holds_what_is_in_front_and_inside_the_frame() {
    let camera = stop("Concept", [0.0, 1.0, 5.0], [0.0, 1.0, 0.0]);
    let at = |x: f32, y: f32, z: f32| ([x - 0.1, y - 0.1, z - 0.1], [x + 0.1, y + 0.1, z + 0.1]);
    assert!(in_view(&camera, at(0.0, 1.0, 0.0)));
    assert!(!in_view(&camera, at(0.0, 1.0, 8.0)), "behind the camera");
    // At 5 m away a 35 degree view is about 1.58 m tall and 2.8 m wide.
    assert!(in_view(&camera, at(2.5, 1.0, 0.0)));
    assert!(!in_view(&camera, at(3.5, 1.0, 0.0)), "off to the side");
    assert!(!in_view(&camera, at(0.0, 3.0, 0.0)), "above the frame");
    // A big box around the camera counts.
    assert!(in_view(&camera, ([-9.0, -9.0, -9.0], [9.0, 9.0, 9.0])));
    // Looking straight down still works.
    let down = stop("Top", [0.0, 5.0, 0.0], [0.0, 0.0, 0.0]);
    assert!(in_view(&down, at(0.0, 0.0, 0.0)));
}

fn asset(id: &str, file: &str) -> Asset {
    Asset {
        id: id.into(),
        name: id.replace('-', " "),
        kind: AssetKind::Prop,
        stage: Stage::Blender,
        status: AssetStatus::Review,
        versions: vec![AssetVersion {
            version: 1,
            file: file.into(),
            created_at: "2026-10-24T09:30:00.000Z".into(),
            stats: None,
        }],
        thumbnail: None,
        created_at: "2026-10-24T09:30:00.000Z".into(),
        updated_at: "2026-10-24T09:30:00.000Z".into(),
    }
}

fn project(dir: &Path) -> ProjectManifest {
    let models = dir.join("models");
    fs::create_dir_all(&models).expect("models");
    for name in ["lamp_v1.glb", "bowl_v1.glb", "plinth_v1.glb"] {
        fs::write(models.join(name), fixture::clean_prop()).expect("model");
    }
    ProjectManifest {
        version: PROJECT_VERSION,
        id: "maquette".into(),
        name: "Maquette".into(),
        template: ProjectTemplate::FloatingDiorama,
        site_repo: None,
        source_folder: None,
        created_at: "2026-10-24T09:30:00.000Z".into(),
        assets: vec![
            asset("lamp", "models/lamp_v1.glb"),
            asset("plinth", "models/plinth_v1.glb"),
            asset("bowl", "models/bowl_v1.glb"),
            Asset {
                versions: Vec::new(),
                ..asset("concept-only", "")
            },
        ],
    }
}

#[test]
fn models_are_measured_and_the_unplaced_ones_get_their_row() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = project(dir.path());
    let def = for_template(ProjectTemplate::FloatingDiorama);
    fs::create_dir_all(dir.path().join("web/raw")).expect("raw");
    fs::create_dir_all(dir.path().join("web/dist")).expect("dist");
    fs::write(dir.path().join("web/raw/lamp_v1.glb"), vec![0u8; 900]).expect("raw");
    fs::write(dir.path().join("web/dist/lamp_v1.glb"), vec![0u8; 300]).expect("dist");
    fs::write(dir.path().join("web/raw/bowl_v1.glb"), vec![0u8; 700]).expect("raw");

    let mut layout = SceneLayout::default();
    let found = models(dir.path(), &manifest, &def, &layout);
    let ids: Vec<&str> = found.iter().map(|m| m.asset.as_str()).collect();
    assert_eq!(
        ids,
        ["bowl", "lamp", "plinth"],
        "by file name, no concept-only"
    );
    assert_eq!(known_assets(&manifest), ["bowl", "lamp", "plinth"]);
    let by =
        |list: &[LayoutModel], id: &str| list.iter().find(|m| m.asset == id).cloned().expect(id);
    assert_eq!(
        (by(&found, "lamp").bytes, by(&found, "lamp").measured),
        (300.0, Measured::Shipped)
    );
    assert_eq!(
        (by(&found, "bowl").bytes, by(&found, "bowl").measured),
        (700.0, Measured::Raw)
    );
    assert_eq!(by(&found, "plinth").measured, Measured::Source);
    assert!(close(
        by(&found, "lamp").min.expect("min"),
        [-0.25, 0.0, -0.25]
    ));
    // Three crates at the origin overlap, so they stand in a row.
    let xs: Vec<f32> = found.iter().map(|m| m.fallback[0]).collect();
    assert!(xs[0] < xs[1] && xs[1] < xs[2], "{xs:?}");
    assert!(xs[1].abs() < 1e-4, "the row is centred");

    // Placing two leaves the third alone at its origin.
    for (id, x) in [("lamp", 2.0), ("bowl", -2.0)] {
        layout.models.insert(
            id.into(),
            Placement {
                position: [x, 0.0, 0.0],
                ..Placement::default()
            },
        );
    }
    let found = models(dir.path(), &manifest, &def, &layout);
    assert_eq!(by(&found, "plinth").fallback, [0.0; 3]);
}

#[test]
fn the_first_view_counts_what_its_camera_sees_or_its_own_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = project(dir.path());
    let def = for_template(ProjectTemplate::FloatingDiorama);
    let mut layout = SceneLayout::default();
    let none = first_view_of(dir.path(), &manifest, &def, 3.0);
    assert_eq!((none.stop, none.bytes), (None, 0.0));

    let spots = BTreeMap::from([("lamp", -0.6), ("plinth", 0.0), ("bowl", 6.0)]);
    for (id, x) in &spots {
        layout.models.insert(
            (*id).into(),
            Placement {
                position: [*x, 0.0, 0.0],
                ..Placement::default()
            },
        );
    }
    layout
        .stops
        .push(stop("Concept", [0.0, 0.6, 2.5], [0.0, 0.25, 0.0]));
    layout
        .stops
        .push(stop("Ship", [6.0, 0.6, 2.5], [6.0, 0.25, 0.0]));
    save(dir.path(), &layout).expect("save");
    let first = first_view_of(dir.path(), &manifest, &def, 3.0);
    assert_eq!(first.stop.as_deref(), Some("Concept"));
    assert!(!first.custom);
    let seen: Vec<&str> = first.models.iter().map(|m| m.asset.as_str()).collect();
    assert_eq!(seen, ["lamp", "plinth"], "the bowl is six metres off");
    let source = fs::metadata(dir.path().join("models/lamp_v1.glb"))
        .expect("size")
        .len() as f64;
    assert!((first.bytes - 2.0 * source).abs() < 1.0);
    assert!((first.target_mb - 3.0).abs() < f64::EPSILON);

    layout.stops[0].models = Some(vec!["bowl".into()]);
    save(dir.path(), &layout).expect("save");
    let custom = first_view_of(dir.path(), &manifest, &def, 3.0);
    assert!(custom.custom);
    assert_eq!(custom.models.len(), 1);
    assert_eq!(custom.models[0].asset, "bowl");

    let full = view(dir.path(), &manifest, &def, 3.0).expect("view");
    assert!(full.saved);
    assert_eq!(full.first_view, custom);
    assert_eq!(full.models.len(), 3);
}
