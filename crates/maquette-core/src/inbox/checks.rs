//! Turns a [`GlbReport`] into the pass, warning and fail rows the Inbox
//! shows, against the workspace budgets.

use serde::{Deserialize, Serialize};

use super::analyse::{GlbReport, RigKind};
use crate::assets::AssetKind;
use crate::workspace::{TextureSize, WorkspaceConfig};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum CheckStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub id: String,
    pub status: CheckStatus,
    pub title: String,
    pub detail: Option<String>,
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
    out.push(check(
        "triangles",
        status,
        format!("{} triangles", short_count(tris)),
        match status {
            CheckStatus::Pass => format!("Within the {} per model budget.", short_count(budget)),
            _ => format!(
                "Over the {} per model budget. Decimate in Blender or raise the budget in Settings.",
                short_count(budget)
            ),
        },
    ));

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
            out.push(check(
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
            ));
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
            out.push(check(
                "merged",
                CheckStatus::Warning,
                format!("Looks like {} objects in one mesh", islands.groups),
                format!(
                    "{} has {} far apart parts. Split them in Blender (Separate by loose parts) so each can move.",
                    islands.mesh,
                    islands.groups
                ),
            ));
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
mod tests {
    use std::path::Path;

    use super::*;
    use crate::inbox::{analyse::analyse_bytes, fixture};

    fn checks(bytes: &[u8], kind: AssetKind, config: &WorkspaceConfig) -> Vec<Check> {
        run(
            &analyse_bytes(bytes, Path::new(".")).expect("analyse"),
            kind,
            config,
            SendNotes::default(),
        )
    }

    fn status(list: &[Check], id: &str) -> Option<CheckStatus> {
        list.iter().find(|c| c.id == id).map(|c| c.status)
    }

    #[test]
    fn counts_read_short() {
        assert_eq!(short_count(94_600), "94.6k");
        assert_eq!(short_count(60_000), "60k");
        assert_eq!(short_count(812), "812");
        assert_eq!(short_count(120_000), "120k");
        assert_eq!(short_count(1_260_000), "1.3M");
        assert_eq!(short_count(2_000_000), "2M");
    }

    #[test]
    fn a_clean_prop_passes_everything() {
        let list = checks(
            &fixture::clean_prop(),
            AssetKind::Prop,
            &WorkspaceConfig::default(),
        );
        assert!(
            list.iter().all(|c| c.status == CheckStatus::Pass),
            "{list:#?}"
        );
        assert_eq!(worst(&list), CheckStatus::Pass);
    }

    #[test]
    fn budgets_drive_the_triangle_and_texture_rows() {
        let mut config = WorkspaceConfig::default();
        config.budgets.tris_per_model = 10;
        let list = checks(&fixture::clean_prop(), AssetKind::Prop, &config);
        assert_eq!(status(&list, "triangles"), Some(CheckStatus::Warning));
        config.budgets.tris_per_model = 5;
        let list = checks(&fixture::clean_prop(), AssetKind::Prop, &config);
        assert_eq!(status(&list, "triangles"), Some(CheckStatus::Fail));
        let mut report = analyse_bytes(&fixture::clean_prop(), Path::new(".")).expect("analyse");
        report.textures[0].width = Some(4096);
        let list = run(
            &report,
            AssetKind::Prop,
            &WorkspaceConfig::default(),
            SendNotes::default(),
        );
        let row = list.iter().find(|c| c.id == "textures").expect("row");
        assert_eq!(row.status, CheckStatus::Warning);
        assert!(
            row.detail
                .as_deref()
                .unwrap_or_default()
                .contains("over the 2048 px default")
        );
    }

    #[test]
    fn character_rows_cover_rig_and_finger_weights() {
        let config = WorkspaceConfig::default();
        let list = checks(&fixture::character(true), AssetKind::Character, &config);
        assert_eq!(status(&list, "rig"), Some(CheckStatus::Pass));
        let finger = list.iter().find(|c| c.id == "finger-weights").expect("row");
        assert_eq!(finger.status, CheckStatus::Fail);
        let detail = finger.detail.as_deref().unwrap_or_default();
        assert!(detail.starts_with("48 vertices"), "{detail}");
        assert!(detail.contains("LeftHandIndex1 (24)"));
        assert_eq!(status(&list, "animations"), Some(CheckStatus::Pass));
        assert_eq!(worst(&list), CheckStatus::Fail);

        let clean = checks(&fixture::character(false), AssetKind::Character, &config);
        assert_eq!(status(&clean, "finger-weights"), Some(CheckStatus::Pass));
        let mut still = analyse_bytes(&fixture::character(false), Path::new(".")).expect("analyse");
        still.animations.clear();
        let list = run(&still, AssetKind::Character, &config, SendNotes::default());
        assert_eq!(status(&list, "animations"), Some(CheckStatus::Warning));
        // The same rig on a prop is suspicious; no rig on a character fails.
        let prop = checks(&fixture::character(false), AssetKind::Prop, &config);
        assert_eq!(status(&prop, "rig"), Some(CheckStatus::Warning));
        let unrigged = checks(&fixture::clean_prop(), AssetKind::Character, &config);
        assert_eq!(status(&unrigged, "rig"), Some(CheckStatus::Fail));
    }

    #[test]
    fn merged_objects_warn_except_in_scenes() {
        let config = WorkspaceConfig::default();
        let list = checks(&fixture::merged_islands(), AssetKind::Prop, &config);
        assert_eq!(status(&list, "merged"), Some(CheckStatus::Warning));
        assert_eq!(status(&list, "pbr"), Some(CheckStatus::Warning));
        let scene = checks(&fixture::merged_islands(), AssetKind::Scene, &config);
        assert_eq!(status(&scene, "merged"), Some(CheckStatus::Pass));
    }

    #[test]
    fn a_send_without_pbr_does_not_warn_about_missing_maps() {
        let config = WorkspaceConfig::default();
        let mut report =
            analyse_bytes(&fixture::merged_islands(), Path::new(".")).expect("analyse");
        report.pbr.base_color = true;
        let sent = |maps| {
            let notes = SendNotes {
                maps,
                large_textures: false,
            };
            run(&report, AssetKind::Scene, &config, notes)
        };
        assert_eq!(
            status(&sent(SentMaps::Any), "pbr"),
            Some(CheckStatus::Warning)
        );
        let list = sent(SentMaps::NoPbr);
        assert_eq!(status(&list, "pbr"), Some(CheckStatus::Pass));
        let row = list.iter().find(|c| c.id == "pbr").expect("row");
        assert_eq!(row.title, "PBR off, as sent");
        assert_eq!(worst(&list), CheckStatus::Pass);
        assert_eq!(
            status(&sent(SentMaps::NoTexture), "pbr"),
            Some(CheckStatus::Pass)
        );
    }

    #[test]
    fn detailed_textures_keep_the_warning_and_say_why() {
        let mut report = analyse_bytes(&fixture::clean_prop(), Path::new(".")).expect("analyse");
        report.textures[0].width = Some(4096);
        let notes = SendNotes {
            large_textures: true,
            ..SendNotes::default()
        };
        let list = run(&report, AssetKind::Prop, &WorkspaceConfig::default(), notes);
        let row = list.iter().find(|c| c.id == "textures").expect("row");
        assert_eq!(row.status, CheckStatus::Warning);
        let detail = row.detail.as_deref().unwrap_or_default();
        assert!(detail.contains("sent with detailed textures"), "{detail}");
        assert!(detail.contains("Compress resizes them"));
    }
}
