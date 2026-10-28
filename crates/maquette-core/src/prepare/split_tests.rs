//! Split into parts in a real, headless Blender (skipped without one): the
//! fix-up script cuts a one mesh lantern into named parts, joins the tiny
//! bits, keeps the texture and ships the names the Inspector gave them.

use std::path::Path;

use crate::test_blender::{blender, glb_json, run};

const MODELS: &str = include_str!("../parts/fixtures/parts_models.py");

fn path(p: &Path) -> String {
    p.display().to_string()
}

/// Runs the fix-up script with `entry` on `model`; returns the node names
/// of the result and its stats.
fn fix(
    blender: &Path,
    dir: &Path,
    model: &Path,
    entry: &serde_json::Value,
) -> (Vec<String>, serde_json::Value) {
    let script = dir.join("fixups.py");
    std::fs::write(&script, super::SCRIPT).expect("script");
    let plan = dir.join("plan.json");
    let out = dir.join("out.glb");
    let stats = dir.join("stats.json");
    std::fs::write(&plan, entry.to_string()).expect("plan");
    run(
        blender,
        &[
            "--python",
            &path(&script),
            "--",
            "--input",
            &path(model),
            "--plan",
            &path(&plan),
            "--output",
            &path(&out),
            "--stats",
            &path(&stats),
        ],
    );
    let doc = glb_json(&out);
    let mut names: Vec<String> = doc["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|n| n.get("mesh").is_some())
        .map(|n| n["name"].as_str().unwrap_or("").to_string())
        .collect();
    names.sort();
    assert_eq!(
        doc["images"].as_array().map(Vec::len),
        Some(1),
        "the texture stays"
    );
    let stats =
        serde_json::from_str(&std::fs::read_to_string(stats).expect("stats")).expect("json");
    (names, stats)
}

fn lantern(blender: &Path, dir: &Path) -> std::path::PathBuf {
    let fixture = dir.join("parts_models.py");
    let model = dir.join("lantern.glb");
    std::fs::write(&fixture, MODELS).expect("fixture");
    run(
        blender,
        &["--python", &path(&fixture), "--", "lantern", &path(&model)],
    );
    model
}

#[test]
fn a_split_by_loose_pieces_names_the_parts_top_first_and_merges_the_bits() {
    let Some(blender) = blender() else { return };
    let dir = tempfile::tempdir().expect("tempdir");
    let model = lantern(&blender, dir.path());
    let entry = serde_json::json!({
        "fixups": [{ "op": "split-parts", "by": "loose", "mergeUnder": 0.01 }],
    });
    let (names, stats) = fix(&blender, dir.path(), &model, &entry);
    assert_eq!(names, ["part_1", "part_2", "part_3", "part_4", "part_5"]);
    let step = &stats["steps"][0];
    assert_eq!(step["op"], "split-parts");
    assert_eq!(step["split"]["pieces"], 9, "{step}");
    assert_eq!(step["split"]["merged"], 4, "{step}");
    assert_eq!(step["split"]["byLoose"], 5);
    assert_eq!(step["split"]["byMaterial"], 4);
    assert_eq!(stats["before"]["objects"], 1);
    assert_eq!(stats["after"]["objects"], 5);
    assert_eq!(stats["before"]["triangles"], stats["after"]["triangles"]);
    assert_eq!(stats["before"]["drawCalls"], 4);
    assert_eq!(stats["after"]["drawCalls"], 5);
}

#[test]
fn a_split_by_material_joins_the_parts_that_share_one() {
    let Some(blender) = blender() else { return };
    let dir = tempfile::tempdir().expect("tempdir");
    let model = lantern(&blender, dir.path());
    let entry = serde_json::json!({
        "fixups": [{ "op": "split-parts", "by": "material", "mergeUnder": 0.01 }],
    });
    let (names, _) = fix(&blender, dir.path(), &model, &entry);
    assert_eq!(
        names.len(),
        4,
        "the lid and the post are both stone: {names:?}"
    );
}

#[test]
fn part_names_and_joins_ship_in_the_export() {
    let Some(blender) = blender() else { return };
    let dir = tempfile::tempdir().expect("tempdir");
    let model = lantern(&blender, dir.path());
    let entry = serde_json::json!({
        "fixups": [{ "op": "split-parts", "by": "loose", "mergeUnder": 0.01 }],
        "parts": {
            "names": { "part_1": "lid", "part_2": "cage", "part_3": "crystal" },
            "merge": [["part_4", "part_5"]],
            "hidden": ["part_1"],
        },
    });
    let (names, stats) = fix(&blender, dir.path(), &model, &entry);
    assert_eq!(names, ["cage", "crystal", "lid", "part_4"]);
    assert_eq!(stats["after"]["objects"], 4);
}
