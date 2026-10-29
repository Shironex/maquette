//! What a send should cost, from the providers' published API price lists
//! (Tripo's H series table and Meshy's pricing page, as of October 2026).
//! The providers report what a finished task really cost; these numbers
//! stand in until then, and the credits ledger marks them as estimated.
//!
//! Tripo has no published table for the P series, so it is priced like the
//! H series. Failed and cancelled tasks are not charged on either service.

use super::catalog::{is_smart_topology, is_tripo_p_series};
use super::jobs::{Provider, ProviderJob};
use super::presets::{
    MeshyPreset, MeshyTexture, PresetVariant, TripoGeometryQuality, TripoPreset,
    TripoTextureQuality,
};

/// Tripo image or multiview to model.
const TRIPO_BASE: f64 = 20.0;
const TRIPO_TEXTURE: f64 = 10.0;
const TRIPO_HD_TEXTURE: f64 = 10.0;
const TRIPO_ULTRA_TEXTURE: f64 = 20.0;
const TRIPO_HD_GEOMETRY: f64 = 20.0;
const TRIPO_SMART_LOW_POLY: f64 = 10.0;
const TRIPO_RIG: f64 = 25.0;
/// Per animation; the rig check is free.
const TRIPO_RETARGET: f64 = 10.0;
/// Tripo retargets at most this many animations in one task.
const TRIPO_MAX_ANIMATIONS: usize = 5;

const MESHY_RIG: f64 = 5.0;

/// Credits for one Tripo send with these settings.
pub fn tripo_credits(preset: &TripoPreset, variant: PresetVariant) -> f64 {
    let legacy = preset.model.starts_with("v2");
    let p_series = is_tripo_p_series(&preset.model);
    let mut credits = TRIPO_BASE;
    if preset.texture {
        credits += TRIPO_TEXTURE;
        if !legacy {
            credits += match preset.texture_quality {
                TripoTextureQuality::Standard => 0.0,
                TripoTextureQuality::Detailed => TRIPO_HD_TEXTURE,
                TripoTextureQuality::Extreme => TRIPO_ULTRA_TEXTURE,
            };
        }
    }
    if !legacy && !p_series {
        if preset.geometry_quality == TripoGeometryQuality::Detailed {
            credits += TRIPO_HD_GEOMETRY;
        }
        if preset.smart_low_poly {
            credits += TRIPO_SMART_LOW_POLY;
        }
    }
    if variant == PresetVariant::Character && preset.rig {
        credits += TRIPO_RIG;
        let animations = preset.animations.len().min(TRIPO_MAX_ANIMATIONS);
        credits += TRIPO_RETARGET * animations as f64;
    }
    credits
}

/// Credits for one Meshy send with these settings. Multi-image costs the
/// same as one image; Smart Topology and Lite models are cheaper.
pub fn meshy_credits(preset: &MeshyPreset, variant: PresetVariant) -> f64 {
    let cheap = is_smart_topology(&preset.ai_model) || preset.ai_model == "meshy-6-lite";
    let mut credits = match (cheap, preset.should_texture, preset.texture_resolution) {
        (true, false, _) => 5.0,
        (true, true, MeshyTexture::K8) => 20.0,
        (true, true, _) => 15.0,
        (false, false, _) => 20.0,
        (false, true, MeshyTexture::K8) => 35.0,
        (false, true, _) => 30.0,
    };
    if variant == PresetVariant::Character && preset.rig {
        credits += MESHY_RIG;
    }
    credits
}

/// The expected credits for a send, or `None` for By hand.
pub fn estimate(
    provider: Provider,
    variant: PresetVariant,
    tripo: Option<&TripoPreset>,
    meshy: Option<&MeshyPreset>,
) -> Option<f64> {
    match provider {
        Provider::Tripo => tripo.map(|t| tripo_credits(t, variant)),
        Provider::Meshy => meshy.map(|m| meshy_credits(m, variant)),
        Provider::ByHand => None,
    }
}

impl ProviderJob {
    /// What this send should cost; `None` for By hand.
    pub fn estimate(&self) -> Option<f64> {
        if let Some(split) = &self.split {
            return super::split::credits(split.tripo.as_ref(), split.meshy.as_ref());
        }
        estimate(
            self.provider,
            self.variant,
            self.tripo.as_ref(),
            self.meshy.as_ref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tripo_adds_texture_quality_geometry_and_the_rig_chain() {
        let mut prop = TripoPreset::prop();
        assert_eq!(tripo_credits(&prop, PresetVariant::Prop), 30.0);
        prop.texture = false;
        prop.pbr = false;
        assert_eq!(tripo_credits(&prop, PresetVariant::Prop), 20.0);
        prop.texture = true;
        prop.texture_quality = TripoTextureQuality::Detailed;
        prop.geometry_quality = TripoGeometryQuality::Detailed;
        assert_eq!(tripo_credits(&prop, PresetVariant::Prop), 60.0);
        // The rig only counts for characters: 30 + 25 + 2 animations.
        let character = TripoPreset::character();
        assert_eq!(tripo_credits(&character, PresetVariant::Character), 75.0);
        assert_eq!(tripo_credits(&character, PresetVariant::Prop), 30.0);
    }

    #[test]
    fn tripo_skips_what_older_and_p_series_models_do_not_take() {
        let mut preset = TripoPreset::prop();
        preset.geometry_quality = TripoGeometryQuality::Detailed;
        preset.smart_low_poly = true;
        preset.model = "P1-20260311".into();
        assert_eq!(tripo_credits(&preset, PresetVariant::Prop), 30.0);
        preset.model = "v2.5-20250123".into();
        preset.texture_quality = TripoTextureQuality::Extreme;
        assert_eq!(tripo_credits(&preset, PresetVariant::Prop), 30.0);
    }

    #[test]
    fn meshy_prices_follow_the_model_and_texture_size() {
        let mut preset = MeshyPreset::prop();
        assert_eq!(meshy_credits(&preset, PresetVariant::Prop), 30.0);
        preset.texture_resolution = MeshyTexture::K8;
        assert_eq!(meshy_credits(&preset, PresetVariant::Prop), 35.0);
        preset.ai_model = "meshy-t2".into();
        assert_eq!(meshy_credits(&preset, PresetVariant::Prop), 20.0);
        preset.should_texture = false;
        assert_eq!(meshy_credits(&preset, PresetVariant::Prop), 5.0);
        let character = MeshyPreset::character();
        assert_eq!(meshy_credits(&character, PresetVariant::Character), 35.0);
    }

    #[test]
    fn by_hand_costs_nothing_to_estimate() {
        assert_eq!(
            estimate(Provider::ByHand, PresetVariant::Prop, None, None),
            None
        );
        let tripo = TripoPreset::prop();
        assert_eq!(
            estimate(Provider::Tripo, PresetVariant::Prop, Some(&tripo), None),
            Some(30.0)
        );
    }
}
