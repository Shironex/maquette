//! Weak ref warnings: cheap local checks a Send to 3D runs on its images
//! before any credits go, so a weak ref can be fixed first. They never
//! stop a send.
//!
//! - **Small:** the shorter edge is under [`MIN_EDGE`] pixels (any PNG,
//!   JPEG or WebP, from its header).
//! - **Busy background:** the four corners disagree in colour, or the
//!   band along the frame is full of edges. PNG only.
//! - **Cropped:** on a plain background, the object reaches the edge of
//!   the frame. PNG only.
//! - **One view:** one image goes while the preset has multi-view on.
//! - **Views disagree:** a side or the back does not match the front in
//!   its main colours or its proportions (see [`agree`]). PNG only.
//!
//! The pixel checks read a grid of at most [`GRID`] samples a side, so a
//! 4k image costs the same as a small one.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::png::{self, Pixels};
use super::{RefView, send_order};
use crate::assets::Asset;

/// Under this many pixels on the shorter edge, a model reads poorly.
pub const MIN_EDGE: u32 = 512;
/// The most samples the pixel checks take along one side.
const GRID: usize = 160;
/// How far apart two colours are (largest channel difference) before they
/// count as different: a soft shadow stays background, an object does not.
const DIFFERENT: u8 = 48;
/// Corners further apart than this are not one background.
const CORNERS_APART: u8 = 60;
/// The mean step between neighbouring samples along the frame that reads
/// as clutter rather than a plain or softly lit ground.
const BUSY_STEP: f64 = 14.0;
/// The share of an edge an object may touch before it reads as cut off.
const CROP_SHARE: f64 = 0.03;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum WarningKind {
    Small,
    Cropped,
    BusyBackground,
    OneView,
    ViewsDisagree,
}

/// A one click fix the panel offers next to a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RefFix {
    CleanBackground,
    MakeViews,
    LeaveOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RefWarning {
    pub kind: WarningKind,
    /// The image it is about, `None` for the send as a whole.
    pub file: Option<String>,
    /// The view that image goes as, when it goes as one.
    pub view: Option<RefView>,
    pub title: String,
    pub detail: String,
    pub fixes: Vec<RefFix>,
}

fn diff(a: [u8; 4], b: [u8; 4]) -> u8 {
    (0..4).map(|i| a[i].abs_diff(b[i])).max().unwrap_or(0)
}

/// The image sampled on a grid of at most `GRID` a side.
struct Grid {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
}

impl Grid {
    fn of(image: &Pixels) -> Self {
        let step = image.width.max(image.height).div_ceil(GRID).max(1);
        let (w, h) = (image.width.div_ceil(step), image.height.div_ceil(step));
        let mut px = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                px.push(image.at(
                    (x * step).min(image.width - 1),
                    (y * step).min(image.height - 1),
                ));
            }
        }
        Self { w, h, px }
    }

    fn at(&self, x: usize, y: usize) -> [u8; 4] {
        self.px[y * self.w + x]
    }

    /// The four corners, each the mean of a small square: the ground.
    fn corners(&self) -> [[u8; 4]; 4] {
        let side = (self.w.min(self.h) / 25).max(1);
        [
            self.corner(false, false, side),
            self.corner(true, false, side),
            self.corner(false, true, side),
            self.corner(true, true, side),
        ]
    }

    /// The mean colour of a square of `side` samples at a corner.
    fn corner(&self, right: bool, bottom: bool, side: usize) -> [u8; 4] {
        let mut sum = [0u64; 4];
        for dy in 0..side {
            for dx in 0..side {
                let x = if right { self.w - 1 - dx } else { dx };
                let y = if bottom { self.h - 1 - dy } else { dy };
                for (s, c) in sum.iter_mut().zip(self.at(x, y)) {
                    *s += u64::from(c);
                }
            }
        }
        let n = (side * side) as u64;
        sum.map(|s| u8::try_from(s / n).unwrap_or(u8::MAX))
    }

    /// Samples along the frame, `depth` deep, in order around it.
    fn band(&self, depth: usize) -> Vec<Vec<[u8; 4]>> {
        let mut lines = Vec::new();
        for d in 0..depth {
            lines.push((0..self.w).map(|x| self.at(x, d)).collect());
            lines.push((0..self.w).map(|x| self.at(x, self.h - 1 - d)).collect());
            lines.push((0..self.h).map(|y| self.at(d, y)).collect());
            lines.push((0..self.h).map(|y| self.at(self.w - 1 - d, y)).collect());
        }
        lines
    }
}

/// What the pixels say about one image.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Looks {
    pub busy: bool,
    /// The edges the object touches: top, bottom, left, right.
    pub cut: Vec<&'static str>,
}

pub(crate) fn look(image: &Pixels) -> Option<Looks> {
    let grid = Grid::of(image);
    if grid.w < 8 || grid.h < 8 {
        return None;
    }
    let corners = grid.corners();
    let apart = corners
        .iter()
        .flat_map(|a| corners.iter().map(move |b| diff(*a, *b)))
        .max()
        .unwrap_or(0);
    let depth = (grid.w.min(grid.h) / 30).max(1);
    let (mut steps, mut count) = (0u64, 0u64);
    for line in grid.band(depth) {
        for pair in line.windows(2) {
            steps += u64::from(diff(pair[0], pair[1]));
            count += 1;
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "sample counts stay small")]
    let mean_step = steps as f64 / count.max(1) as f64;
    if apart > CORNERS_APART || mean_step > BUSY_STEP {
        return Some(Looks {
            busy: true,
            cut: Vec::new(),
        });
    }
    let object = |p: [u8; 4]| is_object(corners[0], p);
    let edges: [(&'static str, Vec<[u8; 4]>); 4] = [
        ("top", (0..grid.w).map(|x| grid.at(x, 0)).collect()),
        (
            "bottom",
            (0..grid.w).map(|x| grid.at(x, grid.h - 1)).collect(),
        ),
        ("left", (0..grid.h).map(|y| grid.at(0, y)).collect()),
        (
            "right",
            (0..grid.h).map(|y| grid.at(grid.w - 1, y)).collect(),
        ),
    ];
    let cut = edges
        .into_iter()
        .filter(|(_, line)| {
            let hits = line.iter().filter(|p| object(**p)).count();
            #[expect(clippy::cast_precision_loss, reason = "sample counts stay small")]
            let share = hits as f64 / line.len() as f64;
            share > CROP_SHARE
        })
        .map(|(side, _)| side)
        .collect();
    Some(Looks { busy: false, cut })
}

/// Whether a sample is the object rather than the `ground` behind it: any
/// opaque sample on a transparent ground, else one clearly off its colour.
fn is_object(ground: [u8; 4], p: [u8; 4]) -> bool {
    if ground[3] < 128 {
        p[3] >= 128
    } else {
        diff(p, ground) > DIFFERENT
    }
}

fn name_of(file: &str) -> &str {
    file.rsplit('/').next().unwrap_or(file)
}

fn joined(sides: &[&str]) -> String {
    match sides {
        [one] => format!("the {one} edge"),
        [rest @ .., last] => format!("the {} and {last} edges", rest.join(", ")),
        [] => String::new(),
    }
}

/// The warnings for one image of a send. `view` is the view it goes as.
pub fn image_warnings(project_dir: &Path, file: &str, view: Option<RefView>) -> Vec<RefWarning> {
    let Ok(bytes) = fs::read(project_dir.join(file)) else {
        return Vec::new();
    };
    let name = name_of(file);
    let who = view.map_or_else(|| name.to_string(), |v| v.label().to_string());
    let warn = |kind, title: String, detail: &str, fixes: &[RefFix]| RefWarning {
        kind,
        file: Some(file.to_string()),
        view,
        title,
        detail: detail.to_string(),
        fixes: fixes.to_vec(),
    };
    let mut out = Vec::new();
    if let Some((w, h)) = crate::inbox::images::dimensions(&bytes)
        && w.min(h) < MIN_EDGE
    {
        out.push(warn(
            WarningKind::Small,
            format!("{who} is small: {w} × {h} px"),
            &format!(
                "Below {MIN_EDGE} px a model reads poorly. Draw it again at 1024 or leave it out."
            ),
            &[RefFix::LeaveOut],
        ));
    }
    let Some(looks) = png::decode(&bytes).as_ref().and_then(look) else {
        return out;
    };
    if looks.busy {
        out.push(warn(
            WarningKind::BusyBackground,
            format!("Busy background in {name}"),
            "Clutter around the object can end up in the model. Clean background keeps the object and clears the rest.",
            &[RefFix::CleanBackground, RefFix::LeaveOut],
        ));
    } else if !looks.cut.is_empty() {
        out.push(warn(
            WarningKind::Cropped,
            format!("{who} crops the object"),
            &format!(
                "The object touches {}, so the model can end flat there.",
                joined(&looks.cut)
            ),
            &[RefFix::CleanBackground, RefFix::LeaveOut],
        ));
    }
    out
}

/// Every warning for a send of `images` (as picked) for `asset`, in send
/// order, with the send wide ones last. `multi_view` is the preset's
/// multi-view switch.
pub fn send_warnings(
    project_dir: &Path,
    asset: &Asset,
    images: &[String],
    multi_view: bool,
) -> Vec<RefWarning> {
    let (sent, views) = send_order(asset, images);
    let mut out: Vec<RefWarning> = sent
        .iter()
        .zip(&views)
        .flat_map(|(file, view)| {
            let view = RefView::SLOTS.into_iter().find(|v| v.name() == view);
            image_warnings(project_dir, file, view.filter(|_| sent.len() > 1))
        })
        .collect();
    if sent.len() == 1 && multi_view {
        out.push(RefWarning {
            kind: WarningKind::OneView,
            file: None,
            view: None,
            title: "One view only: the sides and back are guessed".to_string(),
            detail:
                "Multi-view is on, but only one image goes. Make the other views to send all four."
                    .to_string(),
            fixes: vec![RefFix::MakeViews],
        });
    }
    out.extend(agree::view_warnings(project_dir, &sent, &views));
    out
}

mod agree;
#[cfg(test)]
mod tests;
