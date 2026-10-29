//! VRM 1.0 readiness, read without Blender: how a rigged model's joints map
//! onto the VRM humanoid bones, what its rest pose measures against the VRM
//! T-pose, and the findings that stand between it and a VRM file.
//!
//! **Bones.** All 55 VRM humanoid bones (15 required), matched by name from
//! VRM's own and VRoid's names, Mixamo, Unreal and Tripo's own rig, with an
//! avatar spec's overrides on top ([`bones`]). Each mapped bone's nearest
//! humanoid ancestor in the node tree must be the one VRM's tree puts above
//! it; other nodes may sit between.
//!
//! **Rest pose.** The VRM T-pose faces +Z with the character's left on +X,
//! arms level along X, soles on Y = 0, the hips on the Y axis, both sides
//! mirrored, and every node scaled the same on each axis and positively
//! ([`pose`]).
//!
//! **Findings.** Blockers stop a VRM file being written as the model is;
//! warnings are what VRM asks for but a runtime copes without; notes say
//! what the export will drop or leave out. All of them are kept, blockers
//! first.

mod bones;
mod findings;
mod pose;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::analyse::{load_uri, world_matrices};
use maquette_base::error::{CoreError, CoreResult};

pub use bones::{BoneNaming, BoneOverrides, HUMAN_BONES, REQUIRED_BONES};
pub use findings::{ARM_SLACK_DEG, FACING_SLACK_DEG, GROUND_SLACK};
pub use pose::{Axis, RestPose, SCALE_NOISE};

/// How much a finding matters, most first.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum FindingLevel {
    /// Stops a VRM file being written from the model as it is.
    Blocker,
    /// VRM asks for it; runtimes cope without.
    Warning,
    /// What the export drops or leaves out.
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrmFinding {
    pub id: String,
    pub level: FindingLevel,
    pub title: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrmBone {
    /// The VRM name: `hips`, `leftUpperArm`.
    pub bone: String,
    pub required: bool,
    /// The joint (node name) it maps to.
    pub joint: Option<String>,
    /// The naming the joint matched by.
    pub naming: Option<BoneNaming>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrmReadiness {
    /// No blocker: a VRM file can be written from the model as it is.
    pub ready: bool,
    /// Bones mapped, of the 55.
    pub mapped: u32,
    /// Required bones mapped, of the 15.
    pub required: u32,
    /// Every humanoid bone, parents before children.
    pub bones: Vec<VrmBone>,
    pub pose: RestPose,
    /// Blockers first, then warnings, then notes.
    pub findings: Vec<VrmFinding>,
}

impl VrmReadiness {
    /// How many findings of `level` there are.
    pub fn count(&self, level: FindingLevel) -> usize {
        self.findings.iter().filter(|f| f.level == level).count()
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Every skin's joints, each once, as node index and name.
fn skin_joints(doc: &gltf::Document) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    for skin in doc.skins() {
        for joint in skin.joints() {
            if !out.iter().any(|(i, _)| *i == joint.index()) {
                out.push((joint.index(), node_name(&joint)));
            }
        }
    }
    out
}

fn node_name(node: &gltf::Node<'_>) -> String {
    node.name()
        .map_or_else(|| format!("node {}", node.index()), str::to_string)
}

/// Each node's parent.
pub(crate) fn parents(doc: &gltf::Document) -> Vec<Option<usize>> {
    let mut out = vec![None; doc.nodes().len()];
    for node in doc.nodes() {
        for child in node.children() {
            if let Some(slot) = out.get_mut(child.index()) {
                *slot = Some(node.index());
            }
        }
    }
    out
}

/// The readiness of a parsed model.
pub(crate) fn readiness_of(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    overrides: &BoneOverrides,
) -> VrmReadiness {
    let human = bones::human_bones();
    let joints = skin_joints(doc);
    let nodes: Vec<(usize, String)> = doc.nodes().map(|n| (n.index(), node_name(&n))).collect();
    let mapped = bones::map_joints(&human, &joints, &nodes, overrides);
    let wrong = bones::wrong_parents(&human, &mapped, &parents(doc));
    let worlds = world_matrices(doc);
    let pose = pose::measure(doc, buffers, &worlds, &mapped);
    let mut findings = findings::bone_findings(&findings::RigFacts {
        bones: &human,
        mapped: &mapped,
        wrong: &wrong,
        skinned: doc
            .nodes()
            .any(|n| n.skin().is_some() && n.mesh().is_some()),
        animations: doc.animations().len(),
    });
    findings.extend(findings::pose_findings(&pose));
    // Stable, so each level keeps the order the checks run in.
    findings.sort_by_key(|f| f.level);
    let required = human
        .iter()
        .filter(|b| b.required && mapped.iter().any(|m| m.bone == b.name))
        .count();
    let bones = human
        .iter()
        .map(|b| {
            let m = mapped.iter().find(|m| m.bone == b.name);
            VrmBone {
                bone: b.name.clone(),
                required: b.required,
                joint: m.and_then(|m| nodes.get(m.node)).map(|(_, n)| n.clone()),
                naming: m.map(|m| m.naming),
            }
        })
        .collect();
    VrmReadiness {
        ready: !findings.iter().any(|f| f.level == FindingLevel::Blocker),
        mapped: count(mapped.len()),
        required: count(required),
        bones,
        pose,
        findings,
    }
}

/// Reads the VRM readiness of model bytes; external files resolve against
/// `base`.
///
/// # Errors
///
/// Fails when the bytes are not glTF.
pub fn readiness(bytes: &[u8], base: &Path, overrides: &BoneOverrides) -> CoreResult<VrmReadiness> {
    let run = || {
        let gltf = gltf::Gltf::from_slice_without_validation(bytes)
            .map_err(|e| CoreError::Invalid(format!("This is not a glTF model. {e}")))?;
        let buffers: Vec<Option<Vec<u8>>> = gltf
            .document
            .buffers()
            .map(|b| match b.source() {
                gltf::buffer::Source::Bin => gltf.blob.clone(),
                gltf::buffer::Source::Uri(uri) => load_uri(uri, base),
            })
            .collect();
        Ok(readiness_of(&gltf.document, &buffers, overrides))
    };
    // The parser trusts indices it has not validated and can panic on a
    // malformed file; that is a bad file, not a crash.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or_else(|_| {
        Err(CoreError::Invalid(
            "This file is not a well formed glTF model.".to_string(),
        ))
    })
}

/// Registers the readiness types with the shell's TypeScript export.
pub(crate) fn register(types: specta::Types) -> specta::Types {
    types
        .register::<BoneNaming>()
        .register::<Axis>()
        .register::<RestPose>()
        .register::<FindingLevel>()
        .register::<VrmFinding>()
        .register::<VrmBone>()
        .register::<VrmReadiness>()
}

#[cfg(test)]
mod tests;
