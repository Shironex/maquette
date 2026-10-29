//! Which waiting By hand send a download looks like. The download is
//! rendered headless from the front on a plain ground; each waiting send's
//! front ref (its first image when none is tagged front) is read the same
//! way, and the two objects are compared on:
//!
//! - **Outline:** both silhouettes scaled into one square, longer side
//!   filling it, and how much they cover the same cells (intersection over
//!   union). [`SHAPE_WEIGHT`] of the score.
//! - **Colours:** the share of colour the two have in common, by hue for
//!   coloured samples and by lightness for greys. [`COLOUR_WEIGHT`].
//! - **Proportions:** how close the boxes' height to width ratios are.
//!   [`RATIO_WEIGHT`].
//!
//! The name only breaks ties ([`NAME_WEIGHT`]). Every waiting send is
//! ranked, however late the download came; the best is only suggested,
//! with its score, and never attached by itself.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::handback::name_fit;
use crate::jobs::ProviderJob;
use maquette_model::refs::picture::{self, Object};
use maquette_model::refs::png;

pub(super) const SHAPE_WEIGHT: f64 = 0.45;
pub(super) const COLOUR_WEIGHT: f64 = 0.35;
pub(super) const RATIO_WEIGHT: f64 = 0.2;
/// The share of the ranking the name has once there is a picture.
pub(super) const NAME_WEIGHT: f64 = 0.1;
/// The least score that suggests a send by its picture.
pub(super) const LOOKS_LIKE: f64 = 0.6;
/// How far the best must lead the next to be suggested.
pub(super) const LEAD: f64 = 0.05;

/// One waiting send, ranked against a download.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SendSuggestion {
    pub job: String,
    pub project: String,
    pub asset: String,
    pub asset_name: String,
    /// The ranking, 0 to 1: mostly the picture, the name breaking ties.
    pub score: f64,
    /// How alike the pictures are, 0 to 1; none before the render, or when
    /// either picture could not be read.
    pub picture: Option<f64>,
    /// How close the file name is to the send's, 0 to 1.
    pub name: f64,
    /// The ref compared with, relative to the project folder.
    pub image: Option<String>,
}

/// How alike two pictures' objects are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PictureScore {
    pub shape: f64,
    pub colour: f64,
    pub ratio: f64,
}

impl PictureScore {
    pub(crate) fn total(self) -> f64 {
        SHAPE_WEIGHT * self.shape + COLOUR_WEIGHT * self.colour + RATIO_WEIGHT * self.ratio
    }
}

pub(crate) fn score(download: &Object, reference: &Object) -> PictureScore {
    PictureScore {
        shape: picture::shape_overlap(download, reference),
        colour: picture::colour_overlap(download, reference),
        ratio: picture::ratio_fit(download, reference),
    }
}

/// The object in a PNG file, `None` when it cannot be read.
pub(crate) fn object_in(file: &Path) -> Option<Object> {
    let bytes = fs::read(file).ok()?;
    picture::object(&png::decode(&bytes)?)
}

/// A send's front ref: the image tagged front, else its first.
pub(super) fn front_ref(job: &ProviderJob) -> Option<&str> {
    job.views
        .iter()
        .position(|v| v == "front")
        .and_then(|i| job.images.get(i))
        .or_else(|| job.images.first())
        .map(String::as_str)
}

/// Every waiting send ranked against a download named `name` (and `inner`
/// for a zip), best first. With `render` (the download's front picture),
/// each send's front ref is read from its project folder (`project_dir`)
/// and the picture leads; without it the name alone ranks them.
pub(super) fn rank(
    name: &str,
    inner: Option<&str>,
    render: Option<&Object>,
    waiting: &[ProviderJob],
    project_dir: &dyn Fn(&str) -> Option<PathBuf>,
) -> Vec<SendSuggestion> {
    let mut out: Vec<SendSuggestion> = waiting
        .iter()
        .map(|job| {
            let fit = name_fit(name, inner, job);
            let image = front_ref(job).map(str::to_string);
            let picture = render.and_then(|seen| {
                let file = project_dir(&job.project)?.join(image.as_deref()?);
                Some(score(seen, &object_in(&file)?).total())
            });
            let score = picture.map_or(fit, |p| (1.0 - NAME_WEIGHT) * p + NAME_WEIGHT * fit);
            SendSuggestion {
                job: job.id.clone(),
                project: job.project.clone(),
                asset: job.asset.clone(),
                asset_name: job.asset_name.clone(),
                score,
                picture,
                name: fit,
                image,
            }
        })
        .collect();
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    out
}

/// The send a picture ranking clearly points at: scored by its picture,
/// at least [`LOOKS_LIKE`], and [`LEAD`] ahead of the next.
pub(super) fn looks_like(ranked: &[SendSuggestion]) -> Option<&SendSuggestion> {
    let (best, rest) = ranked.split_first()?;
    best.picture?;
    let clear = rest
        .first()
        .is_none_or(|next| best.score - next.score >= LEAD);
    (best.score >= LOOKS_LIKE && clear).then_some(best)
}

/// A score as a whole percent, for reasons.
pub(super) fn percent(score: f64) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a score is in 0 to 1"
    )]
    let pct = (score.clamp(0.0, 1.0) * 100.0).round() as u32;
    pct
}

#[cfg(test)]
mod tests;
