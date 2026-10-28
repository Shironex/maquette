//! Views that disagree with the front: each side and the back is compared
//! with the front on what must hold for one object, whatever the angle.
//!
//! - **Colours:** a small histogram of the object's samples (the ground
//!   left out), by hue for coloured samples and by lightness for greys,
//!   so a view in softer light still matches. The two must overlap by at
//!   least [`COLOURS_OVERLAP`].
//! - **Proportions:** the object's height in the frame agrees within
//!   [`HEIGHT_APART`]. The back shows the same outline as the front, so
//!   its height to width ratio agrees within [`BACK_RATIO_APART`]; a side
//!   shows the depth, so it may be narrower or wider, within
//!   [`SIDE_RATIO_APART`].
//!
//! A view on a busy ground is skipped (the object cannot be told from it),
//! and proportions are skipped when either object is cut off by the frame.
//! The warning only says the view looks different; it never says the
//! view shows another object, since light and a redraw change looks too.

use std::fs;
use std::path::Path;

use super::png::{self, Pixels};
use super::{Grid, RefFix, RefView, RefWarning, WarningKind, is_object, look};

/// Hue bins for coloured samples, then dark, mid and light grey.
const HUES: usize = 12;
const BINS: usize = HUES + 3;
/// The channel spread under which a sample counts as grey.
const GREY_SPREAD: u8 = 40;
/// The least share of colour two views of one object have in common.
pub(super) const COLOURS_OVERLAP: f64 = 0.45;
/// How many times taller or shorter in the frame a view may stand.
pub(super) const HEIGHT_APART: f64 = 1.45;
/// How far the back's height to width ratio may stray from the front's.
pub(super) const BACK_RATIO_APART: f64 = 1.4;
/// How far a side's ratio may stray, since it shows the depth.
pub(super) const SIDE_RATIO_APART: f64 = 4.0;
/// Under this share of the samples the object is too small to compare.
const LEAST_OBJECT: f64 = 0.01;

/// What one view's object looks like, as far as the comparison goes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Shape {
    /// The object's box: height over width.
    pub ratio: f64,
    /// The object's box height as a share of the frame's height.
    pub height: f64,
    /// The share of the object's samples in each colour bin.
    pub colours: [f64; BINS],
    /// Whether the object touches the frame, so its box is not whole.
    pub cut: bool,
}

/// The colour bin of one sample.
fn bin(p: [u8; 4]) -> usize {
    let (r, g, b) = (f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
    let max = p[0].max(p[1]).max(p[2]);
    let min = p[0].min(p[1]).min(p[2]);
    if max - min < GREY_SPREAD {
        return HUES + usize::from(max >= 85) + usize::from(max >= 170);
    }
    let spread = f64::from(max - min);
    let hue = if max == p[0] {
        ((g - b) / spread).rem_euclid(6.0)
    } else if max == p[1] {
        (b - r) / spread + 2.0
    } else {
        (r - g) / spread + 4.0
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the hue is in 0 to 6"
    )]
    let at = (hue / 6.0 * HUES as f64) as usize;
    at.min(HUES - 1)
}

/// The object in an image, `None` on a busy ground or with no object.
pub(crate) fn shape(image: &Pixels) -> Option<Shape> {
    let looks = look(image)?;
    if looks.busy {
        return None;
    }
    let grid = Grid::of(image);
    let ground = grid.corners()[0];
    let (mut left, mut right, mut top, mut bottom) = (grid.w, 0, grid.h, 0);
    let mut counts = [0u32; BINS];
    let mut total = 0u32;
    for y in 0..grid.h {
        for x in 0..grid.w {
            let p = grid.at(x, y);
            if !is_object(ground, p) {
                continue;
            }
            (left, right) = (left.min(x), right.max(x));
            (top, bottom) = (top.min(y), bottom.max(y));
            counts[bin(p)] += 1;
            total += 1;
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "sample counts stay small")]
    let share = f64::from(total) / (grid.w * grid.h) as f64;
    if share < LEAST_OBJECT {
        return None;
    }
    #[expect(clippy::cast_precision_loss, reason = "sample counts stay small")]
    let (w, h) = ((right - left + 1) as f64, (bottom - top + 1) as f64);
    #[expect(clippy::cast_precision_loss, reason = "sample counts stay small")]
    let height = h / grid.h as f64;
    Some(Shape {
        ratio: h / w,
        height,
        colours: counts.map(|c| f64::from(c) / f64::from(total)),
        cut: !looks.cut.is_empty(),
    })
}

/// What does not match between the front and a view, if anything.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Mismatch {
    pub colours: bool,
    pub proportions: bool,
}

/// How many times apart two positive numbers are, at least 1.
fn apart(a: f64, b: f64) -> f64 {
    a.max(b) / a.min(b).max(f64::EPSILON)
}

pub(crate) fn compare(front: &Shape, other: &Shape, view: RefView) -> Mismatch {
    let overlap: f64 = front
        .colours
        .iter()
        .zip(&other.colours)
        .map(|(a, b)| a.min(*b))
        .sum();
    let whole = !front.cut && !other.cut;
    let ratio_apart = if view == RefView::Back {
        BACK_RATIO_APART
    } else {
        SIDE_RATIO_APART
    };
    let proportions = whole
        && (apart(front.height, other.height) > HEIGHT_APART
            || apart(front.ratio, other.ratio) > ratio_apart);
    Mismatch {
        colours: overlap < COLOURS_OVERLAP,
        proportions,
    }
}

fn shape_of(project_dir: &Path, file: &str) -> Option<Shape> {
    let bytes = fs::read(project_dir.join(file)).ok()?;
    shape(&png::decode(&bytes)?)
}

/// A warning for each side or back view in `sent` (with its `views`, as
/// [`crate::refs::send_order`] gives them) that does not match the front.
pub(super) fn view_warnings(
    project_dir: &Path,
    sent: &[String],
    views: &[String],
) -> Vec<RefWarning> {
    let view_of = |name: &str| RefView::SLOTS.into_iter().find(|v| v.name() == name);
    let Some(front) = sent
        .iter()
        .zip(views)
        .find(|(_, v)| view_of(v) == Some(RefView::Front))
        .and_then(|(file, _)| shape_of(project_dir, file))
    else {
        return Vec::new();
    };
    sent.iter()
        .zip(views)
        .filter_map(|(file, name)| {
            let view = view_of(name).filter(|v| *v != RefView::Front)?;
            let found = compare(&front, &shape_of(project_dir, file)?, view);
            let what = match (found.colours, found.proportions) {
                (true, true) => "colours and proportions do",
                (true, false) => "colours do",
                (false, true) => "proportions do",
                (false, false) => return None,
            };
            Some(RefWarning {
                kind: WarningKind::ViewsDisagree,
                file: Some(file.clone()),
                view: Some(view),
                title: format!("{} looks different from the front: its {what} not match", view.label()),
                detail: "Views that do not match can mix into the model. It may only be the light or the drawing; check it against the front, or draw it again from the front.".to_string(),
                fixes: vec![RefFix::MakeViews, RefFix::LeaveOut],
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
