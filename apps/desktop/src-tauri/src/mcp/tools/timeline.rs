//! The Timeline view's edits for agents: `set_layout` takes explode tracks,
//! the render style track and the hero text track through the edits here. The stop and scroll triggers the view places go
//! through the `states` edits of [`super::motion`].

use std::collections::BTreeMap;

use maquette_core::layout::{ExplodeKey, ExplodeTrack, PathKey, PathTrack, StyleTrack, TrackLook};
use rmcp::schemars;
use serde::Deserialize;

use super::motion::EasingEdit;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExplodeKeyEdit {
    /// The camera stop id, as get_layout lists it.
    pub stop: String,
    /// 0 with the parts together, 1 spread the whole distance.
    pub amount: f32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExplodeEdit {
    /// The asset id.
    pub asset: String,
    /// Metres each part moves out from the model's middle at an amount of 1; 0.25 when left
    /// out.
    pub distance: Option<f32>,
    /// How the amount eases from one key to the next; ease-in-out when left out.
    pub easing: Option<EasingEdit>,
    /// The amount at camera stops, at most one key per stop. Replaces the track's keys.
    #[serde(default)]
    pub keys: Vec<ExplodeKeyEdit>,
    /// Drop the asset's explode track.
    #[serde(default)]
    pub remove: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LookEdit {
    Blueprint,
    Toon,
    Clay,
}

impl LookEdit {
    fn look(self) -> TrackLook {
        match self {
            Self::Blueprint => TrackLook::Blueprint,
            Self::Toon => TrackLook::Toon,
            Self::Clay => TrackLook::Clay,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StyleTrackEdit {
    /// The look the scene fades into; blueprint when left out.
    pub look: Option<LookEdit>,
    /// How the amount eases from one key to the next; linear when left out.
    pub easing: Option<EasingEdit>,
    /// 0 as built, 1 the whole look, at camera stops, at most one key per stop.
    #[serde(default)]
    pub keys: Vec<ExplodeKeyEdit>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextTrackEdit {
    /// How the amount eases from one key to the next; linear when left out.
    pub easing: Option<EasingEdit>,
    /// 0 hidden, 1 shown, at camera stops, at most one key per stop.
    #[serde(default)]
    pub keys: Vec<ExplodeKeyEdit>,
}

fn keys_of(edits: &[ExplodeKeyEdit]) -> Vec<PathKey> {
    edits
        .iter()
        .map(|k| PathKey {
            stop: k.stop.trim().to_string(),
            amount: k.amount,
        })
        .collect()
}

/// The layout patch's render style track.
pub(super) fn style_patch(edit: &StyleTrackEdit) -> StyleTrack {
    let base = StyleTrack::default();
    StyleTrack {
        look: edit.look.map_or(base.look, LookEdit::look),
        easing: edit.easing.map_or(base.easing, EasingEdit::easing),
        keys: keys_of(&edit.keys),
    }
}

/// The layout patch's hero text track.
pub(super) fn text_patch(edit: &TextTrackEdit) -> PathTrack {
    PathTrack {
        easing: edit
            .easing
            .map_or_else(|| PathTrack::default().easing, EasingEdit::easing),
        keys: keys_of(&edit.keys),
    }
}

/// The layout patch's explode tracks: each asset's whole track, or `None`
/// to drop it.
pub(super) fn explode_patch(edits: &[ExplodeEdit]) -> BTreeMap<String, Option<ExplodeTrack>> {
    edits
        .iter()
        .map(|e| {
            let track = (!e.remove).then(|| {
                let base = ExplodeTrack::default();
                ExplodeTrack {
                    distance: e.distance.unwrap_or(base.distance),
                    easing: e.easing.map_or(base.easing, EasingEdit::easing),
                    keys: e
                        .keys
                        .iter()
                        .map(|k| ExplodeKey {
                            stop: k.stop.trim().to_string(),
                            amount: k.amount,
                        })
                        .collect(),
                }
            });
            (e.asset.trim().to_string(), track)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::layout::Easing;

    #[test]
    fn edits_fill_the_defaults_and_remove_drops_a_track() {
        let edits: Vec<ExplodeEdit> = serde_json::from_value(serde_json::json!([
            { "asset": " cabinet ", "keys": [{ "stop": "porch", "amount": 0 }, { "stop": " canopy ", "amount": 1 }] },
            { "asset": "lamp", "distance": 0.5, "easing": "linear" },
            { "asset": "crate", "remove": true }
        ]))
        .expect("edits");
        let patch = explode_patch(&edits);
        let cabinet = patch["cabinet"].as_ref().expect("cabinet");
        assert!((cabinet.distance - 0.25).abs() < f32::EPSILON);
        assert_eq!(cabinet.easing, Easing::EaseInOut);
        assert_eq!(cabinet.keys[1].stop, "canopy");
        let lamp = patch["lamp"].as_ref().expect("lamp");
        assert_eq!(lamp.easing, Easing::Linear);
        assert!(patch["crate"].is_none());
    }

    #[test]
    fn style_and_text_edits_fill_the_defaults() {
        let style: StyleTrackEdit = serde_json::from_value(serde_json::json!({
            "keys": [{ "stop": " porch ", "amount": 1 }]
        }))
        .expect("style");
        let track = style_patch(&style);
        assert_eq!(track.look, TrackLook::Blueprint);
        assert_eq!(track.keys[0].stop, "porch");
        let text: TextTrackEdit = serde_json::from_value(serde_json::json!({
            "easing": "ease-out", "keys": []
        }))
        .expect("text");
        assert_eq!(text_patch(&text).easing, Easing::EaseOut);
    }
}
