//! The object in a picture, as the ref checks and the Downloads matching
//! both read it: where it stands against its ground, its box, its
//! silhouette and a small colour histogram.
//!
//! The colour histogram bins coloured samples by hue and greys by
//! lightness, so the same object in softer light still matches. The
//! silhouette is the object's samples scaled into a square, its longer side
//! filling it and centred, so two pictures of one object framed apart still
//! line up.

use super::checks::{Grid, is_object, look};
use super::png::Pixels;

/// Hue bins for coloured samples, then dark, mid and light grey.
pub(crate) const HUES: usize = 12;
pub(crate) const BINS: usize = HUES + 3;
/// The channel spread under which a sample counts as grey.
const GREY_SPREAD: u8 = 40;
/// Under this share of the samples the object is too small to read.
const LEAST_OBJECT: f64 = 0.01;
/// The edge of a normalised silhouette, in cells.
pub(crate) const SILHOUETTE: usize = 32;

/// The object in one picture.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Object {
    /// The object's box: height over width.
    pub ratio: f64,
    /// The object's box height as a share of the frame's height.
    pub height: f64,
    /// The share of the object's samples in each colour bin.
    pub colours: [f64; BINS],
    /// Whether the object touches the frame, so its box is not whole.
    pub cut: bool,
    /// The object's samples scaled into a [`SILHOUETTE`] square, row major.
    pub silhouette: Vec<bool>,
}

/// The colour bin of one sample.
pub(crate) fn bin(p: [u8; 4]) -> usize {
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

/// The object's samples in a square: its box scaled so the longer side
/// fills it, centred on the shorter side.
fn silhouette(
    mask: &[bool],
    w: usize,
    (left, top, right, bottom): (usize, usize, usize, usize),
) -> Vec<bool> {
    let (bw, bh) = (right - left + 1, bottom - top + 1);
    let side = bw.max(bh);
    let (pad_x, pad_y) = ((side - bw) / 2, (side - bh) / 2);
    let mut out = vec![false; SILHOUETTE * SILHOUETTE];
    for (i, cell) in out.iter_mut().enumerate() {
        let (cx, cy) = (i % SILHOUETTE, i / SILHOUETTE);
        // The sample under the cell's centre, in the box's square.
        let sx = (cx * 2 + 1) * side / (SILHOUETTE * 2);
        let sy = (cy * 2 + 1) * side / (SILHOUETTE * 2);
        let (Some(x), Some(y)) = (sx.checked_sub(pad_x), sy.checked_sub(pad_y)) else {
            continue;
        };
        if x < bw && y < bh {
            *cell = mask[(top + y) * w + left + x];
        }
    }
    out
}

/// The object in a picture, `None` on a busy ground or with no object.
pub(crate) fn object(image: &Pixels) -> Option<Object> {
    let looks = look(image)?;
    if looks.busy {
        return None;
    }
    let grid = Grid::of(image);
    let ground = grid.corners()[0];
    let (mut left, mut right, mut top, mut bottom) = (grid.w, 0, grid.h, 0);
    let mut counts = [0u32; BINS];
    let mut total = 0u32;
    let mut mask = vec![false; grid.w * grid.h];
    for y in 0..grid.h {
        for x in 0..grid.w {
            let p = grid.at(x, y);
            if !is_object(ground, p) {
                continue;
            }
            mask[y * grid.w + x] = true;
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
    Some(Object {
        ratio: h / w,
        height,
        colours: counts.map(|c| f64::from(c) / f64::from(total)),
        cut: !looks.cut.is_empty(),
        silhouette: silhouette(&mask, grid.w, (left, top, right, bottom)),
    })
}

/// The share of colour two objects have in common, 0 to 1.
pub(crate) fn colour_overlap(a: &Object, b: &Object) -> f64 {
    a.colours
        .iter()
        .zip(&b.colours)
        .map(|(x, y)| x.min(*y))
        .sum()
}

/// How much two silhouettes cover the same cells: intersection over union,
/// 0 to 1.
pub(crate) fn shape_overlap(a: &Object, b: &Object) -> f64 {
    let (mut both, mut either) = (0u32, 0u32);
    for (x, y) in a.silhouette.iter().zip(&b.silhouette) {
        both += u32::from(*x && *y);
        either += u32::from(*x || *y);
    }
    if either == 0 {
        return 0.0;
    }
    f64::from(both) / f64::from(either)
}

/// How close two box ratios are, 0 to 1 (1 for the same ratio).
pub(crate) fn ratio_fit(a: &Object, b: &Object) -> f64 {
    a.ratio.min(b.ratio) / a.ratio.max(b.ratio).max(f64::EPSILON)
}
