//! The VRM row: how far a rigged character is from a VRM 1.0 file.

use super::{Check, CheckStatus, check};
use crate::assets::AssetKind;
use crate::inbox::analyse::GlbReport;
use crate::inbox::vrm::{FindingLevel, HUMAN_BONES};

/// One row for a rigged character: a pass when a VRM file can be written
/// from it as it is, a warning otherwise. VRM is an export a character may
/// never need, so it never fails the model. The detail names every finding,
/// blockers first.
pub(super) fn vrm_check(report: &GlbReport, kind: AssetKind) -> Option<Check> {
    if kind != AssetKind::Character {
        return None;
    }
    let vrm = report.vrm.as_ref()?;
    let blockers = vrm.count(FindingLevel::Blocker);
    let (status, title) = if vrm.ready {
        (
            CheckStatus::Pass,
            format!("VRM ready, {} of {HUMAN_BONES} bones", vrm.mapped),
        )
    } else {
        (
            CheckStatus::Warning,
            format!(
                "Not VRM ready: {blockers} {} to fix",
                if blockers == 1 { "thing" } else { "things" }
            ),
        )
    };
    let detail: Vec<String> = vrm
        .findings
        .iter()
        .map(|f| format!("{}.", f.title))
        .collect();
    Some(check("vrm", status, title, detail.join(" ")))
}
