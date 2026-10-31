//! The read, which needs no Blender: which route a model takes, what stops
//! it, and the facts the doctor's left column shows.

use std::path::Path;

use super::{RigIssue, RigModel, RigRoute};
use crate::error::{CoreError, CoreResult};
use crate::inbox::analyse::{self, GlbReport, RigKind, RigProbe};

/// The route of a rigged report; `None` without a skin.
pub fn route_of(report: &GlbReport) -> Option<RigRoute> {
    match report.rig {
        RigKind::None => None,
        RigKind::Mixamo => Some(RigRoute::Humanoid),
        _ => Some(RigRoute::Creature),
    }
}

fn issue(ok: bool, title: impl Into<String>, detail: impl Into<String>) -> RigIssue {
    RigIssue {
        ok,
        title: title.into(),
        detail: detail.into(),
    }
}

/// Why the scripts cannot work on this model, with what was fine about it.
fn issues(report: &GlbReport, probe: Option<&RigProbe>) -> Vec<RigIssue> {
    let mut out = Vec::new();
    if !report.geometry_readable {
        out.push(issue(
            false,
            "Compressed geometry",
            "Draco or meshopt data cannot be patched byte for byte. Add the file before it is compressed.",
        ));
    }
    let Some(probe) = probe else {
        out.push(issue(
            false,
            "No skin",
            "The file has no skeleton with weights. Redo it as rigged.",
        ));
        return out;
    };
    if report.geometry_readable && probe.skinned_primitives == 0 {
        out.push(issue(
            false,
            "No skin",
            "The file has bones but no weights. Redo it as rigged.",
        ));
        return out;
    }
    if probe.skinned_primitives > 1 {
        out.push(issue(
            false,
            format!("{} skinned meshes", probe.skinned_primitives),
            format!(
                "{} are skinned apart. The scripts fix one skinned mesh at a time.",
                probe.skinned_meshes.join(", ")
            ),
        ));
    }
    if probe.interleaved {
        out.push(issue(
            false,
            "Interleaved vertex buffers",
            "The weights cannot be patched byte for byte, so v1 could change in ways the checks cannot prove.",
        ));
    }
    if !out.is_empty() {
        let name = if report.rig == RigKind::Mixamo {
            "Mixamo rig found"
        } else {
            "Its own rig found"
        };
        out.push(issue(
            true,
            name,
            format!("{} joints, names read.", report.joints),
        ));
    }
    out
}

fn finding(report: &GlbReport, probe: Option<&RigProbe>) -> Option<String> {
    if let Some(fw) = report.finger_weights.as_ref().filter(|f| f.vertices > 0) {
        let names: Vec<String> = fw
            .joints
            .iter()
            .take(2)
            .map(|j| format!("{} ({})", analyse::bare_joint_name(&j.name), j.vertices))
            .collect();
        return Some(format!(
            "Hair vertices weighted to finger bones. {} vertices far from the hands follow {}.",
            fw.vertices,
            names.join(" and ")
        ));
    }
    if let Some(p) = probe.filter(|p| p.arm_far > 0) {
        return Some(format!(
            "Cloth or hair weighted to arm bones. {} vertices sit far from the arms that drive them.",
            p.arm_far
        ));
    }
    if report.rig != RigKind::Mixamo && report.rig != RigKind::None {
        let clips = if report.animations.is_empty() {
            ", no clips"
        } else {
            ""
        };
        return Some(format!(
            "Not a Mixamo rig. {} bones with their own names{clips}.",
            report.joints
        ));
    }
    None
}

/// Reads the model at `file`: what the left column shows, the route and what
/// refuses it.
///
/// # Errors
///
/// Fails when the file is not a glTF model.
pub fn read_model(file: &Path) -> CoreResult<(RigModel, Option<RigRoute>, Vec<RigIssue>)> {
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !ext.eq_ignore_ascii_case("glb") {
        return Err(CoreError::Invalid(
            "The rig doctor reads a .glb file. Export it as a single .glb first.".to_string(),
        ));
    }
    let report = analyse::analyse(file)?;
    let probe = report.rig_probe.as_ref();
    let rig = match report.rig {
        RigKind::Mixamo => format!("Mixamo, {} joints", report.joints),
        RigKind::None => "no skeleton".to_string(),
        _ => format!("own names, {} joints", report.joints),
    };
    let model = RigModel {
        rig,
        joints: report.joints,
        skinned_meshes: probe.map_or(0, |p| p.skinned_primitives),
        clips: report.animations.clone(),
        triangles: report.triangles,
        height: report.size[1],
        textures: u32::try_from(report.textures.len()).unwrap_or(u32::MAX),
        hand_far: probe.map_or(0, |p| p.hand_far),
        arm_far: probe.map_or(0, |p| p.arm_far),
        islands: probe.map_or(0, |p| p.islands),
        one_island: probe.is_some_and(|p| p.one_island),
        finding: finding(&report, probe),
    };
    Ok((model, route_of(&report), issues(&report, probe)))
}
