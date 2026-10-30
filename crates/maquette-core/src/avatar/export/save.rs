//! Writing a build's `.vrm` and the report and record beside it.

use std::fs;
use std::path::Path;

use super::report::{
    self, AvatarReport, ReportSource, bone_map, report_file, round_trip_finding, written_readiness,
};
use super::{AvatarExport, RECORD_FILE};
use crate::avatar::AvatarSpec;
use crate::avatar::normalize::{Elbows, Fingers};
use crate::avatar::reweight::ReweightReport;
use crate::avatar::springs::SpringsSummary;
use crate::error::{CoreError, CoreResult};
use crate::fs::write_atomic;
use crate::inbox::vrm::{self, VrmReadiness};
use crate::json::write_json;
use crate::pipeline::cache::hash_file;
use crate::project::now_rfc3339;

/// What a build wrote, and from what.
pub(super) struct Built<'a> {
    pub(super) written: vrm::Written,
    /// The source version's model file.
    pub(super) source: &'a Path,
    pub(super) source_version: u32,
    /// The elbows and fingers of the normalise; `None` when nothing was
    /// normalised.
    pub(super) options: Option<(Elbows, Fingers)>,
    /// What the approved reweight the arms were lifted from changed.
    pub(super) reweight: Option<ReweightReport>,
    pub(super) spec: &'a AvatarSpec,
    /// The source version's readiness.
    pub(super) before: &'a VrmReadiness,
    /// The springs the file carries; `None` without.
    pub(super) springs: Option<SpringsSummary>,
}

/// Writes the `.vrm`, reads it back, and writes the report beside it and
/// the record of it.
pub(super) fn save(dir: &Path, output: &Path, built: &Built<'_>) -> CoreResult<AvatarExport> {
    let written = &built.written;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    write_atomic(output, &written.bytes).map_err(|e| CoreError::io(output, e))?;
    let round_trip = vrm::round_trip(&written.bytes);
    let mut unresolved = written.unresolved.clone();
    let base = built.source.parent().unwrap_or(dir);
    let after = written_readiness(
        &written.bytes,
        base,
        &built.spec.bones,
        built.reweight.is_some(),
        &mut unresolved,
    );
    if let Some(blocker) = round_trip_finding(&round_trip) {
        unresolved.insert(0, blocker);
    }
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
        normalized: built.options.is_some(),
        elbows: built.options.map(|o| o.0),
        fingers: built.options.map(|o| o.1),
        reweight: built.reweight.clone(),
        before: built.before.into(),
        after: after.as_ref().map(Into::into),
        unresolved: unresolved.clone(),
        bones: bone_map(after.as_ref().unwrap_or(built.before)),
        meta: (&built.spec.meta).into(),
        round_trip: round_trip.clone(),
        springs: built.springs.clone(),
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
        normalized: built.options.is_some(),
        reweighted: built.reweight.is_some(),
        material: built.spec.material,
        unresolved,
        written_at,
        exists: true,
        report: Some(report_path.display().to_string()),
        round_trip: Some(round_trip),
        springs: built
            .springs
            .as_ref()
            .map_or(0, |s| u32::try_from(s.springs.len()).unwrap_or(u32::MAX)),
    };
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    write_json(&dir.join(RECORD_FILE), &export)?;
    Ok(export)
}
