//! Reads a GLB or glTF file and measures what the inbox checks need:
//! counts, texture sizes, the rig, and two geometry heuristics.
//!
//! **Merged objects.** Within one primitive, vertices at the same position
//! are welded and triangles are joined into connected islands. Islands
//! whose boxes sit within [`GROUP_GAP`] of the primitive's diagonal of each
//! other are one object (a lid resting on a crate is still one crate); two
//! or more groups that each hold at least [`GROUP_MIN_SHARE`] of the
//! triangles look like separate objects that were joined into one mesh.
//!
//! **Hair weighted to finger bones.** For every skinned vertex, the joint
//! with the largest weight is its dominant joint. When that joint is a hand
//! or finger bone (Mixamo names) but the vertex sits above the head joint,
//! or more than [`FINGER_REACH`] of the model height from that hand's chain
//! (forearm to hand to fingers) without hanging below the wrist (at most
//! [`HANG_REACH`] under it), it is flagged: long hair and tails near the
//! hands often get auto weighted to the fingers and then fly around with
//! them, while sleeve ends and wrist chains rightly follow the hand.

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use gltf::buffer::Source as BufferSource;
use gltf::mesh::Mode;
use serde::{Deserialize, Serialize};

use crate::math::{self, Aabb, IDENTITY};
use crate::vrm::{BoneOverrides, VrmReadiness, readiness_of};
use maquette_base::error::{CoreError, CoreResult};

mod fingers;
mod geometry;
mod islands;
mod rig;
mod source;

pub use geometry::bounds;
pub use geometry::world_matrices;
pub use rig::{bare_joint_name, is_hand_joint};

use fingers::finger_weights;
pub(crate) use geometry::primitive_corners;
use islands::primitive_islands;
use rig::rig_kind;
pub use source::load_uri;
use source::textures;

/// Islands closer than this share of the primitive's diagonal are grouped.
pub const GROUP_GAP: f32 = 0.1;

/// A group needs this share of the primitive's triangles to count.
pub const GROUP_MIN_SHARE: f32 = 0.01;

/// How far, as a share of model height, a vertex may sit from its hand's
/// chain before it is flagged.
pub const FINGER_REACH: f32 = 0.15;

/// How far, as a share of model height, a vertex may hang below its wrist
/// (a sleeve end, a cuff, a chain) and still belong to the hand.
pub const HANG_REACH: f32 = 0.2;

/// Islands considered per primitive, largest first, to bound the work.
const MAX_ISLANDS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RigKind {
    None,
    Mixamo,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TextureInfo {
    pub name: String,
    pub mime: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bytes: f64,
    /// Material slots that use it: "base color", "normal" and so on.
    pub uses: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PbrMaps {
    pub base_color: bool,
    pub metallic_roughness: bool,
    pub normal: bool,
    pub occlusion: bool,
    pub emissive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct JointCount {
    pub name: String,
    pub vertices: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FingerWeights {
    pub vertices: u32,
    /// The finger and hand joints holding flagged vertices, most first.
    pub joints: Vec<JointCount>,
    /// The distance used, in model units.
    pub reach: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Islands {
    /// The primitive with the most groups.
    pub mesh: String,
    pub islands: u32,
    pub groups: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GlbReport {
    pub bytes: f64,
    pub generator: Option<String>,
    pub triangles: u32,
    pub vertices: u32,
    pub meshes: u32,
    pub primitives: u32,
    pub materials: u32,
    pub textures: Vec<TextureInfo>,
    /// The longest texture edge, 0 without textures.
    pub max_texture: u32,
    pub skins: u32,
    pub joints: u32,
    pub joint_names: Vec<String>,
    pub rig: RigKind,
    pub animations: Vec<String>,
    /// Width, height and depth of the default scene.
    pub size: [f32; 3],
    /// Its bounds, metres (Y up); zero without geometry.
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// Nodes that draw a mesh.
    pub objects: u32,
    /// Of those, the ones whose transform is not the identity.
    pub transformed: u32,
    pub pbr: PbrMaps,
    pub extensions: Vec<String>,
    /// False for meshopt or Draco compressed geometry, where the heuristics
    /// below are skipped.
    pub geometry_readable: bool,
    pub islands: Option<Islands>,
    pub finger_weights: Option<FingerWeights>,
    /// How far a rigged model is from a VRM 1.0 file; `None` without a skin.
    #[serde(default)]
    pub vrm: Option<VrmReadiness>,
}

/// Analyses the model at `path`.
///
/// # Errors
///
/// Fails when the file cannot be read or is not glTF.
pub fn analyse(path: &Path) -> CoreResult<GlbReport> {
    let bytes = fs::read(path).map_err(|e| CoreError::io(path, e))?;
    let base = path.parent().unwrap_or(Path::new("."));
    // The parser trusts indices it has not validated and can panic on a
    // malformed file; that is a bad file, not a crash.
    catch_unwind(AssertUnwindSafe(|| analyse_bytes(&bytes, base))).unwrap_or_else(|_| {
        Err(CoreError::Invalid(
            "This file is not a well formed glTF model.".to_string(),
        ))
    })
}

/// Analyses model bytes; external files resolve against `base`.
///
/// # Errors
///
/// Fails when the bytes are not glTF.
pub fn analyse_bytes(bytes: &[u8], base: &Path) -> CoreResult<GlbReport> {
    let gltf = gltf::Gltf::from_slice_without_validation(bytes)
        .map_err(|e| CoreError::Invalid(format!("This is not a glTF model. {e}")))?;
    let doc = &gltf.document;
    let buffers: Vec<Option<Vec<u8>>> = doc
        .buffers()
        .map(|buffer| match buffer.source() {
            BufferSource::Bin => gltf.blob.clone(),
            BufferSource::Uri(uri) => load_uri(uri, base),
        })
        .collect();
    let extensions: Vec<String> = doc.extensions_used().map(str::to_string).collect();
    let compressed = extensions.iter().any(|e| {
        e == "EXT_meshopt_compression"
            || e == "KHR_meshopt_compression"
            || e == "KHR_draco_mesh_compression"
    });

    let worlds = world_matrices(doc);
    let mut bounds = Aabb::EMPTY;
    let (mut triangles, mut vertices, mut primitives) = (0u64, 0u64, 0u32);
    let mut islands: Option<Islands> = None;
    let (mut objects, mut transformed) = (0u32, 0u32);
    for node in doc.nodes() {
        let Some(mesh) = node.mesh() else { continue };
        let world = worlds.get(node.index()).copied().unwrap_or(IDENTITY);
        objects += 1;
        if world
            .iter()
            .flatten()
            .zip(IDENTITY.iter().flatten())
            .any(|(a, b)| (a - b).abs() > 1e-6)
        {
            transformed += 1;
        }
        for prim in mesh.primitives() {
            primitives += 1;
            let count = prim
                .get(&gltf::Semantic::Positions)
                .map_or(0, |a| a.count()) as u64;
            vertices += count;
            let elements = prim.indices().map_or(count, |a| a.count() as u64);
            triangles += match prim.mode() {
                Mode::Triangles => elements / 3,
                Mode::TriangleStrip | Mode::TriangleFan => elements.saturating_sub(2),
                _ => 0,
            };
            for corner in primitive_corners(&prim) {
                bounds.add(math::point(&world, corner));
            }
            if !compressed && prim.mode() == Mode::Triangles {
                let name = mesh.name().or(node.name()).unwrap_or("mesh").to_string();
                if let Some(found) = primitive_islands(&prim, &buffers, &name)
                    && islands.as_ref().is_none_or(|i| found.groups > i.groups)
                {
                    islands = Some(found);
                }
            }
        }
    }

    let (textures, pbr) = textures(doc, &buffers, base);
    let max_texture = textures
        .iter()
        .map(|t| t.width.unwrap_or(0).max(t.height.unwrap_or(0)))
        .max()
        .unwrap_or(0);
    let joint_names: Vec<String> = {
        let mut seen = Vec::new();
        for skin in doc.skins() {
            for joint in skin.joints() {
                let name = joint.name().unwrap_or("joint").to_string();
                if !seen.contains(&name) {
                    seen.push(name);
                }
            }
        }
        seen
    };
    let rig = rig_kind(&joint_names);
    let finger_weights = if compressed {
        None
    } else {
        finger_weights(doc, &buffers, &worlds)
    };

    let vrm =
        (doc.skins().next().is_some()).then(|| readiness_of(doc, &buffers, &BoneOverrides::new()));

    Ok(GlbReport {
        bytes: bytes.len() as f64,
        generator: doc_generator(&gltf),
        triangles: u32::try_from(triangles).unwrap_or(u32::MAX),
        vertices: u32::try_from(vertices).unwrap_or(u32::MAX),
        meshes: count(doc.meshes().len()),
        primitives,
        materials: count(doc.materials().len()),
        max_texture,
        textures,
        skins: count(doc.skins().len()),
        joints: count(joint_names.len()),
        joint_names,
        rig,
        animations: doc
            .animations()
            .enumerate()
            .map(|(i, a)| {
                a.name()
                    .map_or_else(|| format!("Animation {}", i + 1), str::to_string)
            })
            .collect(),
        size: bounds.size(),
        min: if bounds.is_empty() {
            [0.0; 3]
        } else {
            bounds.min
        },
        max: if bounds.is_empty() {
            [0.0; 3]
        } else {
            bounds.max
        },
        objects,
        transformed,
        pbr,
        extensions,
        geometry_readable: !compressed,
        islands,
        finger_weights,
        vrm,
    })
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn doc_generator(gltf: &gltf::Gltf) -> Option<String> {
    gltf.document.as_json().asset.generator.clone()
}

#[cfg(test)]
mod tests;
