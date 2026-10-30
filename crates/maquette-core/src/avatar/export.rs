//! Exporting a character asset as a VRM 1.0 avatar: what the Avatar view
//! and the avatar tools read ([`view`]), the patch `set_avatar` applies
//! ([`AvatarPatch`]), and the build ([`build`]): when the rest pose is not
//! VRM's yet, first free cloth and hair from the arms if the readiness
//! finds them there and wait for a person's yes to the new weights, then
//! normalise the rest pose and wait for a yes to the new pose, then write
//! the file.
//!
//! The `.vrm` is a derived export, never a new version: it is written only
//! to the project's `exports/<asset>.vrm`, over the last one, with a
//! validation report beside it ([`report`]). Each written file is read
//! back and checked ([`vrm::round_trip`]); a failure joins what the file
//! still lacks as a blocker. What the last build wrote (the file, its
//! size, the source version, what the file still lacks and the round trip)
//! is kept in the asset's avatar cache folder as `export.json`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::normalize::{
    self, AVATAR_DIR, Elbows, NormalizeRequest, NormalizeState, Normalized, needs_normalize,
};
use super::reweight::{self, ReweightMode, ReweightReport, ReweightRequest, Reweighted};
use super::{AvatarSpec, SceneAvatars};
use crate::Asset;
use crate::assets;
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::fs::write_atomic;
use crate::inbox::vrm::{self, MaterialMode, RoundTrip, VrmFinding, VrmReadiness, WriteSpec};
use crate::json::write_json;
use crate::pipeline::cache::hash_file;
use crate::pipeline::tools::Toolbox;
use crate::project::{self, now_rfc3339};
use crate::workspace::WorkspaceConfig;

/// The project folder exports are written into.
pub const EXPORTS_DIR: &str = "exports";
const RECORD_FILE: &str = "export.json";

mod patch;
pub mod report;

mod view;

pub use patch::{AvatarPatch, apply_patch};
use report::{AvatarReport, ReportSource, bone_map, report_file, round_trip_finding};
pub use view::{AvatarView, view};
use view::{lifts_cloth, readiness_of};

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

/// `exports/<asset>.vrm` inside the project folder.
pub fn vrm_file(project_dir: &Path, asset: &str) -> PathBuf {
    project_dir.join(EXPORTS_DIR).join(format!("{asset}.vrm"))
}

/// `<workspace cache>/avatar/<project>/<asset>`, which holds a folder per
/// normalised version and the last export's record.
fn asset_dir(root: &Path, config: &WorkspaceConfig, project: &str, asset: &str) -> PathBuf {
    root.join(&config.layout.cache)
        .join(AVATAR_DIR)
        .join(project)
        .join(asset)
}

/// What the last build wrote.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AvatarExport {
    /// The `.vrm`, an absolute path.
    pub path: String,
    pub bytes: f64,
    pub source_version: u32,
    /// Written from a normalised rest pose (else from the model as it is).
    pub normalized: bool,
    /// The arms were lifted from an approved reweight that freed cloth and
    /// hair from them. False for a build that predates the reweight.
    #[serde(default)]
    pub reweighted: bool,
    pub material: MaterialMode,
    /// What the file still lacks, blockers first.
    pub unresolved: Vec<VrmFinding>,
    pub written_at: String,
    /// The file is still there. Read, never stored.
    #[serde(default)]
    pub exists: bool,
    /// The validation report beside the file, an absolute path; `None`
    /// when it is gone or the build predates it.
    #[serde(default)]
    pub report: Option<String>,
    /// The written bytes read back and checked; `None` for a build that
    /// predates the check.
    #[serde(default)]
    pub round_trip: Option<RoundTrip>,
}

/// Which asset to build, and how.
pub struct AvatarRequest<'a> {
    pub workspace_root: &'a Path,
    pub config: &'a WorkspaceConfig,
    pub project: &'a str,
    pub asset: &'a str,
    /// The newest version when `None`.
    pub version: Option<u32>,
    pub elbows: Elbows,
    pub reweight: ReweightMode,
}

/// The asset and the version a request names.
struct Source {
    project_dir: PathBuf,
    asset: Asset,
    version: Option<u32>,
    file: Option<PathBuf>,
}

fn source(req: &AvatarRequest<'_>) -> CoreResult<Source> {
    let manifest = project::load(req.workspace_root, &req.config.layout, req.project)?;
    let project_dir = project::dir(req.workspace_root, &req.config.layout, req.project)?;
    let asset = assets::find(&manifest, req.asset)?.clone();
    let found = match req.version {
        Some(n) => Some(
            asset
                .versions
                .iter()
                .find(|v| v.version == n)
                .ok_or_else(|| CoreError::NotFound(format!("version {n} of {}", asset.name)))?,
        ),
        None => asset.latest(),
    };
    let (version, file) = found.map_or((None, None), |v| {
        (Some(v.version), Some(project_dir.join(&v.file)))
    });
    Ok(Source {
        project_dir,
        asset,
        version,
        file,
    })
}

/// The asset's stored spec, or a new one from the workspace defaults.
/// True when it was stored.
pub fn spec_for(
    avatars: &SceneAvatars,
    config: &WorkspaceConfig,
    asset: &Asset,
) -> (AvatarSpec, bool) {
    avatars.avatars.get(&asset.id).map_or_else(
        || (config.avatar.spec(&asset.name), false),
        |spec| (spec.clone(), true),
    )
}

/// What a build would do, without doing it.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AvatarPlan {
    pub project: String,
    pub asset: String,
    pub version: u32,
    /// The model it reads, an absolute path. Only read.
    pub source: String,
    /// It normalises the rest pose first, and waits for a yes.
    pub normalize: bool,
    /// Before that it frees cloth and hair from the arms, and waits for a
    /// yes of its own.
    pub reweight: bool,
    /// The state of that reweight when one is there: pending, approved or
    /// denied.
    pub reweight_state: Option<NormalizeState>,
    pub elbows: Elbows,
    /// A normalised result with these options is already there, so Blender
    /// does not run again.
    pub cached: bool,
    /// The state of that result: pending, approved or denied.
    pub state: Option<NormalizeState>,
    pub material: MaterialMode,
    /// Where the `.vrm` goes, an absolute path.
    pub output: String,
    /// What writing would refuse in the meta.
    pub meta_problem: Option<String>,
    /// The source's readiness, findings blockers first.
    pub readiness: VrmReadiness,
}

/// What a build did.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum AvatarBuild {
    /// The weights that free cloth and hair from the arms wait for a person
    /// to look at the sheets; the rest pose comes after.
    #[serde(rename = "reweight waiting for approval")]
    ReweightWaiting { reweighted: Reweighted },
    /// The new rest pose waits for a person to look at the sheets.
    #[serde(rename = "waiting for approval")]
    Waiting { normalized: Normalized },
    /// A person turned the new rest pose down.
    #[serde(rename = "denied")]
    Denied { normalized: Normalized },
    /// The `.vrm` is written.
    #[serde(rename = "written")]
    Written {
        export: AvatarExport,
        normalized: Option<Normalized>,
    },
}

/// A request's normalise request, against `spec`'s bone overrides.
fn normalize_request<'a>(req: &'a AvatarRequest<'a>, spec: &'a AvatarSpec) -> NormalizeRequest<'a> {
    NormalizeRequest {
        workspace_root: req.workspace_root,
        config: req.config,
        project: req.project,
        asset: req.asset,
        version: req.version,
        overrides: &spec.bones,
        elbows: req.elbows,
    }
}

/// A request's reweight request, against `spec`'s bone overrides.
fn reweight_request<'a>(req: &'a AvatarRequest<'a>, spec: &'a AvatarSpec) -> ReweightRequest<'a> {
    ReweightRequest {
        workspace_root: req.workspace_root,
        config: req.config,
        project: req.project,
        asset: req.asset,
        version: req.version,
        overrides: &spec.bones,
    }
}

/// Whether a build with `req` reweights before it lifts the arms.
fn reweights(req: &AvatarRequest<'_>, readiness: &VrmReadiness) -> bool {
    req.reweight == ReweightMode::Auto && lifts_cloth(readiness)
}

/// Reads a request's spec, source and readiness.
fn prepare(req: &AvatarRequest<'_>) -> CoreResult<(Source, AvatarSpec, VrmReadiness)> {
    let src = source(req)?;
    let avatars = super::load(&src.project_dir)?;
    let (spec, _) = spec_for(&avatars, req.config, &src.asset);
    let file = src
        .file
        .clone()
        .ok_or_else(|| invalid(format!("{} has no model yet.", src.asset.name)))?;
    let readiness = readiness_of(&file, &spec.bones)?;
    Ok((src, spec, readiness))
}

/// Plans a build: whether it normalises, whether a result is cached and
/// in what state, and where it writes.
///
/// # Errors
///
/// As [`build`] would before running anything.
pub fn plan(req: &AvatarRequest<'_>) -> CoreResult<AvatarPlan> {
    let (src, spec, readiness) = prepare(req)?;
    let normalize = needs_normalize(&readiness);
    let reweight = reweights(req, &readiness);
    let reweight_state = if reweight {
        let planned = reweight::plan(&reweight_request(req, &spec))?;
        if planned.cached() {
            reweight::load(&planned.dir)?.map(|r| r.state)
        } else {
            None
        }
    } else {
        None
    };
    let (version, source, cached, state) = if normalize {
        let plan = normalize::plan(&normalize_request(req, &spec))?;
        let cached = plan.cached();
        let state = if cached {
            normalize::load(&plan.dir)?.map(|n| n.state)
        } else {
            None
        };
        (plan.version, plan.source, cached, state)
    } else {
        let file = src.file.clone().unwrap_or_default();
        (src.version.unwrap_or_default(), file, false, None)
    };
    Ok(AvatarPlan {
        project: req.project.to_string(),
        asset: src.asset.id.clone(),
        version,
        source: source.display().to_string(),
        normalize,
        reweight,
        reweight_state,
        elbows: req.elbows,
        cached,
        state,
        material: spec.material,
        output: vrm_file(&src.project_dir, &src.asset.id)
            .display()
            .to_string(),
        meta_problem: spec.meta.validate().err().map(|e| e.to_string()),
        readiness,
    })
}

/// Builds the avatar: normalises the rest pose when it is not VRM's yet
/// (or reuses the cached result), and once a person approved the new pose,
/// writes `exports/<asset>.vrm`. Blocks while Blender runs.
///
/// # Errors
///
/// Fails for a meta VRM refuses (no name or author), a rig normalising
/// cannot fix, a failed Blender run, or a writer refusal.
pub fn build(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    req: &AvatarRequest<'_>,
) -> CoreResult<AvatarBuild> {
    let (src, spec, readiness) = prepare(req)?;
    spec.meta.validate()?;
    let write_spec = WriteSpec {
        meta: spec.meta.clone(),
        overrides: spec.bones.clone(),
        material: spec.material,
        thumbnail: None,
    };
    let dir = asset_dir(req.workspace_root, req.config, req.project, &src.asset.id);
    let output = vrm_file(&src.project_dir, &src.asset.id);
    let file = src.file.clone().unwrap_or_default();
    let built = |written, source_version, elbows, reweight| Built {
        written,
        source: &file,
        source_version,
        elbows,
        reweight,
        spec: &spec,
        before: &readiness,
    };
    if !needs_normalize(&readiness) {
        let bytes = fs::read(&file).map_err(|e| CoreError::io(&file, e))?;
        let written = vrm::write(&bytes, &write_spec, file.parent().unwrap_or(&dir))?;
        let export = save(
            &dir,
            &output,
            &built(written, src.version.unwrap_or(0), None, None),
        )?;
        return Ok(AvatarBuild::Written {
            export,
            normalized: None,
        });
    }
    if reweights(req, &readiness) {
        let planned = reweight::plan(&reweight_request(req, &spec))?;
        let freed = reweight::run(runner, tools, &planned)?;
        // A no lifts the arms from the source as it is.
        if freed.state == NormalizeState::Pending {
            return Ok(AvatarBuild::ReweightWaiting { reweighted: freed });
        }
    }
    let plan = normalize::plan(&normalize_request(req, &spec))?;
    let found = normalize::run(runner, tools, &plan)?;
    match found.state {
        NormalizeState::Pending => Ok(AvatarBuild::Waiting { normalized: found }),
        NormalizeState::Denied => Ok(AvatarBuild::Denied { normalized: found }),
        NormalizeState::Approved => {
            let written = normalize::write_vrm(&plan.dir, &write_spec)?;
            let export = save(
                &dir,
                &output,
                &built(
                    written,
                    plan.version,
                    Some(req.elbows),
                    found.reweight.clone(),
                ),
            )?;
            Ok(AvatarBuild::Written {
                export,
                normalized: Some(found),
            })
        }
    }
}

/// What a build wrote, and from what.
struct Built<'a> {
    written: vrm::Written,
    /// The source version's model file.
    source: &'a Path,
    source_version: u32,
    /// The elbows mode of the normalise; `None` when nothing was normalised.
    elbows: Option<Elbows>,
    /// What the approved reweight the arms were lifted from changed.
    reweight: Option<ReweightReport>,
    spec: &'a AvatarSpec,
    /// The source version's readiness.
    before: &'a VrmReadiness,
}

/// Writes the `.vrm`, reads it back, and writes the report beside it and
/// the record of it.
fn save(dir: &Path, output: &Path, built: &Built<'_>) -> CoreResult<AvatarExport> {
    let written = &built.written;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    write_atomic(output, &written.bytes).map_err(|e| CoreError::io(output, e))?;
    let round_trip = vrm::round_trip(&written.bytes);
    let mut unresolved = written.unresolved.clone();
    if let Some(blocker) = round_trip_finding(&round_trip) {
        unresolved.insert(0, blocker);
    }
    let base = built.source.parent().unwrap_or(dir);
    // A file that does not read back has no readiness of its own; the
    // round trip says why.
    let after = vrm::readiness(&written.bytes, base, &built.spec.bones).ok();
    let written_at = now_rfc3339();
    let report = AvatarReport {
        file: output.display().to_string(),
        written_at: written_at.clone(),
        writer: report::WRITER.to_string(),
        source: ReportSource {
            version: built.source_version,
            file: built.source.display().to_string(),
            sha256: hash_file(built.source).map_err(|e| CoreError::io(built.source, e))?,
        },
        material: built.spec.material,
        normalized: built.elbows.is_some(),
        elbows: built.elbows,
        reweight: built.reweight.clone(),
        before: built.before.into(),
        after: after.as_ref().map(Into::into),
        unresolved: unresolved.clone(),
        bones: bone_map(after.as_ref().unwrap_or(built.before)),
        meta: (&built.spec.meta).into(),
        round_trip: round_trip.clone(),
    };
    let report_path = report_file(output);
    write_json(&report_path, &report)?;
    let export = AvatarExport {
        path: output.display().to_string(),
        #[expect(
            clippy::cast_precision_loss,
            reason = "a model is far below 2^53 bytes"
        )]
        bytes: written.bytes.len() as f64,
        source_version: built.source_version,
        normalized: built.elbows.is_some(),
        reweighted: built.reweight.is_some(),
        material: built.spec.material,
        unresolved,
        written_at,
        exists: true,
        report: Some(report_path.display().to_string()),
        round_trip: Some(round_trip),
    };
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    write_json(&dir.join(RECORD_FILE), &export)?;
    Ok(export)
}

/// The avatar folder of the version a request names.
fn version_dir(req: &AvatarRequest<'_>) -> CoreResult<PathBuf> {
    let src = source(req)?;
    let version = src
        .version
        .ok_or_else(|| invalid(format!("{} has no model yet.", src.asset.name)))?;
    Ok(
        asset_dir(req.workspace_root, req.config, req.project, &src.asset.id)
            .join(format!("v{version}")),
    )
}

/// A person's yes or no to a version's new rest pose, from the app.
///
/// # Errors
///
/// Fails when nothing intact was normalised for that version.
pub fn decide(req: &AvatarRequest<'_>, approve: bool) -> CoreResult<Normalized> {
    let dir = version_dir(req)?;
    if approve {
        normalize::approve(&dir)
    } else {
        normalize::deny(&dir)
    }
}

/// A person's yes or no to the weights that free a version's cloth and
/// hair from the arms, from the app.
///
/// # Errors
///
/// Fails when nothing intact was reweighted for that version.
pub fn decide_reweight(req: &AvatarRequest<'_>, approve: bool) -> CoreResult<Reweighted> {
    let dir = reweight::work_dir(&version_dir(req)?);
    if approve {
        reweight::approve(&dir)
    } else {
        reweight::deny(&dir)
    }
}

#[cfg(test)]
mod tests;
