//! The model lists for both providers, kept in one place. The Settings
//! selects and the send panel read them through [`catalog`], and the
//! request builders use the helpers below, so a new model version is a
//! change to this file only.

use serde::Serialize;

pub const TRIPO_DEFAULT_MODEL: &str = "v3.1-20260211";
pub const TRIPO_DEFAULT_RIG_MODEL: &str = "v3.0-20260909";
pub const MESHY_DEFAULT_MODEL: &str = "latest";

/// One choice in a model select.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    /// What the API takes.
    pub id: String,
    /// What the select shows.
    pub label: String,
    /// One line on when to pick it.
    pub note: String,
}

/// Everything the send panel and Settings offer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalog {
    pub tripo_models: Vec<ModelOption>,
    pub tripo_rig_models: Vec<ModelOption>,
    /// Biped preset animations Tripo can retarget onto a rig.
    pub tripo_animations: Vec<ModelOption>,
    pub meshy_models: Vec<ModelOption>,
}

const TRIPO_MODELS: [(&str, &str, &str); 5] = [
    ("v3.1-20260211", "H3.1", "Latest high detail model"),
    ("v3.0-20250812", "H3.0", "Previous stable version"),
    ("v2.5-20250123", "H2.5", "Legacy, for older projects"),
    (
        "P2-20260801",
        "P2 Preview",
        "Game ready, 48 to 50,000 triangles",
    ),
    ("P1-20260311", "P1", "Low poly, 50 to 20,000 faces"),
];

const TRIPO_RIG_MODELS: [(&str, &str, &str); 3] = [
    ("v3.0-20260909", "Rig 3.0", "Best for humanoids"),
    ("v2.5-20260210", "Rig 2.5", "Creatures and other shapes"),
    (
        "v1.0-20240301",
        "Rig 1.0",
        "Humanoids only, the most presets",
    ),
];

const TRIPO_ANIMATIONS: [(&str, &str); 11] = [
    ("preset:idle", "Idle"),
    ("preset:walk", "Walk"),
    ("preset:run", "Run"),
    ("preset:jump", "Jump"),
    ("preset:turn", "Turn"),
    ("preset:fall", "Fall"),
    ("preset:hurt", "Hurt"),
    ("preset:slash", "Slash"),
    ("preset:shoot", "Shoot"),
    ("preset:climb", "Climb"),
    ("preset:dive", "Dive"),
];

const MESHY_MODELS: [(&str, &str, &str); 5] = [
    ("latest", "Meshy 7.1", "Latest standard model"),
    ("meshy-6", "Meshy 6", "Removes baked lighting"),
    ("meshy-6-lite", "Meshy 6 Lite", "Faster and cheaper"),
    (
        "meshy-t2.5",
        "Meshy T2.5",
        "Smart Topology, sharp game assets",
    ),
    ("meshy-t2", "Meshy T2", "Smart Topology, separated parts"),
];

fn options(rows: &[(&str, &str, &str)]) -> Vec<ModelOption> {
    rows.iter()
        .map(|(id, label, note)| ModelOption {
            id: (*id).to_string(),
            label: (*label).to_string(),
            note: (*note).to_string(),
        })
        .collect()
}

pub fn catalog() -> ProviderCatalog {
    ProviderCatalog {
        tripo_models: options(&TRIPO_MODELS),
        tripo_rig_models: options(&TRIPO_RIG_MODELS),
        tripo_animations: TRIPO_ANIMATIONS
            .iter()
            .map(|(id, label)| ModelOption {
                id: (*id).to_string(),
                label: (*label).to_string(),
                note: String::new(),
            })
            .collect(),
        meshy_models: options(&MESHY_MODELS),
    }
}

fn label_in<'a>(rows: &'a [(&'a str, &'a str, &'a str)], id: &'a str) -> &'a str {
    rows.iter()
        .find(|(row, _, _)| *row == id)
        .map_or(id, |(_, label, _)| label)
}

pub fn tripo_label(model: &str) -> &str {
    label_in(&TRIPO_MODELS, model)
}

pub fn meshy_label(model: &str) -> &str {
    label_in(&MESHY_MODELS, model)
}

/// The P series takes fewer parameters: no quad, smart low poly, parts or
/// geometry quality.
pub fn is_tripo_p_series(model: &str) -> bool {
    model.starts_with('P') || model.starts_with('p')
}

/// The face limits Tripo accepts for a model, inclusive.
pub fn tripo_face_range(model: &str, smart_low_poly: bool, ultra: bool) -> (u32, u32) {
    if model.starts_with("P1") {
        return (50, 20_000);
    }
    if model.starts_with("P2") {
        return (48, 50_000);
    }
    if smart_low_poly {
        return (500, 20_000);
    }
    let high = match (model.starts_with("v3.1"), model.starts_with("v3.0"), ultra) {
        (_, _, true) if !model.starts_with("v2") => 2_000_000,
        (true, _, _) => 1_500_000,
        (_, true, _) => 1_000_000,
        _ => 500_000,
    };
    (1, high)
}

/// Smart Topology models need `model_type: smart-topology`.
pub fn is_smart_topology(model: &str) -> bool {
    model.starts_with("meshy-t")
}

/// `remove_lighting` is only allowed with standard models other than lite.
pub fn meshy_takes_remove_lighting(model: &str) -> bool {
    !is_smart_topology(model) && model != "meshy-6-lite"
}

/// The face counts Meshy aims for, inclusive: a remesh, or the Smart
/// Topology face count.
pub fn meshy_polycount_range(model: &str) -> (u32, u32) {
    match model {
        "meshy-t2" => (100, 15_000),
        "meshy-t2.5" => (100, 30_000),
        _ => (100, 300_000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_every_model_once_with_the_defaults_first() {
        let c = catalog();
        assert_eq!(c.tripo_models[0].id, TRIPO_DEFAULT_MODEL);
        assert_eq!(c.tripo_rig_models[0].id, TRIPO_DEFAULT_RIG_MODEL);
        assert_eq!(c.meshy_models[0].id, MESHY_DEFAULT_MODEL);
        assert!(c.meshy_models.iter().all(|m| !m.id.contains("lowpoly")));
        assert_eq!(tripo_label("P2-20260801"), "P2 Preview");
        assert_eq!(tripo_label("v9-unknown"), "v9-unknown");
        assert!(is_tripo_p_series("P1-20260311"));
        assert!(!is_tripo_p_series("v3.1-20260211"));
        assert_eq!(
            tripo_face_range("v3.1-20260211", false, true),
            (1, 2_000_000)
        );
        assert_eq!(tripo_face_range("v2.5-20250123", false, true), (1, 500_000));
        assert_eq!(
            tripo_face_range("v3.0-20250812", true, false),
            (500, 20_000)
        );
        assert!(is_smart_topology("meshy-t2.5"));
        assert!(!meshy_takes_remove_lighting("meshy-6-lite"));
        assert!(meshy_takes_remove_lighting("meshy-6"));
    }
}
