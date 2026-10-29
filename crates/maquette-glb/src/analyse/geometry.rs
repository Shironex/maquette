//! Bounding boxes of a model's primitives in world space.

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use crate::math::{self, Aabb, IDENTITY, Mat4, Vec3};

/// The corners of a primitive's position bounds in its node's frame.
pub(crate) fn primitive_corners(prim: &gltf::Primitive<'_>) -> [Vec3; 8] {
    let bb = prim.bounding_box();
    // Quantized positions (KHR_mesh_quantization) store normalized
    // integers, and their min and max are in those integer units.
    let unit = prim
        .get(&gltf::Semantic::Positions)
        .filter(gltf::Accessor::normalized)
        .map_or(1.0, |a| normalized_max(a.data_type()));
    let scaled = |v: [f32; 3]| [v[0] / unit, v[1] / unit, v[2] / unit];
    corners(scaled(bb.min), scaled(bb.max))
}

/// The min and max corners of every mesh in the model at `path`, in its own
/// glTF frame (Y up, metres). `None` when it cannot be read or has no mesh.
/// Reads only the JSON, so it stays quick for large files.
pub fn bounds(path: &Path) -> Option<([f32; 3], [f32; 3])> {
    let bytes = fs::read(path).ok()?;
    catch_unwind(AssertUnwindSafe(|| {
        let gltf = gltf::Gltf::from_slice_without_validation(&bytes).ok()?;
        let doc = &gltf.document;
        let worlds = world_matrices(doc);
        let mut bounds = Aabb::EMPTY;
        for node in doc.nodes() {
            let Some(mesh) = node.mesh() else { continue };
            let world = worlds.get(node.index()).copied().unwrap_or(IDENTITY);
            for prim in mesh.primitives() {
                for corner in primitive_corners(&prim) {
                    bounds.add(math::point(&world, corner));
                }
            }
        }
        (!bounds.is_empty() && bounds.min.iter().chain(&bounds.max).all(|v| v.is_finite()))
            .then_some((bounds.min, bounds.max))
    }))
    .ok()
    .flatten()
}

/// The largest stored value of a normalized integer type, which reads 1.0.
fn normalized_max(kind: gltf::accessor::DataType) -> f32 {
    use gltf::accessor::DataType;
    match kind {
        DataType::I8 => 127.0,
        DataType::U8 => 255.0,
        DataType::I16 => 32767.0,
        DataType::U16 => 65535.0,
        DataType::U32 => 4_294_967_295.0,
        DataType::F32 => 1.0,
    }
}

fn corners(min: Vec3, max: Vec3) -> [Vec3; 8] {
    let mut out = [[0.0; 3]; 8];
    for (i, c) in out.iter_mut().enumerate() {
        *c = [
            if i & 1 == 0 { min[0] } else { max[0] },
            if i & 2 == 0 { min[1] } else { max[1] },
            if i & 4 == 0 { min[2] } else { max[2] },
        ];
    }
    out
}

/// World matrices of every node, from the scene roots down.
pub fn world_matrices(doc: &gltf::Document) -> Vec<Mat4> {
    let mut out = vec![IDENTITY; doc.nodes().len()];
    let mut has_parent = vec![false; doc.nodes().len()];
    for node in doc.nodes() {
        for child in node.children() {
            if let Some(flag) = has_parent.get_mut(child.index()) {
                *flag = true;
            }
        }
    }
    let mut stack: Vec<(gltf::Node<'_>, Mat4)> = doc
        .nodes()
        .filter(|n| !has_parent[n.index()])
        .map(|n| (n, IDENTITY))
        .collect();
    let mut visited = vec![false; doc.nodes().len()];
    while let Some((node, parent)) = stack.pop() {
        if std::mem::replace(&mut visited[node.index()], true) {
            continue;
        }
        let world = math::mul(&parent, &node.transform().matrix());
        out[node.index()] = world;
        for child in node.children() {
            stack.push((child, world));
        }
    }
    out
}
