//! What happens to the keys, time edits and changes a recipe holds when its
//! clips are replaced: each follows the clip it sits on to that clip's new
//! place, and a change that takes away a clip something still sits on is
//! refused, so no motion silently moves to another clip.
//!
//! A change's start and end are shares of the clips' own frames laid end to
//! end (see `timeline`), so joining, taking out or moving a clip moves every
//! share: each is read back to its clip and frame, and written again where
//! that clip now sits. A change across a seam keeps the clips it spans
//! together, in order. `fixtures/clip_moves.json` holds cases the web app's
//! copy of these rules agrees on.

use super::timeline::wide;
use super::{LayerKind, RecipeBase, RecipeFile, RecipeLayer};
use crate::error::{CoreError, CoreResult};

/// How a refusal of clips that take away a clip keys or time edits still
/// sit on starts, so an answer can carry its own code.
pub const CLIP_IN_USE: &str = "Taking out clip";

/// The same clip as far as what sits on it goes: its name, frames, hash and
/// rest pose mark, whatever its trim or blend.
fn same_clip(a: &RecipeBase, b: &RecipeBase) -> bool {
    a.name == b.name
        && a.frames == b.frames
        && a.hash == b.hash
        && a.rest == b.rest
        && a.by_name == b.by_name
}

/// For each clip of `current`, where it goes among `clips`: the first new
/// place that keeps it (`kept[j]` is the index of the clip of `current`
/// that clip `j` keeps), or none when no clip sent keeps it. Without
/// `kept`, a clip sent keeps the first clip of `current` it matches
/// ([`same_clip`]) that no clip before it took.
fn places(
    current: &RecipeFile,
    clips: &[RecipeBase],
    kept: Option<&[Option<u32>]>,
) -> CoreResult<Vec<Option<u32>>> {
    let old: Vec<&RecipeBase> = current.clips().collect();
    let origins: Vec<Option<usize>> = match kept {
        Some(kept) => {
            if kept.len() != clips.len() {
                return Err(CoreError::Invalid(
                    "Say for every clip sent which clip it keeps, or none for a new one.".into(),
                ));
            }
            kept.iter()
                .zip(clips)
                .enumerate()
                .map(|(j, (k, clip))| match k {
                    None => Ok(None),
                    Some(i) => {
                        let i = usize::try_from(*i).unwrap_or(usize::MAX);
                        match old.get(i) {
                            None => Err(CoreError::Invalid(format!(
                                "The recipe has no clip {} to keep.",
                                i.saturating_add(1)
                            ))),
                            Some(was) if !same_clip(was, clip) => {
                                Err(CoreError::Invalid(format!(
                                    "Clip {} sent keeps clip {} ({}), but it is {}. Keep a clip as it is, or send it as a new one.",
                                    j + 1,
                                    i + 1,
                                    was.name,
                                    clip.name
                                )))
                            }
                            Some(_) => Ok(Some(i)),
                        }
                    }
                })
                .collect::<CoreResult<_>>()?
        }
        None => {
            let mut taken = vec![false; old.len()];
            clips
                .iter()
                .map(|clip| {
                    let found = (0..old.len()).find(|&i| !taken[i] && same_clip(old[i], clip));
                    if let Some(i) = found {
                        taken[i] = true;
                    }
                    found
                })
                .collect()
        }
    };
    let mut out = vec![None; old.len()];
    for (j, origin) in origins.iter().enumerate() {
        if let Some(i) = origin
            && out[*i].is_none()
        {
            out[*i] = u32::try_from(j).ok();
        }
    }
    Ok(out)
}

/// Why a change's start and end cannot follow their clips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stranded {
    /// The change sits wholly or partly on this clip, which goes.
    Dropped(usize),
    /// The change runs across a seam onto this clip, which no longer comes
    /// right after the clip before it.
    Split(usize),
}

/// The clip a frame `x` of the clips' frames laid end to end sits on, and
/// that clip's own frame. A start on a seam belongs to the clip after it,
/// an end on a seam to the clip before.
fn locate(frames: &[f64], x: f64, end: bool) -> Option<(usize, f64)> {
    let mut before = 0.0;
    let mut last = None;
    for (i, &n) in frames.iter().enumerate() {
        if n > 0.0 {
            let inside = if end { x <= before + n } else { x < before + n };
            if inside {
                return Some((i, (x - before).clamp(0.0, n)));
            }
            last = Some((i, n));
        }
        before += n;
    }
    last
}

/// A share as written: to a millionth, as the web app rounds it.
fn rounded(v: f64) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    let out = ((v.clamp(0.0, 1.0) * 1e6).round() / 1e6) as f32;
    out
}

/// A change's `start` and `end` on clips of `from` frames, moved to where
/// their clips went among clips of `to` frames (`places[i]` is clip `i`'s new
/// place, none when it goes). A share whose clip keeps its place and whose
/// clips keep their length comes back as it was.
///
/// # Errors
///
/// [`Stranded`] when a clip the change sits on goes, or the clips it spans
/// no longer follow one another in order.
pub(super) fn move_share(
    from: &[u32],
    to: &[u32],
    places: &[Option<u32>],
    start: f32,
    end: f32,
) -> Result<(f32, f32), Stranded> {
    let old: Vec<f64> = from.iter().map(|&n| f64::from(n)).collect();
    let new: Vec<f64> = to.iter().map(|&n| f64::from(n)).collect();
    let all_old: f64 = old.iter().sum();
    let all_new: f64 = new.iter().sum();
    if all_old <= 0.0 || all_new <= 0.0 {
        return Ok((start, end));
    }
    let before = |frames: &[f64], i: usize| frames[..i.min(frames.len())].iter().sum::<f64>();
    let Some((a, a_at)) = locate(&old, wide(start) * all_old, false) else {
        return Ok((start, end));
    };
    let Some((b, b_at)) = locate(&old, wide(end) * all_old, true) else {
        return Ok((start, end));
    };
    let place = |i: usize| {
        places
            .get(i)
            .copied()
            .flatten()
            .and_then(|p| usize::try_from(p).ok())
    };
    for i in a..=b {
        if place(i).is_none() {
            return Err(Stranded::Dropped(i));
        }
    }
    for i in a + 1..=b {
        if place(i) != place(i - 1).map(|p| p + 1) {
            return Err(Stranded::Split(i));
        }
    }
    let moved = |i: usize, at: f64, share: f32| {
        let to = place(i).unwrap_or(i);
        let (was, now) = (before(&old, i), before(&new, to));
        if was == now && all_old == all_new {
            share
        } else {
            rounded((now + at) / all_new)
        }
    };
    Ok((moved(a, a_at, start), moved(b, b_at, end)))
}

/// What sits on clips that go, by clip, as a refusal names it.
type Dropped = Vec<(u32, String)>;

/// What a person calls the clip at `index` of `clips`: "clip 2 (Walk)".
fn named(clips: &[&RecipeBase], index: usize) -> String {
    let name = clips
        .get(index)
        .map(|c| c.name.as_str())
        .unwrap_or_default();
    format!("{} ({name})", index + 1)
}

/// Each change of `layers` but pose keys (which always cover the whole
/// recipe and sit on clips by their keys) with its start and end moved to
/// where its clips went; the changes whose clips go, by clip, and the first
/// change whose clips no longer follow one another.
fn follow_shares(
    layers: &mut [RecipeLayer],
    from: &[u32],
    to: &[u32],
    places: &[Option<u32>],
) -> (Dropped, Option<(usize, String)>) {
    let mut dropped = Vec::new();
    let mut split = None;
    for layer in layers.iter_mut().filter(|l| l.kind != LayerKind::Pose) {
        let s = &mut layer.settings;
        match move_share(from, to, places, s.start, s.end) {
            Ok((start, end)) => {
                s.start = start;
                s.end = end;
            }
            Err(Stranded::Dropped(clip)) => dropped.push((
                u32::try_from(clip).unwrap_or(u32::MAX),
                format!("the change {}", layer.id),
            )),
            Err(Stranded::Split(clip)) => {
                split.get_or_insert((clip, layer.id.clone()));
            }
        }
    }
    (dropped, split)
}

/// `next` with `clips` in place of its clips, and the keys, changes and time
/// edits it kept from `current` (the lists not sent in the same change,
/// `layers_sent` and `time_sent` say which) moved to where their clip went.
/// Lists sent in the same change already count clips, and read shares, in
/// the new order.
///
/// # Errors
///
/// `invalid` for no clips, a `kept` that does not fit or names a clip other
/// than the one sent in its place, or (starting
/// [`CLIP_IN_USE`]) a clip taken away that a kept key, change or time edit
/// sits on, or a clip moved away from the clip before it that a kept change
/// runs across to.
pub(super) fn replace_clips(
    next: &mut RecipeFile,
    current: &RecipeFile,
    clips: Vec<RecipeBase>,
    kept: Option<&[Option<u32>]>,
    layers_sent: bool,
    time_sent: bool,
) -> CoreResult<()> {
    if clips.is_empty() {
        return Err(CoreError::Invalid(
            "Send at least one clip: the first is the base.".into(),
        ));
    }
    let places = places(current, &clips, kept)?;
    let place = |clip: u32| {
        places
            .get(usize::try_from(clip).unwrap_or(usize::MAX))
            .copied()
            .flatten()
    };
    let mut dropped: Dropped = Vec::new();
    let mut split = None;
    if !layers_sent {
        let from: Vec<u32> = current.clips().map(|c| c.frames).collect();
        let to: Vec<u32> = clips.iter().map(|c| c.frames).collect();
        let (gone, apart) = follow_shares(&mut next.layers, &from, &to, &places);
        dropped.extend(gone);
        split = apart;
        for layer in &mut next.layers {
            for key in &mut layer.keys {
                match place(key.clip) {
                    Some(to) => key.clip = to,
                    None => dropped.push((key.clip, format!("the pose keys {}", layer.id))),
                }
            }
        }
    }
    if !time_sent {
        for edit in &mut next.time {
            match place(edit.clip) {
                Some(to) => edit.clip = to,
                None => dropped.push((edit.clip, "a time edit".into())),
            }
        }
    }
    let old: Vec<&RecipeBase> = current.clips().collect();
    if let Some(&(clip, _)) = dropped.iter().min_by_key(|(c, _)| *c) {
        let mut what: Vec<String> = dropped
            .iter()
            .filter(|(c, _)| *c == clip)
            .map(|(_, w)| w.clone())
            .collect();
        what.dedup();
        return Err(CoreError::Invalid(format!(
            "{CLIP_IN_USE} {} would leave {} on no clip. Keep that clip (keep {clip}), or send changes and time without what sits on it.",
            named(&old, usize::try_from(clip).unwrap_or(usize::MAX)),
            what.join(", ")
        )));
    }
    if let Some((clip, id)) = split {
        return Err(CoreError::Invalid(format!(
            "{CLIP_IN_USE} {} away from clip {} would put other motion under the change {id}, which runs across both. Keep them together in that order, or send changes without it.",
            named(&old, clip),
            clip
        )));
    }
    let mut clips = clips.into_iter();
    if let Some(base) = clips.next() {
        next.base = base;
    }
    next.joined = clips.collect();
    Ok(())
}
