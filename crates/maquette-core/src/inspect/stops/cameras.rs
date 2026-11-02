//! Stops from the cameras inside a scene's models (the fourth source in
//! [`super`]): a glTF camera has no target, so each looks along its view
//! axis as far ahead as the scene's centre, at least a metre.

use std::fs;
use std::path::Path;

use super::{CameraStopDef, DEFAULT_FOV, stop_label};
use crate::inbox::math::{self, Mat4};

/// Camera nodes of one glTF file as stops looking at `center`'s depth.
pub fn cameras_in(file: &Path, center: [f32; 3]) -> Vec<CameraStopDef> {
    let Ok(bytes) = fs::read(file) else {
        return Vec::new();
    };
    let Ok(gltf) = gltf::Gltf::from_slice_without_validation(&bytes) else {
        return Vec::new();
    };
    let doc = &gltf.document;
    let worlds = crate::inbox::analyse::world_matrices(doc);
    doc.nodes()
        .filter_map(|node| {
            let camera = node.camera()?;
            let world: Mat4 = *worlds.get(node.index())?;
            let position = math::translation(&world);
            // A glTF camera looks down its local -Z.
            let ahead = math::point(&world, [0.0, 0.0, -1.0]);
            let mut forward = [
                ahead[0] - position[0],
                ahead[1] - position[1],
                ahead[2] - position[2],
            ];
            let len = math::distance(ahead, position);
            if len < 1e-6 {
                return None;
            }
            forward.iter_mut().for_each(|c| *c /= len);
            let to_center = [
                center[0] - position[0],
                center[1] - position[1],
                center[2] - position[2],
            ];
            let depth =
                (to_center[0] * forward[0] + to_center[1] * forward[1] + to_center[2] * forward[2])
                    .max(1.0);
            let fov = match camera.projection() {
                gltf::camera::Projection::Perspective(p) => p.yfov().to_degrees(),
                gltf::camera::Projection::Orthographic(_) => DEFAULT_FOV,
            };
            let name = node
                .name()
                .or(camera.name())
                .map_or_else(|| format!("Camera {}", node.index() + 1), stop_label);
            Some(CameraStopDef {
                name,
                position,
                target: [
                    position[0] + forward[0] * depth,
                    position[1] + forward[1] * depth,
                    position[2] + forward[2] * depth,
                ],
                fov,
                poster: None,
            })
        })
        .collect()
}
