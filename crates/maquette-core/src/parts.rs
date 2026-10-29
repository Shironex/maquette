//! An asset's parts: the meshes its model is drawn with, by the names the
//! export gives them. Object states, explode tracks and the Inspector's
//! Parts tab pick parts by these names; a split fix-up or a provider makes
//! them, and `prepare.json` renames and joins them ([`PartsPlan`]).
//!
//! The parts of a model are read from its file, one per node that draws a
//! mesh, without Blender: a model the split fix-up cuts is read from the
//! fix-up preview's copy. Compare reads the numbers of two versions side
//! by side ([`numbers`]): their parts, draw calls, triangles, textures,
//! transfer size and Inbox checks.
//!
//! A split on a provider can drop the texture: [`bake_back`] bakes it back
//! from the model it came from. [`assemble`] does the opposite of a split,
//! joining parts made as models of their own into one prop.

pub mod assemble;
pub mod bake_back;
mod blender;
pub(crate) mod versions;

// Reading the parts from the model file lives in the glb crate.
pub use maquette_glb::parts::{PartInfo, read, read_bytes};

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::{self, AssetKind};
use crate::error::{CoreError, CoreResult};
use crate::game::FolderBudget;
use crate::inbox::analyse;
use crate::inbox::checks::{self, CheckStatus, SendNotes};
use crate::prepare::{Fixup, ModelPrep, PartsPlan};
use crate::project::ProjectManifest;
use crate::workspace::WorkspaceConfig;

/// One part as the Inspector lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PartRow {
    /// The name in the model, which the plan keys by.
    pub id: String,
    /// The name it ships as.
    pub name: String,
    pub triangles: u32,
    pub materials: Vec<String>,
    /// Drawn as a ghost in the Inspector; it still ships.
    pub hidden: bool,
    /// The parts joined into this one (their ids), when it is a group's first.
    pub joined: Vec<String>,
}

/// Whether `prep`'s fix-ups change which parts the model has, so its parts
/// are read from the fix-up preview's copy instead of the file.
pub fn changes_parts(prep: &ModelPrep) -> bool {
    prep.fixups
        .iter()
        .any(|f| matches!(f, Fixup::SplitParts { .. } | Fixup::Merge { .. }))
}

/// The plan a preview runs to list the parts by their model names: the
/// fix-ups alone, without the real size and the part names.
pub fn listing_plan(prep: &ModelPrep) -> ModelPrep {
    ModelPrep {
        fixups: prep.fixups.clone(),
        ..ModelPrep::default()
    }
}

/// The parts as they ship under `plan`: joined groups as one row under
/// their first part, each with its new name and whether it is hidden.
pub fn rows(found: &[PartInfo], plan: Option<&PartsPlan>) -> Vec<PartRow> {
    let empty = PartsPlan::default();
    let plan = plan.unwrap_or(&empty);
    let mut out: Vec<PartRow> = Vec::new();
    for part in found {
        if let Some(first) = plan.joined_into(&part.name).filter(|f| *f != part.name)
            && found.iter().any(|p| p.name == first)
        {
            continue;
        }
        let mut row = PartRow {
            id: part.name.clone(),
            name: plan.name_of(&part.name).to_string(),
            triangles: part.triangles,
            materials: part.materials.clone(),
            hidden: plan.hidden.contains(&part.name),
            joined: Vec::new(),
        };
        let group = plan.merge.iter().find(|g| g.first() == Some(&part.name));
        for other in group.into_iter().flatten().skip(1) {
            let Some(found) = found.iter().find(|p| &p.name == other) else {
                continue;
            };
            row.triangles += found.triangles;
            for m in &found.materials {
                if !row.materials.contains(m) {
                    row.materials.push(m.clone());
                }
            }
            row.joined.push(other.clone());
        }
        out.push(row);
    }
    out
}

/// How many of a model's Inbox checks pass, warn and fail.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CheckCounts {
    pub pass: u32,
    pub warning: u32,
    pub fail: u32,
}

/// One side of a compare: what a version (or its fixed copy) ships with.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelParts {
    /// The parts Motion can move, by the names they ship as. Empty for a
    /// model that is one mesh: it moves only as a whole.
    pub parts: Vec<String>,
    pub draw_calls: u32,
    pub triangles: u32,
    pub textures: u32,
    /// The longest texture edge in pixels, 0 without textures.
    pub max_texture: u32,
    /// The file's size: what a page or game loads.
    pub bytes: f64,
    pub checks: CheckCounts,
}

/// The numbers of the model at `path` as an asset of `kind` checks them,
/// its parts named by `plan` (null when the file already has the names).
///
/// # Errors
///
/// Fails when the file cannot be read or is not glTF.
pub fn numbers(
    path: &Path,
    kind: AssetKind,
    config: &WorkspaceConfig,
    folder: Option<&FolderBudget>,
    plan: Option<&PartsPlan>,
) -> CoreResult<ModelParts> {
    let report = analyse::analyse(path)?;
    let found = read(path)?;
    let list = checks::run(&report, kind, config, SendNotes::default(), folder);
    let mut counts = CheckCounts::default();
    for check in &list {
        match check.status {
            CheckStatus::Pass => counts.pass += 1,
            CheckStatus::Warning => counts.warning += 1,
            CheckStatus::Fail => counts.fail += 1,
        }
    }
    let shipped = rows(&found, plan);
    Ok(ModelParts {
        parts: if shipped.len() >= 2 {
            shipped.into_iter().map(|r| r.name).collect()
        } else {
            Vec::new()
        },
        draw_calls: found.iter().map(|p| p.draw_calls).sum(),
        triangles: report.triangles,
        textures: u32::try_from(report.textures.len()).unwrap_or(u32::MAX),
        max_texture: report.max_texture,
        bytes: report.bytes,
        checks: counts,
    })
}

/// Makes `version` the asset's current one: it moves to the end of the
/// list, so the Board, the export and the next build use it. Its number
/// stays; every version stays in the asset.
///
/// # Errors
///
/// Fails for an unknown asset or version.
pub fn keep_version(manifest: &mut ProjectManifest, asset: &str, version: u32) -> CoreResult<()> {
    let id = assets::find(manifest, asset)?.id.clone();
    let Some(found) = manifest.assets.iter_mut().find(|a| a.id == id) else {
        return Err(CoreError::Invalid(format!("No asset {asset}.")));
    };
    let Some(at) = found.versions.iter().position(|v| v.version == version) else {
        return Err(CoreError::Invalid(format!(
            "{} has no v{version}.",
            found.name
        )));
    };
    let kept = found.versions.remove(at);
    found.versions.push(kept);
    Ok(())
}

#[cfg(test)]
mod tests;
