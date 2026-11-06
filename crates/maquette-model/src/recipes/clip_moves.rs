//! What happens to the keys and time edits a recipe holds when its clips are
//! replaced: each follows the clip it sits on to that clip's new place, and
//! a change that takes away a clip something still sits on is refused, so
//! no motion silently moves to another clip.

use super::{RecipeBase, RecipeFile};
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

/// `next` with `clips` in place of its clips, and the keys and time edits it
/// kept from `current` (the lists not sent in the same change, `layers_sent`
/// and `time_sent` say which) moved to where their clip went. Lists sent in
/// the same change already count clips in the new order.
///
/// # Errors
///
/// `invalid` for no clips, a `kept` that does not fit or names a clip other
/// than the one sent in its place, or (starting
/// [`CLIP_IN_USE`]) a clip taken away that a kept key or time edit sits on.
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
    let mut dropped: Vec<(u32, String)> = Vec::new();
    if !layers_sent {
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
    if let Some((clip, _)) = dropped.first() {
        let clip = *clip;
        let mut what: Vec<String> = dropped
            .iter()
            .filter(|(c, _)| *c == clip)
            .map(|(_, w)| w.clone())
            .collect();
        what.dedup();
        let name = current
            .clip(clip)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        return Err(CoreError::Invalid(format!(
            "{CLIP_IN_USE} {} ({name}) would leave {} on no clip. Keep that clip (keep {clip}), or send changes and time without what sits on it.",
            u64::from(clip) + 1,
            what.join(", ")
        )));
    }
    let mut clips = clips.into_iter();
    if let Some(base) = clips.next() {
        next.base = base;
    }
    next.joined = clips.collect();
    Ok(())
}
