//! Small generated GLB models with known flaws: the inbox tests check the
//! analysis against them, the mock UI previews one, and the end to end run
//! drops them into a scratch inbox. Everything is built from boxes, so the
//! numbers are easy to reason about.

mod builder;
mod skeleton;

use serde_json::{Value, json};

use builder::{Builder, Mesh};
use skeleton::{skeleton, world_positions};

/// A full PBR material set (base colour, metal/rough, normal) at `size`.
fn pbr_material(builder: &mut Builder, size: u32, name: &str) -> (Value, Value, Value) {
    let views = [
        builder.checker(size, [214, 196, 255], [120, 104, 170]),
        builder.checker(size, [0, 140, 0], [0, 200, 0]),
        builder.checker(size, [128, 128, 255], [128, 128, 255]),
    ];
    let images: Vec<Value> = views
        .iter()
        .map(|v| json!({ "bufferView": v, "mimeType": "image/png" }))
        .collect();
    let textures: Vec<Value> = (0..3)
        .map(|i| json!({ "source": i, "sampler": 0 }))
        .collect();
    let material = json!({
        "name": name,
        "pbrMetallicRoughness": {
            "baseColorTexture": { "index": 0 },
            "metallicRoughnessTexture": { "index": 1 },
        },
        "normalTexture": { "index": 2 },
    });
    (
        Value::Array(images),
        Value::Array(textures),
        json!([material]),
    )
}

/// A textured crate: one box, 12 triangles, a full PBR set at 64 px. Passes
/// every check.
pub fn clean_prop() -> Vec<u8> {
    let mut b = Builder::default();
    let mut mesh = Mesh::default();
    mesh.cuboid([0.0, 0.25, 0.0], [0.5, 0.5, 0.5], 0);
    let (images, textures, materials) = pbr_material(&mut b, 64, "Crate");
    let prim = b.primitive(&mesh, 0, false);
    let root = json!({
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "name": "Crate", "mesh": 0 }],
        "meshes": [{ "name": "Crate", "primitives": [prim] }],
        "materials": materials,
        "images": images,
        "textures": textures,
        "samplers": [{}],
    });
    b.finish(root)
}

/// Two crates three metres apart baked into one primitive, the way an
/// export sometimes joins separate objects. One material, no textures.
pub fn merged_islands() -> Vec<u8> {
    let mut b = Builder::default();
    let mut mesh = Mesh::default();
    mesh.cuboid([-1.5, 0.25, 0.0], [0.5, 0.5, 0.5], 0);
    mesh.cuboid([1.5, 0.4, 0.0], [0.6, 0.8, 0.6], 0);
    // A third box touching the second counts as part of the same object.
    mesh.cuboid([1.5, 0.9, 0.0], [0.3, 0.2, 0.3], 0);
    let prim = b.primitive(&mesh, 0, false);
    let root = json!({
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "name": "Props", "mesh": 0 }],
        "meshes": [{ "name": "Props", "primitives": [prim] }],
        "materials": [{ "name": "Wood", "pbrMetallicRoughness": { "baseColorFactor": [0.6, 0.45, 0.3, 1.0] } }],
    });
    b.finish(root)
}

/// A box character on the 65 joint Mixamo rig, with an idle animation and
/// twin tails. With `bug` the tails are weighted to the index finger bones
/// (what an auto rig does to long hair near the hands), otherwise to the
/// head.
pub fn character(bug: bool) -> Vec<u8> {
    let joints = skeleton();
    let world = world_positions(&joints);
    let index = |name: &str| -> u16 {
        let full = format!("mixamorig:{name}");
        joints
            .iter()
            .position(|(n, _, _)| *n == full)
            .and_then(|i| u16::try_from(i).ok())
            .unwrap_or(0)
    };
    let mut mesh = Mesh::default();
    mesh.cuboid([0.0, 0.98, 0.0], [0.32, 0.16, 0.18], index("Hips"));
    mesh.cuboid([0.0, 1.25, 0.0], [0.36, 0.42, 0.2], index("Spine1"));
    mesh.cuboid([0.0, 1.66, 0.0], [0.22, 0.26, 0.22], index("Head"));
    for (side, s) in [("Left", 1.0f32), ("Right", -1.0f32)] {
        mesh.cuboid(
            [s * 0.31, 1.44, 0.0],
            [0.24, 0.08, 0.08],
            index(&format!("{side}Arm")),
        );
        mesh.cuboid(
            [s * 0.56, 1.44, 0.0],
            [0.24, 0.07, 0.07],
            index(&format!("{side}ForeArm")),
        );
        mesh.cuboid(
            [s * 0.73, 1.44, 0.0],
            [0.1, 0.03, 0.08],
            index(&format!("{side}Hand")),
        );
        mesh.cuboid(
            [s * 0.8, 1.44, 0.02],
            [0.06, 0.02, 0.02],
            index(&format!("{side}HandIndex1")),
        );
        mesh.cuboid(
            [s * 0.09, 0.74, 0.0],
            [0.12, 0.4, 0.12],
            index(&format!("{side}UpLeg")),
        );
        mesh.cuboid(
            [s * 0.09, 0.32, 0.0],
            [0.1, 0.42, 0.1],
            index(&format!("{side}Leg")),
        );
        mesh.cuboid(
            [s * 0.09, 0.04, 0.05],
            [0.1, 0.06, 0.2],
            index(&format!("{side}Foot")),
        );
        let tail = if bug {
            index(&format!("{side}HandIndex1"))
        } else {
            index("Head")
        };
        mesh.cuboid([s * 0.17, 1.47, -0.08], [0.07, 0.45, 0.07], tail);
    }

    let mut b = Builder::default();
    let (images, textures, materials) = pbr_material(&mut b, 128, "Body");
    let prim = b.primitive(&mesh, 0, true);
    // Translation only rig, so each inverse bind matrix is a translation by
    // minus the joint's world position.
    let ibm: Vec<f32> = world
        .iter()
        .flat_map(|p| {
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -p[0], -p[1], -p[2],
                1.0,
            ]
        })
        .collect();
    let ibm = b.floats(&ibm, "MAT4", 16, None, false);
    let times = b.floats(&[0.0, 1.0, 2.0], "SCALAR", 1, None, true);
    let hips = world[0];
    let values = b.floats(
        &[
            hips[0],
            hips[1],
            hips[2],
            hips[0],
            hips[1] - 0.02,
            hips[2],
            hips[0],
            hips[1],
            hips[2],
        ],
        "VEC3",
        3,
        None,
        false,
    );

    // Nodes: 0 body, 1 armature, then the joints from 2.
    let offset = 2;
    let mut nodes = vec![
        json!({ "name": "Body", "mesh": 0, "skin": 0 }),
        json!({ "name": "Armature", "children": [offset] }),
    ];
    for (i, (name, _, t)) in joints.iter().enumerate() {
        let children: Vec<usize> = joints
            .iter()
            .enumerate()
            .filter(|(_, (_, parent, _))| *parent == Some(i))
            .map(|(c, _)| c + offset)
            .collect();
        let mut node = json!({ "name": name, "translation": t });
        if !children.is_empty() {
            node["children"] = json!(children);
        }
        nodes.push(node);
    }
    let root = json!({
        "scene": 0,
        "scenes": [{ "nodes": [0, 1] }],
        "nodes": nodes,
        "meshes": [{ "name": "Body", "primitives": [prim] }],
        "skins": [{
            "name": "Armature",
            "inverseBindMatrices": ibm,
            "skeleton": offset,
            "joints": (0..joints.len()).map(|i| i + offset).collect::<Vec<_>>(),
        }],
        "animations": [{
            "name": "Idle",
            "samplers": [{ "input": times, "output": values, "interpolation": "LINEAR" }],
            "channels": [{ "sampler": 0, "target": { "node": offset, "path": "translation" } }],
        }],
        "materials": materials,
        "images": images,
        "textures": textures,
        "samplers": [{}],
    });
    b.finish(root)
}

/// A desk lamp the way a generator sends it: under a scaled and moved root
/// node, a dense shade (19,200 triangles) with a 1024 px texture, a base,
/// and a 1 cm speck two metres away. For the fix-ups: decimate, apply
/// transforms, merge, remove loose parts, shrink textures, origin.
pub fn messy_prop() -> Vec<u8> {
    let mut b = Builder::default();
    let mut shade = Mesh::default();
    shade.dense_cuboid([0.0, 1.0, 0.0], [1.0, 0.8, 1.0], 40);
    let mut base = Mesh::default();
    base.cuboid([0.0, 0.2, 0.0], [0.6, 0.4, 0.6], 0);
    let mut speck = Mesh::default();
    speck.cuboid([2.0, 0.0, 0.0], [0.01, 0.01, 0.01], 0);
    let (images, textures, mut materials) = pbr_material(&mut b, 1024, "Shade");
    if let Some(list) = materials.as_array_mut() {
        list.push(json!({ "name": "Metal", "pbrMetallicRoughness": { "metallicFactor": 1.0 } }));
    }
    let prims = [
        b.primitive(&shade, 0, false),
        b.primitive(&base, 1, false),
        b.primitive(&speck, 1, false),
    ];
    let root = json!({
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [
            { "name": "Root", "translation": [0.3, 0.5, -0.2], "scale": [0.5, 0.5, 0.5], "children": [1, 2, 3] },
            { "name": "Shade", "mesh": 0 },
            { "name": "Base", "mesh": 1 },
            { "name": "Speck", "mesh": 2 },
        ],
        "meshes": [
            { "name": "Shade", "primitives": [prims[0].clone()] },
            { "name": "Base", "primitives": [prims[1].clone()] },
            { "name": "Speck", "primitives": [prims[2].clone()] },
        ],
        "materials": materials,
        "images": images,
        "textures": textures,
        "samplers": [{}],
    });
    b.finish(root)
}

/// Every fixture with a file name, for the e2e run and the mock assets.
pub fn all() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("crate_clean.glb", clean_prop()),
        ("props_merged.glb", merged_islands()),
        ("character_tails.glb", character(true)),
        ("character_clean.glb", character(false)),
    ]
}

#[cfg(test)]
mod tests;
