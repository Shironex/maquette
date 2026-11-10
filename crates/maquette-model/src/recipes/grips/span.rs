//! A grip change on the recipe's timeline, as the bake and the rough
//! preview lay it: the frames it holds on (the first whose share reaches
//! its start to the last within its end, as a pin's are) and how much of it
//! shows at each, easing in and out over its own frames
//! ([`GRIP_EASE_FRAMES`] each unless told). Its pose goes on in the order
//! the changes are laid, in place of the clip's turns on those bones
//! unless it says add.

use serde::Serialize;

use crate::recipes::pins::{eased, frames_within};
use crate::recipes::timeline::{Timeline, wide};
use crate::recipes::{LayerKind, LayerMode, RecipeFile, RecipeLayer};

/// The frames a grip eases in and out over unless told.
pub const GRIP_EASE_FRAMES: u32 = 4;

/// One grip change on the timeline's frames.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GripSpan {
    /// The change's id.
    pub layer: String,
    /// The hand that holds, by VRM name.
    pub bone: String,
    /// The timeline frames it holds on, first and last.
    pub first: u32,
    pub last: u32,
    pub ease_in: u32,
    pub ease_out: u32,
    /// 0 to 1.
    pub weight: f64,
    pub mode: LayerMode,
}

/// Where `layer`, a grip, holds on timeline `t`; none when no frame falls
/// within its range.
pub fn span_on(t: &Timeline, layer: &RecipeLayer) -> Option<GripSpan> {
    let s = &layer.settings;
    let (first, last) = frames_within(t, s.start, s.end)?;
    Some(GripSpan {
        layer: layer.id.clone(),
        bone: layer.bone.clone(),
        first,
        last,
        ease_in: s.ease_in.unwrap_or(GRIP_EASE_FRAMES),
        ease_out: s.ease_out.unwrap_or(GRIP_EASE_FRAMES),
        weight: wide(s.weight).clamp(0.0, 1.0),
        mode: layer.mode.unwrap_or(LayerMode::Replace),
    })
}

/// Every grip of `recipe` on its timeline `t`, in the order they are laid.
pub fn spans(recipe: &RecipeFile, t: &Timeline) -> Vec<GripSpan> {
    recipe
        .layers
        .iter()
        .filter(|l| l.kind == LayerKind::Grip)
        .filter_map(|l| span_on(t, l))
        .collect()
}

/// How much of `grip` shows at timeline frame `frame`, 0 to 1, weight in.
pub fn amount(grip: &GripSpan, frame: f64) -> f64 {
    grip.weight * eased(grip.first, grip.last, grip.ease_in, grip.ease_out, frame)
}
