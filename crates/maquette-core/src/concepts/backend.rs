//! Which way a Lab image is made: Codex (the default, on the Codex
//! subscription) or OpenAI's Image API (paid per image from an API key),
//! chosen per kind of job in Settings and per request by an agent. The
//! job itself is the same either way: the same prompt, attached images,
//! file, queue and retries. The settings themselves are
//! [`crate::settings::images`].

use std::path::PathBuf;

use crate::providers::openai_images::{ImageQuality, estimate_usd};
pub use crate::settings::images::{
    ImageBackend, ImageBackends, ImageJobKind, ImageSettings, MAX_MONTHLY_CAP_USD,
};

/// How a batch runs on the Image API: what the shell resolved from the
/// settings, a request's override and the workspace. `None` in its place
/// means Codex.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiPlan {
    pub kind: ImageJobKind,
    pub quality: ImageQuality,
    pub monthly_cap_usd: u32,
    /// The workspace root, where `credits.jsonl` lives.
    pub ledger_root: PathBuf,
    /// Started by an agent over MCP.
    pub via_agent: bool,
    /// The jobs wait in [`ConceptJobState::Approval`](super::ConceptJobState)
    /// until a person approves the round in the app (an agent's round).
    pub hold: bool,
    /// What one image at this quality has cost lately, from the ledger
    /// ([`crate::providers::ledger::recent_average_usd`]). `None` uses the
    /// price list's guess.
    pub per_image_usd: Option<f64>,
}

impl ApiPlan {
    /// What `images` images with `attached` inputs each likely cost: the
    /// recent average per image when the ledger has one, else the price
    /// list's guess for the quality and inputs.
    pub fn estimate_usd(&self, images: usize, attached: usize) -> f64 {
        #[expect(clippy::cast_precision_loss, reason = "a batch is small")]
        let count = images as f64;
        self.per_image_usd
            .unwrap_or_else(|| estimate_usd(self.quality, attached))
            * count
    }
}
