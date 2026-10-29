//! The per provider presets in `maquette.json`: what a "Send to 3D" starts
//! from, one variant for props and scenes and one for characters. Every
//! send can change them for that send only.

use serde::{Deserialize, Serialize};

use super::catalog;
use crate::workspace::Topology;

/// The two services Maquette can call.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    Tripo,
    Meshy,
}

impl ProviderId {
    pub const ALL: [ProviderId; 2] = [ProviderId::Tripo, ProviderId::Meshy];

    pub fn label(self) -> &'static str {
        match self {
            ProviderId::Tripo => "Tripo",
            ProviderId::Meshy => "Meshy",
        }
    }
}

/// Which preset a send starts from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PresetVariant {
    #[default]
    Prop,
    Character,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum TripoTextureQuality {
    #[default]
    Standard,
    Detailed,
    Extreme,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum TripoGeometryQuality {
    #[default]
    Standard,
    Detailed,
}

/// The bone naming a Tripo rig uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum TripoRigSpec {
    #[default]
    Mixamo,
    Tripo,
}

/// One Tripo preset. A character preset also rigs and animates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TripoPreset {
    /// A Tripo model id from the catalog, for example `v3.1-20260211`.
    pub model: String,
    /// The face budget; 0 lets Tripo pick (adaptive).
    pub face_limit: u32,
    pub texture: bool,
    pub pbr: bool,
    pub texture_quality: TripoTextureQuality,
    /// Ultra geometry. H3.0 and newer only.
    pub geometry_quality: TripoGeometryQuality,
    /// Clean hand made style low poly topology. H series only.
    pub smart_low_poly: bool,
    /// Scales the model to real world size in metres.
    pub auto_size: bool,
    /// Lets Tripo clean up the input image first.
    pub image_autofix: bool,
    /// Several images go to the multiview endpoint, front first.
    pub multi_view: bool,
    /// Rig check, then rig, then retarget the animations.
    pub rig: bool,
    /// A Tripo rig model id from the catalog.
    pub rig_model: String,
    pub rig_spec: TripoRigSpec,
    /// Preset animation ids, for example `preset:idle`. At most 5.
    pub animations: Vec<String>,
}

impl TripoPreset {
    pub fn prop() -> Self {
        Self {
            model: catalog::TRIPO_DEFAULT_MODEL.to_string(),
            face_limit: 0,
            texture: true,
            pbr: true,
            texture_quality: TripoTextureQuality::Standard,
            geometry_quality: TripoGeometryQuality::Standard,
            smart_low_poly: false,
            auto_size: true,
            image_autofix: false,
            multi_view: false,
            rig: false,
            rig_model: catalog::TRIPO_DEFAULT_RIG_MODEL.to_string(),
            rig_spec: TripoRigSpec::Mixamo,
            animations: Vec::new(),
        }
    }

    pub fn character() -> Self {
        Self {
            face_limit: 50_000,
            multi_view: true,
            rig: true,
            animations: vec!["preset:idle".to_string(), "preset:walk".to_string()],
            ..Self::prop()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TripoPresets {
    pub prop: TripoPreset,
    pub character: TripoPreset,
}

impl Default for TripoPresets {
    fn default() -> Self {
        Self {
            prop: TripoPreset::prop(),
            character: TripoPreset::character(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum MeshyTexture {
    #[default]
    #[serde(rename = "2k")]
    K2,
    #[serde(rename = "4k")]
    K4,
    #[serde(rename = "8k")]
    K8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PoseMode {
    #[default]
    None,
    APose,
    TPose,
}

/// One Meshy preset. A character preset also rigs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeshyPreset {
    /// A Meshy model id from the catalog. `meshy-t2` and
    /// `meshy-t2.5` are Smart Topology models.
    pub ai_model: String,
    pub should_texture: bool,
    pub enable_pbr: bool,
    pub texture_resolution: MeshyTexture,
    /// Remesh to `target_polycount` (standard models only).
    pub should_remesh: bool,
    pub topology: Topology,
    /// The face count to aim for when remeshing, or the Smart Topology
    /// face count.
    pub target_polycount: u32,
    pub pose_mode: PoseMode,
    /// Takes effect with meshy-6 only; other models ignore it.
    pub remove_lighting: bool,
    /// Several images go to Multi-Image to 3D, front first.
    pub multi_view: bool,
    /// Rigs a humanoid after generating.
    pub rig: bool,
    /// The character height the rig scales to, in metres.
    pub height_meters: f64,
}

impl MeshyPreset {
    pub fn prop() -> Self {
        Self {
            ai_model: catalog::MESHY_DEFAULT_MODEL.to_string(),
            should_texture: true,
            enable_pbr: true,
            texture_resolution: MeshyTexture::K2,
            should_remesh: false,
            topology: Topology::Triangle,
            target_polycount: 30_000,
            pose_mode: PoseMode::None,
            remove_lighting: true,
            multi_view: false,
            rig: false,
            height_meters: 1.7,
        }
    }

    pub fn character() -> Self {
        Self {
            should_remesh: true,
            target_polycount: 50_000,
            pose_mode: PoseMode::APose,
            multi_view: true,
            rig: true,
            ..Self::prop()
        }
    }

    /// True for the Smart Topology models.
    pub fn smart_topology(&self) -> bool {
        catalog::is_smart_topology(&self.ai_model)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeshyPresets {
    pub prop: MeshyPreset,
    pub character: MeshyPreset,
}

impl Default for MeshyPresets {
    fn default() -> Self {
        Self {
            prop: MeshyPreset::prop(),
            character: MeshyPreset::character(),
        }
    }
}

/// The "3D providers" section of the workspace config. The API keys are
/// not here: they live in the OS keychain.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettings {
    pub tripo: TripoPresets,
    pub meshy: MeshyPresets,
}

impl ProviderSettings {
    pub fn tripo(&self, variant: PresetVariant) -> &TripoPreset {
        match variant {
            PresetVariant::Prop => &self.tripo.prop,
            PresetVariant::Character => &self.tripo.character,
        }
    }

    pub fn meshy(&self, variant: PresetVariant) -> &MeshyPreset {
        match variant {
            PresetVariant::Prop => &self.meshy.prop,
            PresetVariant::Character => &self.meshy.character,
        }
    }
}

/// Why a Tripo preset would be refused, in the words the Settings field
/// shows. `None` when it is fine.
pub fn tripo_problem(preset: &TripoPreset) -> Option<String> {
    if preset.model.trim().is_empty() {
        return Some("Pick a Tripo model.".to_string());
    }
    if preset.face_limit > 0 {
        let (low, high) = catalog::tripo_face_range(
            &preset.model,
            preset.smart_low_poly,
            preset.geometry_quality == TripoGeometryQuality::Detailed,
        );
        if !(low..=high).contains(&preset.face_limit) {
            return Some(format!(
                "Tripo {} takes a face limit from {} to {}, or 0 for adaptive.",
                catalog::tripo_label(&preset.model),
                group(low),
                group(high)
            ));
        }
    }
    if preset.rig && preset.rig_model.trim().is_empty() {
        return Some("Pick a Tripo rig model.".to_string());
    }
    if preset.animations.len() > 5 {
        return Some("Tripo retargets at most 5 animations at once.".to_string());
    }
    None
}

/// Why a Meshy preset would be refused, or `None`.
pub fn meshy_problem(preset: &MeshyPreset) -> Option<String> {
    if preset.ai_model.trim().is_empty() {
        return Some("Pick a Meshy model.".to_string());
    }
    let (low, high) = catalog::meshy_polycount_range(&preset.ai_model);
    if (preset.should_remesh || preset.smart_topology())
        && !(low..=high).contains(&preset.target_polycount)
    {
        return Some(format!(
            "{} aims for {} to {} faces.",
            catalog::meshy_label(&preset.ai_model),
            group(low),
            group(high)
        ));
    }
    if preset.smart_topology() && preset.topology == Topology::Quad {
        return Some("Smart Topology models only make triangles.".to_string());
    }
    if !preset.height_meters.is_finite() || preset.height_meters <= 0.0 {
        return Some("The character height must be more than 0 m.".to_string());
    }
    None
}

/// 20000 becomes "20,000".
pub fn group(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_spelled_for_the_apis() {
        let settings = ProviderSettings::default();
        for variant in [PresetVariant::Prop, PresetVariant::Character] {
            assert_eq!(tripo_problem(settings.tripo(variant)), None);
            assert_eq!(meshy_problem(settings.meshy(variant)), None);
        }
        let json = serde_json::to_string(&settings).expect("json");
        for needle in [
            "\"model\":\"v3.1-20260211\"",
            "\"rigModel\":\"v3.0-20260909\"",
            "\"rigSpec\":\"mixamo\"",
            "\"aiModel\":\"latest\"",
            "\"textureResolution\":\"2k\"",
            "\"poseMode\":\"a-pose\"",
            "\"preset:idle\"",
        ] {
            assert!(json.contains(needle), "missing {needle} in {json}");
        }
        assert!(!json.contains("lowpoly"));
    }

    #[test]
    fn face_limits_follow_the_model() {
        let mut p = TripoPreset::prop();
        p.model = "P1-20260311".into();
        p.face_limit = 30_000;
        assert!(tripo_problem(&p).is_some_and(|m| m.contains("20,000")));
        p.face_limit = 0;
        assert_eq!(tripo_problem(&p), None);
        p.model = "P2-20260801".into();
        p.face_limit = 50_000;
        assert_eq!(tripo_problem(&p), None);
        p.animations = (0..6).map(|i| format!("preset:{i}")).collect();
        assert!(tripo_problem(&p).is_some());

        let mut m = MeshyPreset::prop();
        m.ai_model = "meshy-t2".into();
        m.target_polycount = 20_000;
        assert!(meshy_problem(&m).is_some_and(|t| t.contains("15,000")));
        m.target_polycount = 4000;
        assert_eq!(meshy_problem(&m), None);
        m.topology = Topology::Quad;
        assert!(meshy_problem(&m).is_some());
        assert_eq!(group(1_500_000), "1,500,000");
    }
}
