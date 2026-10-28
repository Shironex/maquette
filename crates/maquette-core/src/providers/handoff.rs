//! By hand: no API call. A send writes `projects/<p>/handoff/<id>/` with
//! the input images named by view and a settings card (`settings.md` and
//! `settings.json`) listing the exact values to enter in the service's web
//! app, and `send.json`, a sidecar that names the send and the file name to
//! export as (`maq_<asset>_v<n>_<provider>`, see [`super::naming`]). The
//! model comes back through the Downloads watcher, Add model, a drop on the
//! asset's card, or saved into the inbox under that name.

pub mod sidecar;
pub mod studio;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::catalog::{is_smart_topology, meshy_label, meshy_takes_remove_lighting};
use super::naming::export_stem;
use super::presets::{MeshyPreset, MeshyTexture, PoseMode, PresetVariant, ProviderId, TripoPreset};
use super::subscription::web_cost;
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
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
    /// The name to export as on the site, without an extension:
    /// `maq_<asset>_v<n>_<provider>`.
    #[serde(default)]
    pub export_name: String,
    /// About what the site charges for it, from a subscription.
    #[serde(default)]
    pub credits: Option<f64>,
    /// What those credits buy, in the site's words: "textured image to 3D".
    #[serde(default)]
    pub credits_note: Option<String>,
    /// The text to paste into the site's prompt box, if the send has one.
    #[serde(default)]
    pub prompt: Option<String>,
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

/// The images' names in the folder: `front.png`, `left.png`, ..., from
/// each image's view (`views`, from the asset's tags; empty takes
/// [`VIEWS`] in order).
pub fn view_names(images: &[PathBuf], views: &[String]) -> Vec<String> {
    let named: Vec<&str> = if views.len() == images.len() {
        views.iter().map(String::as_str).collect()
    } else {
        VIEWS.to_vec()
    };
    images
        .iter()
        .zip(named)
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
    pub project: &'a str,
    pub asset: &'a str,
    pub asset_name: &'a str,
    pub service: ProviderId,
    pub variant: PresetVariant,
    pub images: &'a [PathBuf],
    /// Each image's view, in order; empty for front, left, back, right.
    pub views: &'a [String],
    pub tripo: Option<&'a TripoPreset>,
    pub meshy: Option<&'a MeshyPreset>,
    /// Meshy only: the texture prompt, for the web app's prompt box.
    pub prompt: Option<&'a str>,
}

impl Handoff<'_> {
    pub fn card(&self) -> HandoffCard {
        let multi = self.images.len() >= 2;
        let export_name = export_stem(self.id, self.asset, self.service);
        let (rows, mode) = match self.service {
            ProviderId::Tripo => {
                let preset = self.tripo.cloned().unwrap_or_else(TripoPreset::prop);
                let multi = multi && preset.multi_view;
                (
                    studio::tripo_rows(&preset, self.variant, &export_name),
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
        let mut rows = rows;
        let prompt = self.prompt.filter(|_| self.service == ProviderId::Meshy);
        if let Some(prompt) = prompt {
            rows.push(row("Texture prompt", prompt));
        }
        let cost = web_cost(self.service, self.tripo, self.meshy);
        let names = view_names(self.images, self.views);
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
            return_name: format!("{export_name}.glb"),
            export_name,
            credits: cost.map(|c| c.credits),
            credits_note: cost.map(|c| c.what.to_string()),
            prompt: prompt.map(str::to_string),
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
            "\n4. Download the result as GLB, named `{}`.\n5. Maquette picks it up from Downloads. Or drop it on the {} card on the Board, or save it into the inbox as `{}`.\n",
            card.export_name, self.asset_name, card.return_name
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
        for (from, name) in self.images.iter().zip(view_names(self.images, self.views)) {
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
        sidecar::write(&dir, self, &card)?;
        Ok((dir, card))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::presets::{TripoGeometryQuality, TripoTextureQuality};

    fn value(rows: &[CardRow], label: &str) -> Option<String> {
        rows.iter()
            .find(|r| r.label == label)
            .map(|r| r.value.clone())
    }

    #[test]
    fn the_tripo_card_speaks_studio_words() {
        let rows = studio::tripo_rows(
            &TripoPreset::character(),
            PresetVariant::Character,
            "maq_hero_v1_tripo",
        );
        let get = |label: &str| value(&rows, label);
        assert_eq!(get("Tab").as_deref(), Some("HD Model"));
        assert_eq!(get("AI Model").as_deref(), Some("H3.1"));
        assert_eq!(get("Ultra Mesh Quality").as_deref(), Some("Off"));
        assert_eq!(get("Texture Quality").as_deref(), Some("2K"));
        assert_eq!(get("Topology").as_deref(), Some("Triangle"));
        assert_eq!(get("Polycount").as_deref(), Some("50000"));
        assert_eq!(get("AI Complete").as_deref(), Some("On"));
        assert_eq!(get("Remove Lighting").as_deref(), Some("On"));
        assert_eq!(get("Generate in Parts").as_deref(), Some("Off"));
        assert_eq!(get("Animations").as_deref(), Some("Idle, Walk"));
        assert_eq!(get("Export").as_deref(), Some("maq_hero_v1_tripo"));
        assert!(get("Rig").is_some_and(|r| r.starts_with("Mixamo skeleton, export GLB")));
        for api in [
            "Model",
            "Face limit",
            "Geometry",
            "Smart low poly",
            "Quad mesh",
        ] {
            assert_eq!(get(api), None, "{api} is an API name");
        }
        for gone in ["Real-world size", "Fix the image first"] {
            assert_eq!(get(gone), None, "Studio has no {gone}");
        }

        let mut prop = TripoPreset::prop();
        prop.smart_low_poly = true;
        prop.geometry_quality = TripoGeometryQuality::Detailed;
        prop.texture_quality = TripoTextureQuality::Detailed;
        let rows = studio::tripo_rows(&prop, PresetVariant::Prop, "maq_cup_v2_tripo");
        let get = |label: &str| value(&rows, label);
        assert_eq!(get("Tab").as_deref(), Some("Smart Mesh"));
        assert_eq!(get("Ultra Mesh Quality").as_deref(), Some("On"));
        assert_eq!(get("Texture Quality").as_deref(), Some("4K"));
        assert_eq!(get("Polycount").as_deref(), Some("20000"), "adaptive");
        assert_eq!(get("Rig"), None, "a prop has no rig");

        let mut p1 = TripoPreset::prop();
        p1.model = "P1-20260311".into();
        let rows = studio::tripo_rows(&p1, PresetVariant::Prop, "maq_cup_v3_tripo");
        assert!(rows.iter().all(|r| r.label != "Tab"));
    }

    #[test]
    fn settings_md_lists_the_studio_rows_and_the_export_name() {
        let dir = tempfile::tempdir().expect("dir");
        let image = dir.path().join("lamp.png");
        fs::write(&image, b"png").expect("image");
        let preset = TripoPreset::prop();
        let handoff = Handoff {
            id: "iron-street-lamp-1",
            project: "p",
            asset: "iron-street-lamp",
            asset_name: "Iron street lamp",
            service: ProviderId::Tripo,
            variant: PresetVariant::Prop,
            images: std::slice::from_ref(&image),
            views: &[],
            tripo: Some(&preset),
            meshy: None,
            prompt: None,
        };
        let (folder, card) = handoff.write(dir.path()).expect("write");
        assert_eq!(card.export_name, "maq_iron-street-lamp_v1_tripo");
        let md = fs::read_to_string(folder.join("settings.md")).expect("md");
        for line in [
            "| Tab | HD Model |  |",
            "| Ultra Mesh Quality | Off |  |",
            "| Texture Quality | 2K |  |",
            "| Polycount | 20000 | Studio has no adaptive |",
            "| Topology | Triangle | quads export FBX only |",
            "| Generate in Parts | Off | recommended |",
            "| Export | maq_iron-street-lamp_v1_tripo | GLB, with this file name |",
        ] {
            assert!(md.contains(line), "{line} in {md}");
        }
        assert!(!md.contains("Face limit") && !md.contains("Real-world size"));
    }

    #[test]
    fn the_meshy_card_keeps_its_words() {
        let rows = meshy_rows(&MeshyPreset::character(), PresetVariant::Character);
        let get = |label: &str| value(&rows, label);
        assert_eq!(get("AI model").as_deref(), Some("Meshy 7.1"));
        assert_eq!(get("Remesh").as_deref(), Some("50,000 triangles"));
        assert_eq!(get("Pose").as_deref(), Some("A-pose"));
        assert_eq!(get("Rig").as_deref(), Some("1.70 m tall"));
        assert_eq!(get("Remove lighting"), None);
    }
}
