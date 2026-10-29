//! What a By hand send asks Tripo Studio for beyond the preset: the mesh
//! tab (HD Model or Smart Mesh (P2.0)) and the rig. They are settings of
//! the send, saved on the job, and the card is written from them. Multi-view
//! has no setting: it follows the refs the send carries.

use serde::{Deserialize, Serialize};

use crate::catalog::is_tripo_p_series;
use crate::presets::{PresetVariant, TripoPreset};

/// Tripo Studio's mesh tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum StudioMesh {
    HdModel,
    SmartMesh,
}

impl StudioMesh {
    /// The tab's name in Studio.
    pub fn label(self) -> &'static str {
        match self {
            Self::HdModel => "HD Model",
            Self::SmartMesh => "Smart Mesh (P2.0)",
        }
    }
}

/// The rig Tripo Studio adds once the model is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum StudioRig {
    /// A humanoid on the Mixamo skeleton.
    Biped,
    Quadruped,
    MultiLegged,
    None,
}

impl StudioRig {
    pub fn label(self) -> &'static str {
        match self {
            Self::Biped => "Biped (Mixamo)",
            Self::Quadruped => "Quadruped",
            Self::MultiLegged => "Multi-legged",
            Self::None => "None",
        }
    }
}

/// What a send asks for; a field left out follows the preset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StudioChoice {
    #[serde(default)]
    pub mesh: Option<StudioMesh>,
    #[serde(default)]
    pub rig: Option<StudioRig>,
}

impl StudioChoice {
    /// This choice laid over `base`: a field it leaves out takes `base`'s.
    pub fn over(self, base: Option<StudioChoice>) -> StudioChoice {
        let base = base.unwrap_or_default();
        StudioChoice {
            mesh: self.mesh.or(base.mesh),
            rig: self.rig.or(base.rig),
        }
    }

    /// The settings the send goes with.
    pub fn resolve(self, preset: &TripoPreset, variant: PresetVariant) -> StudioSettings {
        let defaults = StudioSettings::defaults(preset, variant);
        StudioSettings {
            mesh: self.mesh.unwrap_or(defaults.mesh),
            rig: self.rig.unwrap_or(defaults.rig),
        }
    }
}

/// The Tripo Studio settings a By hand send went with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StudioSettings {
    pub mesh: StudioMesh,
    pub rig: StudioRig,
}

impl StudioSettings {
    /// What the preset says: Smart Mesh for a P series or smart low poly
    /// preset, and a biped rig for a character preset that rigs.
    pub fn defaults(preset: &TripoPreset, variant: PresetVariant) -> Self {
        let mesh = if is_tripo_p_series(&preset.model) || preset.smart_low_poly {
            StudioMesh::SmartMesh
        } else {
            StudioMesh::HdModel
        };
        let rig = if variant == PresetVariant::Character && preset.rig {
            StudioRig::Biped
        } else {
            StudioRig::None
        };
        Self { mesh, rig }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_follow_the_preset() {
        let character =
            StudioSettings::defaults(&TripoPreset::character(), PresetVariant::Character);
        assert_eq!(character.mesh, StudioMesh::HdModel);
        assert_eq!(character.rig, StudioRig::Biped);
        let prop = StudioSettings::defaults(&TripoPreset::prop(), PresetVariant::Prop);
        assert_eq!(prop.rig, StudioRig::None);
        let mut p2 = TripoPreset::prop();
        p2.model = "P2-20260801".into();
        assert_eq!(
            StudioSettings::defaults(&p2, PresetVariant::Prop).mesh,
            StudioMesh::SmartMesh
        );
    }

    #[test]
    fn a_choice_fills_its_gaps_from_the_batch_then_the_preset() {
        let batch = StudioChoice {
            mesh: Some(StudioMesh::SmartMesh),
            rig: Some(StudioRig::Quadruped),
        };
        let own = StudioChoice {
            mesh: None,
            rig: Some(StudioRig::None),
        };
        let merged = own.over(Some(batch));
        assert_eq!(merged.mesh, Some(StudioMesh::SmartMesh));
        assert_eq!(merged.rig, Some(StudioRig::None));
        let settings =
            StudioChoice::default().resolve(&TripoPreset::character(), PresetVariant::Character);
        assert_eq!(settings.rig, StudioRig::Biped);
    }

    #[test]
    fn older_records_and_requests_read_without_the_fields() {
        let choice: StudioChoice = serde_json::from_str("{}").expect("empty choice");
        assert_eq!(choice, StudioChoice::default());
        let settings: StudioSettings =
            serde_json::from_str(r#"{"mesh":"smart-mesh","rig":"multi-legged"}"#).expect("read");
        assert_eq!(settings.rig, StudioRig::MultiLegged);
    }
}
