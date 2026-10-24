//! By hand: no API call. A send writes `projects/<p>/handoff/<id>/` with
//! the input images named by view and a settings card (`settings.md` and
//! `settings.json`) listing the exact values to enter in the service's web
//! app. The model comes back as a GLB dropped on the asset's card, or saved
//! into the inbox as `<id>.glb`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::catalog::{
    is_smart_topology, is_tripo_p_series, meshy_label, meshy_takes_remove_lighting, tripo_label,
};
use super::presets::{
    MeshyPreset, MeshyTexture, PoseMode, PresetVariant, ProviderId, TripoGeometryQuality,
    TripoPreset, TripoRigSpec, TripoTextureQuality,
};
use super::tripo::VIEWS;
use crate::error::{CoreError, CoreResult};
use crate::fs::write_atomic;
use crate::json::write_json;
use crate::workspace::Topology;

pub const HANDOFF_DIR: &str = "handoff";

/// One line of the card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CardRow {
    pub label: String,
    pub value: String,
    /// A short hint, for example where the setting hides.
    pub note: Option<String>,
}

/// What the By hand panel shows, with a copy button per row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HandoffCard {
    pub service: ProviderId,
    /// The web app to open.
    pub site: String,
    /// "Image to 3D" or "Multiview to 3D".
    pub mode: String,
    pub rows: Vec<CardRow>,
    /// The images in the folder, in upload order (front first).
    pub images: Vec<String>,
    /// The name to give the GLB when saving it into the inbox.
    pub return_name: String,
}

pub fn site(service: ProviderId) -> &'static str {
    match service {
        ProviderId::Tripo => "https://studio.tripo3d.ai",
        ProviderId::Meshy => "https://www.meshy.ai/workspace",
    }
}

fn on_off(on: bool) -> String {
    if on { "On" } else { "Off" }.to_string()
}

fn row(label: &str, value: impl Into<String>) -> CardRow {
    CardRow {
        label: label.to_string(),
        value: value.into(),
        note: None,
    }
}

fn row_note(label: &str, value: impl Into<String>, note: &str) -> CardRow {
    CardRow {
        note: Some(note.to_string()),
        ..row(label, value)
    }
}

fn count(n: u32) -> String {
    super::presets::group(n)
}

fn animation_label(id: &str) -> String {
    let name = id.rsplit(':').next().unwrap_or(id).replace('_', " ");
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

pub fn tripo_rows(preset: &TripoPreset, variant: PresetVariant, images: usize) -> Vec<CardRow> {
    let p_series = is_tripo_p_series(&preset.model);
    let legacy = preset.model.starts_with("v2");
    let mut rows = vec![
        row_note(
            "Model",
            tripo_label(&preset.model),
            &format!("model id {}", preset.model),
        ),
        row(
            "Face limit",
            if preset.face_limit == 0 {
                "Adaptive".to_string()
            } else {
                count(preset.face_limit)
            },
        ),
        row("Texture", on_off(preset.texture)),
        row("PBR", on_off(preset.texture && preset.pbr)),
    ];
    if preset.texture && !legacy {
        rows.push(row(
            "Texture quality",
            match preset.texture_quality {
                TripoTextureQuality::Standard => "Standard",
                TripoTextureQuality::Detailed => "Detailed",
                TripoTextureQuality::Extreme => "Extreme",
            },
        ));
    }
    if !p_series && !legacy {
        rows.push(row(
            "Geometry",
            match preset.geometry_quality {
                TripoGeometryQuality::Standard => "Standard",
                TripoGeometryQuality::Detailed => "Ultra",
            },
        ));
        rows.push(row("Smart low poly", on_off(preset.smart_low_poly)));
    }
    if !legacy {
        rows.push(row("Real-world size", on_off(preset.auto_size)));
    }
    if images < 2 || !preset.multi_view {
        rows.push(row("Fix the image first", on_off(preset.image_autofix)));
    }
    rows.push(row_note("Quad mesh", "Off", "quads export FBX only"));
    if variant == PresetVariant::Character && preset.rig {
        rows.push(row_note(
            "Auto rig",
            match preset.rig_spec {
                TripoRigSpec::Mixamo => "Mixamo bones",
                TripoRigSpec::Tripo => "Tripo bones",
            },
            &format!("rig model {}", preset.rig_model),
        ));
        if !preset.animations.is_empty() {
            let names: Vec<String> = preset
                .animations
                .iter()
                .map(|a| animation_label(a))
                .collect();
            rows.push(row_note(
                "Animations",
                names.join(", "),
                "retarget them onto the rig",
            ));
        }
    }
    rows
}

pub fn meshy_rows(preset: &MeshyPreset, variant: PresetVariant) -> Vec<CardRow> {
    let smart = is_smart_topology(&preset.ai_model);
    let mut rows = vec![row("AI model", meshy_label(&preset.ai_model))];
    if smart {
        rows.push(row("Faces", count(preset.target_polycount)));
    } else if preset.should_remesh {
        rows.push(row_note(
            "Remesh",
            format!(
                "{} {}",
                count(preset.target_polycount),
                match preset.topology {
                    Topology::Triangle => "triangles",
                    Topology::Quad => "quads",
                }
            ),
            "target polycount",
        ));
    } else {
        rows.push(row("Remesh", "Off"));
    }
    rows.push(row("Texture", on_off(preset.should_texture)));
    if preset.should_texture {
        rows.push(row(
            "Texture size",
            match preset.texture_resolution {
                MeshyTexture::K2 => "2K",
                MeshyTexture::K4 => "4K",
                MeshyTexture::K8 => "8K",
            },
        ));
    }
    rows.push(row(
        "PBR",
        on_off(preset.should_texture && preset.enable_pbr),
    ));
    rows.push(row(
        "Pose",
        match preset.pose_mode {
            PoseMode::None => "As drawn",
            PoseMode::APose => "A-pose",
            PoseMode::TPose => "T-pose",
        },
    ));
    if preset.ai_model == "meshy-6" && meshy_takes_remove_lighting(&preset.ai_model) {
        rows.push(row("Remove lighting", on_off(preset.remove_lighting)));
    }
    if variant == PresetVariant::Character && preset.rig {
        rows.push(row_note(
            "Rig",
            format!("{:.2} m tall", preset.height_meters),
            "Animate > Rig after it finishes",
        ));
    }
    rows
}

/// The images' names in the folder: `front.png`, `left.png`, ...
pub fn view_names(images: &[PathBuf]) -> Vec<String> {
    images
        .iter()
        .zip(VIEWS)
        .map(|(path, view)| {
            let ext = path
                .extension()
                .map_or_else(|| "png".to_string(), |e| e.to_string_lossy().to_lowercase());
            format!("{view}.{ext}")
        })
        .collect()
}

pub struct Handoff<'a> {
    pub id: &'a str,
    pub asset_name: &'a str,
    pub service: ProviderId,
    pub variant: PresetVariant,
    pub images: &'a [PathBuf],
    pub tripo: Option<&'a TripoPreset>,
    pub meshy: Option<&'a MeshyPreset>,
}

impl Handoff<'_> {
    pub fn card(&self) -> HandoffCard {
        let multi = self.images.len() >= 2;
        let (rows, mode) = match self.service {
            ProviderId::Tripo => {
                let preset = self.tripo.cloned().unwrap_or_else(TripoPreset::prop);
                let multi = multi && preset.multi_view;
                (
                    tripo_rows(&preset, self.variant, self.images.len()),
                    if multi {
                        "Multiview to 3D"
                    } else {
                        "Image to 3D"
                    },
                )
            }
            ProviderId::Meshy => {
                let preset = self.meshy.cloned().unwrap_or_else(MeshyPreset::prop);
                let multi = multi && preset.multi_view && !preset.smart_topology();
                (
                    meshy_rows(&preset, self.variant),
                    if multi {
                        "Multi-image to 3D"
                    } else {
                        "Image to 3D"
                    },
                )
            }
        };
        let names = view_names(self.images);
        let used = if mode == "Image to 3D" {
            1
        } else {
            names.len()
        };
        HandoffCard {
            service: self.service,
            site: site(self.service).to_string(),
            mode: mode.to_string(),
            rows,
            images: names.into_iter().take(used).collect(),
            return_name: format!("{}.glb", self.id),
        }
    }

    fn markdown(&self, card: &HandoffCard) -> String {
        let service = self.service.label();
        let mut text = format!(
            "# {} for {}\n\nMake this model in the {service} web app, then bring the GLB back to Maquette.\n\n",
            self.id, self.asset_name
        );
        text.push_str(&format!(
            "1. Open {} and start a new {}.\n",
            card.site, card.mode
        ));
        text.push_str(&format!("2. Upload {}.\n", card.images.join(", ")));
        text.push_str(
            "3. Enter these settings:\n\n| Setting | Value | Note |\n| --- | --- | --- |\n",
        );
        for row in &card.rows {
            text.push_str(&format!(
                "| {} | {} | {} |\n",
                row.label,
                row.value,
                row.note.as_deref().unwrap_or("")
            ));
        }
        text.push_str(&format!(
            "\n4. Download the result as GLB.\n5. Drop it on the {} card on the Board, or save it into the inbox as `{}`.\n",
            self.asset_name, card.return_name
        ));
        text
    }

    /// Writes the folder and returns the card. Fails when the folder exists.
    pub fn write(&self, project_dir: &Path) -> CoreResult<(PathBuf, HandoffCard)> {
        let dir = project_dir.join(HANDOFF_DIR).join(self.id);
        if dir.exists() {
            return Err(CoreError::Invalid(format!(
                "handoff/{} already exists. Remove it or send again.",
                self.id
            )));
        }
        fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
        let card = self.card();
        for (from, name) in self.images.iter().zip(view_names(self.images)) {
            let to = dir.join(&name);
            fs::copy(from, &to).map_err(|e| CoreError::io(from, e))?;
        }
        let md = dir.join("settings.md");
        write_atomic(&md, self.markdown(&card).as_bytes()).map_err(|e| CoreError::io(&md, e))?;
        write_json(
            &dir.join("settings.json"),
            &json!({
                "id": self.id,
                "asset": self.asset_name,
                "service": self.service,
                "variant": self.variant,
                "tripo": self.tripo,
                "meshy": self.meshy,
                "card": card,
            }),
        )?;
        Ok((dir, card))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_lists_the_preset_in_web_app_words() {
        let rows = tripo_rows(&TripoPreset::character(), PresetVariant::Character, 2);
        let get = |label: &str| {
            rows.iter()
                .find(|r| r.label == label)
                .map(|r| r.value.clone())
        };
        assert_eq!(get("Model").as_deref(), Some("H3.1"));
        assert_eq!(get("Face limit").as_deref(), Some("50,000"));
        assert_eq!(get("Auto rig").as_deref(), Some("Mixamo bones"));
        assert_eq!(get("Animations").as_deref(), Some("Idle, Walk"));
        assert_eq!(get("Fix the image first"), None, "multiview has no autofix");

        let mut p1 = TripoPreset::prop();
        p1.model = "P1-20260311".into();
        let rows = tripo_rows(&p1, PresetVariant::Prop, 1);
        assert!(rows.iter().all(|r| r.label != "Smart low poly"));

        let rows = meshy_rows(&MeshyPreset::character(), PresetVariant::Character);
        let get = |label: &str| {
            rows.iter()
                .find(|r| r.label == label)
                .map(|r| r.value.clone())
        };
        assert_eq!(get("AI model").as_deref(), Some("Meshy 7.1"));
        assert_eq!(get("Remesh").as_deref(), Some("50,000 triangles"));
        assert_eq!(get("Pose").as_deref(), Some("A-pose"));
        assert_eq!(get("Rig").as_deref(), Some("1.70 m tall"));
        assert_eq!(get("Remove lighting"), None);
    }
}
