//! Baked highlights in an MToon export. A generated model's base color
//! has its lighting painted in: on dark leather the studio light's shine
//! is a bright, nearly white patch of the texture. PBR lights the model
//! again and the patch passes as gloss; MToon shades flat, so it reads as
//! a white blotch. With [`Highlights::Soften`] (the default) a headless
//! Blender step ([`SCRIPT`]) writes a copy of each base color image with
//! only strong highlights on dark material toned down toward the colour
//! around them, and the written VRM shades from that copy. The model's own
//! texture is never changed: it stays in the version's file, and in the
//! VRM unused by the materials. An image with almost no highlights is left
//! as it is.
//!
//! The run is cached in the asset's avatar folder under `highlights`,
//! keyed by the images, the script and the strength. Softening never stops
//! a build: when it fails, the VRM keeps the texture as it is and the
//! report says why.

use std::fs;
use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::avatar::Highlights;
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::inbox::vrm::{self, BaseColorImage, BaseColorSwap, MaterialMode, Written};
use crate::json::write_json;
use crate::pipeline::cache::{self, CacheRecord};
use crate::pipeline::tools::Toolbox;

/// The script, run headless on the images written out of the VRM.
pub const SCRIPT: &str = include_str!("avatar_highlights.py");
const SCRIPT_FILE: &str = "avatar_highlights.py";
/// The folder inside the asset's avatar folder.
pub const HIGHLIGHTS_DIR: &str = "highlights";
const JOB_FILE: &str = "job.json";
const REPORT_FILE: &str = "report.json";
const STEP: &str = "highlights";
const KEY_VERSION: &str = "maquette avatar highlights v1";
/// How far a highlight moves toward the colour around it.
pub const STRENGTH: f64 = 0.75;
/// Added to the name of a softened copy's image.
const SUFFIX: &str = " highlights softened";

/// What softening did to one base color image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageHighlights {
    /// The image's index in the file.
    pub image: u32,
    pub width: u32,
    pub height: u32,
    /// The share of the texture's pixels that are baked highlights, 0 to 1.
    pub coverage: f64,
    /// The same share within the dark areas of the texture.
    pub dark_coverage: f64,
    /// A softened copy is what the materials shade from.
    pub softened: bool,
    /// Why it was left as it is.
    pub note: Option<String>,
}

/// What an export did with the highlights baked into its base color.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HighlightsReport {
    pub mode: Highlights,
    /// How far a highlight moved toward the colour around it, 0 to 1.
    pub strength: f64,
    pub images: Vec<ImageHighlights>,
    /// Why nothing was softened, when nothing was.
    pub note: Option<String>,
}

impl HighlightsReport {
    fn kept(mode: Highlights, note: Option<String>) -> Self {
        Self {
            mode,
            strength: STRENGTH,
            images: Vec::new(),
            note,
        }
    }

    /// True when the VRM shades from a softened copy.
    pub fn softened(&self) -> bool {
        self.images.iter().any(|i| i.softened)
    }
}

/// What the script reads.
#[derive(Debug, Serialize)]
struct Job {
    images: Vec<JobImage>,
    strength: f64,
    report: &'static str,
}

#[derive(Debug, Serialize)]
struct JobImage {
    input: String,
    output: String,
}

/// What the script writes, per image.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Found {
    output: Option<String>,
    width: u32,
    height: u32,
    coverage: f64,
    dark_coverage: f64,
    skipped: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FoundFile {
    images: Vec<Found>,
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

fn extension(mime: &str) -> &'static str {
    if mime == "image/png" { "png" } else { "jpg" }
}

fn names(images: &[BaseColorImage]) -> Vec<(String, String)> {
    images
        .iter()
        .map(|i| {
            let ext = extension(&i.mime);
            (
                format!("image_{}.{ext}", i.image),
                format!("soft_{}.{ext}", i.image),
            )
        })
        .collect()
}

/// The run's key: the images, the script and the strength.
fn run_key(images: &[BaseColorImage]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(KEY_VERSION.as_bytes());
    hasher.update(SCRIPT.replace("\r\n", "\n").as_bytes());
    hasher.update(STRENGTH.to_string().as_bytes());
    for image in images {
        hasher.update(image.image.to_le_bytes());
        hasher.update(image.mime.as_bytes());
        hasher.update(Sha256::digest(&image.bytes));
    }
    hex::encode(hasher.finalize())
}

/// Runs the script on `images` in `dir`, or reads the last run on the same
/// images, and returns what it found per image.
fn run(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    dir: &Path,
    images: &[BaseColorImage],
) -> CoreResult<Vec<Found>> {
    let key = run_key(images);
    let outputs = vec![REPORT_FILE.to_string()];
    let fresh =
        cache::load_record(dir, STEP).is_some_and(|r| cache::is_fresh(&r, &key, dir, &outputs));
    if !fresh {
        if dir.is_dir() {
            fs::remove_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
        }
        fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
        let mut job = Job {
            images: Vec::new(),
            strength: STRENGTH,
            report: REPORT_FILE,
        };
        for (image, (input, output)) in images.iter().zip(names(images)) {
            let path = dir.join(&input);
            fs::write(&path, &image.bytes).map_err(|e| CoreError::io(&path, e))?;
            job.images.push(JobImage { input, output });
        }
        write_json(&dir.join(JOB_FILE), &job)?;
        let started = Instant::now();
        let extra = ["--job".to_string(), JOB_FILE.to_string()];
        runner.run_script(tools, dir, (SCRIPT_FILE, SCRIPT), &extra)?;
        if !dir.join(REPORT_FILE).is_file() {
            return Err(invalid(format!(
                "Blender finished without writing {REPORT_FILE}."
            )));
        }
        cache::save_record(
            dir,
            STEP,
            &CacheRecord {
                key,
                outputs: cache::stamp_outputs(dir, &outputs),
                duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            },
        )?;
    }
    let file = dir.join(REPORT_FILE);
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    let found: FoundFile = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if found.images.len() != images.len() {
        return Err(invalid("The highlights report does not list every image."));
    }
    Ok(found.images)
}

/// Softens the base color images of `bytes` in `dir`: the new bytes, or
/// `None` when no image changed, and what was found per image.
fn soften_bytes(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    dir: &Path,
    bytes: &[u8],
) -> CoreResult<(Option<Vec<u8>>, Vec<ImageHighlights>)> {
    let images = vrm::base_color_images(bytes)?;
    if images.is_empty() {
        return Ok((None, Vec::new()));
    }
    let found = run(runner, tools, dir, &images)?;
    let mut swaps = Vec::new();
    let mut report = Vec::new();
    for ((image, found), (_, output)) in images.iter().zip(found).zip(names(&images)) {
        let softened = found.output.is_some();
        if softened {
            let path = dir.join(&output);
            swaps.push(BaseColorSwap {
                image: image.image,
                mime: image.mime.clone(),
                bytes: fs::read(&path).map_err(|e| CoreError::io(&path, e))?,
                suffix: SUFFIX.to_string(),
            });
        }
        report.push(ImageHighlights {
            image: u32::try_from(image.image).unwrap_or(u32::MAX),
            width: found.width,
            height: found.height,
            coverage: found.coverage,
            dark_coverage: found.dark_coverage,
            softened,
            note: found.skipped,
        });
    }
    if swaps.is_empty() {
        return Ok((None, report));
    }
    Ok((Some(vrm::swap_base_color(bytes, &swaps)?), report))
}

/// Applies `mode` to written MToon bytes, with the run cached in
/// `<asset avatar folder>/highlights`. PBR bytes come back as they are
/// with no report. A failed run keeps the texture and says why.
pub(crate) fn apply(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    asset_dir: &Path,
    material: MaterialMode,
    mode: Highlights,
    written: Written,
) -> (Written, Option<HighlightsReport>) {
    if material != MaterialMode::MtoonSoft {
        return (written, None);
    }
    if mode == Highlights::Keep {
        return (written, Some(HighlightsReport::kept(mode, None)));
    }
    let dir = asset_dir.join(HIGHLIGHTS_DIR);
    match soften_bytes(runner, tools, &dir, &written.bytes) {
        Ok((bytes, images)) => {
            let note = if images.is_empty() {
                Some("The model has no base color texture to soften.".to_string())
            } else if bytes.is_none() {
                Some("No strong baked highlights were found.".to_string())
            } else {
                None
            };
            let report = HighlightsReport {
                images,
                note,
                ..HighlightsReport::kept(mode, None)
            };
            let written = match bytes {
                Some(bytes) => Written { bytes, ..written },
                None => written,
            };
            (written, Some(report))
        }
        Err(e) => {
            let note = format!("The texture was kept as it is: {e}");
            (written, Some(HighlightsReport::kept(mode, Some(note))))
        }
    }
}

#[cfg(test)]
mod blender_tests;
#[cfg(test)]
pub(crate) mod tests;
