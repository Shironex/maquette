//! The export starter in a real, headless Blender (skipped without one).

use super::EXPORT_WEB;
use crate::test_blender::{blender, glb_json, run};

#[test]
fn shared_materials_ship_with_clean_names_and_no_pipeline_tags() {
    let Some(blender) = blender() else { return };
    let dir = tempfile::tempdir().expect("tempdir");
    let make = dir.path().join("make.py");
    let script = dir.path().join("export_web.py");
    let blend = dir.path().join("scene.blend");
    let out = dir.path().join("raw");
    std::fs::write(&make, include_str!("../fixtures/shared_materials.py")).expect("fixture");
    std::fs::write(&script, EXPORT_WEB.text).expect("script");
    let path = |p: &std::path::Path| p.display().to_string();
    run(&blender, &["--python", &path(&make), "--", &path(&blend)]);
    run(
        &blender,
        &[
            &path(&blend),
            "--python",
            &path(&script),
            "--",
            "--bake",
            &path(&dir.path().join("no-bake")),
            "--out",
            &path(&out),
        ],
    );
    let laptop = glb_json(&out.join("laptop.glb"));
    let mut names: Vec<String> = laptop["materials"]
        .as_array()
        .expect("materials")
        .iter()
        .map(|m| m["name"].as_str().unwrap_or_default().to_string())
        .collect();
    names.sort();
    assert_eq!(names, ["palette", "palette_2", "palette_3", "screen"]);
    let desk = glb_json(&out.join("desk.glb"));
    assert_eq!(desk["materials"][0]["name"], "palette");
    for file in ["laptop.glb", "desk.glb"] {
        let text = glb_json(&out.join(file)).to_string();
        assert!(!text.contains("maquette"), "{file} names the tool: {text}");
    }
}

/// A rigged model keeps its skin and clips through Build scene and Export
/// GLB, except the clips the layout keeps off the site.
#[test]
fn clips_ship_with_their_model_unless_the_layout_keeps_them_off() {
    let Some(blender) = blender() else { return };
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let path = |p: &std::path::Path| p.display().to_string();
    let make = root.join("motion_models.py");
    std::fs::write(
        &make,
        include_str!("../../motion/fixtures/motion_models.py"),
    )
    .expect("fixture");
    std::fs::create_dir_all(root.join("models")).expect("models");
    std::fs::create_dir_all(root.join("tools")).expect("tools");
    for (which, file) in [
        ("figure", "figure_v1.glb"),
        ("turntable", "turntable_v1.glb"),
    ] {
        let out = root.join("models").join(file);
        run(
            &blender,
            &["--python", &path(&make), "--", which, &path(&out)],
        );
    }
    std::fs::write(
        root.join("project.json"),
        serde_json::json!({ "assets": [
            { "id": "figure", "versions": [{ "file": "models/figure_v1.glb" }] },
            { "id": "turntable", "versions": [{ "file": "models/turntable_v1.glb" }] }
        ] })
        .to_string(),
    )
    .expect("project");
    std::fs::write(
        root.join("layout.json"),
        r#"{ "version": 1, "models": {}, "stops": [], "desk": null,
             "states": {}, "clips": { "figure": { "Wave": { "ship": false } } } }"#,
    )
    .expect("layout");
    let build = root.join("tools/build_scene.py");
    let export = root.join("tools/export_web.py");
    std::fs::write(&build, super::BUILD_SCENE.text).expect("build");
    std::fs::write(&export, EXPORT_WEB.text).expect("export");
    let built = root.join("blender/scene_built.blend");
    let log = run(
        &blender,
        &[
            "--python",
            &path(&build),
            "--",
            "--base",
            &path(&root.join("blender/scene.blend")),
            "--models",
            &path(&root.join("models")),
            "--out",
            &path(&built),
            "--layout",
            &path(&root.join("layout.json")),
            "--project",
            &path(&root.join("project.json")),
            "--prepare",
            &path(&root.join("prepare.json")),
        ],
    );
    assert!(
        log.contains("figure_v1: left 1 clips off the site"),
        "{log}"
    );
    let out = root.join("raw");
    let log = run(
        &blender,
        &[
            &path(&built),
            "--python",
            &path(&export),
            "--",
            "--bake",
            &path(&root.join("no-bake")),
            "--out",
            &path(&out),
        ],
    );
    assert!(log.contains("2 clips"), "{log}");
    let figure = glb_json(&out.join("figure_v1.glb"));
    let mut names: Vec<&str> = figure["animations"]
        .as_array()
        .expect("animations")
        .iter()
        .filter_map(|a| a["name"].as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["Idle breathe", "Walk loop"]);
    assert_eq!(figure["skins"].as_array().map(Vec::len), Some(1));
    let turntable = glb_json(&out.join("turntable_v1.glb"));
    assert_eq!(turntable["animations"][0]["name"], "Spin");
}
