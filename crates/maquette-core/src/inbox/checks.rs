//! Turns a [`GlbReport`] into the pass, warning and fail rows the Inbox
//! shows, against the workspace budgets, or in a game project against the
//! budget of the folder the model's asset maps to.

use serde::{Deserialize, Serialize};

use super::analyse::{GlbReport, RigKind};
use crate::assets::AssetKind;
// The card numbers are written below the pipeline too, so the count
// format lives with the assets; this keeps the old path.
pub use crate::assets::short_count;
use crate::game::FolderBudget;
use crate::prepare::Fixup;

mod shape;
mod vrm;
// What a send asked for is shared with the provider sends, so it lives in
// the base crate; this keeps the old path.
use crate::workspace::{TextureSize, WorkspaceConfig};
pub use maquette_base::send_notes::{RetextureSource, SendNotes, SentMaps, SentTo};
use shape::{merged_check, retexture_check, shape_checks};
use vrm::vrm_check;

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

fn plural(n: u32, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// "the props budget", "the model folder's budget" for its top level.
fn folder_label(folder: &FolderBudget) -> String {
    if folder.folder.is_empty() {
        "the model folder's budget".to_string()
    } else {
        format!("the {} budget", folder.folder)
    }
}

/// What to do about missing maps, in the words of where the model came from.
fn pbr_advice(notes: SendNotes) -> String {
    let provider = match notes.sent_to {
        SentTo::Nobody => return "Turn on PBR where you make the next model.".to_string(),
        SentTo::Tripo => "Tripo",
        SentTo::Meshy => "Meshy",
    };
    if notes.retexture_of.is_some() {
        format!("The re-texture asked {provider} for them, so it left them out this time.")
    } else if notes.by_hand {
        format!("Turn on PBR on {provider}'s site for the next model.")
    } else if notes.maps == SentMaps::NoPbr {
        format!("Turn on PBR in the {provider} settings for the next model.")
    } else {
        format!("The send asked {provider} for them, so it left them out this time.")
    }
}

/// The rows for one model, in display order. `folder` is the game folder
/// budget of the asset the model was sent for, in a game project; without
/// it the workspace's Tris per model and texture size apply.
pub fn run(
    report: &GlbReport,
    kind: AssetKind,
    config: &WorkspaceConfig,
    notes: SendNotes,
    folder: Option<&FolderBudget>,
) -> Vec<Check> {
    let mut out = Vec::new();
    let budget = folder.map_or(config.budgets.tris_per_model, |f| f.triangles);
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
            match (status, folder) {
                (CheckStatus::Pass, None) => {
                    format!("Within the {} per model budget.", short_count(budget))
                }
                (CheckStatus::Pass, Some(f)) => format!(
                    "Within {}, {} tris.",
                    folder_label(f),
                    short_count(budget)
                ),
                (_, None) => format!(
                    "Over the {} per model budget. Decimate it here, or raise the budget in Settings.",
                    short_count(budget)
                ),
                (_, Some(f)) => format!(
                    "Over {}, {} tris. Decimate it here, or raise the folder's budget in Inspector > Budget.",
                    folder_label(f),
                    short_count(budget)
                ),
            },
        )
        .with_fixes(decimate.into_iter().collect()),
    );

    let limit = folder.map_or_else(
        || texture_px(config.compression.texture_size),
        |f| f.texture,
    );
    let against = folder.map_or_else(
        || format!("the {limit} px default"),
        |f| format!("{}, {limit} px", folder_label(f)),
    );
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
                format!("At or under {against}."),
            ));
        } else {
            out.push(
                check(
                    "textures",
                    CheckStatus::Warning,
                    title,
                    format!(
                        "{} over {against}: {}.{} Compress resizes them, or set a per model size.",
                        plural(
                            u32::try_from(over.len()).unwrap_or(u32::MAX),
                            "texture is",
                            "textures are"
                        ),
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
    out.extend(vrm_check(report, kind));

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
    } else if notes.by_hand_tripo && kind == AssetKind::Character && p.base_color {
        out.push(check(
            "pbr",
            CheckStatus::Pass,
            "Base color only, as exported",
            "Base color only, as Tripo's Smart Mesh exports it.".to_string(),
        ));
    } else if notes.split {
        // No setting asks for these maps, so there is nothing to turn on.
        out.push(check(
            "pbr",
            CheckStatus::Pass,
            if p.base_color {
                "Base color only, as split"
            } else {
                "No texture, as split"
            },
            "A split has no PBR setting. A bake back or a re-texture brings the maps.".to_string(),
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
            format!("No {} map. {}", missing.join(", "), pbr_advice(notes)),
        ));
    }

    let retexture = notes
        .retexture_of
        .map(|source| retexture_check(report, source));
    // A row saying one object per mesh reads as all is well next to lost parts.
    if retexture
        .as_ref()
        .is_none_or(|c| c.status == CheckStatus::Pass)
    {
        out.extend(merged_check(report, kind));
    }
    out.extend(retexture);

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

    if let Some(probe) = report.rig_probe.as_ref().filter(|p| p.arm_far > 0) {
        out.push(check(
            "arm-weights",
            CheckStatus::Warning,
            "Cloth or hair weighted to the arm bones",
            format!(
                "{} far from the arms swing with them. The rig doctor can fix it.",
                plural(probe.arm_far, "vertex", "vertices"),
            ),
        ));
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
