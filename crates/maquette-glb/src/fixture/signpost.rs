//! Two plain props for the preview renders.
//!
//! The signpost is a prop whose front is known, to hold the axes of every layer together:
//! a grey body with a green marker on its front (+Z in glTF and three.js,
//! -Y once Blender has imported it), a red one on its right in the front
//! view (+X) and a blue one on top (+Y). A render that calls itself the
//! front view must show green in the middle, red to the right of it and no
//! blue face; a top view shows blue in the middle and green below it.

use serde_json::{Value, json};

use super::builder::{Builder, Mesh};

/// The markers' base colours, linear, by the side they sit on.
pub const SIGNPOST_FRONT: [f32; 3] = [0.02, 0.5, 0.05];
pub const SIGNPOST_RIGHT: [f32; 3] = [0.6, 0.02, 0.02];
pub const SIGNPOST_TOP: [f32; 3] = [0.02, 0.05, 0.6];
const BODY: [f32; 3] = [0.35, 0.35, 0.35];

/// One box of a prop: its name, centre, size and base colour.
type Slab = (&'static str, [f32; 3], [f32; 3], [f32; 3]);

/// The signpost: four boxes as four named objects (`body`, `front`,
/// `right`, `top`) under one root, 48 triangles, plain materials. Its bounds
/// run from (-0.2, 0, -0.1) to (0.26, 0.36, 0.16) metres.
pub fn signpost() -> Vec<u8> {
    let mut b = Builder::default();
    let parts: [Slab; 4] = [
        ("body", [0.0, 0.15, 0.0], [0.4, 0.3, 0.2], BODY),
        (
            "front",
            [0.0, 0.15, 0.13],
            [0.16, 0.16, 0.06],
            SIGNPOST_FRONT,
        ),
        (
            "right",
            [0.23, 0.15, 0.0],
            [0.06, 0.16, 0.12],
            SIGNPOST_RIGHT,
        ),
        ("top", [0.0, 0.33, 0.0], [0.16, 0.06, 0.12], SIGNPOST_TOP),
    ];
    let mut nodes: Vec<Value> = vec![json!({ "name": "signpost", "children": [1, 2, 3, 4] })];
    let mut meshes = Vec::new();
    let mut materials = Vec::new();
    for (n, (name, centre, size, colour)) in parts.iter().enumerate() {
        let mut mesh = Mesh::default();
        mesh.cuboid(*centre, *size, 0);
        let prim = b.primitive(&mesh, n, false);
        meshes.push(json!({ "name": name, "primitives": [prim] }));
        nodes.push(json!({ "name": name, "mesh": n }));
        materials.push(json!({
            "name": name,
            "pbrMetallicRoughness": {
                "baseColorFactor": [colour[0], colour[1], colour[2], 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 1.0,
            },
        }));
    }
    let root = json!({
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
    });
    b.finish(root)
}

/// The pale prop's base colours, linear: a lavender (#c9b8e8) cover, cream
/// (#f3e9d2) pages and a paper white (#f7f5ef) lid.
pub const PALE_COLOURS: [[f32; 3]; 3] = [
    [0.584, 0.479, 0.807],
    [0.896, 0.815, 0.645],
    [0.930, 0.913, 0.863],
];

/// A prop in pale flat colours, as a built script's palette gives them:
/// three stacked slabs under plain materials of roughness 0.5. A light that
/// is too strong washes all three out to white.
pub fn pale_prop() -> Vec<u8> {
    let mut b = Builder::default();
    let [lavender, cream, paper] = PALE_COLOURS;
    let slabs: [Slab; 3] = [
        ("cover", [0.0, 0.03, 0.0], [0.3, 0.06, 0.22], lavender),
        ("pages", [0.0, 0.09, 0.0], [0.26, 0.06, 0.18], cream),
        ("lid", [0.0, 0.135, 0.0], [0.2, 0.03, 0.14], paper),
    ];
    let mut nodes: Vec<Value> = vec![json!({ "name": "pale_prop", "children": [1, 2, 3] })];
    let mut meshes = Vec::new();
    let mut materials = Vec::new();
    for (n, (name, centre, size, colour)) in slabs.iter().enumerate() {
        let mut mesh = Mesh::default();
        mesh.cuboid(*centre, *size, 0);
        let prim = b.primitive(&mesh, n, false);
        meshes.push(json!({ "name": name, "primitives": [prim] }));
        nodes.push(json!({ "name": name, "mesh": n }));
        materials.push(json!({
            "name": name,
            "pbrMetallicRoughness": {
                "baseColorFactor": [colour[0], colour[1], colour[2], 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 0.5,
            },
        }));
    }
    let root = json!({
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
    });
    b.finish(root)
}

/// A plain light grey picture, 2 px square, to stand in for a ref image
/// where a test needs one beside a model.
pub fn ref_card() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x00, 0x00, 0x00, 0x00, 0x57,
        0xdd, 0x52, 0xf8, 0x00, 0x00, 0x00, 0x0e, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xb8,
        0x70, 0x81, 0xe1, 0xc2, 0x05, 0x00, 0x09, 0xc6, 0x03, 0x41, 0x7e, 0xab, 0xd7, 0x0b, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}
