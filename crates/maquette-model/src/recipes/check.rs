//! The checks a recipe passes before it is written or after it is read:
//! its name, the base clip, the limits, each blank and each change by its
//! kind, with the bones that kind may move. A refusal says what to fix.

use std::collections::BTreeSet;

use maquette_glb::vrm::is_human_bone;

use super::{
    BlankKind, HASH_MAX_CHARS, ID_MAX_CHARS, LABEL_MAX_CHARS, LayerKind, LayerSettings, MAX_BLANKS,
    MAX_CYCLES, MAX_FRAMES, MAX_LAYERS, MAX_OFFSET_METRES, MAX_POINT_METRES, MAX_WAVE_DEGREES,
    NAME_MAX_CHARS, RecipeBase, RecipeBlank, RecipeFile, RecipeLayer,
};
use crate::error::{CoreError, CoreResult};
use crate::plans::{is_item_id, is_link_id, trimmed};
use crate::project::ProjectManifest;

/// The hands and feet a reach moves.
const REACH_BONES: [&str; 4] = ["leftHand", "rightHand", "leftFoot", "rightFoot"];
/// What turns to look.
const LOOK_BONES: [&str; 2] = ["head", "neck"];
/// The arm bones a wave swings.
const WAVE_BONES: [&str; 6] = [
    "leftUpperArm",
    "rightUpperArm",
    "leftLowerArm",
    "rightLowerArm",
    "leftHand",
    "rightHand",
];

fn invalid<T>(message: String) -> CoreResult<T> {
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
        check_layer(layer, &recipe.blanks)?;
        if !layer_ids.insert(layer.id.as_str()) {
            return invalid(format!("Two changes have the id {}.", layer.id));
        }
    }
    Ok(())
}

fn check_base(base: &RecipeBase) -> CoreResult<()> {
    check_line(&base.name, "base clip", NAME_MAX_CHARS)?;
    if base.frames == 0 || base.frames > MAX_FRAMES {
        return invalid(format!(
            "The base clip has {} frames; a recipe takes 1 to {MAX_FRAMES}.",
            base.frames
        ));
    }
    if !base.length.is_finite() || base.length <= 0.0 {
        return invalid("The base clip's length has to be more than 0 seconds.".into());
    }
    if base.hash.chars().count() > HASH_MAX_CHARS || base.hash.chars().any(char::is_control) {
        return invalid("The base clip's hash is not one the clip library writes.".into());
    }
    Ok(())
}

fn finite_within(values: &[f32], limit: f32) -> bool {
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
    }
}

/// The bones a change of `kind` may move, or none for any humanoid bone.
fn bones_for(kind: LayerKind) -> Option<&'static [&'static str]> {
    match kind {
        LayerKind::Reach => Some(&REACH_BONES),
        LayerKind::LookAt => Some(&LOOK_BONES),
        LayerKind::Wave => Some(&WAVE_BONES),
        LayerKind::Nudge => None,
    }
}

fn check_layer(layer: &RecipeLayer, blanks: &[RecipeBlank]) -> CoreResult<()> {
    if !is_item_id(&layer.id) {
        return invalid(format!(
            "{:?} is not a change id. Use up to {ID_MAX_CHARS} letters, digits, dashes and underscores.",
            layer.id
        ));
    }
    let word = kind_word(layer.kind);
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
    check_settings(layer, word, blanks)
}

fn check_settings(layer: &RecipeLayer, word: &str, blanks: &[RecipeBlank]) -> CoreResult<()> {
    let LayerSettings {
        weight,
        start,
        end,
        target,
        rotation,
        offset,
        cycles,
        amount,
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
    }
    Ok(())
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
