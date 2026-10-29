//! Which way a Lab image is made: Codex (the default, on the Codex
//! subscription) or OpenAI's Image API (paid per image from an API key),
//! chosen per kind of job in Settings and per request by an agent. The
//! job itself is the same either way: the same prompt, attached images,
//! file, queue and retries.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::providers::openai_images::{ImageQuality, estimate_usd};

/// How a Lab image is made.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ImageBackend {
    #[default]
    Codex,
    OpenaiImages,
}

impl ImageBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::OpenaiImages => "OpenAI Images",
        }
    }
}

/// The kinds of Lab job, each with its own backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ImageJobKind {
    Concepts,
    ObjectRefs,
    OtherViews,
    CleanBackground,
}

/// The backend per kind of job.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ImageBackends {
    pub concepts: ImageBackend,
    pub object_refs: ImageBackend,
    pub other_views: ImageBackend,
    pub clean_background: ImageBackend,
}

impl ImageBackends {
    pub fn get(&self, kind: ImageJobKind) -> ImageBackend {
        match kind {
            ImageJobKind::Concepts => self.concepts,
            ImageJobKind::ObjectRefs => self.object_refs,
            ImageJobKind::OtherViews => self.other_views,
            ImageJobKind::CleanBackground => self.clean_background,
        }
    }
}

/// The Lab's image settings, in `maquette.json` under `lab.images`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ImageSettings {
    pub backends: ImageBackends,
    /// The Image API's quality.
    pub quality: ImageQuality,
    /// Whole US dollars the Image API may spend per calendar month. 0
    /// stops it.
    pub monthly_cap_usd: u32,
}

/// The highest cap Settings takes.
pub const MAX_MONTHLY_CAP_USD: u32 = 10_000;

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            backends: ImageBackends::default(),
            quality: ImageQuality::Low,
            monthly_cap_usd: 10,
        }
    }
}

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
