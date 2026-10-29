//! What an image costs, in US dollars. One price list for [`super::MODEL`]:
//! per million tokens, standard tier, Image API (cached input prices only
//! apply to the Responses API, so they are not used here).
//!
//! Prices checked 2026-10-28 at <https://developers.openai.com/api/docs/pricing>
//! (GPT Image 2.5: image input $8, image output $30, text input $5).
//! The per image estimates come from the image generation guide's table
//! for a 1024x1024 image, <https://developers.openai.com/api/docs/guides/image-generation>.

use super::{ImageQuality, Usage};

pub const PRICES_CHECKED: &str = "2026-10-28";
pub const PRICES_URL: &str = "https://developers.openai.com/api/docs/pricing";

const PER_MILLION: f64 = 1_000_000.0;
const TEXT_INPUT: f64 = 5.0;
const IMAGE_INPUT: f64 = 8.0;
const IMAGE_OUTPUT: f64 = 30.0;

/// Dollars for the tokens a request reported. Input without a breakdown
/// counts as image input, the dearer of the two.
pub fn cost_usd(usage: &Usage) -> f64 {
    let (text, image) = match usage.input_tokens_details {
        Some(d) if d.text_tokens + d.image_tokens > 0 => (d.text_tokens, d.image_tokens),
        _ => (0, usage.input_tokens),
    };
    #[expect(clippy::cast_precision_loss, reason = "token counts stay small")]
    let usd = (text as f64 * TEXT_INPUT
        + image as f64 * IMAGE_INPUT
        + usage.output_tokens as f64 * IMAGE_OUTPUT)
        / PER_MILLION;
    usd
}

/// The guess before a request (and for an answer without usage): the
/// guide's output cost per image at `quality`, plus about 1,000 input
/// tokens per attached image and the prompt's text.
pub fn estimate_usd(quality: ImageQuality, images: usize) -> f64 {
    let output = match quality {
        ImageQuality::Low => 0.006,
        ImageQuality::Medium => 0.053,
        ImageQuality::High => 0.211,
    };
    #[expect(clippy::cast_precision_loss, reason = "at most 16 images")]
    let input = images as f64 * 1_000.0 * IMAGE_INPUT / PER_MILLION;
    let text = 1_000.0 * TEXT_INPUT / PER_MILLION;
    output + input + text
}
