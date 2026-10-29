//! Game export of rigged models in a real, headless Blender (skipped
//! without Blender or node): each rig keeps only its own clips, and the
//! drop maps setting leaves normal and metal-roughness maps out.

use std::fs;
use std::path::Path;

use super::blender_tests::run_chain;
use super::tests::{game_project, glb_game};
use super::*;
use crate::prepare::{self, ModelPrep, RealSize, SizeAxis, SizeUnit};
use crate::project;
use crate::test_blender::{blender, glb_json, run};

/// Two rigged boxes, each with one clip of its own, the first with a
/// normal map and a metal-roughness map.
const MAKE_RIGS: &str = r#"
import sys
import bpy
walker, biter = sys.argv[sys.argv.index("--") + 1 :]

def image(name):
    img = bpy.data.images.new(name, 4, 4)
    img.pixels = [0.5] * 64
    img.pack()
    return img

def mapped(material):
    tree = material.node_tree
    bsdf = tree.nodes["Principled BSDF"]
    normal_tex = tree.nodes.new("ShaderNodeTexImage")
    normal_tex.image = image("normal")
    normal_map = tree.nodes.new("ShaderNodeNormalMap")
    tree.links.new(normal_tex.outputs["Color"], normal_map.inputs["Color"])
    tree.links.new(normal_map.outputs["Normal"], bsdf.inputs["Normal"])
    mr_tex = tree.nodes.new("ShaderNodeTexImage")
    mr_tex.image = image("metal_rough")
    split = tree.nodes.new("ShaderNodeSeparateColor")
    tree.links.new(mr_tex.outputs["Color"], split.inputs["Color"])
    tree.links.new(split.outputs["Green"], bsdf.inputs["Roughness"])
    tree.links.new(split.outputs["Blue"], bsdf.inputs["Metallic"])

def rig(name, clip, path, maps):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.object.armature_add()
    arm = bpy.context.active_object
    arm.name = name
    bpy.ops.mesh.primitive_cube_add(size=0.3)
    box = bpy.context.active_object
    material = bpy.data.materials.new(name)
    material.use_nodes = True
    if maps:
        mapped(material)
    box.data.materials.append(material)
    box.parent = arm
    box.modifiers.new("Armature", "ARMATURE").object = arm
    group = box.vertex_groups.new(name="Bone")
    group.add(range(len(box.data.vertices)), 1.0, "REPLACE")
    bone = arm.pose.bones["Bone"]
    arm.animation_data_create().action = bpy.data.actions.new(clip)
    for frame, z in ((0, 0.0), (12, 0.5)):
        bone.location = (0.0, z, 0.0)
        bone.keyframe_insert("location", frame=frame)
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB")

rig("walker", "walk", walker, True)
rig("biter", "bite", biter, False)
"#;

fn clips(json: &serde_json::Value) -> Vec<String> {
    json["animations"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn has_map(json: &serde_json::Value, key: &str) -> bool {
    json["materials"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|m| !m[key].is_null() || !m["pbrMetallicRoughness"][key].is_null())
}

/// Exports the two rigs with `drop_maps`, returning the walker's and the
/// biter's exported GLB JSON, or `None` without Blender or node.
fn export_rigs(drop_maps: bool) -> Option<(serde_json::Value, serde_json::Value)> {
    let blender = blender()?;
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let characters = ws.repo.join("assets/models/characters");
    let maker = ws.root.join("make_rigs.py");
    fs::write(&maker, MAKE_RIGS).expect("maker");
    let at = |p: &Path| p.display().to_string();
    run(
        &blender,
        &[
            "--python",
            &at(&maker),
            "--",
            &at(&characters.join("walker.glb")),
            &at(&characters.join("biter.glb")),
        ],
    );
    let files = [
        "characters/walker.glb".to_string(),
        "characters/biter.glb".to_string(),
    ];
    let imported =
        import_models(&ws.root, &ws.config.layout, &manifest.id, &files).expect("import");
    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    // A real size gives both work, so both export rather than copy.
    prepare::update(&project_dir, |current| {
        let mut next = current.clone();
        for asset in &imported.imported {
            let size = RealSize {
                axis: SizeAxis::Longest,
                value: 1.0,
                unit: SizeUnit::M,
            };
            next.models.insert(
                asset.clone(),
                ModelPrep {
                    size: Some(size),
                    ..ModelPrep::default()
                },
            );
        }
        Ok(next)
    })
    .expect("prep");
    let manifest = project::modify(&ws.root, &ws.config.layout, &manifest.id, |m| {
        if let Some(game) = m.game.as_mut() {
            game.export.drop_maps = drop_maps;
        }
        Ok(())
    })
    .expect("settings");
    crate::pipeline::ensure(&project_dir, &manifest).expect("pipeline");
    index::write(&project_dir, &manifest).expect("index");
    run_chain(&project_dir, settings_of(&manifest).expect("game"))?;
    let raw = project_dir.join("web/raw/characters");
    Some((
        glb_json(&raw.join("walker.glb")),
        glb_json(&raw.join("biter.glb")),
    ))
}

#[test]
fn each_rig_keeps_its_own_clips_and_maps_drop_only_when_asked() {
    let Some((walker, biter)) = export_rigs(false) else {
        return;
    };
    assert_eq!(clips(&walker), ["walk"], "the walker has no bite");
    assert_eq!(clips(&biter), ["bite"], "the biter has no walk");
    assert!(has_map(&walker, "normalTexture"), "maps kept by default");
    assert!(has_map(&walker, "metallicRoughnessTexture"));

    let Some((walker, _)) = export_rigs(true) else {
        return;
    };
    assert!(
        !has_map(&walker, "normalTexture"),
        "the normal map is dropped"
    );
    assert!(!has_map(&walker, "metallicRoughnessTexture"));
}
