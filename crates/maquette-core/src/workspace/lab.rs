//! The Concept lab's settings in `maquette.json`: how many images run at
//! once, when a job times out, and which backend makes each kind of image.

use serde::{Deserialize, Serialize};

use crate::concepts::backend::{ImageSettings, MAX_MONTHLY_CAP_USD};
use crate::error::{CoreError, CoreResult};

/// The Concept lab's Codex jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LabSettings {
    /// How many images generate at once. Each Codex job takes about a
    /// minute; six or seven in parallel work fine.
    pub concurrency: u32,
    /// A job still running after this many minutes is stopped.
    pub timeout_minutes: u32,
    /// Codex or OpenAI's Image API per kind of job, and the API's quality
    /// and monthly cap.
    #[serde(default)]
    pub images: ImageSettings,
}

impl Default for LabSettings {
    fn default() -> Self {
        Self {
            concurrency: 4,
            timeout_minutes: 10,
            images: ImageSettings::default(),
        }
    }
}

pub const LAB_MAX_CONCURRENCY: u32 = 8;

impl LabSettings {
    /// Refuses settings the Lab cannot run with.
    pub fn validate(&self) -> CoreResult<()> {
        let invalid = |msg: String| Err(CoreError::Invalid(msg));
        if !(1..=LAB_MAX_CONCURRENCY).contains(&self.concurrency) {
            return invalid(format!(
                "Run between 1 and {LAB_MAX_CONCURRENCY} concept jobs at a time."
            ));
        }
        if !(1..=60).contains(&self.timeout_minutes) {
            return invalid("The concept job timeout must be between 1 and 60 minutes.".into());
        }
        if self.images.monthly_cap_usd > MAX_MONTHLY_CAP_USD {
            return invalid(format!(
                "The monthly OpenAI image cap must be at most ${MAX_MONTHLY_CAP_USD}."
            ));
        }
        Ok(())
    }
}
