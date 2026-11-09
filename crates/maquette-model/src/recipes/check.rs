//! The checks a recipe passes before it is written or after it is read:
//! its name, the base clip, the limits, each blank and each change by its
//! kind, with the bones that kind may move. A refusal says what to fix. The
//! checks of what version 2 adds (joined clips, trims, blends, time edits,
//! pose keys and grips) are in `check_edits`.

use std::collections::BTreeSet;

use maquette_glb::vrm::is_human_bone;

use super::check_edits::{
    check_clips, check_grip, check_length, check_pin, check_pose, check_time, check_totals,
};
use super::{
    BlankKind, GoneProp, HASH_MAX_CHARS, ID_MAX_CHARS, LABEL_MAX_CHARS, LayerKind, LayerSettings,
    MAX_BLANKS, MAX_CYCLES, MAX_FRAMES, MAX_LAYERS, MAX_OFFSET_METRES, MAX_POINT_METRES,
    MAX_REST_FRAMES, MAX_WAVE_DEGREES, NAME_MAX_CHARS, RecipeBase, RecipeBlank, RecipeFile,
    RecipeLayer,
};
use crate::error::{CoreError, CoreResult};
use crate::plans::{is_item_id, is_link_id, trimmed};
use crate::project::ProjectManifest;

/// The hands and feet a reach or a pin moves.
pub(super) const REACH_BONES: [&str; 4] = ["leftHand", "rightHand", "leftFoot", "rightFoot"];
/// What turns to look.
const LOOK_BONES: [&str; 2] = ["head", "neck"];
/// The hands a grip holds with.
const GRIP_BONES: [&str; 2] = ["leftHand", "rightHand"];
/// The arm bones a wave swings.
const WAVE_BONES: [&str; 6] = [
    "leftUpperArm",
    "rightUpperArm",
    "leftLowerArm",
    "rightLowerArm",
    "leftHand",
    "rightHand",
];

pub(super) fn invalid<T>(message: String) -> CoreResult<T> {
    Err(CoreError::Invalid(message))
}

/// One line of at most `max` characters, not blank.
fn check_line(text: &str, what: &str, max: usize) -> CoreResult<()> {
    let text = trimmed(text);
    if text.is_empty() {
        return invalid(format!("Give the {what} a name."));
    }
    if text.chars().count() > max {
        return invalid(format!("Keep the {what} under {max} characters."));
    }
    if text.chars().any(char::is_control) {
        return invalid(format!("Keep the {what} on one line."));
    }
    Ok(())
}

/// Checks a recipe name: one line, not blank, not too long.
///
/// # Errors
///
/// [`CoreError::Invalid`] saying what to fix.
pub fn check_name(name: &str) -> CoreResult<()> {
    check_line(name, "recipe", NAME_MAX_CHARS)
}

/// Checks everything a recipe holds.
///
/// # Errors
///
/// [`CoreError::Invalid`] saying what to fix.
pub fn validate(recipe: &RecipeFile) -> CoreResult<()> {
    check_name(&recipe.name)?;
    if recipe.revision == 0 {
        return invalid("A recipe's revision starts at 1.".into());
    }
    check_base(&recipe.base)?;
    check_clips(recipe)?;
    check_time(recipe)?;
    check_length(recipe)?;
    check_totals(recipe)?;
    if let Some(from) = &recipe.from
        && (!super::is_recipe_id(&from.entry)
            || !super::is_entry_uid(&from.uid)
            || from.revision == 0)
    {
        return invalid("The recipe names a library recipe that cannot be one.".into());
    }
    if recipe.blanks.len() > MAX_BLANKS {
        return invalid(format!("A recipe holds at most {MAX_BLANKS} blanks."));
    }
    if recipe.layers.len() > MAX_LAYERS {
        return invalid(format!("A recipe holds at most {MAX_LAYERS} changes."));
    }
    let mut blank_ids = BTreeSet::new();
    for blank in &recipe.blanks {
        check_blank(blank)?;
        if !blank_ids.insert(blank.id.as_str()) {
            return invalid(format!("Two blanks are called {}.", blank.id));
        }
    }
    let mut layer_ids = BTreeSet::new();
    for layer in &recipe.layers {
        check_layer(layer, recipe)?;
        if !layer_ids.insert(layer.id.as_str()) {
            return invalid(format!("Two changes have the id {}.", layer.id));
        }
    }
    Ok(())
}

pub(super) fn check_base(base: &RecipeBase) -> CoreResult<()> {
    check_clip(base, "base clip")
}

/// Checks one clip's name, frames, length and hash, `what` naming it in a
/// refusal ("base clip", "clip 2").
pub(super) fn check_clip(base: &RecipeBase, what: &str) -> CoreResult<()> {
    if base.rest {
        if !(2..=MAX_REST_FRAMES).contains(&base.frames) {
            return invalid(format!(
                "A recipe on the rest pose runs 2 to {MAX_REST_FRAMES} frames, not {}.",
                base.frames
            ));
        }
        if !base.hash.is_empty() || base.mixamo {
            return invalid(
                "A recipe on the rest pose has no base clip hash and is never Mixamo's.".into(),
            );
        }
        return Ok(());
    }
    check_line(&base.name, what, NAME_MAX_CHARS)?;
    if base.frames == 0 || base.frames > MAX_FRAMES {
        return invalid(format!(
            "The {what} has {} frames; a recipe takes 1 to {MAX_FRAMES}.",
            base.frames
        ));
    }
    if !base.length.is_finite() || base.length <= 0.0 {
        return invalid(format!(
            "The {what}'s length has to be more than 0 seconds."
        ));
    }
    if base.hash.chars().count() > HASH_MAX_CHARS || base.hash.chars().any(char::is_control) {
        return invalid(format!(
            "The {what}'s hash is not one the clip library writes."
        ));
    }
    Ok(())
}

pub(super) fn finite_within(values: &[f32], limit: f32) -> bool {
    values.iter().all(|v| v.is_finite() && v.abs() <= limit)
}

fn check_blank(blank: &RecipeBlank) -> CoreResult<()> {
    if !is_item_id(&blank.id) {
        return invalid(format!(
            "{:?} is not a blank id. Use up to {ID_MAX_CHARS} letters, digits, dashes and underscores.",
            blank.id
        ));
    }
    check_line(
        &blank.label,
        &format!("blank {}", blank.id),
        LABEL_MAX_CHARS,
    )?;
    match blank.kind {
        BlankKind::Prop => {
            if let Some(prop) = &blank.prop
                && !is_link_id(prop)
            {
                return invalid(format!(
                    "{prop:?} is not an asset id, so the blank {} cannot hold it.",
                    blank.id
                ));
            }
        }
        BlankKind::Point => {
            if let Some(point) = &blank.point
                && !finite_within(point, MAX_POINT_METRES)
            {
                return invalid(format!(
                    "The blank {} holds a point more than {MAX_POINT_METRES} m away.",
                    blank.id
                ));
            }
        }
    }
    Ok(())
}

fn kind_word(kind: LayerKind) -> &'static str {
    match kind {
        LayerKind::Reach => "reach",
        LayerKind::Nudge => "nudge",
        LayerKind::LookAt => "look at",
        LayerKind::Wave => "wave",
        LayerKind::Pin => "pin",
        LayerKind::Pose => "pose keys",
        LayerKind::Grip => "grip",
    }
}

/// The bones a change of `kind` may move, or none for any humanoid bone.
fn bones_for(kind: LayerKind) -> Option<&'static [&'static str]> {
    match kind {
        LayerKind::Reach => Some(&REACH_BONES),
        LayerKind::LookAt => Some(&LOOK_BONES),
        LayerKind::Wave => Some(&WAVE_BONES),
        LayerKind::Pin => Some(&REACH_BONES),
        LayerKind::Grip => Some(&GRIP_BONES),
        LayerKind::Nudge | LayerKind::Pose => None,
    }
}

fn check_layer(layer: &RecipeLayer, recipe: &RecipeFile) -> CoreResult<()> {
    if !is_item_id(&layer.id) {
        return invalid(format!(
            "{:?} is not a change id. Use up to {ID_MAX_CHARS} letters, digits, dashes and underscores.",
            layer.id
        ));
    }
    let word = kind_word(layer.kind);
    if layer.kind == LayerKind::Pose {
        // Pose keys name their bones in each key.
        if !layer.bone.is_empty() {
            return invalid(format!(
                "The pose keys {} name their bones in each key, so leave its bone empty.",
                layer.id
            ));
        }
        return check_settings(layer, word, recipe);
    }
    match bones_for(layer.kind) {
        Some(bones) if !bones.contains(&layer.bone.as_str()) => {
            return invalid(format!(
                "A {word} moves {}, not {:?}.",
                bones.join(", "),
                layer.bone
            ));
        }
        None if !is_human_bone(&layer.bone) => {
            return invalid(format!(
                "{:?} is not a humanoid bone. Use VRM names such as hips, spine, head, leftUpperArm or rightHand.",
                layer.bone
            ));
        }
        _ => {}
    }
    check_settings(layer, word, recipe)
}

fn check_settings(layer: &RecipeLayer, word: &str, recipe: &RecipeFile) -> CoreResult<()> {
    let blanks = &recipe.blanks;
    let LayerSettings {
        weight,
        start,
        end,
        target,
        rotation,
        offset,
        cycles,
        amount,
        ..
    } = &layer.settings;
    let id = &layer.id;
    if !(weight.is_finite() && (0.0..=1.0).contains(weight)) {
        return invalid(format!("The change {id}'s weight has to be 0 to 1."));
    }
    let share = |v: &f32| v.is_finite() && (0.0..=1.0).contains(v);
    if !share(start) || !share(end) || start >= end {
        return invalid(format!(
            "The change {id} has to start before it ends, both as shares of the clip from 0 to 1."
        ));
    }
    match layer.kind {
        LayerKind::Reach | LayerKind::LookAt => {
            let Some(target) = target else {
                return invalid(format!(
                    "A {word} needs a target: the id of a blank that holds it."
                ));
            };
            if !blanks.iter().any(|b| &b.id == target) {
                return invalid(format!(
                    "The change {id} aims at the blank {target}, which the recipe does not have."
                ));
            }
        }
        LayerKind::Nudge => {
            if rotation.is_none() && offset.is_none() {
                return invalid(format!(
                    "The nudge {id} needs a rotation, an offset or both."
                ));
            }
            if rotation.is_some_and(|r| !finite_within(&r, 180.0)) {
                return invalid(format!(
                    "Keep the nudge {id}'s rotation within 180 degrees each way."
                ));
            }
            if offset.is_some_and(|o| !finite_within(&o, MAX_OFFSET_METRES)) {
                return invalid(format!(
                    "Keep the nudge {id}'s offset within {MAX_OFFSET_METRES} m each way."
                ));
            }
        }
        LayerKind::Wave => {
            if !cycles.is_some_and(|c| c.is_finite() && c > 0.0 && c <= MAX_CYCLES) {
                return invalid(format!(
                    "The wave {id} needs cycles: more than 0 and at most {MAX_CYCLES}."
                ));
            }
            if !amount.is_some_and(|a| a.is_finite() && a > 0.0 && a <= MAX_WAVE_DEGREES) {
                return invalid(format!(
                    "The wave {id} needs an amount: more than 0 and at most {MAX_WAVE_DEGREES} degrees."
                ));
            }
        }
        LayerKind::Pin => check_pin(layer, blanks)?,
        LayerKind::Pose => check_pose(layer, recipe)?,
        LayerKind::Grip => check_grip(layer, blanks)?,
    }
    Ok(())
}

/// A blank `target` names, refused when the recipe has none of that id.
pub(super) fn check_target(
    id: &str,
    target: Option<&String>,
    blanks: &[RecipeBlank],
) -> CoreResult<()> {
    match target {
        Some(target) if !blanks.iter().any(|b| &b.id == target) => invalid(format!(
            "The change {id} aims at the blank {target}, which the recipe does not have."
        )),
        _ => Ok(()),
    }
}

/// Checks that every filled prop blank names an asset of `manifest`'s project.
///
/// # Errors
///
/// [`CoreError::Invalid`] naming the blank and the asset it names.
pub fn check_props(recipe: &RecipeFile, manifest: &ProjectManifest) -> CoreResult<()> {
    for blank in &recipe.blanks {
        if let Some(prop) = &blank.prop
            && !manifest.assets.iter().any(|a| &a.id == prop)
        {
            return invalid(format!(
                "The blank {} names {prop}, which is not an asset of {}.",
                blank.id, manifest.name
            ));
        }
    }
    Ok(())
}

/// The prop blanks whose prop is no longer an asset of `manifest`'s project
/// (removed, or a recipe synced in from another computer).
pub fn gone_props(recipe: &RecipeFile, manifest: &ProjectManifest) -> Vec<GoneProp> {
    recipe
        .blanks
        .iter()
        .filter_map(|blank| {
            let prop = blank.prop.as_ref()?;
            (!manifest.assets.iter().any(|a| &a.id == prop)).then(|| GoneProp {
                blank: blank.id.clone(),
                prop: prop.clone(),
            })
        })
        .collect()
}

/// Opens each prop blank whose prop is no longer an asset of `manifest`'s
/// project (removed, or a recipe synced in from another computer), so it
/// reads as open. The file is not touched. Answers with a line per blank
/// it opened.
pub fn settle_props(recipe: &mut RecipeFile, manifest: &ProjectManifest) -> Vec<String> {
    let mut warnings = Vec::new();
    for blank in &mut recipe.blanks {
        if let Some(prop) = &blank.prop
            && !manifest.assets.iter().any(|a| &a.id == prop)
        {
            warnings.push(format!(
                "The blank {} names {prop}, which is no longer an asset of {}, so it reads as open. The file keeps {prop}: restoring the asset fills it again, and clearing the blank empties it for good.",
                blank.id, manifest.name
            ));
            blank.prop = None;
        }
    }
    warnings
}
