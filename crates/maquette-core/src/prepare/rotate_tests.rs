//! The rotate fix-up in a real, headless Blender (skipped without one): a
//! post that leans stands up straight, its turn baked into the mesh.

use std::path::Path;

use crate::test_blender::{blender, run};

/// A post 0.2 m square and 1 m tall (glTF y), leaning 24.5 degrees about x.
const MAKE_POST: &str = r#"
import math, sys
import bpy
out = sys.argv[sys.argv.index("--") + 1]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.mesh.primitive_cube_add(size=1.0)
post = bpy.context.active_object
post.scale = (0.2, 0.2, 1.0)
bpy.ops.object.transform_apply(scale=True)
post.rotation_euler = (math.radians(24.5), 0.0, 0.0)
bpy.ops.export_scene.gltf(filepath=out, export_format="GLB")
"#;

fn fixed_bounds(blender: &Path, dir: &Path, fixups: serde_json::Value) -> serde_json::Value {
    let path = |p: &Path| p.display().to_string();
    let maker = dir.join("make_post.py");
    let script = dir.join("fixups.py");
    let model = dir.join("post.glb");
    if !model.is_file() {
        std::fs::write(&maker, MAKE_POST).expect("maker");
        std::fs::write(&script, super::SCRIPT).expect("script");
        run(blender, &["--python", &path(&maker), "--", &path(&model)]);
    }
    let plan = dir.join("plan.json");
    let stats = dir.join("stats.json");
    std::fs::write(&plan, serde_json::json!({ "fixups": fixups }).to_string()).expect("plan");
    let log = run(
        blender,
        &[
            "--python",
            &path(&script),
            "--",
            "--input",
            &path(&model),
            "--plan",
            &path(&plan),
            "--output",
            &path(&dir.join("out.glb")),
            "--stats",
            &path(&stats),
        ],
    );
    let text = std::fs::read_to_string(&stats).unwrap_or_else(|_| panic!("stats: {log}"));
    serde_json::from_str(&text).expect("stats json")
}

fn extent(stats: &serde_json::Value, when: &str) -> [f64; 3] {
    let at = |end: &str, i: usize| stats[when][end][i].as_f64().expect("bound");
    [0, 1, 2].map(|i| at("max", i) - at("min", i))
}

#[test]
fn rotate_stands_a_leaning_model_up_before_the_origin_moves() {
    let Some(blender) = blender() else { return };
    let dir = tempfile::tempdir().expect("tempdir");
    let fixups = serde_json::json!([
        { "op": "origin-bottom" },
        { "op": "rotate", "x": -24.5, "y": 0, "z": 0 },
    ]);
    let stats = fixed_bounds(&blender, dir.path(), fixups);
    let leaning = extent(&stats, "before");
    assert!(leaning[2] > 0.5, "the post leans first: {leaning:?}");
    let upright = extent(&stats, "after");
    for (got, want) in upright.iter().zip([0.2, 1.0, 0.2]) {
        assert!((got - want).abs() < 0.005, "upright post: {upright:?}");
    }
    let ops: Vec<&str> = stats["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .filter_map(|s| s["op"].as_str())
        .collect();
    assert_eq!(ops, ["rotate", "origin-bottom"], "rotate runs first");
    assert!(
        stats["after"]["min"][1].as_f64().expect("floor").abs() < 1e-4,
        "then it stands on the floor"
    );
}
