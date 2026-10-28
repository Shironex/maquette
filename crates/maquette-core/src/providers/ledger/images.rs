//! Image API spend in the credits ledger: one `image` line per image the
//! Lab made with OpenAI's Image API, in US dollars from the tokens the API
//! reported (or the price list's estimate when it reported none). Their
//! lines are version 3, which an older Maquette skips, so its credit
//! totals never take dollars in.

use chrono::{Datelike, Local, TimeZone};
use serde::{Deserialize, Serialize};

use crate::providers::openai_images::ImageQuality;

pub const IMAGE_LINE_VERSION: u32 = 3;
const IMAGE_EVENT: &str = "image";

/// One image the Image API made, as written and as read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageSpend {
    pub v: u32,
    /// Always `image`.
    pub event: String,
    pub at_ms: f64,
    pub project: String,
    /// The concept job id.
    pub job: String,
    /// The image, relative to the project folder.
    pub file: String,
    /// `openai`.
    pub service: String,
    pub model: String,
    pub quality: ImageQuality,
    pub usd: f64,
    /// True when `usd` is the price list's estimate.
    pub estimated: bool,
    pub input_tokens: Option<f64>,
    pub output_tokens: Option<f64>,
    pub via_agent: bool,
}

impl ImageSpend {
    pub fn new(at_ms: f64, model: &str, quality: ImageQuality, usd: f64) -> Self {
        Self {
            v: IMAGE_LINE_VERSION,
            event: IMAGE_EVENT.to_string(),
            at_ms,
            project: String::new(),
            job: String::new(),
            file: String::new(),
            service: "openai".to_string(),
            model: model.to_string(),
            quality,
            usd,
            estimated: false,
            input_tokens: None,
            output_tokens: None,
            via_agent: false,
        }
    }
}

/// An image line, or `None` for anything else.
pub(super) fn parse_line(raw: &str) -> Option<ImageSpend> {
    serde_json::from_str::<ImageSpend>(raw)
        .ok()
        .filter(|s| s.v == IMAGE_LINE_VERSION && s.event == IMAGE_EVENT && s.usd.is_finite())
}

/// Midnight on the first of the local month `now_ms` falls in, in ms.
pub fn month_start_ms(now_ms: f64) -> f64 {
    #[expect(clippy::cast_possible_truncation, reason = "ms since the epoch fit")]
    let now = Local
        .timestamp_millis_opt(now_ms as i64)
        .single()
        .unwrap_or_else(Local::now);
    Local
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .earliest()
        .map_or(0.0, |start| start.timestamp_millis() as f64)
}

/// Dollars spent on images since the start of the local month.
pub fn month_to_date(images: &[ImageSpend], now_ms: f64) -> f64 {
    let start = month_start_ms(now_ms);
    images
        .iter()
        .filter(|s| s.at_ms >= start && s.at_ms <= now_ms + 60_000.0)
        .map(|s| s.usd.max(0.0))
        .sum()
}
