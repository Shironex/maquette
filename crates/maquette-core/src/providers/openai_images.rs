//! OpenAI's Image API, the Lab's other way to make an image: one request
//! per image, with the prompt Codex would read and the same attached images
//! in the same order. Without attached images it is a generation
//! (`POST /images/generations`); with them an edit (`POST /images/edits`),
//! which takes up to 16 images as base64 data URLs in a JSON body. GPT
//! Image models always answer with the image as base64 (`b64_json`) and,
//! for most models, the tokens it used, which [`price`] turns into dollars.
//!
//! Checked against <https://developers.openai.com/api/reference/resources/images/methods/edit>
//! and `.../methods/generate` on 2026-10-28.

mod price;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::http::{Api, ApiError, ApiResult, StopFlag, Timing};

pub use price::{PRICES_CHECKED, PRICES_URL, cost_usd, estimate_usd};

pub const SERVICE: &str = "OpenAI";
pub const BASE_URL: &str = "https://api.openai.com/v1";
/// The model every request names: GPT Image 2.5 for editing precision, the
/// API's own default for edits.
pub const MODEL: &str = "gpt-image-2.5-sunburst";
/// The most input images an edit takes.
pub const MAX_IMAGES: usize = 16;
/// The longest data URL an input image may become.
const MAX_DATA_URL: usize = 20_971_520;

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

/// One image to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageCall {
    /// The whole prompt, style note included.
    pub prompt: String,
    /// The attached images in order: the brief's refs, then the style board.
    pub images: Vec<PathBuf>,
    /// `1536x1024`, or `auto` to follow the input.
    pub size: String,
    pub quality: ImageQuality,
    /// A transparent PNG (a clean ref on a transparent ground).
    pub transparent: bool,
}

/// The tokens a request used, as the API reports them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub input_tokens_details: Option<InputDetails>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct InputDetails {
    #[serde(default)]
    pub text_tokens: u64,
    #[serde(default)]
    pub image_tokens: u64,
}

/// A made image and what it cost.
#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    /// PNG bytes.
    pub bytes: Vec<u8>,
    pub usage: Option<Usage>,
    pub usd: f64,
    /// True when the answer carried no usage and `usd` is [`estimate_usd`].
    pub estimated: bool,
}

/// The MIME type an input image goes as, from its extension.
fn mime(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// An input image as the data URL the edit takes.
fn data_url(path: &Path) -> ApiResult<String> {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let mime = mime(path)
        .ok_or_else(|| ApiError::Malformed(format!("{name} is not a PNG, JPEG or WebP image.")))?;
    let bytes = std::fs::read(path)
        .map_err(|e| ApiError::Malformed(format!("could not read {name}: {e}")))?;
    let url = format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
    if url.len() > MAX_DATA_URL {
        return Err(ApiError::Malformed(format!(
            "{name} is too large to attach (the API takes up to 20 MB per image)."
        )));
    }
    Ok(url)
}

/// The JSON body of the request: an edit when images are attached.
fn body(call: &ImageCall) -> ApiResult<Value> {
    let mut body = json!({
        "model": MODEL,
        "prompt": call.prompt,
        "n": 1,
        "size": call.size,
        "quality": call.quality.as_str(),
        "output_format": "png",
    });
    if call.transparent {
        body["background"] = json!("transparent");
    }
    if !call.images.is_empty() {
        if call.images.len() > MAX_IMAGES {
            return Err(ApiError::Malformed(format!(
                "{} images are attached; the API takes up to {MAX_IMAGES}.",
                call.images.len()
            )));
        }
        let images = call
            .images
            .iter()
            .map(|p| data_url(p).map(|url| json!({ "image_url": url })))
            .collect::<ApiResult<Vec<_>>>()?;
        body["images"] = Value::Array(images);
    }
    Ok(body)
}

/// Makes one image. A 429 waits as long as `Retry-After` asks (see
/// [`Api::send`]); [`ApiError::RateLimited`] means it kept asking.
pub fn make(api: &Api, stop: &StopFlag, call: &ImageCall) -> ApiResult<Made> {
    let body = body(call)?;
    let path = if call.images.is_empty() {
        "images/generations"
    } else {
        "images/edits"
    };
    let answer = api.json(stop, || Ok(api.post_json(path, &body)))?;
    read_answer(&answer, call)
}

/// The image and its cost from an answer.
fn read_answer(answer: &Value, call: &ImageCall) -> ApiResult<Made> {
    let b64 = answer
        .get("data")
        .and_then(Value::as_array)
        .and_then(|d| d.first())
        .and_then(|i| i.get("b64_json"))
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::Malformed("the answer held no image.".to_string()))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| ApiError::Malformed(format!("the image was not base64: {e}")))?;
    let usage = answer
        .get("usage")
        .and_then(|u| serde_json::from_value::<Usage>(u.clone()).ok())
        .filter(|u| u.input_tokens + u.output_tokens > 0);
    let (usd, estimated) = match usage {
        Some(usage) => (cost_usd(&usage), false),
        None => (estimate_usd(call.quality, call.images.len()), true),
    };
    Ok(Made {
        bytes,
        usage,
        usd,
        estimated,
    })
}

/// What Test this key found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiKeyTest {
    pub ok: bool,
    /// True when the account lists [`MODEL`].
    pub has_model: bool,
    pub message: String,
}

/// A sentence for an error, with a pointer to the right Settings section.
pub fn error_message(error: &ApiError) -> String {
    match error {
        ApiError::Auth => format!(
            "{SERVICE} did not accept the API key. Check it in Settings > Image generation."
        ),
        ApiError::Credits(_) => {
            format!("{SERVICE} says the account has no credit left for the API.")
        }
        error => error.message(SERVICE),
    }
}

/// Checks a key by listing the models, which costs nothing.
pub fn test_key(base: &str, key: &str, timing: Timing) -> OpenAiKeyTest {
    let stop = StopFlag::default();
    let listed = Api::new(SERVICE, base, key, timing)
        .and_then(|api| api.json(&stop, || Ok(api.get("models"))));
    match listed {
        Ok(answer) => {
            let ids: Vec<&str> = answer
                .get("data")
                .and_then(Value::as_array)
                .map(|d| {
                    d.iter()
                        .filter_map(|m| m.get("id").and_then(Value::as_str))
                        .collect()
                })
                .unwrap_or_default();
            let has_model = ids.contains(&MODEL);
            OpenAiKeyTest {
                ok: true,
                has_model,
                message: if has_model {
                    format!("The key works and can use {MODEL}.")
                } else {
                    format!(
                        "The key works, but its project does not list {MODEL}. Images may fail until the organization is verified for GPT Image models."
                    )
                },
            }
        }
        Err(error) => OpenAiKeyTest {
            ok: false,
            has_model: false,
            message: error_message(&error),
        },
    }
}
