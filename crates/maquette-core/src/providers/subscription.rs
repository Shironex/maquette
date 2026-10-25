//! What a By hand send costs on the provider's own site, paid from a web
//! subscription or a student plan instead of API credits. Those credits are
//! far cheaper than the API's, and the sites publish no per setting table,
//! so this is one small table of round numbers to update by hand when a
//! plan changes. The credits ledger keeps these sends apart from API spend.

use super::presets::{MeshyPreset, ProviderId, TripoPreset};

/// One row of the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WebCost {
    pub service: ProviderId,
    /// True for a textured model, false for the bare mesh.
    pub textured: bool,
    pub credits: f64,
    /// What the row is, in the site's words.
    pub what: &'static str,
}

/// Credits per generation on the sites, as of October 2026. Edit here.
pub const WEB_COSTS: [WebCost; 4] = [
    WebCost {
        service: ProviderId::Tripo,
        textured: true,
        credits: 30.0,
        what: "textured image to 3D",
    },
    WebCost {
        service: ProviderId::Tripo,
        textured: false,
        credits: 20.0,
        what: "image to 3D, no texture",
    },
    WebCost {
        service: ProviderId::Meshy,
        textured: false,
        credits: 20.0,
        what: "mesh only",
    },
    WebCost {
        service: ProviderId::Meshy,
        textured: true,
        credits: 30.0,
        what: "mesh and texture",
    },
];

/// The row for a send's settings: textured or not, per service.
pub fn web_cost(
    service: ProviderId,
    tripo: Option<&TripoPreset>,
    meshy: Option<&MeshyPreset>,
) -> Option<WebCost> {
    let textured = match service {
        ProviderId::Tripo => tripo.is_none_or(|t| t.texture),
        ProviderId::Meshy => meshy.is_some_and(|m| m.should_texture),
    };
    WEB_COSTS
        .iter()
        .find(|c| c.service == service && c.textured == textured)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_has_a_textured_and_a_bare_row() {
        for service in ProviderId::ALL {
            for textured in [true, false] {
                let rows = WEB_COSTS
                    .iter()
                    .filter(|c| c.service == service && c.textured == textured)
                    .count();
                assert_eq!(rows, 1, "{service:?} textured {textured}");
            }
        }
    }

    #[test]
    fn the_settings_pick_the_row() {
        let tripo = TripoPreset::prop();
        let cost = web_cost(ProviderId::Tripo, Some(&tripo), None).expect("tripo row");
        assert_eq!(cost.credits, 30.0);
        assert_eq!(cost.what, "textured image to 3D");

        let mut meshy = MeshyPreset::prop();
        meshy.should_texture = false;
        let cost = web_cost(ProviderId::Meshy, None, Some(&meshy)).expect("meshy row");
        assert_eq!(cost.credits, 20.0);
        assert_eq!(cost.what, "mesh only");
    }
}
