//! The parts of a model, as its file has them: one per node that draws a
//! mesh, read without Blender.

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use gltf::mesh::Mode;
use maquette_base::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

/// One part as the model file has it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PartInfo {
    /// The node's name, or `part_<n>` for a node without one.
    pub name: String,
    pub triangles: u32,
    /// Its materials' names, in primitive order, each once.
    pub materials: Vec<String>,
    /// One per primitive.
    pub draw_calls: u32,
}

/// The parts of the model at `path`, in node order.
///
/// # Errors
///
/// Fails when the file cannot be read or is not glTF.
pub fn read(path: &Path) -> CoreResult<Vec<PartInfo>> {
    let bytes = fs::read(path).map_err(|e| CoreError::io(path, e))?;
    catch_unwind(AssertUnwindSafe(|| read_bytes(&bytes))).unwrap_or_else(|_| {
        Err(CoreError::Invalid(
            "This file is not a well formed glTF model.".to_string(),
        ))
    })
}

/// The parts of model bytes, in node order.
///
/// # Errors
///
/// Fails when the bytes are not glTF.
pub fn read_bytes(bytes: &[u8]) -> CoreResult<Vec<PartInfo>> {
    let gltf = gltf::Gltf::from_slice_without_validation(bytes)
        .map_err(|e| CoreError::Invalid(format!("This is not a glTF model. {e}")))?;
    let mut parts = Vec::new();
    for node in gltf.document.nodes() {
        let Some(mesh) = node.mesh() else { continue };
        let mut part = PartInfo {
            name: node
                .name()
                .or(mesh.name())
                .map_or_else(|| format!("part_{}", parts.len() + 1), str::to_string),
            triangles: 0,
            materials: Vec::new(),
            draw_calls: 0,
        };
        for prim in mesh.primitives() {
            part.draw_calls += 1;
            let count = prim
                .get(&gltf::Semantic::Positions)
                .map_or(0, |a| a.count());
            let elements = prim.indices().map_or(count, |a| a.count());
            let tris = match prim.mode() {
                Mode::Triangles => elements / 3,
                Mode::TriangleStrip | Mode::TriangleFan => elements.saturating_sub(2),
                _ => 0,
            };
            part.triangles += u32::try_from(tris).unwrap_or(u32::MAX);
            let material = prim.material().name().unwrap_or("material").to_string();
            if !part.materials.contains(&material) {
                part.materials.push(material);
            }
        }
        parts.push(part);
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    #[test]
    fn a_model_lists_one_part_per_mesh_node() {
        let parts = read_bytes(&fixture::messy_prop()).expect("parts");
        let found: Vec<(&str, u32, &[String])> = parts
            .iter()
            .map(|p| (p.name.as_str(), p.draw_calls, p.materials.as_slice()))
            .collect();
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].0, "Shade");
        assert_eq!(found[0].2, ["Shade".to_string()]);
        assert_eq!(found[1].0, "Base");
        assert_eq!(found[1].2, ["Metal".to_string()]);
        assert_eq!(parts[1].triangles, 12);
        assert!(parts.iter().all(|p| p.draw_calls == 1));
        let one = read_bytes(&fixture::merged_islands()).expect("parts");
        assert_eq!(one.len(), 1, "one mesh is one part");
    }
}
