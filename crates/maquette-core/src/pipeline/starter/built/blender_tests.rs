//! The helper file for built asset scripts in a real, headless Blender
//! (skipped without one): the example built through the built runner, and
//! the case `join()` and `finish()` have to keep.

use std::fs;

use serde_json::Value;

use super::tests::{add_built, built_state, fixture};
use super::{BUILT_EXAMPLE, BUILT_HELPERS};
use crate::built::{BuildContext, BuildResult, Builder, BuiltState};
use crate::project::ProjectTemplate;
use crate::test_blender::{blender, glb_json, run, toolbox};

/// The primitives of the mesh the node `name` draws: each one's material
/// name with the lowest and highest corner of its positions (glTF axes, y up).
fn primitives(glb: &Value, name: &str) -> Vec<(String, [f64; 3], [f64; 3])> {
    let node = glb["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["name"] == name)
        .unwrap_or_else(|| panic!("no node called {name}"));
    let mesh = &glb["meshes"][node["mesh"].as_u64().expect("a mesh") as usize];
    let corner = |value: &Value| -> [f64; 3] {
        let v: Vec<f64> = value
            .as_array()
            .expect("a corner")
            .iter()
            .filter_map(Value::as_f64)
            .collect();
        [v[0], v[1], v[2]]
    };
    mesh["primitives"]
        .as_array()
        .expect("primitives")
        .iter()
        .map(|p| {
            let material = &glb["materials"][p["material"].as_u64().expect("a material") as usize];
            let positions = &glb["accessors"]
                [p["attributes"]["POSITION"].as_u64().expect("positions") as usize];
            (
                material["name"].as_str().unwrap_or_default().to_string(),
                corner(&positions["min"]),
                corner(&positions["max"]),
            )
        })
        .collect()
}

fn part<'a>(
    parts: &'a [(String, [f64; 3], [f64; 3])],
    material: &str,
) -> &'a (String, [f64; 3], [f64; 3]) {
    parts
        .iter()
        .find(|(name, _, _)| name == material)
        .unwrap_or_else(|| panic!("no faces with the material {material}: {parts:?}"))
}

#[test]
fn the_example_builds_through_the_built_runner_and_keeps_its_materials() {
    let Some(blender) = blender() else { return };
    let f = fixture(ProjectTemplate::Room);
    let id = add_built(&f, "Desk set", BUILT_EXAMPLE.path, &[BUILT_HELPERS.path]);
    let tools = toolbox(&blender);
    let ctx = BuildContext {
        workspace_root: &f.root,
        config: &f.config,
        project: &f.project,
        tools: &tools,
    };
    let outcome = Builder::default()
        .build(&ctx, &id, false, &|_| {})
        .expect("build");
    assert_eq!(outcome.result, BuildResult::Built, "{:#?}", outcome.log);

    // The report: what it made, in which materials, within a prop's budget.
    let record = outcome.record.expect("record");
    assert_eq!(
        record.objects,
        [
            "desk_set",
            "bell",
            "bell_button",
            "book_low",
            "book_top",
            "mug",
            "tray"
        ]
    );
    assert_eq!(
        record.materials,
        [
            "brass",
            "cover_blue",
            "cover_red",
            "ebony",
            "glaze",
            "oak",
            "page_line",
            "paper",
            "ribbon"
        ]
    );
    assert!(
        (4_000..=15_000).contains(&record.triangles),
        "{} triangles",
        record.triangles
    );
    // Its origin is at the bottom centre and nothing else needs a look.
    assert_eq!(record.warnings, Vec::<String>::new());
    assert_eq!(built_state(&f, &id), BuiltState::Current);

    // The model: every joined piece still has each face on its own material.
    let version = outcome.version.expect("a version");
    let glb = glb_json(&f.project_dir.join(&version.file));
    let book = primitives(&glb, "book_top");
    let mut names: Vec<&str> = book.iter().map(|(name, _, _)| name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["cover_red", "page_line", "paper", "ribbon"]);
    let (_, cover_low, cover_high) = part(&book, "cover_red");
    let (_, paper_low, paper_high) = part(&book, "paper");
    let (_, line_low, line_high) = part(&book, "page_line");
    let (_, ribbon_low, _) = part(&book, "ribbon");
    for axis in 0..3 {
        // The pages sit inside the cover, and the cuts of the page lines inside the pages.
        assert!(paper_low[axis] >= cover_low[axis] - 1e-6, "{book:?}");
        assert!(paper_high[axis] <= cover_high[axis] + 1e-6, "{book:?}");
        assert!(line_low[axis] >= paper_low[axis] - 1e-6, "{book:?}");
        assert!(line_high[axis] <= paper_high[axis] + 1e-6, "{book:?}");
    }
    assert!(paper_high[1] < cover_high[1] - 0.001, "{book:?}");
    assert!(line_high[1] - line_low[1] > 0.005, "{book:?}");
    // Only the ribbon hangs below the book.
    assert!(ribbon_low[1] < cover_low[1] - 0.01, "{book:?}");

    let bell = primitives(&glb, "bell");
    assert_eq!(bell.len(), 2, "{bell:?}");
    assert!(part(&bell, "brass").2[1] > part(&bell, "ebony").2[1] + 0.02);
    let mug = primitives(&glb, "mug");
    assert_eq!(mug.len(), 1, "body and handle share one material: {mug:?}");
    assert_eq!(mug[0].0, "glaze");

    // The two materials with a look of their own reach the model as they are.
    let material = |name: &str| {
        glb["materials"]
            .as_array()
            .expect("materials")
            .iter()
            .find(|m| m["name"] == name)
            .unwrap_or_else(|| panic!("no material called {name}"))
            .clone()
    };
    let brass = material("brass");
    let metallic = brass["pbrMetallicRoughness"]["metallicFactor"]
        .as_f64()
        .unwrap_or(1.0);
    assert!((0.5..1.0).contains(&metallic), "{brass}");
    let glaze = material("glaze");
    assert!(
        glaze["extensions"]["KHR_materials_clearcoat"]["clearcoatFactor"].as_f64() == Some(1.0),
        "{glaze}"
    );
    // No image and no UV map anywhere: every material is one flat colour.
    assert!(glb.get("images").is_none() && glb.get("textures").is_none());
    assert!(!glb.to_string().contains("TEXCOORD_0"));

    // The button is its own part, with its origin where it meets the dome.
    let nodes = glb["nodes"].as_array().expect("nodes");
    let button = nodes
        .iter()
        .position(|n| n["name"] == "bell_button")
        .expect("button");
    let parent = nodes.iter().find(|n| n["name"] == "bell").expect("bell");
    assert!(
        parent["children"]
            .as_array()
            .is_some_and(|c| c.contains(&Value::from(button)))
    );
    let lift = nodes[button]["translation"][1].as_f64().expect("lift");
    assert!((lift - 0.038).abs() < 1e-4, "{lift}");
}

#[test]
fn join_and_finish_keep_every_face_on_its_own_material() {
    let Some(blender) = blender() else { return };
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("join_slots.py");
    fs::write(dir.path().join("maquette_helpers.py"), BUILT_HELPERS.text).expect("helpers");
    fs::write(&script, include_str!("../../fixtures/join_slots.py")).expect("fixture");
    let said = run(&blender, &["--python", &script.display().to_string()]);
    let line = said
        .lines()
        .find_map(|l| l.strip_prefix("SLOTS "))
        .unwrap_or_else(|| panic!("the fixture said nothing:\n{said}"));
    let found: Value = serde_json::from_str(line).expect("json");
    // The caps and the sides of each piece, by where it stands: they have the
    // material they had before the join, whatever slot it was in.
    assert_eq!(
        found["joined"],
        serde_json::json!({
            "bare cap": [null],
            "bare side": [null],
            "blue cap": ["blue"],
            "blue side": ["blue"],
            "both cap": ["blue"],
            "both side": ["red"],
            "mirrored cap": ["green"],
            "mirrored side": ["green"],
            "red cap": ["red"],
            "red side": ["red"],
        }),
        "{found}"
    );
    assert_eq!(found["before"], found["joined"], "{found}");
    assert_eq!(
        found["slots"],
        serde_json::json!(["red", "blue", null, "green"])
    );
    // An object given twice joins once and is still there.
    assert_eq!(
        found["twice"],
        serde_json::json!({ "name": "twice", "faces": 6 }),
        "{found}"
    );
    // Mirrored faces were turned back to face out.
    assert_eq!(found["outward"], true, "{found}");
    // finish() fills in only for the faces with no material, and changes no slot.
    assert_eq!(found["finished"]["bare cap"], serde_json::json!(["fill"]));
    assert_eq!(found["finished"]["bare side"], serde_json::json!(["fill"]));
    assert_eq!(found["finished"]["both cap"], serde_json::json!(["blue"]));
    assert_eq!(found["finished"]["both side"], serde_json::json!(["red"]));
    assert_eq!(
        found["finished"]["mirrored side"],
        serde_json::json!(["green"])
    );
    assert_eq!(
        found["finishedSlots"],
        serde_json::json!(["red", "blue", "fill", "green"])
    );
    // And finish() without a material leaves every slot as it is.
    assert_eq!(found["untouchedSlots"], found["slots"], "{found}");
}

/// What the fixture measured of the shape `name`.
fn shape<'a>(found: &'a Value, name: &str) -> &'a Value {
    let shape = &found[name];
    assert!(shape.is_object(), "the fixture made no {name}: {found}");
    shape
}

#[test]
fn every_shape_is_closed_faces_out_and_stands_where_it_is_put() {
    let Some(blender) = blender() else { return };
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("helper_shapes.py");
    fs::write(dir.path().join("maquette_helpers.py"), BUILT_HELPERS.text).expect("helpers");
    fs::write(&script, include_str!("../../fixtures/helper_shapes.py")).expect("fixture");
    let said = run(&blender, &["--python", &script.display().to_string()]);
    let line = said
        .lines()
        .find_map(|l| l.strip_prefix("SHAPES "))
        .unwrap_or_else(|| panic!("the fixture said nothing:\n{said}"));
    let found: Value = serde_json::from_str(line).expect("json");

    // Every mesh: no open edge, no face without area, every face pointing out.
    let shapes = found.as_object().expect("shapes");
    let meshes: Vec<&String> = shapes
        .iter()
        .filter(|(_, s)| s.get("open").is_some())
        .map(|(name, _)| name)
        .collect();
    assert!(meshes.len() >= 30, "{meshes:?}");
    for name in meshes {
        let s = &found[name.as_str()];
        assert_eq!(s["open"], 0, "{name} is not closed: {s}");
        assert_eq!(s["flat"], 0, "{name} has a face with no area: {s}");
        assert_eq!(s["outward"], true, "{name} faces in: {s}");
    }

    // A shape stands on its location, a sphere is centred on it.
    let corners = |name: &str| {
        (
            shape(&found, name)["low"].clone(),
            shape(&found, name)["high"].clone(),
        )
    };
    let at = |low: [f64; 3], high: [f64; 3]| (serde_json::json!(low), serde_json::json!(high));
    assert_eq!(corners("box"), at([0.2, -0.05, 0.0], [0.4, 0.05, 0.05]));
    assert_eq!(
        corners("box_bevel"),
        at([0.2, -0.05, 0.0], [0.4, 0.05, 0.05])
    );
    assert_eq!(
        corners("cylinder_12"),
        at([-0.03, -0.03, 0.2], [0.03, 0.03, 0.3])
    );
    assert_eq!(
        corners("lathe"),
        at([-0.04, -0.04, 0.1], [0.04, 0.04, 0.19])
    );
    assert_eq!(
        corners("pages_-X"),
        at([-0.07, -0.1, 0.1], [0.07, 0.1, 0.12])
    );
    assert_eq!(
        corners("extrude_XZ"),
        at([-0.1, 0.5, -0.05], [0.1, 0.8, 0.05])
    );
    assert_eq!(
        corners("extrude_YZ"),
        at([0.5, -0.1, -0.05], [0.8, 0.1, 0.05])
    );

    // Round in few steps still shades as one curve: only the rims are crisp.
    assert_eq!(shape(&found, "box")["crisp"], 12);
    assert_eq!(shape(&found, "cylinder_12")["crisp"], 24);
    assert_eq!(shape(&found, "sphere_8")["crisp"], 0);
    assert_eq!(shape(&found, "lathe")["crisp"], 0);
    assert_eq!(shape(&found, "lathe_12")["crisp"], 0);
    assert_eq!(shape(&found, "tube")["crisp"], 32);
    assert_eq!(shape(&found, "tube_12")["crisp"], 24);
    assert_eq!(shape(&found, "ring")["crisp"], 0);
    assert_eq!(shape(&found, "ring_3d")["crisp"], 0);

    // Two lip corners close together share the wall: a full round lip, no higher than the wall.
    assert_eq!(found["lip"]["top"], 0.09);
    assert!(found["lip"]["points"].as_u64() > Some(20), "{found}");
    assert_eq!(found["lip"]["left_sharp"], 6);

    // Page blocks keep their materials apart, and no lines is a plain box.
    assert_eq!(
        shape(&found, "pages_two")["slots"],
        serde_json::json!(["paper", "ink"])
    );
    assert_eq!(
        shape(&found, "pages_lines_only")["slots"],
        serde_json::json!([null, "ink"])
    );
    assert_eq!(shape(&found, "pages_plain")["triangles"], 12);

    // A cut leaves no cutter and no spare slot.
    assert_eq!(shape(&found, "cup")["slots"], serde_json::json!(["oak"]));
    assert_eq!(shape(&found, "cup")["cutter_left"], false);

    // ground() stands the piece on its root; set_origin() moves no geometry.
    assert_eq!(found["grounded"]["low"][2], 0.0, "{found}");
    let middle = |axis: usize| {
        let low = found["grounded"]["low"][axis].as_f64().expect("low");
        let high = found["grounded"]["high"][axis].as_f64().expect("high");
        (low + high) / 2.0
    };
    assert!((middle(0) - 1.0).abs() < 1e-5 && (middle(1) - 2.0).abs() < 1e-5);
    assert_eq!(
        shape(&found, "lid")["origin"],
        serde_json::json!([0.0, 3.05, 0.1])
    );
    assert_eq!(corners("lid"), at([-0.1, 2.95, 0.1], [0.1, 3.05, 0.11]));
}
