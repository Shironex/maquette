//! A recipe's timeline: which frame of which clip each frame of its bake
//! shows, so the bake and the rough preview play the same motion for the
//! same number of frames. The web app keeps a copy of these rules
//! (`state/recipe-timeline.ts`); `fixtures/timeline.json` holds cases both
//! must agree on.
//!
//! Each clip is cut to its trim, then its time edits play out on its own
//! frames: a retime range steps faster or slower (easing into and out of its
//! speed over `ease` frames), a freeze stops on its frame and adds its
//! frames, a hold over keeps its frame's pose across its range with the
//! length unchanged. The clips are then joined in order, each overlapping the
//! one before across its blend window. A frame of the timeline counts at the
//! base clip's rate; a joined clip of another rate is stepped by time.
//!
//! Round 1's changes keep their start and end as shares, read on the clips'
//! own frames laid end to end (`share`): on one clip with no edits that is the
//! share of its length, as before, and a trim, retime or freeze carries a
//! change with the motion it sat on. Joining, taking out or moving a clip
//! changes what the shares are read on, so `clip_moves` moves them too.

use serde::Serialize;

use super::{LineUp, MAX_TIMELINE_FRAMES, REST_FPS, RecipeBase, RecipeFile, TimeEdit, TimeKind};
use crate::recipes::BlendCurve;

/// How long one breath of a held pose lasts, seconds.
pub const BREATHE_SECONDS: f64 = 3.0;
const EPS: f64 = 1e-6;
/// The most frames one clip makes, so a file that skipped the checks cannot
/// run away: past what a whole recipe may run ([`MAX_TIMELINE_FRAMES`]).
const MAX_KEYS: usize = MAX_TIMELINE_FRAMES as usize + 2;

/// A frame of one clip, as one frame of the timeline shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipSample {
    /// The clip: 0 the base, then the joined ones.
    pub clip: u32,
    /// The clip's own frame, before its trim; between two keys when a
    /// retime steps by a part of a frame.
    pub at: f64,
    /// A freeze or hold over showing here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold: Option<HoldSample>,
}

/// A held pose: its frame, how much of it shows over the motion and how
/// much it breathes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldSample {
    pub at: u32,
    /// 0 to 1.
    pub weight: f64,
    /// The hold's breathe, 0 to 1, before its weight.
    pub breathe: f64,
}

/// One frame of the timeline.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineKey {
    pub from: ClipSample,
    /// In a blend window, the next clip's frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<ClipSample>,
    /// How much of `to` shows, 0 to 1.
    pub blend: f64,
    /// Where round 1's changes stand: the share of the clips' own frames
    /// laid end to end.
    pub share: f64,
}

/// Where a joined clip starts, and what its blend lines up.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Seam {
    pub clip: u32,
    /// The timeline frame its blend window starts on.
    pub frame: u32,
    /// The window's length in timeline frames (0 cuts).
    pub frames: u32,
    pub line_up: LineUp,
    /// The clip before's frame and the clip's own first frame at `frame`,
    /// which the line up compares (at a cut the timeline shows only the
    /// clip's own).
    pub from: ClipSample,
    pub to: ClipSample,
}

/// The frames of the timeline a clip shows on, first and last.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipSpan {
    pub clip: u32,
    pub first: u32,
    pub last: u32,
}

/// A recipe's whole timeline.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    /// Frames a second: the base clip's rate.
    pub fps: f64,
    /// The last frame (keys run 0 to `frames`), as the clip library counts.
    pub frames: u32,
    pub keys: Vec<TimelineKey>,
    pub seams: Vec<Seam>,
    pub clips: Vec<ClipSpan>,
    pub mirror: bool,
}

/// `x` as the decimal it is written as, so the web app, reading the file's
/// number, steps by the same amount.
pub(super) fn wide(x: f32) -> f64 {
    x.to_string().parse().unwrap_or_else(|_| f64::from(x))
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A clip's frames a second, to a thousandth: its frames over its length.
pub fn clip_fps(clip: &RecipeBase) -> f64 {
    let length = wide(clip.length);
    if clip.rest || clip.frames == 0 || length <= 0.0 {
        return f64::from(REST_FPS);
    }
    (f64::from(clip.frames) / length * 1000.0).round() / 1000.0
}

/// The time edits of one clip, read once.
struct ClipEdits<'a> {
    retimes: Vec<&'a TimeEdit>,
    holds: Vec<&'a TimeEdit>,
}

impl ClipEdits<'_> {
    fn speed(&self, p: f64) -> f64 {
        for r in &self.retimes {
            let (Some(from), Some(to), Some(speed)) = (r.from, r.to, r.speed) else {
                continue;
            };
            let (from, to, s) = (f64::from(from), f64::from(to), wide(speed));
            if p < from || p >= to {
                continue;
            }
            let ease = f64::from(r.ease.unwrap_or(0));
            if ease > 0.0 && p < from + ease {
                return 1.0 + (s - 1.0) * ((p - from) / ease);
            }
            if ease > 0.0 && p > to - ease {
                return 1.0 + (s - 1.0) * ((to - p) / ease);
            }
            return s;
        }
        1.0
    }

    /// The hold that shows most at moving frame `p`, if any shows.
    fn hold_at(&self, p: f64) -> Option<HoldSample> {
        let mut best: Option<HoldSample> = None;
        for h in &self.holds {
            let (Some(at), Some(n)) = (h.at, h.frames) else {
                continue;
            };
            let weight = hold_weight(h, f64::from(at), f64::from(n), p);
            if weight > EPS && best.as_ref().is_none_or(|b| weight > b.weight) {
                best = Some(HoldSample {
                    at,
                    weight,
                    breathe: h.breathe.map_or(0.0, wide),
                });
            }
        }
        best
    }
}

fn hold_weight(h: &TimeEdit, at: f64, n: f64, p: f64) -> f64 {
    let ease_in = f64::from(h.ease_in.unwrap_or(0));
    let ease_out = f64::from(h.ease_out.unwrap_or(0));
    match h.kind {
        TimeKind::HoldOver => {
            if p < at - EPS || p > at + n + EPS {
                return 0.0;
            }
            let k = (p - at).clamp(0.0, n);
            let w_in = if ease_in > 0.0 {
                smooth(k / ease_in)
            } else {
                1.0
            };
            let w_out = if ease_out > 0.0 {
                smooth((n - k) / ease_out)
            } else if k < n - EPS {
                1.0
            } else {
                0.0
            };
            w_in.min(w_out)
        }
        TimeKind::Freeze => {
            if (p - at).abs() < EPS {
                1.0
            } else if ease_in > 0.0 && p >= at - ease_in && p < at {
                smooth((p - (at - ease_in)) / ease_in)
            } else if ease_out > 0.0 && p > at && p <= at + ease_out {
                smooth((at + ease_out - p) / ease_out)
            } else {
                0.0
            }
        }
        TimeKind::Retime => 0.0,
    }
}

/// One clip as it plays: its kept frames, its rate against the base clip's,
/// its time edits and its freezes in order.
struct ClipWalk<'a> {
    index: u32,
    a: f64,
    b: f64,
    rate: f64,
    edits: ClipEdits<'a>,
    freezes: Vec<(f64, u32, &'a TimeEdit)>,
}

impl<'a> ClipWalk<'a> {
    fn of(recipe: &'a RecipeFile, index: u32, fps: f64) -> Option<Self> {
        let clip = recipe.clip(index)?;
        let [a, b] = clip.trim.unwrap_or([0, clip.frames]);
        let (a, b) = (f64::from(a), f64::from(b));
        let mine = recipe.time.iter().filter(|t| t.clip == index);
        let mut freezes: Vec<(f64, u32, &TimeEdit)> = mine
            .clone()
            .filter(|t| t.kind == TimeKind::Freeze)
            .filter_map(|t| Some((f64::from(t.at?), t.frames?, t)))
            .filter(|(at, _, _)| *at >= a && *at <= b)
            .collect();
        freezes.sort_by(|x, y| x.0.total_cmp(&y.0));
        Some(Self {
            index,
            a,
            b,
            rate: (clip_fps(clip) / fps).clamp(0.1, 10.0),
            edits: ClipEdits {
                retimes: mine
                    .clone()
                    .filter(|t| t.kind == TimeKind::Retime)
                    .collect(),
                holds: mine.filter(|t| t.kind != TimeKind::Retime).collect(),
            },
            freezes,
        })
    }

    /// Steps through the clip's moving frames, up to about `limit` frames:
    /// `each` gets every moving frame, and after a freeze's frame the
    /// freeze, whose added frames count too.
    fn steps(&self, limit: usize, mut each: impl FnMut(f64, Option<&(f64, u32, &'a TimeEdit)>)) {
        let mut count = 0usize;
        let mut emitted = vec![false; self.freezes.len()];
        let mut visit = |p: f64, emitted: &mut Vec<bool>, count: &mut usize| {
            each(p, None);
            *count += 1;
            for (k, freeze) in self.freezes.iter().enumerate() {
                if !emitted[k] && (p - freeze.0).abs() < EPS {
                    emitted[k] = true;
                    each(freeze.0, Some(freeze));
                    *count += freeze.1 as usize;
                }
            }
        };
        let mut p = self.a;
        visit(p, &mut emitted, &mut count);
        while p < self.b - EPS && count < limit {
            let mut next = p + self.edits.speed(p) * self.rate;
            if let Some((at, _, _)) = self
                .freezes
                .iter()
                .zip(&emitted)
                .find(|((at, _, _), done)| !**done && p + EPS < *at && *at < next - EPS)
                .map(|(f, _)| f)
            {
                next = *at;
            }
            p = next.min(self.b);
            visit(p, &mut emitted, &mut count);
        }
    }
}

/// One clip's frames once trimmed and its time edits played out.
fn clip_samples(recipe: &RecipeFile, index: u32, fps: f64) -> Vec<ClipSample> {
    let Some(walk) = ClipWalk::of(recipe, index, fps) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk.steps(MAX_KEYS, |p, freeze| match freeze {
        None => out.push(ClipSample {
            clip: index,
            at: p,
            hold: walk.edits.hold_at(p),
        }),
        Some((at, frames, edit)) => {
            let hold = HoldSample {
                at: edit.at.unwrap_or(0),
                weight: 1.0,
                breathe: edit.breathe.map_or(0.0, wide),
            };
            for _ in 0..*frames {
                out.push(ClipSample {
                    clip: walk.index,
                    at: *at,
                    hold: Some(hold.clone()),
                });
            }
        }
    });
    out
}

/// How many frames one clip makes, counted without making them, up to
/// about `limit`.
fn clip_count(recipe: &RecipeFile, index: u32, fps: f64, limit: usize) -> usize {
    let Some(walk) = ClipWalk::of(recipe, index, fps) else {
        return 0;
    };
    let mut count = 0usize;
    walk.steps(limit, |_, freeze| {
        count += freeze.map_or(1, |f| f.1 as usize);
    });
    count
}

/// Where each clip starts on the timeline and how long its blend window is,
/// from each clip's frame count (`lens`, its samples less one), and the
/// last frame.
fn layout(recipe: &RecipeFile, lens: &[usize]) -> (Vec<usize>, Vec<usize>, usize) {
    let clips: Vec<&RecipeBase> = recipe.clips().collect();
    let mut starts = vec![0usize; lens.len()];
    let mut windows = vec![0usize; lens.len()];
    for i in 1..lens.len() {
        let wanted = clips[i].blend.as_ref().map_or(0, |b| b.frames) as usize;
        let room = lens[i - 1].saturating_sub(windows[i - 1]);
        windows[i] = wanted.min(room).min(lens[i]);
        starts[i] = starts[i - 1] + lens[i - 1] - windows[i];
    }
    let last = lens.len() - 1;
    let total = starts[last] + lens[last];
    (starts, windows, total)
}

/// How many frames `recipe`'s timeline runs (its last frame, as
/// [`Timeline::frames`]), counted without making them, or none when it
/// runs over `most`.
pub fn length_within(recipe: &RecipeFile, most: u32) -> Option<u32> {
    let fps = clip_fps(&recipe.base);
    let limit = most as usize + 2;
    let count = u32::try_from(recipe.joined.len() + 1).unwrap_or(1);
    let lens: Vec<usize> = (0..count)
        .map(|i| clip_count(recipe, i, fps, limit).saturating_sub(1))
        .collect();
    if lens.iter().any(|&n| n > most as usize) {
        return None;
    }
    let (_, _, total) = layout(recipe, &lens);
    u32::try_from(total).ok().filter(|&n| n <= most)
}

fn curve(kind: BlendCurve, x: f64) -> f64 {
    match kind {
        BlendCurve::Smooth => smooth(x),
        BlendCurve::Linear => x.clamp(0.0, 1.0),
    }
}

fn len(v: &[ClipSample]) -> usize {
    v.len().saturating_sub(1)
}

fn frame(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The timeline of `recipe`.
pub fn timeline(recipe: &RecipeFile) -> Timeline {
    let fps = clip_fps(&recipe.base);
    let count = u32::try_from(recipe.joined.len() + 1).unwrap_or(1);
    let parts: Vec<Vec<ClipSample>> = (0..count).map(|i| clip_samples(recipe, i, fps)).collect();
    let clips: Vec<&RecipeBase> = recipe.clips().collect();
    let lens: Vec<usize> = parts.iter().map(|p| len(p)).collect();
    let (starts, windows, total) = layout(recipe, &lens);
    let before: Vec<f64> = clips
        .iter()
        .scan(0.0, |sum, c| {
            let at = *sum;
            *sum += f64::from(c.frames);
            Some(at)
        })
        .collect();
    let all: f64 = clips.iter().map(|c| f64::from(c.frames)).sum();
    // A freeze's added frames sit on its frame; a hold over lets time run on.
    let place = |s: &ClipSample| before[s.clip as usize] + s.at;
    let share = |x: f64| {
        if all > 0.0 {
            (x / all).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    let mut keys = Vec::with_capacity(total + 1);
    for t in 0..=total {
        let i = (0..parts.len())
            .rev()
            .find(|&i| starts[i] <= t)
            .unwrap_or(0);
        let own = parts[i][t - starts[i]].clone();
        let key = if i > 0 && t < starts[i] + windows[i] {
            let from = parts[i - 1][t - starts[i - 1]].clone();
            let kind = clips[i]
                .blend
                .as_ref()
                .map_or_else(BlendCurve::default, |b| b.curve);
            #[allow(clippy::cast_precision_loss)]
            let w = curve(kind, (t - starts[i]) as f64 / windows[i] as f64);
            TimelineKey {
                share: share((1.0 - w) * place(&from) + w * place(&own)),
                from,
                to: Some(own),
                blend: w,
            }
        } else {
            TimelineKey {
                share: share(place(&own)),
                from: own,
                to: None,
                blend: 0.0,
            }
        };
        keys.push(key);
    }
    let seams = (1..parts.len())
        .map(|i| Seam {
            clip: frame(i),
            frame: frame(starts[i]),
            frames: frame(windows[i]),
            line_up: clips[i]
                .blend
                .as_ref()
                .map_or_else(LineUp::default, |b| b.line_up),
            from: parts[i - 1][starts[i] - starts[i - 1]].clone(),
            to: parts[i][0].clone(),
        })
        .collect();
    let spans = (0..parts.len())
        .map(|i| ClipSpan {
            clip: frame(i),
            first: frame(starts[i]),
            last: frame(starts[i] + len(&parts[i])),
        })
        .collect();
    Timeline {
        fps,
        frames: frame(total),
        keys,
        seams,
        clips: spans,
        mirror: recipe.mirror,
    }
}

/// Whether `recipe` plays its one clip whole, as round 1 did: no joined
/// clips, trim, time edits or mirror.
pub fn plain(recipe: &RecipeFile) -> bool {
    recipe.joined.is_empty()
        && recipe.base.trim.is_none()
        && recipe.time.is_empty()
        && !recipe.mirror
}
