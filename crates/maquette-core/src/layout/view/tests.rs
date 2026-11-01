//! Tests for the layout's read model: models measured at their prepared
//! size, the row of unplaced ones and what the first stop sees.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::*;
use crate::assets::{Asset, AssetKind, AssetStatus, AssetVersion, Stage};
use crate::inbox::fixture;
use crate::pipeline::template::for_template;
use crate::project::{PROJECT_VERSION, ProjectManifest, ProjectTemplate, slugify};

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
        origin: None,
        refs: Vec::new(),
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
        target: Default::default(),
        game: None,
        site_repo: None,
        site_repo_name: None,
        site_repo_state: Default::default(),
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
    let found = models(dir.path(), &manifest, &def, &layout, None);
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
    let found = models(dir.path(), &manifest, &def, &layout, None);
    assert_eq!(by(&found, "plinth").fallback, [0.0; 3]);
}

#[test]
fn models_are_measured_at_their_real_size_after_the_fixups() {
    use crate::prepare::{Fixup, ModelPrep, RealSize, SizeAxis, SizeUnit, save_entry};
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = project(dir.path());
    let def = for_template(ProjectTemplate::FloatingDiorama);
    // The crate is half a metre high; it stands for a box a metre high.
    let entry = ModelPrep {
        skip_bake: None,
        pad_edges: None,
        parts: None,
        size: Some(RealSize {
            axis: SizeAxis::Height,
            value: 100.0,
            unit: SizeUnit::Cm,
        }),
        fixups: vec![Fixup::OriginBottom],
    };
    save_entry(dir.path(), "lamp", &entry).expect("prepare");
    let found = models(dir.path(), &manifest, &def, &SceneLayout::default(), None);
    let lamp = found.iter().find(|m| m.asset == "lamp").expect("lamp");
    assert!((lamp.prep.scale - 2.0).abs() < 1e-5, "{:?}", lamp.prep);
    assert!(close(lamp.min.expect("min"), [-0.5, 0.0, -0.5]));
    assert!(close(lamp.max.expect("max"), [0.5, 1.0, 0.5]));
    let bowl = found.iter().find(|m| m.asset == "bowl").expect("bowl");
    assert!(bowl.prep.is_identity(), "no entry, as it is");

    // Once a preview ran, the bounds its fix-ups left are the ones scaled.
    let previews = dir.path().join("previews");
    let file = dir.path().join("models/lamp_v1.glb");
    let script = crate::prepare::preview::script_for(Some(dir.path()));
    let key = crate::prepare::preview::key(
        &crate::pipeline::cache::hash_file(&file).expect("hash"),
        &ModelPrep {
            skip_bake: None,
            pad_edges: None,
            parts: None,
            size: None,
            fixups: entry.fixups.clone(),
        },
        &script,
    );
    fs::create_dir_all(previews.join("bounds")).expect("bounds");
    fs::write(
        previews.join("bounds").join(format!("{key}.json")),
        r#"{ "triangles": 12, "objects": 1, "maxTexture": 64, "min": [-0.25, 0, -0.25], "max": [0.25, 0.25, 0.25] }"#,
    )
    .expect("fixed");
    let found = models(
        dir.path(),
        &manifest,
        &def,
        &SceneLayout::default(),
        Some(&previews),
    );
    let lamp = found.iter().find(|m| m.asset == "lamp").expect("lamp");
    assert!((lamp.prep.scale - 4.0).abs() < 1e-5, "{:?}", lamp.prep);
    assert!(close(lamp.max.expect("max"), [1.0, 1.0, 1.0]));
}

#[test]
fn the_first_view_counts_what_its_camera_sees_or_its_own_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = project(dir.path());
    let def = for_template(ProjectTemplate::FloatingDiorama);
    let mut layout = SceneLayout::default();
    let none = first_view_of(dir.path(), &manifest, &def, 3.0, None);
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
    update(dir.path(), |_| Ok(layout.clone())).expect("save");
    let first = first_view_of(dir.path(), &manifest, &def, 3.0, None);
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
    update(dir.path(), |_| Ok(layout.clone())).expect("save");
    let custom = first_view_of(dir.path(), &manifest, &def, 3.0, None);
    assert!(custom.custom);
    assert_eq!(custom.models.len(), 1);
    assert_eq!(custom.models[0].asset, "bowl");

    let full = view(dir.path(), &manifest, &def, 3.0, None).expect("view");
    assert!(full.saved);
    assert_eq!(full.first_view, custom);
    assert_eq!(full.models.len(), 3);
}
