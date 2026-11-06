//! The base edits of version 2: the blend window between joined clips and
//! the time edits on a clip's own frames (retime ranges, freezes and holds
//! over), plus which file version a recipe needs and what of version 2 a
//! bake cannot make yet.

use serde::{Deserialize, Serialize};

use super::{BLEND_FRAMES, RecipeFile};

/// How a blend window eases from one clip into the next.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum BlendCurve {
    #[default]
    Smooth,
    Linear,
}

/// What a blend lines up between the end of one clip and the start of the
/// next, so the seam does not jump.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum LineUp {
    /// Where the hips are and which way they face.
    #[default]
    HipsAndFacing,
    /// Where the hips are only.
    Hips,
    /// Nothing: the next clip starts where it was made.
    Off,
}

/// The blend from the clip before into a joined clip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BlendWindow {
    /// How many frames the two clips overlap; 0 cuts straight. At most
    /// half the shorter of the two clips, as trimmed.
    pub frames: u32,
    #[serde(default)]
    #[specta(optional)]
    pub curve: BlendCurve,
    #[serde(default)]
    #[specta(optional)]
    pub line_up: LineUp,
}

impl Default for BlendWindow {
    /// 8 frames, smooth, hips and facing lined up.
    fn default() -> Self {
        Self {
            frames: BLEND_FRAMES,
            curve: BlendCurve::Smooth,
            line_up: LineUp::HipsAndFacing,
        }
    }
}

/// What a time edit does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum TimeKind {
    /// Plays a range of frames faster or slower.
    Retime,
    /// Stops on one frame and adds its length to the recipe.
    Freeze,
    /// Keeps one pose across a range, the length unchanged.
    HoldOver,
}

/// A time edit on one clip's own frames, before its trim, so trimming or
/// joining carries it with the motion it belongs to. Each kind reads its
/// own fields; the rest are none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TimeEdit {
    pub kind: TimeKind,
    /// The clip it sits on: 0 the base, then the joined clips.
    pub clip: u32,
    /// Retime: the first frame of the range.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub from: Option<u32>,
    /// Retime: the frame the range ends at, after `from`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub to: Option<u32>,
    /// Retime: how fast the range plays, 0.25 to 4 (1.6 plays 14 frames in 9).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub speed: Option<f32>,
    /// Retime: frames it eases into and out of the new speed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub ease: Option<u32>,
    /// Freeze and hold over: the frame whose pose is held.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub at: Option<u32>,
    /// Freeze: how many frames it adds. Hold over: how many frames from
    /// `at` keep the pose.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub frames: Option<u32>,
    /// Freeze and hold over: a little breathing so a long hold is not dead
    /// still, 0 to 1. None is 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub breathe: Option<f32>,
    /// Freeze and hold over: frames it eases into the held pose over.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub ease_in: Option<u32>,
    /// Freeze and hold over: frames it eases out of it over.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub ease_out: Option<u32>,
}

impl TimeEdit {
    fn empty(kind: TimeKind, clip: u32) -> Self {
        Self {
            kind,
            clip,
            from: None,
            to: None,
            speed: None,
            ease: None,
            at: None,
            frames: None,
            breathe: None,
            ease_in: None,
            ease_out: None,
        }
    }

    /// Plays frames `from` to `to` of clip `clip` at `speed`.
    pub fn retime(clip: u32, from: u32, to: u32, speed: f32) -> Self {
        Self {
            from: Some(from),
            to: Some(to),
            speed: Some(speed),
            ..Self::empty(TimeKind::Retime, clip)
        }
    }

    /// Holds frame `at` of clip `clip` for `frames` frames, a freeze or a
    /// hold over.
    pub fn hold(kind: TimeKind, clip: u32, at: u32, frames: u32) -> Self {
        Self {
            at: Some(at),
            frames: Some(frames),
            ..Self::empty(kind, clip)
        }
    }

    /// The frames of its clip it covers, `[first, end)`: a retime's range,
    /// a hold over's held frames, a freeze's one frame.
    pub fn span(&self) -> Option<(u32, u32)> {
        match self.kind {
            TimeKind::Retime => Some((self.from?, self.to?)),
            TimeKind::HoldOver => {
                let at = self.at?;
                Some((at, at.checked_add(self.frames?)?))
            }
            TimeKind::Freeze => {
                let at = self.at?;
                Some((at, at.checked_add(1)?))
            }
        }
    }
}

/// `edit` with only the fields its kind reads.
pub(super) fn time_for(edit: &TimeEdit) -> TimeEdit {
    let retime = edit.kind == TimeKind::Retime;
    let hold = !retime;
    TimeEdit {
        kind: edit.kind,
        clip: edit.clip,
        from: edit.from.filter(|_| retime),
        to: edit.to.filter(|_| retime),
        speed: edit.speed.filter(|_| retime),
        ease: edit.ease.filter(|_| retime),
        at: edit.at.filter(|_| hold),
        frames: edit.frames.filter(|_| hold),
        breathe: edit.breathe.filter(|_| hold),
        ease_in: edit.ease_in.filter(|_| hold),
        ease_out: edit.ease_out.filter(|_| hold),
    }
}

/// Whether `recipe` uses anything of version 2, so it has to be written as
/// version 2. One that does not is written as version 1, which a Maquette
/// from before still opens.
pub fn uses_v2(recipe: &RecipeFile) -> bool {
    !bake_unsupported(recipe).is_empty()
        || recipe.base.blend.is_some()
        || recipe.layers.iter().any(|l| {
            l.mode.is_some()
                || !l.keys.is_empty()
                || l.pose.is_some()
                || l.from.is_some()
                || l.settings.ease_in.is_some()
                || l.settings.ease_out.is_some()
                || l.settings.flat.is_some()
        })
}

/// What of version 2 `recipe` uses that a bake cannot make yet, in words
/// ("joined clips", "pose keys"), none for a recipe a bake takes.
pub fn bake_unsupported(recipe: &RecipeFile) -> Vec<&'static str> {
    use super::LayerKind;
    let mut out = Vec::new();
    if !recipe.joined.is_empty() {
        out.push("joined clips");
    }
    if recipe.clips().any(|c| c.trim.is_some()) {
        out.push("trims");
    }
    if recipe.mirror {
        out.push("mirror");
    }
    if !recipe.time.is_empty() {
        out.push("time edits");
    }
    for (kind, word) in [
        (LayerKind::Pin, "pins"),
        (LayerKind::Pose, "pose keys"),
        (LayerKind::Grip, "grips"),
    ] {
        if recipe.layers.iter().any(|l| l.kind == kind) {
            out.push(word);
        }
    }
    out
}
