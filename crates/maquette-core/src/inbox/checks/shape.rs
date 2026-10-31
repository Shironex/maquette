//! The checks on a model's shape: where its origin sits, how many objects
//! draw it and whether one mesh holds what should be separate parts.

use super::{Check, CheckStatus, RetextureSource, check, short_count};
use crate::assets::AssetKind;
use crate::inbox::analyse::GlbReport;
use crate::prepare::{DEFAULT_LOOSE_SHARE, DEFAULT_MERGE_UNDER, Fixup, SplitBy};

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

/// Whether one mesh holds far apart pieces. A prop that is one mesh has no
/// parts at all, so object states and explode tracks have nothing to move:
/// it offers the split first. Scenes are left as they are.
pub(super) fn merged_check(report: &GlbReport, kind: AssetKind) -> Option<Check> {
    let Some(islands) = &report.islands else {
        return (!report.geometry_readable).then(|| {
            check(
                "merged",
                CheckStatus::Pass,
                "Geometry is compressed",
                "Shape checks skipped for meshopt or Draco files.".to_string(),
            )
        });
    };
    if islands.groups < 2 || kind == AssetKind::Scene {
        return Some(check(
            "merged",
            CheckStatus::Pass,
            "One object per mesh",
            None,
        ));
    }
    let remove = Fixup::RemoveLoose {
        min_share: DEFAULT_LOOSE_SHARE,
    };
    if kind == AssetKind::Prop && report.objects == 1 && report.skins == 0 {
        return Some(
            check(
                "merged",
                CheckStatus::Warning,
                "One mesh, no parts",
                format!(
                    "Motion states and explode tracks move named parts. Split it so its {} pieces can move on their own.",
                    islands.groups
                ),
            )
            .with_fixes(vec![
                Fixup::SplitParts {
                    by: SplitBy::Loose,
                    merge_under: DEFAULT_MERGE_UNDER,
                },
                remove,
            ]),
        );
    }
    Some(
        check(
            "merged",
            CheckStatus::Warning,
            format!("Looks like {} objects in one mesh", islands.groups),
            format!(
                "{} has {} far apart parts. Split them so each can move, or remove the small ones when they are stray bits.",
                islands.mesh, islands.groups
            ),
        )
        .with_fixes(vec![
            Fixup::SplitParts {
                by: SplitBy::Loose,
                merge_under: DEFAULT_MERGE_UNDER,
            },
            remove,
        ]),
    )
}

/// A re-texture against the version it painted. Meshy's answer decides whether
/// the parts stay parts, so a loss has to say so.
pub(super) fn retexture_check(report: &GlbReport, source: RetextureSource) -> Check {
    let (before, after) = (source.objects, report.objects);
    let tris = format!(
        "{} tris on v{}, {} here.",
        short_count(source.triangles),
        source.version,
        short_count(report.triangles)
    );
    if after < before {
        return check(
            "retexture-parts",
            CheckStatus::Warning,
            if after <= 1 {
                format!(
                    "Parts gone: v{} has {before}, this is one mesh",
                    source.version
                )
            } else {
                format!(
                    "Parts lost: v{} has {before}, this has {after}",
                    source.version
                )
            },
            format!(
                "Meshy's re-texture merged them, so Motion states and explode tracks cannot move them. {tris} To keep the parts, discard this and use Bake back on v{} in Inspector > Parts.",
                source.version
            ),
        );
    }
    let title = if before >= 2 {
        format!("Parts kept: {after}, as on v{}", source.version)
    } else {
        format!("One mesh, as on v{}", source.version)
    };
    check("retexture-parts", CheckStatus::Pass, title, tris)
}
