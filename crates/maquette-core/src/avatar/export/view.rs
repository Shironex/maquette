//! What the Avatar view and `get_avatar` read for one asset: the spec, the
//! readiness of the version, its last reweight and normalised rest pose,
//! its springs, and the last export.

use std::fs;
use std::path::Path;

use serde::Serialize;

use super::springs::{self, SpringsView};
use super::{AvatarExport, AvatarRequest, RECORD_FILE, asset_dir, source, spec_for, vrm_file};
use crate::AssetKind;
use crate::avatar::AvatarSpec;
use crate::avatar::normalize::{self, Normalized, needs_normalize};
use crate::avatar::reweight::{self, Reweighted};
use crate::error::{CoreError, CoreResult};
use crate::inbox::vrm::{self, BoneOverrides, VrmReadiness};

/// Everything the Avatar view and `get_avatar` show for one asset.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AvatarView {
    pub project: String,
    pub asset: String,
    pub name: String,
    pub kind: AssetKind,
    /// The version read, `None` without a model.
    pub version: Option<u32>,
    /// That version's model file, an absolute path.
    pub model: Option<String>,
    pub spec: AvatarSpec,
    /// The spec is stored in `avatar.json`; otherwise it comes from the
    /// workspace's avatar defaults and is saved on the first change.
    pub stored: bool,
    pub readiness: Option<VrmReadiness>,
    /// Why the readiness could not be read.
    pub readiness_error: Option<String>,
    /// The rest pose is not VRM's yet: a build normalises it and waits for
    /// a person's yes.
    pub needs_normalize: bool,
    /// The readiness finds cloth or hair weighted to the arms and the arms
    /// are lifted: a build frees them first and waits for a person's yes.
    pub needs_reweight: bool,
    /// The last reweight of this version, with its sheets.
    pub reweighted: Option<Reweighted>,
    /// The last normalised rest pose of this version, with its sheets.
    pub normalized: Option<Normalized>,
    /// The parts that could swing, found on the model the VRM is written
    /// from, and the last chains made for the confirmed ones.
    pub springs: SpringsView,
    pub export: Option<AvatarExport>,
    /// Where a build writes the `.vrm`, an absolute path.
    pub export_path: String,
    /// What writing would refuse in the meta (no name, no author).
    pub meta_problem: Option<String>,
}

fn read_export(dir: &Path) -> Option<AvatarExport> {
    let text = fs::read_to_string(dir.join(RECORD_FILE)).ok()?;
    let mut found: AvatarExport = serde_json::from_str(&text).ok()?;
    found.exists = Path::new(&found.path).is_file();
    found.report = found.report.filter(|r| Path::new(r).is_file());
    Some(found)
}

/// Reads one asset's avatar: the spec, the readiness of the version, the
/// last normalised rest pose and the last export.
///
/// # Errors
///
/// Fails for an unknown project, asset or version, or an `avatar.json`
/// that cannot be read.
pub fn view(req: &AvatarRequest<'_>) -> CoreResult<AvatarView> {
    let src = source(req)?;
    let avatars = crate::avatar::load(&src.project_dir)?;
    let (spec, stored) = spec_for(&avatars, req.config, &src.asset);
    let (readiness, readiness_error) = match &src.file {
        Some(file) => match readiness_of(file, &spec.bones) {
            Ok(r) => (Some(r), None),
            Err(e) => (None, Some(e.to_string())),
        },
        None => (None, Some("This asset has no model yet.".to_string())),
    };
    let dir = asset_dir(req.workspace_root, req.config, req.project, &src.asset.id);
    let version_dir = src.version.map(|v| dir.join(format!("v{v}")));
    let normalized = match &version_dir {
        Some(d) => normalize::load(d)?,
        None => None,
    };
    let reweighted = match &version_dir {
        Some(d) => reweight::load(&reweight::work_dir(d))?,
        None => None,
    };
    Ok(AvatarView {
        project: req.project.to_string(),
        asset: src.asset.id.clone(),
        name: src.asset.name.clone(),
        kind: src.asset.kind,
        version: src.version,
        model: src.file.as_ref().map(|f| f.display().to_string()),
        needs_normalize: readiness.as_ref().is_some_and(needs_normalize),
        needs_reweight: readiness.as_ref().is_some_and(lifts_cloth),
        readiness,
        reweighted,
        readiness_error,
        normalized,
        springs: springs::view(req),
        export: read_export(&dir),
        export_path: vrm_file(&src.project_dir, &src.asset.id)
            .display()
            .to_string(),
        meta_problem: spec.meta.validate().err().map(|e| e.to_string()),
        spec,
        stored,
    })
}

/// The arms are lifted and cloth or hair hangs on them.
pub(super) fn lifts_cloth(readiness: &VrmReadiness) -> bool {
    needs_normalize(readiness) && reweight::needs_reweight(readiness)
}

pub(super) fn readiness_of(file: &Path, overrides: &BoneOverrides) -> CoreResult<VrmReadiness> {
    let bytes = fs::read(file).map_err(|e| CoreError::io(file, e))?;
    vrm::readiness(&bytes, file.parent().unwrap_or(Path::new(".")), overrides)
}
