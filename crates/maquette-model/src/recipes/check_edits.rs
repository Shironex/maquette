//! The checks of what version 2 adds: the joined clips with their trims and
//! blend windows, the time edits on each clip's own frames, and the pin,
//! pose key and grip changes. A refusal says what to fix, naming clips as a
//! person counts them (clip 1 is the base).

use std::collections::BTreeSet;

use maquette_glb::vrm::is_human_bone;

use super::check::{REACH_BONES, check_clip, check_target, finite_within, invalid};
use super::{
    BonePose, LayerKind, MAX_CLIPS, MAX_FRAMES, MAX_GRIP_BONES, MAX_HOLD_FRAMES, MAX_KEY_BONES,
    MAX_KEYS, MAX_RECIPE_KEYS, MAX_RECIPE_TURNS, MAX_SPEED, MAX_TIME_EDITS, MAX_TURN_DEGREES,
    MIN_SPEED, RecipeBase, RecipeBlank, RecipeFile, RecipeLayer, TimeEdit, TimeKind,
};
use crate::error::CoreResult;
use crate::plans::is_item_id;

/// Clip `index` as a person counts it: clip 1 is the base.
fn clip_word(index: usize) -> String {
    format!("clip {}", index + 1)
}

/// Checks the joined clips, every trim and every blend window.
pub(super) fn check_clips(recipe: &RecipeFile) -> CoreResult<()> {
    if recipe.joined.len() + 1 > MAX_CLIPS {
        return invalid(format!("A recipe joins at most {MAX_CLIPS} clips."));
    }
    if recipe.base.rest && !recipe.joined.is_empty() {
        return invalid(
            "A recipe on the rest pose has no clips to join. Start it on a clip instead.".into(),
        );
    }
    for (i, clip) in recipe.joined.iter().enumerate() {
        let what = clip_word(i + 1);
        if clip.rest {
            return invalid(format!(
                "Only the base can be the rest pose, and then alone; {what} cannot."
            ));
        }
        check_clip(clip, &what)?;
    }
    let clips: Vec<&RecipeBase> = recipe.clips().collect();
    for (i, clip) in clips.iter().enumerate() {
        check_trim(clip, &clip_word(i))?;
    }
    for (i, pair) in clips.windows(2).enumerate() {
        let [before, clip] = pair else { continue };
        let what = clip_word(i + 1);
        let Some(blend) = &clip.blend else {
            return invalid(format!(
                "The {what} needs a blend window from the clip before."
            ));
        };
        let most = before.kept().min(clip.kept()) / 2;
        if blend.frames > most {
            return invalid(format!(
                "The blend into {what} lasts {} frames; it can last at most {most}, half the shorter clip as trimmed.",
                blend.frames
            ));
        }
    }
    Ok(())
}

fn check_trim(clip: &RecipeBase, what: &str) -> CoreResult<()> {
    let Some([from, to]) = clip.trim else {
        return Ok(());
    };
    if clip.rest {
        return invalid(
            "The rest pose has no clip to trim. Set how many frames it runs instead.".into(),
        );
    }
    if from >= to || to > clip.frames {
        return invalid(format!(
            "The {what} keeps frames {from} to {to}; keep at least one frame within its {} frames, the first before the last.",
            clip.frames
        ));
    }
    Ok(())
}

/// Checks the time edits: each on a clip the recipe has, within that
/// clip's own frames, retime ranges not overlapping one another and holds
/// not overlapping one another.
pub(super) fn check_time(recipe: &RecipeFile) -> CoreResult<()> {
    if recipe.time.len() > MAX_TIME_EDITS {
        return invalid(format!(
            "A recipe holds at most {MAX_TIME_EDITS} time edits."
        ));
    }
    for edit in &recipe.time {
        let Some(clip) = recipe.clip(edit.clip) else {
            return invalid(format!(
                "A time edit sits on clip {}, which the recipe does not have.",
                u64::from(edit.clip) + 1
            ));
        };
        let what = clip_word(usize::try_from(edit.clip).unwrap_or(usize::MAX));
        match edit.kind {
            TimeKind::Retime => check_retime(edit, clip, &what)?,
            TimeKind::Freeze | TimeKind::HoldOver => check_hold(edit, clip, &what)?,
        }
    }
    for (i, edit) in recipe.time.iter().enumerate() {
        let retime = edit.kind == TimeKind::Retime;
        let Some((first, end)) = edit.span() else {
            continue;
        };
        let clash = recipe.time[..i].iter().find(|other| {
            other.clip == edit.clip
                && (other.kind == TimeKind::Retime) == retime
                && other.span().is_some_and(|(a, b)| a < end && first < b)
        });
        if clash.is_some() {
            let what = clip_word(usize::try_from(edit.clip).unwrap_or(usize::MAX));
            return invalid(if retime {
                format!("Two retime ranges on {what} overlap. Keep each range apart.")
            } else {
                format!("Two holds on {what} overlap. Keep each hold apart.")
            });
        }
    }
    Ok(())
}

fn check_retime(edit: &TimeEdit, clip: &RecipeBase, what: &str) -> CoreResult<()> {
    let (Some(from), Some(to), Some(speed)) = (edit.from, edit.to, edit.speed) else {
        return invalid(format!("A retime on {what} needs from, to and speed."));
    };
    if from >= to || to > clip.frames {
        return invalid(format!(
            "A retime on {what} runs from frame {from} to {to}; keep it within its {} frames, from before to.",
            clip.frames
        ));
    }
    if !(speed.is_finite() && (MIN_SPEED..=MAX_SPEED).contains(&speed)) {
        return invalid(format!(
            "A retime plays {MIN_SPEED}x to {MAX_SPEED}x, not {speed}x."
        ));
    }
    if edit.ease.is_some_and(|e| e > (to - from) / 2) {
        return invalid(format!(
            "A retime on {what} eases over at most half its range, {} frames.",
            (to - from) / 2
        ));
    }
    Ok(())
}

fn check_hold(edit: &TimeEdit, clip: &RecipeBase, what: &str) -> CoreResult<()> {
    let word = if edit.kind == TimeKind::Freeze {
        "freeze"
    } else {
        "hold over"
    };
    let (Some(at), Some(frames)) = (edit.at, edit.frames) else {
        return invalid(format!("A {word} on {what} needs at and frames."));
    };
    if at >= clip.frames {
        return invalid(format!(
            "A {word} on {what} holds frame {at}, past its {} frames.",
            clip.frames
        ));
    }
    if frames == 0 || frames > MAX_HOLD_FRAMES {
        return invalid(format!(
            "A {word} lasts 1 to {MAX_HOLD_FRAMES} frames, not {frames}."
        ));
    }
    if edit.kind == TimeKind::HoldOver && at + frames > clip.frames {
        return invalid(format!(
            "A hold over on {what} keeps the length, so it has to end within its {} frames.",
            clip.frames
        ));
    }
    if edit
        .breathe
        .is_some_and(|s| !(s.is_finite() && (0.0..=1.0).contains(&s)))
    {
        return invalid(format!("A {word}'s breathe has to be 0 to 1."));
    }
    for ease in [edit.ease_in, edit.ease_out].into_iter().flatten() {
        if ease > frames {
            return invalid(format!(
                "A {word} on {what} eases over at most its own {frames} frames."
            ));
        }
    }
    Ok(())
}

/// Frames a pin or grip eases in and out over, each within the longest clip.
fn check_eases(layer: &RecipeLayer, word: &str) -> CoreResult<()> {
    for ease in [layer.settings.ease_in, layer.settings.ease_out]
        .into_iter()
        .flatten()
    {
        if ease > MAX_FRAMES {
            return invalid(format!(
                "The {word} {} eases over at most {MAX_FRAMES} frames.",
                layer.id
            ));
        }
    }
    Ok(())
}

/// A pose of at most `most` humanoid bones, each turned within
/// [`MAX_TURN_DEGREES`] each way; `what` names it in a refusal.
fn check_bone_pose(pose: &BonePose, most: usize, what: &str) -> CoreResult<()> {
    if pose.is_empty() {
        return invalid(format!(
            "The {what} turns no bone. Turn one, or take it out."
        ));
    }
    if pose.len() > most {
        return invalid(format!("The {what} turns at most {most} bones."));
    }
    for (bone, turn) in pose {
        if !is_human_bone(bone) {
            return invalid(format!(
                "The {what} turns {bone:?}, which is not a humanoid bone. Use VRM names such as chest, head or rightIndexProximal."
            ));
        }
        if !finite_within(turn, MAX_TURN_DEGREES) {
            return invalid(format!(
                "The {what} turns {bone} more than {MAX_TURN_DEGREES} degrees each way."
            ));
        }
    }
    Ok(())
}

/// A pin: a target blank, when it has one, that the recipe has; keep flat
/// only on a foot.
pub(super) fn check_pin(layer: &RecipeLayer, blanks: &[RecipeBlank]) -> CoreResult<()> {
    check_target(&layer.id, layer.settings.target.as_ref(), blanks)?;
    let foot = REACH_BONES[2..].contains(&layer.bone.as_str());
    if layer.settings.flat.is_some() && !foot {
        return invalid(format!(
            "The pin {} keeps a foot flat, so it needs a foot, not {}.",
            layer.id, layer.bone
        ));
    }
    check_eases(layer, "pin")
}

/// Pose keys: at most [`MAX_KEYS`], each on a clip the recipe has and a
/// frame within it, one key per clip and frame, each turning at most
/// [`MAX_KEY_BONES`] humanoid bones.
pub(super) fn check_pose(layer: &RecipeLayer, recipe: &RecipeFile) -> CoreResult<()> {
    let id = &layer.id;
    if layer.keys.len() > MAX_KEYS {
        return invalid(format!("The pose keys {id} hold at most {MAX_KEYS} keys."));
    }
    let mut seen = BTreeSet::new();
    for key in &layer.keys {
        let Some(clip) = recipe.clip(key.clip) else {
            return invalid(format!(
                "A key of {id} sits on clip {}, which the recipe does not have.",
                u64::from(key.clip) + 1
            ));
        };
        if key.frame >= clip.frames {
            return invalid(format!(
                "A key of {id} sits on frame {}, past the {} frames of {}.",
                key.frame,
                clip.frames,
                clip_word(usize::try_from(key.clip).unwrap_or(usize::MAX))
            ));
        }
        if !seen.insert((key.clip, key.frame)) {
            return invalid(format!(
                "{id} has two keys on frame {} of {}. Keep one.",
                key.frame,
                clip_word(usize::try_from(key.clip).unwrap_or(usize::MAX))
            ));
        }
        check_bone_pose(
            &key.pose,
            MAX_KEY_BONES,
            &format!("key at frame {} of {id}", key.frame),
        )?;
    }
    Ok(())
}

/// A grip: its pose (at most [`MAX_GRIP_BONES`] humanoid bones), a target
/// blank when it holds one, and a library link that can be one.
pub(super) fn check_grip(layer: &RecipeLayer, blanks: &[RecipeBlank]) -> CoreResult<()> {
    let id = &layer.id;
    check_target(id, layer.settings.target.as_ref(), blanks)?;
    let Some(pose) = &layer.pose else {
        return invalid(format!(
            "The grip {id} needs its pose: degrees per hand and finger bone."
        ));
    };
    check_bone_pose(pose, MAX_GRIP_BONES, &format!("grip {id}"))?;
    if let Some(from) = &layer.from
        && (!is_item_id(&from.entry) || from.revision == 0)
    {
        return invalid(format!(
            "The grip {id} names a library grip that cannot be one."
        ));
    }
    check_eases(layer, "grip")
}

/// The recipe-wide caps: at most [`MAX_RECIPE_KEYS`] pose keys across its
/// pose key changes and [`MAX_RECIPE_TURNS`] bone turns across those keys
/// and its grips, so a recipe at every limit still fits in a file.
pub(super) fn check_totals(recipe: &RecipeFile) -> CoreResult<()> {
    let keys: usize = recipe
        .layers
        .iter()
        .filter(|l| l.kind == LayerKind::Pose)
        .map(|l| l.keys.len())
        .sum();
    if keys > MAX_RECIPE_KEYS {
        return invalid(format!(
            "The recipe holds {keys} pose keys; it holds at most {MAX_RECIPE_KEYS} across all its pose keys. Take some out."
        ));
    }
    let turns: usize = recipe
        .layers
        .iter()
        .map(|l| match l.kind {
            LayerKind::Pose => l.keys.iter().map(|k| k.pose.len()).sum(),
            LayerKind::Grip => l.pose.as_ref().map_or(0, BonePose::len),
            _ => 0,
        })
        .sum();
    if turns > MAX_RECIPE_TURNS {
        return invalid(format!(
            "The recipe turns {turns} bones across its pose keys and grips; it turns at most {MAX_RECIPE_TURNS}. Turn fewer bones per key or grip."
        ));
    }
    Ok(())
}
