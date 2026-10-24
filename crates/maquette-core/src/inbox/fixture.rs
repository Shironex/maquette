//! Small generated GLB models with known flaws: the inbox tests check the
//! analysis against them, the mock UI previews one, and the end to end run
//! drops them into a scratch inbox. Everything is built from boxes, so the
//! numbers are easy to reason about.

use serde_json::{Value, json};

use super::images::write_png;

/// The 65 joint Mixamo skeleton: name, parent, translation from the parent.
/// T pose, Y up, facing +Z, metres. The character's left is +X.
fn skeleton() -> Vec<(String, Option<usize>, [f32; 3])> {
    let mut joints: Vec<(String, Option<usize>, [f32; 3])> = Vec::new();
    let add = |joints: &mut Vec<(String, Option<usize>, [f32; 3])>,
               name: &str,
               parent: Option<usize>,
               t: [f32; 3]| {
        joints.push((format!("mixamorig:{name}"), parent, t));
        joints.len() - 1
    };
    let hips = add(&mut joints, "Hips", None, [0.0, 1.0, 0.0]);
    let spine = add(&mut joints, "Spine", Some(hips), [0.0, 0.1, 0.0]);
    let spine1 = add(&mut joints, "Spine1", Some(spine), [0.0, 0.12, 0.0]);
    let spine2 = add(&mut joints, "Spine2", Some(spine1), [0.0, 0.12, 0.0]);
    let neck = add(&mut joints, "Neck", Some(spine2), [0.0, 0.14, 0.0]);
    let head = add(&mut joints, "Head", Some(neck), [0.0, 0.1, 0.0]);
    add(&mut joints, "HeadTop_End", Some(head), [0.0, 0.18, 0.0]);
    for (side, s) in [("Left", 1.0f32), ("Right", -1.0f32)] {
        let shoulder = add(
            &mut joints,
            &format!("{side}Shoulder"),
            Some(spine2),
            [s * 0.06, 0.1, 0.0],
        );
        let arm = add(
            &mut joints,
            &format!("{side}Arm"),
            Some(shoulder),
            [s * 0.12, 0.0, 0.0],
        );
        let fore = add(
            &mut joints,
            &format!("{side}ForeArm"),
            Some(arm),
            [s * 0.26, 0.0, 0.0],
        );
        let hand = add(
            &mut joints,
            &format!("{side}Hand"),
            Some(fore),
            [s * 0.24, 0.0, 0.0],
        );
        for (finger, first) in [
            ("Thumb", [s * 0.03, 0.0, 0.03]),
            ("Index", [s * 0.08, 0.0, 0.025]),
            ("Middle", [s * 0.08, 0.0, 0.0]),
            ("Ring", [s * 0.075, 0.0, -0.02]),
            ("Pinky", [s * 0.07, 0.0, -0.04]),
        ] {
            let mut parent = add(
                &mut joints,
                &format!("{side}Hand{finger}1"),
                Some(hand),
                first,
            );
            for n in 2..=4 {
                parent = add(
                    &mut joints,
                    &format!("{side}Hand{finger}{n}"),
                    Some(parent),
                    [s * 0.03, 0.0, 0.0],
                );
            }
        }
    }
    for (side, s) in [("Left", 1.0f32), ("Right", -1.0f32)] {
        let up = add(
            &mut joints,
            &format!("{side}UpLeg"),
            Some(hips),
            [s * 0.09, -0.05, 0.0],
        );
        let leg = add(
            &mut joints,
            &format!("{side}Leg"),
            Some(up),
            [0.0, -0.42, 0.0],
        );
        let foot = add(
            &mut joints,
            &format!("{side}Foot"),
            Some(leg),
            [0.0, -0.42, 0.0],
        );
        let toe = add(
            &mut joints,
            &format!("{side}ToeBase"),
            Some(foot),
            [0.0, -0.08, 0.1],
        );
        add(
            &mut joints,
            &format!("{side}Toe_End"),
            Some(toe),
            [0.0, 0.0, 0.06],
        );
    }
    joints
}

fn world_positions(joints: &[(String, Option<usize>, [f32; 3])]) -> Vec<[f32; 3]> {
    let mut out: Vec<[f32; 3]> = Vec::with_capacity(joints.len());
    for (_, parent, t) in joints {
        let base = parent.map_or([0.0; 3], |p| out[p]);
        out.push([base[0] + t[0], base[1] + t[1], base[2] + t[2]]);
    }
    out
}

#[derive(Default)]
struct Mesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    joints: Vec<[u16; 4]>,
}

impl Mesh {
    /// Appends a box: 24 vertices (flat normals), 12 triangles.
    fn cuboid(&mut self, center: [f32; 3], size: [f32; 3], joint: u16) {
        let h = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
        // Normal axis, sign, and the two axes spanning the face.
        let faces = [
            (0, 1.0, 1, 2),
            (0, -1.0, 2, 1),
            (1, 1.0, 2, 0),
            (1, -1.0, 0, 2),
            (2, 1.0, 0, 1),
            (2, -1.0, 1, 0),
        ];
        for (axis, sign, u, v) in faces {
            let base = u32::try_from(self.positions.len()).unwrap_or(u32::MAX);
            for (du, dv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let mut p = center;
                p[axis] += sign * h[axis];
                p[u] += du * h[u];
                p[v] += dv * h[v];
                let mut n = [0.0; 3];
                n[axis] = sign;
                self.positions.push(p);
                self.normals.push(n);
                self.uvs.push([(du + 1.0) / 2.0, (dv + 1.0) / 2.0]);
                self.joints.push([joint, 0, 0, 0]);
            }
            self.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
}

#[derive(Default)]
struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Builder {
    fn view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let mut view =
            json!({ "buffer": 0, "byteOffset": self.bin.len(), "byteLength": bytes.len() });
        if let Some(target) = target {
            view["target"] = json!(target);
        }
        self.bin.extend_from_slice(bytes);
        self.views.push(view);
        self.views.len() - 1
    }

    fn floats(
        &mut self,
        data: &[f32],
        kind: &str,
        width: usize,
        target: Option<u32>,
        bounds: bool,
    ) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        let view = self.view(&bytes, target);
        let mut accessor = json!({
            "bufferView": view,
            "componentType": 5126,
            "count": data.len() / width,
            "type": kind,
        });
        if bounds {
            let mut min = vec![f32::INFINITY; width];
            let mut max = vec![f32::NEG_INFINITY; width];
            for item in data.chunks(width) {
                for (k, v) in item.iter().enumerate() {
                    min[k] = min[k].min(*v);
                    max[k] = max[k].max(*v);
                }
            }
            accessor["min"] = json!(min);
            accessor["max"] = json!(max);
        }
        self.accessors.push(accessor);
        self.accessors.len() - 1
    }

    fn indices(&mut self, data: &[u32]) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|i| i.to_le_bytes()).collect();
        let view = self.view(&bytes, Some(34963));
        self.accessors.push(json!({ "bufferView": view, "componentType": 5125, "count": data.len(), "type": "SCALAR" }));
        self.accessors.len() - 1
    }

    fn joints(&mut self, data: &[[u16; 4]]) -> usize {
        let bytes: Vec<u8> = data
            .iter()
            .flatten()
            .flat_map(|j| j.to_le_bytes())
            .collect();
        let view = self.view(&bytes, Some(34962));
        self.accessors.push(json!({ "bufferView": view, "componentType": 5123, "count": data.len(), "type": "VEC4" }));
        self.accessors.len() - 1
    }

    /// The primitive JSON for `mesh`, with skin attributes when `skinned`.
    fn primitive(&mut self, mesh: &Mesh, material: usize, skinned: bool) -> Value {
        let flat3 = |v: &[[f32; 3]]| v.iter().flatten().copied().collect::<Vec<f32>>();
        let position = self.floats(&flat3(&mesh.positions), "VEC3", 3, Some(34962), true);
        let normal = self.floats(&flat3(&mesh.normals), "VEC3", 3, Some(34962), false);
        let uv: Vec<f32> = mesh.uvs.iter().flatten().copied().collect();
        let uv = self.floats(&uv, "VEC2", 2, Some(34962), false);
        let indices = self.indices(&mesh.indices);
        let mut attributes = json!({ "POSITION": position, "NORMAL": normal, "TEXCOORD_0": uv });
        if skinned {
            let joints = self.joints(&mesh.joints);
            let weights: Vec<f32> = mesh
                .joints
                .iter()
                .flat_map(|_| [1.0, 0.0, 0.0, 0.0])
                .collect();
            let weights = self.floats(&weights, "VEC4", 4, Some(34962), false);
            attributes["JOINTS_0"] = json!(joints);
            attributes["WEIGHTS_0"] = json!(weights);
        }
        json!({ "attributes": attributes, "indices": indices, "material": material })
    }

    fn finish(mut self, mut root: Value) -> Vec<u8> {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        root["asset"] = json!({ "version": "2.0", "generator": "Maquette fixtures" });
        root["buffers"] = json!([{ "byteLength": self.bin.len() }]);
        root["bufferViews"] = Value::Array(self.views);
        root["accessors"] = Value::Array(self.accessors);
        let mut text = serde_json::to_vec(&root).unwrap_or_default();
        while !text.len().is_multiple_of(4) {
            text.push(b' ');
        }
        let total = 12 + 8 + text.len() + 8 + self.bin.len();
        let mut out = Vec::with_capacity(total);
        let u = |n: usize| u32::try_from(n).unwrap_or(u32::MAX).to_le_bytes();
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&u(total));
        out.extend_from_slice(&u(text.len()));
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&text);
        out.extend_from_slice(&u(self.bin.len()));
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&self.bin);
        out
    }

    /// Embeds a checker texture and returns the image index count so far.
    fn checker(&mut self, size: u32, a: [u8; 3], b: [u8; 3]) -> usize {
        let mut rgb = Vec::with_capacity((size * size * 3) as usize);
        for y in 0..size {
            for x in 0..size {
                let on = ((x / 8) + (y / 8)) % 2 == 0;
                rgb.extend_from_slice(if on { &a } else { &b });
            }
        }
        self.view(&write_png(size, size, &rgb), None)
    }
}

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
mod tests {
    use super::*;

    #[test]
    fn the_rig_is_the_65_joint_mixamo_set() {
        let joints = skeleton();
        assert_eq!(joints.len(), 65);
        assert!(joints.iter().all(|(n, _, _)| n.starts_with("mixamorig:")));
        let world = world_positions(&joints);
        let head = joints
            .iter()
            .position(|(n, _, _)| n == "mixamorig:Head")
            .expect("head");
        assert!((world[head][1] - 1.58).abs() < 1e-4);
    }

    #[test]
    fn fixtures_parse_as_gltf() {
        for (name, bytes) in all() {
            let gltf = gltf::Gltf::from_slice(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(gltf.blob.is_some(), "{name} has a BIN chunk");
            assert_eq!(gltf.meshes().count(), 1);
        }
    }
}
