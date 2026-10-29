//! `send.json` in a handoff folder: which send the folder is for and the
//! file name the model should come back under, so the Downloads watcher (or
//! a person, or another tool) can tell what a download belongs to.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{Handoff, HandoffCard};
use crate::error::CoreResult;
use crate::json::write_json;
use crate::naming::send_number;
use crate::presets::ProviderId;
use maquette_base::history::now_ms;

pub const SIDECAR_FILE: &str = "send.json";
const SIDECAR_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sidecar {
    pub v: u32,
    pub job: String,
    pub project: String,
    pub asset: String,
    pub asset_name: String,
    /// The send's number for the asset, the `v<n>` of the export name.
    pub version: Option<u32>,
    pub provider: ProviderId,
    pub prompt: Option<String>,
    /// Without an extension; .glb, or the .zip the site downloads.
    pub expected_name: String,
    /// About what the site charges, from a subscription.
    pub credits: Option<f64>,
    pub created_at_ms: f64,
}

pub(super) fn write(dir: &Path, handoff: &Handoff<'_>, card: &HandoffCard) -> CoreResult<()> {
    let sidecar = Sidecar {
        v: SIDECAR_VERSION,
        job: handoff.id.to_string(),
        project: handoff.project.to_string(),
        asset: handoff.asset.to_string(),
        asset_name: handoff.asset_name.to_string(),
        version: send_number(handoff.id, handoff.asset),
        provider: handoff.service,
        prompt: card.prompt.clone(),
        expected_name: card.export_name.clone(),
        credits: card.credits,
        created_at_ms: now_ms(),
    };
    write_json(&dir.join(SIDECAR_FILE), &sidecar)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::presets::{MeshyPreset, PresetVariant};

    #[test]
    fn the_sidecar_names_the_send_and_its_export_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let image = dir.path().join("lamp_front.png");
        std::fs::write(&image, b"png").expect("image");
        let images: Vec<PathBuf> = vec![image];
        let meshy = MeshyPreset::prop();
        let handoff = Handoff {
            id: "desk-lamp-2",
            project: "studio",
            asset: "desk-lamp",
            asset_name: "Desk lamp",
            service: ProviderId::Meshy,
            variant: PresetVariant::Prop,
            images: &images,
            views: &[],
            tripo: None,
            meshy: Some(&meshy),
            studio: None,
            prompt: Some("brushed brass, warm bulb"),
        };
        let (folder, card) = handoff.write(dir.path()).expect("write");
        assert_eq!(card.export_name, "maq_desk-lamp_v2_meshy");
        assert_eq!(card.return_name, "maq_desk-lamp_v2_meshy.glb");
        assert_eq!(card.prompt.as_deref(), Some("brushed brass, warm bulb"));
        assert!(card.credits.is_some());

        let text = std::fs::read_to_string(folder.join(SIDECAR_FILE)).expect("sidecar");
        let read: Sidecar = serde_json::from_str(&text).expect("parse");
        assert_eq!(read.job, "desk-lamp-2");
        assert_eq!(read.asset, "desk-lamp");
        assert_eq!(read.version, Some(2));
        assert_eq!(read.provider, ProviderId::Meshy);
        assert_eq!(read.expected_name, "maq_desk-lamp_v2_meshy");
        assert_eq!(read.prompt.as_deref(), Some("brushed brass, warm bulb"));
        let md = std::fs::read_to_string(folder.join("settings.md")).expect("md");
        assert!(md.contains("maq_desk-lamp_v2_meshy"), "{md}");
    }
}
