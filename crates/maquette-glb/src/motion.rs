//! What a model file holds that moves, read without Blender: its rig
//! (skins and joints, mapped onto a humanoid skeleton), its animation clips
//! (length, key rate, keyframe bytes, root travel, whether it loops) and
//! its parts (the nodes that draw a mesh, which object states move).

mod clips;
mod rig;

use std::fs;
use std::path::Path;

use maquette_base::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

use crate::analyse::{self, RigKind};

pub use clips::{clip_bytes, json_of};
pub use rig::{BoneMap, BoneMatch, Rig, WEB_INFLUENCES, humanoid_map};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    /// The animation's name in the file, or "Animation 2" without one.
    pub name: String,
    /// Its place among the file's animations.
    pub index: u32,
    /// Seconds from the first key to the last.
    pub duration: f32,
    /// Keys on its longest channel.
    pub keys: u32,
    /// Frames from the first key to the last, at `fps`.
    pub frames: u32,
    /// The rate its keys were sampled at.
    pub fps: f32,
    pub channels: u32,
    /// How many nodes it moves.
    pub targets: u32,
    /// Keyframe bytes in the source model.
    pub bytes: f64,
    /// Keyframe bytes in the file the model is weighed by, when that is a
    /// build (shipped or raw); `None` when the build has no such clip yet.
    pub shipped_bytes: Option<f64>,
    /// How far the root moves over the ground from the first key to the
    /// last, in metres; `None` for a clip that plays in place.
    pub root_travel: Option<f32>,
    /// The last key matches the first (root travel aside); `None` when the
    /// keys are compressed and could not be read.
    pub loops: Option<bool>,
}

/// What a model file holds that moves.
#[derive(Debug, Clone, PartialEq)]
pub struct FileMotion {
    pub rig: Option<Rig>,
    pub clips: Vec<Clip>,
    /// The nodes that draw a mesh, in file order, each name once.
    pub parts: Vec<String>,
}

fn parse(bytes: &[u8], base: &Path) -> CoreResult<FileMotion> {
    let gltf = gltf::Gltf::from_slice_without_validation(bytes)
        .map_err(|e| CoreError::Invalid(format!("This is not a glTF model. {e}")))?;
    let doc = &gltf.document;
    let buffers: Vec<Option<Vec<u8>>> = doc
        .buffers()
        .map(|b| match b.source() {
            gltf::buffer::Source::Bin => gltf.blob.clone(),
            gltf::buffer::Source::Uri(uri) => analyse::load_uri(uri, base),
        })
        .collect();
    let json = clips::json_of(bytes).unwrap_or_default();
    let (sizes, _) = clips::clip_bytes(&json);
    // Meshopt packs any buffer, keys too; Draco only geometry.
    let packed = doc.extensions_used().any(|e| e.contains("meshopt"));
    let compressed = packed || doc.extensions_used().any(|e| e.contains("draco"));
    let mut joint_names: Vec<String> = Vec::new();
    for skin in doc.skins() {
        for joint in skin.joints() {
            let name = joint.name().unwrap_or("joint").to_string();
            if !joint_names.contains(&name) {
                joint_names.push(name);
            }
        }
    }
    let rig = (!joint_names.is_empty()).then(|| {
        let map = rig::humanoid_map(&joint_names);
        Rig {
            joints: u32::try_from(joint_names.len()).unwrap_or(u32::MAX),
            kind: analyse_kind(&joint_names),
            humanoid: rig::is_humanoid(&map),
            map,
            max_influences: rig::max_influences(doc, &buffers, !compressed),
            joint_names,
        }
    });
    let mut parts = Vec::new();
    for node in doc.nodes().filter(|n| n.mesh().is_some()) {
        let name = node
            .name()
            .map_or_else(|| format!("Part {}", node.index() + 1), str::to_string);
        if !parts.contains(&name) {
            parts.push(name);
        }
    }
    Ok(FileMotion {
        rig,
        clips: clips::read(doc, &buffers, &sizes, !packed),
        parts,
    })
}

fn analyse_kind(joints: &[String]) -> RigKind {
    // The Inbox's own test: enough Mixamo names.
    let mixamo = rig::humanoid_map(joints)
        .iter()
        .filter(|m| m.how == BoneMatch::Exact)
        .count();
    if mixamo >= 18 {
        RigKind::Mixamo
    } else {
        RigKind::Other
    }
}

/// Reads a model file for its rig, clips and parts.
///
/// # Errors
///
/// Fails when the file cannot be read or is not glTF.
pub fn read_file(path: &Path) -> CoreResult<FileMotion> {
    let bytes = fs::read(path).map_err(|e| CoreError::io(path, e))?;
    let base = path.parent().unwrap_or(Path::new("."));
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse(&bytes, base))).unwrap_or_else(
        |_| {
            Err(CoreError::Invalid(
                "This file is not a well formed glTF model.".to_string(),
            ))
        },
    )
}

/// The rig and clips of a model file, read without Blender.
///
/// # Errors
///
/// As [`read_file`].
pub fn rig_and_clips(path: &Path) -> CoreResult<(Option<Rig>, Vec<Clip>)> {
    read_file(path).map(|f| (f.rig, f.clips))
}

#[cfg(test)]
mod tests;
