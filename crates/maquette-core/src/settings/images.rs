//! The Lab's image settings, in `maquette.json` under `lab.images`: which
//! way each kind of Lab job makes its images, Codex or OpenAI's Image API,
//! and the Image API's quality and monthly cap.

use serde::{Deserialize, Serialize};

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

/// How much effort an image gets: more costs more and takes longer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ImageQuality {
    #[default]
    Low,
    Medium,
    High,
}

impl ImageQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
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
