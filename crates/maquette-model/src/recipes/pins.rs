//! Pins: a hand or foot that stays put across part of a recipe. The bake
//! solves them last, after the clips are joined, the base edited and every
//! other change laid over, so a planted foot stays planted whatever moved
//! above it; the rough preview holds them the same way.
//!
//! A pin stays where its bone is at its first frame, unless it names a
//! blank: then it stays on that point or prop. A foot pin keeps its toe and
//! heel flat unless told otherwise (`flat`); a hand pin keeps the hand
//! turned as it was at its first frame.
//!
//! Its frames are the timeline's ([`super::timeline`]): the first frame
//! whose share reaches its start and the last whose share is within its
//! end, so a freeze inside it stays pinned. It eases in over `ease_in`
//! frames from there and out over `ease_out` frames to its end
//! ([`PIN_EASE_FRAMES`] each unless told). The web app keeps a copy of
//! these rules (`state/recipe-pins.ts`); `fixtures/pins.json` holds cases
//! both must agree on.

use serde::Serialize;

use super::timeline::Timeline;
use super::{LayerKind, PinFlat, RecipeFile, RecipeLayer};

/// The frames a pin eases in and out over unless told.
pub const PIN_EASE_FRAMES: u32 = 3;
const EPS: f64 = 1e-6;

/// One pin on the timeline's frames, as the bake and the preview hold it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinSpan {
    /// The change's id.
    pub layer: String,
    /// The hand or foot, by VRM name.
    pub bone: String,
    /// The timeline frames it holds on, first and last.
    pub first: u32,
    pub last: u32,
    pub ease_in: u32,
    pub ease_out: u32,
    /// 0 to 1.
    pub weight: f64,
    /// A foot's: what stays flat. None for a hand, which keeps its turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flat: Option<PinFlat>,
    /// The blank it stays on; none stays where the bone is at `first`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// Whether `bone` is a foot.
pub fn is_foot(bone: &str) -> bool {
    bone == "leftFoot" || bone == "rightFoot"
}

/// What of a pinned foot stays flat: the pin's own, else toe and heel. None
/// for a hand.
pub fn flat_of(layer: &RecipeLayer) -> Option<PinFlat> {
    is_foot(&layer.bone).then(|| layer.settings.flat.unwrap_or(PinFlat::ToeAndHeel))
}

/// The timeline frames a change that starts and ends at shares `start` and
/// `end` holds on: the first frame whose share reaches its start and the
/// last whose share is within its end. None when no frame falls within.
/// Pins and grips both stand on these.
pub fn frames_within(t: &Timeline, start: f32, end: f32) -> Option<(u32, u32)> {
    let start = super::timeline::wide(start);
    let end = super::timeline::wide(end);
    let first = t.keys.iter().position(|k| k.share >= start - EPS)?;
    let last = t.keys.iter().rposition(|k| k.share <= end + EPS)?;
    if last < first {
        return None;
    }
    let frame = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    Some((frame(first), frame(last)))
}

/// Where `layer`, a pin, holds on timeline `t`; none when no frame falls
/// within its range.
pub fn span_on(t: &Timeline, layer: &RecipeLayer) -> Option<PinSpan> {
    let s = &layer.settings;
    let (first, last) = frames_within(t, s.start, s.end)?;
    Some(PinSpan {
        layer: layer.id.clone(),
        bone: layer.bone.clone(),
        first,
        last,
        ease_in: s.ease_in.unwrap_or(PIN_EASE_FRAMES),
        ease_out: s.ease_out.unwrap_or(PIN_EASE_FRAMES),
        weight: super::timeline::wide(s.weight).clamp(0.0, 1.0),
        flat: flat_of(layer),
        target: s.target.clone(),
    })
}

/// Refuses a pin that holds its bone with IK over shares another pin or a
/// reach on the same bone also holds: two IK targets on one bone at once
/// fight, and only the last would win. Checked on a write, so a file from
/// before still opens.
///
/// # Errors
///
/// [`crate::error::CoreError::Invalid`] naming both changes and the bone.
pub fn check_ik_overlap(recipe: &RecipeFile) -> crate::error::CoreResult<()> {
    let ik: Vec<&RecipeLayer> = recipe
        .layers
        .iter()
        .filter(|l| matches!(l.kind, LayerKind::Pin | LayerKind::Reach))
        .collect();
    let word = |l: &RecipeLayer| {
        if l.kind == LayerKind::Pin {
            "pin"
        } else {
            "reach"
        }
    };
    for (i, a) in ik.iter().enumerate() {
        for b in &ik[i + 1..] {
            let pinned = a.kind == LayerKind::Pin || b.kind == LayerKind::Pin;
            let (sa, sb) = (&a.settings, &b.settings);
            if pinned && a.bone == b.bone && sa.start < sb.end && sb.start < sa.end {
                return super::check::invalid(format!(
                    "The {} {} and the {} {} both hold {} with IK over the same part of the recipe. Move one so they do not overlap, or take one out.",
                    word(a),
                    a.id,
                    word(b),
                    b.id,
                    a.bone
                ));
            }
        }
    }
    Ok(())
}

/// Every pin of `recipe` on its timeline `t`, in the order they are laid.
pub fn spans(recipe: &RecipeFile, t: &Timeline) -> Vec<PinSpan> {
    recipe
        .layers
        .iter()
        .filter(|l| l.kind == LayerKind::Pin)
        .filter_map(|l| span_on(t, l))
        .collect()
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// How much of a change that holds on frames `first` to `last` shows at
/// timeline frame `frame`, 0 to 1: none outside its frames, easing in over
/// `ease_in` frames from its first and out over `ease_out` to its last (an
/// ease of 0 holds at once).
pub fn eased(first: u32, last: u32, ease_in: u32, ease_out: u32, frame: f64) -> f64 {
    let (first, last) = (f64::from(first), f64::from(last));
    if frame < first - EPS || frame > last + EPS {
        return 0.0;
    }
    let up = if ease_in > 0 {
        smooth((frame - first) / f64::from(ease_in))
    } else {
        1.0
    };
    let down = if ease_out > 0 {
        smooth((last - frame) / f64::from(ease_out))
    } else {
        1.0
    };
    up.min(down)
}

/// How much of `pin` holds at timeline frame `frame`, 0 to 1, weight in:
/// none outside its frames, easing in from its first frame and out to its
/// last (an ease of 0 holds at once).
pub fn amount(pin: &PinSpan, frame: f64) -> f64 {
    pin.weight * eased(pin.first, pin.last, pin.ease_in, pin.ease_out, frame)
}

/// The frames of `pin` where it holds whole: past its ease in and before
/// its ease out, which the drift check measures. None when its eases leave
/// no such frame (the bake reports its drift as not measured then).
pub fn held(pin: &PinSpan) -> Option<[u32; 2]> {
    let from = pin.first.checked_add(pin.ease_in)?;
    let to = pin.last.checked_sub(pin.ease_out)?;
    (from <= to).then_some([from, to])
}
