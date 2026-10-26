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
