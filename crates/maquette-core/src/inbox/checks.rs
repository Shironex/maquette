//! Turns a [`GlbReport`] into the pass, warning and fail rows the Inbox
//! shows, against the workspace budgets.

use serde::{Deserialize, Serialize};

use super::analyse::{GlbReport, RigKind};
use crate::assets::AssetKind;
use crate::prepare::{DEFAULT_LOOSE_SHARE, Fixup};

mod shape;
use crate::workspace::{TextureSize, WorkspaceConfig};
use shape::shape_checks;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum CheckStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub id: String,
    pub status: CheckStatus,
    pub title: String,
    pub detail: Option<String>,
    /// One click fix-ups for what the check found, best first.
    pub fixes: Vec<Fixup>,
}

impl Check {
    fn with_fixes(mut self, fixes: Vec<Fixup>) -> Self {
        self.fixes = fixes;
        self
    }
}

/// Which maps the send behind a model asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SentMaps {
    /// Not from a send, or one that asked for the full set.
    #[default]
    Any,
    /// Sent with PBR off: base color only.
    NoPbr,
    /// Sent with texture off: no maps at all.
    NoTexture,
}

/// What the send record says about a model, so a check does not warn
/// about something that was chosen on purpose.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SendNotes {
    pub maps: SentMaps,
    /// Sent with a texture quality above the standard one.
    pub large_textures: bool,
}

fn check(
    id: &str,
    status: CheckStatus,
    title: impl Into<String>,
    detail: impl Into<Option<String>>,
) -> Check {
    Check {
        id: id.to_string(),
        status,
        title: title.into(),
        detail: detail.into(),
        fixes: Vec::new(),
    }
}

pub fn texture_px(size: TextureSize) -> u32 {
    match size {
        TextureSize::K1 => 1024,
        TextureSize::K2 => 2048,
        TextureSize::K4 => 4096,
    }
}

/// "94.6k", "60k", "1.3M", "812".
pub fn short_count(n: u32) -> String {
    let n = f64::from(n);
    let text = if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 100_000.0 {
        format!("{:.0}k", n / 1000.0)
    } else if n >= 1000.0 {
        format!("{:.1}k", n / 1000.0)
    } else {
        format!("{n:.0}")
    };
    text.replace(".0k", "k").replace(".0M", "M")
}

fn plural(n: u32, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The rows for one model, in display order.
pub fn run(
    report: &GlbReport,
    kind: AssetKind,
    config: &WorkspaceConfig,
    notes: SendNotes,
) -> Vec<Check> {
    let mut out = Vec::new();
    let budget = config.budgets.tris_per_model;
    let tris = report.triangles;
    let status = if tris <= budget {
        CheckStatus::Pass
    } else if u64::from(tris) <= u64::from(budget) * 2 {
        CheckStatus::Warning
    } else {
        CheckStatus::Fail
    };
    let decimate = (status != CheckStatus::Pass && budget >= crate::prepare::MIN_TRIANGLES)
        .then_some(Fixup::Decimate { triangles: budget });
    out.push(
        check(
            "triangles",
            status,
            format!("{} triangles", short_count(tris)),
            match status {
                CheckStatus::Pass => {
                    format!("Within the {} per model budget.", short_count(budget))
                }
                _ => format!(
                    "Over the {} per model budget. Decimate it here, or raise the budget in Settings.",
                    short_count(budget)
                ),
            },
        )
        .with_fixes(decimate.into_iter().collect()),
    );

    let limit = texture_px(config.compression.texture_size);
    if report.textures.is_empty() {
        out.push(check("textures", CheckStatus::Pass, "No textures", None));
    } else {
        let over: Vec<&str> = report
            .textures
            .iter()
            .filter(|t| t.width.unwrap_or(0).max(t.height.unwrap_or(0)) > limit)
            .map(|t| t.name.as_str())
            .collect();
        let max = report.max_texture;
        let title = format!(
            "{}, largest {max} px",
            plural(
                u32::try_from(report.textures.len()).unwrap_or(u32::MAX),
                "texture",
                "textures"
            )
        );
        if over.is_empty() {
            out.push(check(
                "textures",
                CheckStatus::Pass,
                title,
                format!("At or under the {limit} px default."),
            ));
        } else {
            out.push(
                check(
                    "textures",
                    CheckStatus::Warning,
                    title,
                    format!(
                        "{} over the {limit} px default: {}.{} Compress resizes them, or set a per model size.",
                        plural(u32::try_from(over.len()).unwrap_or(u32::MAX), "texture is", "textures are"),
                        over.join(", "),
                        if notes.large_textures {
                            " It was sent with detailed textures."
                        } else {
                            ""
                        }
                    ),
                )
                .with_fixes(vec![Fixup::ShrinkTextures { max_size: limit }]),
            );
        }
    }

    let joints = report.joints;
    match (kind, report.rig) {
        (AssetKind::Character, RigKind::Mixamo) => out.push(check(
            "rig",
            CheckStatus::Pass,
            "Mixamo rig found",
            format!("{} joints, ready for Mixamo animations.", joints),
        )),
        (AssetKind::Character, RigKind::Other) => out.push(check(
            "rig",
            CheckStatus::Warning,
            "Rig is not Mixamo",
            format!(
                "{} joints with other names. Mixamo animations will not retarget as is.",
                joints
            ),
        )),
        (AssetKind::Character, RigKind::None) => out.push(check(
            "rig",
            CheckStatus::Fail,
            "No rig",
            "A character needs a skeleton. Rig it in Tripo or upload it to Mixamo.".to_string(),
        )),
        (_, RigKind::None) => {
            out.push(check("rig", CheckStatus::Pass, "No rig, as expected", None))
        }
        (_, _) => out.push(check(
            "rig",
            CheckStatus::Warning,
            format!("Rigged with {joints} joints"),
            "A prop or scene rarely needs a skeleton. Mark it as a character if it is one."
                .to_string(),
        )),
    }

    let p = &report.pbr;
    let mut missing = Vec::new();
    if !p.base_color {
        missing.push("base color");
    }
    if !p.metallic_roughness {
        missing.push("metal and roughness");
    }
    if !p.normal {
        missing.push("normal");
    }
    if missing.is_empty() {
        let mut extra = Vec::new();
        if p.occlusion {
            extra.push("occlusion");
        }
        if p.emissive {
            extra.push("emissive");
        }
        let detail = if extra.is_empty() {
            "Base color, metal and roughness, normal.".to_string()
        } else {
            format!(
                "Base color, metal and roughness, normal, plus {}.",
                extra.join(" and ")
            )
        };
        out.push(check("pbr", CheckStatus::Pass, "PBR maps present", detail));
    } else if notes.maps == SentMaps::NoPbr && p.base_color {
        out.push(check(
            "pbr",
            CheckStatus::Pass,
            "PBR off, as sent",
            "Sent with PBR off, so only a base color map is expected.".to_string(),
        ));
    } else if notes.maps == SentMaps::NoTexture {
        out.push(check(
            "pbr",
            CheckStatus::Pass,
            "Texture off, as sent",
            "Sent with texture off, so no maps are expected.".to_string(),
        ));
    } else {
        out.push(check(
            "pbr",
            CheckStatus::Warning,
            "PBR maps missing",
            format!(
                "No {} map. Turn on PBR in the Tripo settings for the next model.",
                missing.join(", ")
            ),
        ));
    }

    if let Some(islands) = &report.islands {
        if islands.groups >= 2 && kind != AssetKind::Scene {
            out.push(
                check(
                    "merged",
                    CheckStatus::Warning,
                    format!("Looks like {} objects in one mesh", islands.groups),
                    format!(
                        "{} has {} far apart parts. Split them in Blender (Separate by loose parts) so each can move, or remove the small ones when they are stray bits.",
                        islands.mesh, islands.groups
                    ),
                )
                .with_fixes(vec![Fixup::RemoveLoose {
                    min_share: DEFAULT_LOOSE_SHARE,
                }]),
            );
        } else {
            out.push(check(
                "merged",
                CheckStatus::Pass,
                "One object per mesh",
                None,
            ));
        }
    } else if !report.geometry_readable {
        out.push(check(
            "merged",
            CheckStatus::Pass,
            "Geometry is compressed",
            "Shape checks skipped for meshopt or Draco files.".to_string(),
        ));
    }

    if let Some(fw) = &report.finger_weights {
        if fw.vertices == 0 {
            out.push(check(
                "finger-weights",
                CheckStatus::Pass,
                "Finger weights look right",
                None,
            ));
        } else {
            let names: Vec<String> = fw
                .joints
                .iter()
                .take(4)
                .map(|j| {
                    format!(
                        "{} ({})",
                        super::analyse::bare_joint_name(&j.name),
                        j.vertices
                    )
                })
                .collect();
            out.push(check(
                "finger-weights",
                CheckStatus::Fail,
                "Hair or tail vertices weighted to finger bones",
                format!(
                    "{} far from the hands follow {}. Reweight them to the head in Blender.",
                    plural(fw.vertices, "vertex", "vertices"),
                    names.join(", ")
                ),
            ));
        }
    }

    out.extend(shape_checks(report, kind));

    if report.animations.is_empty() {
        if kind == AssetKind::Character && report.rig != RigKind::None {
            out.push(check(
                "animations",
                CheckStatus::Warning,
                "No animation clips yet",
                "The rig is ready. Add clips in Blender after Accept.".to_string(),
            ));
        }
    } else {
        out.push(check(
            "animations",
            CheckStatus::Pass,
            plural(
                u32::try_from(report.animations.len()).unwrap_or(u32::MAX),
                "animation",
                "animations",
            ),
            report.animations.join(", "),
        ));
    }
    out
}

/// The worst status, for the list badge.
pub fn worst(checks: &[Check]) -> CheckStatus {
    checks
        .iter()
        .map(|c| c.status)
        .max()
        .unwrap_or(CheckStatus::Pass)
}

#[cfg(test)]
mod tests;
