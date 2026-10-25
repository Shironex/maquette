//! The checks on a model's shape: where its origin sits and how many
//! objects draw it.

use super::{Check, CheckStatus, check};
use crate::assets::AssetKind;
use crate::inbox::analyse::GlbReport;
use crate::prepare::Fixup;

/// How far the bottom centre may sit from the origin before it shows: a
/// share of the model's size.
const ORIGIN_SLACK: f32 = 0.02;

/// Where the origin sits and how many objects draw the model, with the
/// fix-ups that tidy them. Scenes are left as they are.
pub(super) fn shape_checks(report: &GlbReport, kind: AssetKind) -> Vec<Check> {
    let mut out = Vec::new();
    if kind == AssetKind::Scene || report.objects == 0 {
        return out;
    }
    let [w, h, d] = report.size;
    let reach = w.max(h).max(d);
    let centre = [
        (report.min[0] + report.max[0]) / 2.0,
        report.min[1],
        (report.min[2] + report.max[2]) / 2.0,
    ];
    let off = centre.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
    if reach > 0.0 && off > ORIGIN_SLACK * reach {
        out.push(
            check(
                "origin",
                CheckStatus::Warning,
                "Origin is off the bottom centre",
                format!(
                    "Its bottom centre sits {:.0} cm from the origin, so it floats, sinks or swings wide when placed.",
                    off * 100.0
                ),
            )
            .with_fixes(vec![Fixup::OriginBottom]),
        );
    } else {
        out.push(check(
            "origin",
            CheckStatus::Pass,
            "Origin at the bottom centre",
            None,
        ));
    }
    if report.objects >= 2 && report.skins == 0 {
        let mut fixes = vec![Fixup::Merge { by_material: false }];
        if report.transformed > 0 {
            fixes.push(Fixup::ApplyTransforms);
        }
        out.push(
            check(
                "objects",
                CheckStatus::Pass,
                format!("{} objects", report.objects),
                "Merging them into one mesh saves draw calls when they never move apart."
                    .to_string(),
            )
            .with_fixes(fixes),
        );
    }
    out
}
