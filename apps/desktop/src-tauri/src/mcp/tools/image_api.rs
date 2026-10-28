//! The Lab's image backends for agents: which one each kind of job uses
//! (Codex or OpenAI's Image API), the API's quality and monthly cap, and
//! the month's spend. The OpenAI key itself is only ever set in the app.

use maquette_core::concepts::backend::{ImageBackend, ImageSettings, MAX_MONTHLY_CAP_USD};
use maquette_core::providers::openai_images::ImageQuality;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::image_api::{self, ImageApiStatus};
use crate::commands::lab::Backend;

use super::{McpTools, Out, respond, to_value};

/// Where an image is made.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum BackendArg {
    /// Codex CLI, on the Codex subscription's usage.
    Codex,
    /// OpenAI's Image API, paid per image from the OpenAI key in Settings, within the monthly cap.
    OpenaiImages,
}

impl From<BackendArg> for ImageBackend {
    fn from(value: BackendArg) -> Self {
        match value {
            BackendArg::Codex => ImageBackend::Codex,
            BackendArg::OpenaiImages => ImageBackend::OpenaiImages,
        }
    }
}

/// An agent's batch: its override, marked as an agent's spend.
pub(super) fn backend(arg: Option<BackendArg>) -> Backend {
    Backend {
        choice: arg.map(ImageBackend::from),
        via_agent: true,
    }
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum QualityArg {
    Low,
    Medium,
    High,
}

impl From<QualityArg> for ImageQuality {
    fn from(value: QualityArg) -> Self {
        match value {
            QualityArg::Low => ImageQuality::Low,
            QualityArg::Medium => ImageQuality::Medium,
            QualityArg::High => ImageQuality::High,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetImageBackendArgs {
    /// The backend for start_concepts.
    pub concepts: Option<BackendArg>,
    /// The backend for make_object_refs.
    pub object_refs: Option<BackendArg>,
    /// The backend for make_other_views.
    pub other_views: Option<BackendArg>,
    /// The backend for clean_ref_background.
    pub clean_background: Option<BackendArg>,
    /// The Image API's quality: low (about $0.01 an image), medium or high (about $0.22).
    pub quality: Option<QualityArg>,
    /// Whole US dollars the Image API may spend per calendar month; 0 stops it.
    pub monthly_cap_usd: Option<u32>,
}

fn status_value(status: &ImageApiStatus) -> Value {
    let s = &status.settings;
    json!({
        "backends": {
            "concepts": to_value(s.backends.concepts),
            "objectRefs": to_value(s.backends.object_refs),
            "otherViews": to_value(s.backends.other_views),
            "cleanBackground": to_value(s.backends.clean_background),
        },
        "quality": to_value(s.quality),
        "monthlyCapUsd": s.monthly_cap_usd,
        "monthSpentUsd": status.month_spent_usd,
        "pendingUsd": status.pending_usd,
        "keySaved": status.key.saved,
        "model": status.model,
        "estimateUsdPerImage": {
            "low": status.estimates.low,
            "medium": status.estimates.medium,
            "high": status.estimates.high,
        },
        "pricesChecked": status.prices_checked,
    })
}

#[tool_router(router = image_api_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Read how the Lab makes images: the backend per kind of job (codex, or openai-images for OpenAI's Image API), the Image API's quality and monthly cap in US dollars, what it spent this calendar month and what its queued jobs will likely cost, whether an OpenAI key is saved, and the estimated cost per image. Codex is the default for every kind. start_concepts, make_object_refs, make_other_views and clean_ref_background take a backend to override it for one request.",
        annotations(read_only_hint = true)
    )]
    async fn get_image_backend(&self) -> Out {
        let found = self
            .blocking(move |host, _| {
                image_api::status(
                    host.session(),
                    host.studio().secrets().as_ref(),
                    host.concepts(),
                )
                .map(|s| status_value(&s))
                .map_err(|e| e.message)
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Change how the Lab makes images, as Settings > Image generation does: the backend per kind of job (codex or openai-images), the Image API's quality (low, medium or high) and its monthly cap in whole US dollars (0 stops it). Fields left out keep their value. openai-images is paid per image from the OpenAI key saved in the app; a batch that would pass the cap is refused before anything is made, and every image is written to the credits ledger. The key itself can only be set in the app. Returns the result like get_image_backend.",
        annotations(destructive_hint = false)
    )]
    async fn set_image_backend(&self, Parameters(args): Parameters<SetImageBackendArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (_, config) = host.session().workspace().map_err(|e| e.to_string())?;
                let mut settings: ImageSettings = config.lab.images;
                let b = &mut settings.backends;
                if let Some(v) = args.concepts {
                    b.concepts = v.into();
                }
                if let Some(v) = args.object_refs {
                    b.object_refs = v.into();
                }
                if let Some(v) = args.other_views {
                    b.other_views = v.into();
                }
                if let Some(v) = args.clean_background {
                    b.clean_background = v.into();
                }
                if let Some(q) = args.quality {
                    settings.quality = q.into();
                }
                if let Some(cap) = args.monthly_cap_usd {
                    if cap > MAX_MONTHLY_CAP_USD {
                        return Err(format!(
                            "The monthly cap must be at most ${MAX_MONTHLY_CAP_USD}."
                        ));
                    }
                    settings.monthly_cap_usd = cap;
                }
                image_api::save_settings(host.session(), settings).map_err(|e| e.message)?;
                host.image_settings_changed(&settings);
                image_api::status(
                    host.session(),
                    host.studio().secrets().as_ref(),
                    host.concepts(),
                )
                .map(|s| status_value(&s))
                .map_err(|e| e.message)
            })
            .await;
        respond(found)
    }
}
