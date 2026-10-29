//! What a Send to 3D asks for, alone or as a batch, and what a batch started.

use serde::{Deserialize, Serialize};

use super::{Provider, ProviderJob};
use crate::handoff::options::StudioChoice;
use crate::presets::{MeshyPreset, PresetVariant, ProviderId, TripoPreset};

/// What a Send to 3D asks for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    pub project: String,
    pub asset: String,
    pub provider: Provider,
    /// For By hand: whose web app and settings. Ignored otherwise.
    pub service: ProviderId,
    pub variant: PresetVariant,
    /// Relative to the project folder, front first, at most 4.
    pub images: Vec<String>,
    /// This send's settings; `None` takes the saved preset.
    pub tripo: Option<TripoPreset>,
    pub meshy: Option<MeshyPreset>,
    /// Meshy only: text that guides the texture, at most 800 characters.
    #[serde(default)]
    pub prompt: Option<String>,
    /// The earlier send of the same asset this one redoes.
    #[serde(default)]
    pub redo_of: Option<String>,
    /// By hand on Tripo only: the Studio tab and rig. A field left out
    /// follows the preset.
    #[serde(default)]
    pub studio: Option<StudioChoice>,
}

/// One asset of a batch send.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchItem {
    pub asset: String,
    /// Per item: a prop or a character.
    pub variant: PresetVariant,
    /// Relative to the project folder, front first, at most 4.
    pub images: Vec<String>,
    /// This asset's own settings, already merged over the batch preset of
    /// its variant. `None` follows the batch preset.
    pub tripo: Option<TripoPreset>,
    pub meshy: Option<MeshyPreset>,
    /// Meshy only: text that guides this asset's texture.
    #[serde(default)]
    pub prompt: Option<String>,
    /// The earlier send of this asset it redoes.
    #[serde(default)]
    pub redo_of: Option<String>,
    /// By hand on Tripo only: this asset's Studio tab and rig, laid over
    /// the batch's.
    #[serde(default)]
    pub studio: Option<StudioChoice>,
}

/// Several assets to one provider: one model per asset. The settings for
/// each variant start from the saved preset; `Some` changes them for this
/// batch only, and an item can carry its own on top.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequest {
    pub project: String,
    pub provider: Provider,
    /// For By hand: whose web app and settings. Ignored otherwise.
    pub service: ProviderId,
    pub items: Vec<BatchItem>,
    pub tripo_prop: Option<TripoPreset>,
    pub tripo_character: Option<TripoPreset>,
    pub meshy_prop: Option<MeshyPreset>,
    pub meshy_character: Option<MeshyPreset>,
    /// By hand on Tripo only: the Studio tab and rig for every asset. A
    /// field left out follows each asset's preset.
    #[serde(default)]
    pub studio: Option<StudioChoice>,
}

impl BatchRequest {
    /// The single send an item makes.
    pub fn send_request(&self, item: &BatchItem) -> SendRequest {
        let service = self.provider.service().unwrap_or(self.service);
        let character = item.variant == PresetVariant::Character;
        SendRequest {
            project: self.project.clone(),
            asset: item.asset.clone(),
            provider: self.provider,
            service,
            variant: item.variant,
            images: item.images.clone(),
            tripo: (service == ProviderId::Tripo)
                .then(|| {
                    item.tripo.clone().or_else(|| {
                        if character {
                            self.tripo_character.clone()
                        } else {
                            self.tripo_prop.clone()
                        }
                    })
                })
                .flatten(),
            meshy: (service == ProviderId::Meshy)
                .then(|| {
                    item.meshy.clone().or_else(|| {
                        if character {
                            self.meshy_character.clone()
                        } else {
                            self.meshy_prop.clone()
                        }
                    })
                })
                .flatten(),
            prompt: item.prompt.clone(),
            redo_of: item.redo_of.clone(),
            studio: match (item.studio, self.studio) {
                (Some(own), batch) => Some(own.over(batch)),
                (None, batch) => batch,
            },
        }
    }
}

/// An item of a batch that did not go out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchFailure {
    pub asset: String,
    pub message: String,
}

/// What a batch send started.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchSent {
    pub batch: String,
    /// In the order asked, one per asset.
    pub jobs: Vec<ProviderJob>,
    pub failed: Vec<BatchFailure>,
}
