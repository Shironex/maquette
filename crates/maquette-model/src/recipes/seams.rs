//! Seams: where a recipe's motion can pop. Two clips meet across a blend
//! window (or a cut), and a freeze or a hold over eases into and out of its
//! held pose. The bake measures, frame by frame, how far the hips move and
//! how far each bone turns from the frame before; a seam pops when, inside
//! its window, the hips move more than [`SEAM_HIPS_CM`] or any bone turns
//! more than [`SEAM_TURN_DEGREES`] beyond the most the clips around it move
//! in one frame ([`AROUND_FRAMES`] frames either side). So a fast swing that
//! is fast on both sides of a seam is judged against itself, as foot slide
//! is judged against the base clip.
//!
//! A step at frame `i` is the move from frame `i - 1` to frame `i`, so a
//! window's frames are the steps it is judged on: a cut at frame 44 is the
//! one step into 44; a blend from 40 over 8 frames is the steps into 40 up
//! to the step into 48, the first frame after it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::timeline::{ClipSample, Timeline};
use super::{RecipeFile, TimeKind};

/// The hips may move this much more in one frame than the clips around, cm.
pub const SEAM_HIPS_CM: f32 = 2.0;
/// A bone may turn this much more in one frame than the clips around,
/// degrees.
pub const SEAM_TURN_DEGREES: f32 = 25.0;
/// How many frames either side of a window stand for the clips around it.
pub const AROUND_FRAMES: u32 = 6;

/// What kind of seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SeamKind {
    /// Two clips meet across a blend window or a cut.
    Join,
    /// A freeze or hold over eases into its held pose.
    HoldIn,
    /// A freeze or hold over eases out of its held pose.
    HoldOut,
}

/// One seam of a recipe on its timeline, and the steps it is judged on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeamWindow {
    pub kind: SeamKind,
    /// A join: the clip that joins (1 the first joined). A hold: the clip
    /// it sits on.
    pub clip: u32,
    /// A hold: the clip frame it holds, which names its time edit with
    /// `clip`.
    pub at: Option<u32>,
    /// A hold: a freeze (else a hold over).
    pub freeze: bool,
    /// The first and last step judged, timeline frames from 1.
    pub first: u32,
    pub last: u32,
}

/// What the bake measured, per timeline frame: where the hips are (metres)
/// and how far each humanoid bone turned from the frame before (degrees,
/// by VRM name; the first frame's is 0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SeamMotion {
    pub hips: Vec<[f32; 3]>,
    pub turns: BTreeMap<String, Vec<f32>>,
}

/// One seam judged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SeamCheck {
    pub kind: SeamKind,
    pub clip: u32,
    pub at: Option<u32>,
    pub freeze: bool,
    pub first: u32,
    pub last: u32,
    /// The frame of the worst step: the bigger of the two overs, against
    /// its limit.
    pub frame: u32,
    /// The hips' biggest step in the window and around it, cm, and how far
    /// the first goes past the second (0 when it does not).
    pub hips_cm: f32,
    pub hips_around_cm: f32,
    pub hips_over_cm: f32,
    /// The bone whose step goes furthest past its own around it, its step
    /// in the window and around, degrees, and how far past.
    pub bone: Option<String>,
    pub turn_degrees: f32,
    pub turn_around_degrees: f32,
    pub turn_over_degrees: f32,
    /// Past either limit.
    pub pops: bool,
    /// The clips that meet ("Slash into Thrust"), or the clip a hold sits
    /// on, by name.
    pub about: String,
    /// A join: its blend window as the recipe has it, frames, and the most
    /// it may take (half the shorter of the two clips, as trimmed).
    pub blend: Option<u32>,
    pub blend_most: Option<u32>,
    /// A hold: the frames it eases in (hold in) or out (hold out) over,
    /// and the most it may (its own frames).
    pub ease: Option<u32>,
    pub ease_most: Option<u32>,
    /// The bake reported the hips over the window; when not, nothing of
    /// it was measured and it never pops.
    #[serde(default = "measured_by_default")]
    pub measured: bool,
}

fn measured_by_default() -> bool {
    true
}

impl SeamCheck {
    /// How far past its limits it is, the worse of the two: 1 at a limit.
    pub fn ratio(&self) -> f32 {
        (self.hips_over_cm / SEAM_HIPS_CM).max(self.turn_over_degrees / SEAM_TURN_DEGREES)
    }

    /// Whether the hips (else a bone) go furthest past their limit.
    pub fn hips_worse(&self) -> bool {
        self.hips_over_cm / SEAM_HIPS_CM >= self.turn_over_degrees / SEAM_TURN_DEGREES
    }
}

fn holds_of(key: &super::timeline::TimelineKey) -> impl Iterator<Item = &ClipSample> {
    std::iter::once(&key.from)
        .chain(key.to.as_ref())
        .filter(|s| s.hold.is_some())
}

/// Every seam of `recipe` on its timeline `t`, in frame order: each join,
/// then each freeze or hold over easing in and out. A window that would
/// judge the first frame (there is no step into it) or run past the last is
/// cut to the steps there are; one with no step left is dropped.
pub fn windows(recipe: &RecipeFile, t: &Timeline) -> Vec<SeamWindow> {
    let last = t.frames;
    let mut out = Vec::new();
    let mut push = |w: SeamWindow| {
        let first = w.first.max(1);
        let end = w.last.min(last);
        if first <= end {
            out.push(SeamWindow {
                first,
                last: end,
                ..w
            });
        }
    };
    for seam in &t.seams {
        push(SeamWindow {
            kind: SeamKind::Join,
            clip: seam.clip,
            at: None,
            freeze: false,
            first: seam.frame,
            last: seam.frame + seam.frames,
        });
    }
    for edit in recipe.time.iter().filter(|e| e.kind != TimeKind::Retime) {
        let Some(at) = edit.at else { continue };
        let held: Vec<(u32, f64)> = t
            .keys
            .iter()
            .enumerate()
            .filter_map(|(i, key)| {
                let w = holds_of(key)
                    .filter(|s| s.clip == edit.clip)
                    .filter_map(|s| s.hold.as_ref())
                    .filter(|h| h.at == at)
                    .map(|h| h.weight)
                    .fold(0.0_f64, f64::max);
                (w > 0.0).then(|| (u32::try_from(i).unwrap_or(u32::MAX), w))
            })
            .collect();
        let (Some(&(h_first, _)), Some(&(h_last, _))) = (held.first(), held.last()) else {
            continue;
        };
        let whole: Vec<u32> = held
            .iter()
            .filter(|(_, w)| *w >= 1.0 - 1e-6)
            .map(|(i, _)| *i)
            .collect();
        let full_first = whole.first().copied().unwrap_or(h_first);
        let full_last = whole.last().copied().unwrap_or(h_last);
        let freeze = edit.kind == TimeKind::Freeze;
        push(SeamWindow {
            kind: SeamKind::HoldIn,
            clip: edit.clip,
            at: Some(at),
            freeze,
            first: h_first,
            last: full_first.max(h_first),
        });
        push(SeamWindow {
            kind: SeamKind::HoldOut,
            clip: edit.clip,
            at: Some(at),
            freeze,
            first: full_last + 1,
            last: h_last + 1,
        });
    }
    out.sort_by_key(|w| (w.first, w.last));
    out
}

fn step_cm(hips: &[[f32; 3]], i: usize) -> f32 {
    match (i.checked_sub(1).and_then(|p| hips.get(p)), hips.get(i)) {
        (Some(a), Some(b)) => {
            let d = (0..3).map(|k| (b[k] - a[k]).powi(2)).sum::<f32>();
            d.sqrt() * 100.0
        }
        _ => 0.0,
    }
}

/// The steps either side of `w`, within the timeline's `frames`, but none
/// inside another of the seams `all`: a near seam's own jump never raises
/// this one's bar.
fn around<'a>(
    w: &SeamWindow,
    all: &'a [SeamWindow],
    frames: u32,
) -> impl Iterator<Item = u32> + 'a {
    let before = w.first.saturating_sub(AROUND_FRAMES).max(1)..w.first;
    let after = (w.last + 1)..=(w.last + AROUND_FRAMES).min(frames);
    before
        .chain(after)
        .filter(move |i| !all.iter().any(|o| (o.first..=o.last).contains(i)))
}

fn worst(values: impl Iterator<Item = (u32, f32)>) -> (u32, f32) {
    values.fold(
        (0, 0.0),
        |best, (i, v)| if v > best.1 { (i, v) } else { best },
    )
}

fn round(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

/// Judges window `w` on `motion`, a timeline of `frames` frames, as the
/// only seam.
pub fn judge(w: &SeamWindow, motion: &SeamMotion, frames: u32) -> SeamCheck {
    judge_among(w, &[], motion, frames)
}

/// Judges window `w`, one of the seams `all`, on `motion`. Motion with no
/// hips up to the window's end is not measured and never pops.
pub fn judge_among(
    w: &SeamWindow,
    all: &[SeamWindow],
    motion: &SeamMotion,
    frames: u32,
) -> SeamCheck {
    let inside = || w.first..=w.last;
    let hips_at = |i: u32| step_cm(&motion.hips, i as usize);
    let (hips_frame, hips) = worst(inside().map(|i| (i, hips_at(i))));
    let (_, hips_around) = worst(around(w, all, frames).map(|i| (i, hips_at(i))));
    let hips_over = (hips - hips_around).max(0.0);
    let mut bone: Option<(String, u32, f32, f32, f32)> = None;
    for (name, steps) in &motion.turns {
        let at = |i: u32| steps.get(i as usize).copied().unwrap_or(0.0);
        let (frame, turn) = worst(inside().map(|i| (i, at(i))));
        let (_, near) = worst(around(w, all, frames).map(|i| (i, at(i))));
        let over = (turn - near).max(0.0);
        if bone.as_ref().is_none_or(|b| over > b.4) {
            bone = Some((name.clone(), frame, turn, near, over));
        }
    }
    let (name, turn_frame, turn, turn_around, turn_over) = bone
        .map_or((None, w.first, 0.0, 0.0, 0.0), |(n, f, t, a, o)| {
            (Some(n), f, t, a, o)
        });
    let mut check = SeamCheck {
        kind: w.kind,
        clip: w.clip,
        at: w.at,
        freeze: w.freeze,
        first: w.first,
        last: w.last,
        frame: w.first,
        hips_cm: round(hips),
        hips_around_cm: round(hips_around),
        hips_over_cm: round(hips_over),
        bone: name,
        turn_degrees: round(turn),
        turn_around_degrees: round(turn_around),
        turn_over_degrees: round(turn_over),
        pops: false,
        about: String::new(),
        blend: None,
        blend_most: None,
        ease: None,
        ease_most: None,
        measured: motion.hips.len() > w.last as usize,
    };
    check.pops = check.measured
        && (check.hips_over_cm > SEAM_HIPS_CM || check.turn_over_degrees > SEAM_TURN_DEGREES);
    check.frame = if check.hips_worse() && hips > 0.0 {
        hips_frame
    } else {
        turn_frame
    };
    check
}

/// What `check` is about in `recipe`: the clips that meet, the hold's clip,
/// and what a fix could change.
fn described(recipe: &RecipeFile, mut check: SeamCheck) -> SeamCheck {
    let name = |i: u32| recipe.clip(i).map_or_else(String::new, |c| c.name.clone());
    if check.kind == SeamKind::Join {
        check.about = format!("{} into {}", name(check.clip - 1), name(check.clip));
        let clip = recipe.clip(check.clip);
        check.blend = clip.map(|c| c.blend.as_ref().map_or(0, |b| b.frames));
        check.blend_most = clip
            .zip(recipe.clip(check.clip - 1))
            .map(|(a, b)| a.kept().min(b.kept()) / 2);
    } else {
        check.about = name(check.clip);
        let edit = recipe
            .time
            .iter()
            .find(|e| e.kind != TimeKind::Retime && e.clip == check.clip && e.at == check.at);
        check.ease = edit.map(|e| {
            if check.kind == SeamKind::HoldIn {
                e.ease_in.unwrap_or(0)
            } else {
                e.ease_out.unwrap_or(0)
            }
        });
        check.ease_most = edit.and_then(|e| e.frames);
    }
    check
}

/// Every seam of `recipe` judged on `motion`, the timeline `t` it was
/// baked on.
pub fn checks(recipe: &RecipeFile, t: &Timeline, motion: &SeamMotion) -> Vec<SeamCheck> {
    let all = windows(recipe, t);
    all.iter()
        .map(|w| described(recipe, judge_among(w, &all, motion, t.frames)))
        .collect()
}
