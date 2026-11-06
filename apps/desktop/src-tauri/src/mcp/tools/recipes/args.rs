//! What the recipe tools take, and how each argument becomes the model's
//! clips, time edits, changes and blanks.

use maquette_core::animations::recipes::ClipPick;
use maquette_core::recipes::{
    BlankKind, BlendCurve, BlendWindow, GripFrom, KeyEase, LayerKind, LayerMode, LayerSettings,
    LineUp, PinFlat, PoseKey, RecipeBlank, RecipeLayer, TimeEdit, TimeKind,
};
use rmcp::schemars;
use serde::Deserialize;
use std::collections::BTreeMap;

use super::super::ToolError;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecipeArgs {
    /// The project id.
    pub project: String,
    /// The recipe id, as list_recipes returns it (its file name without .recipe.json).
    pub recipe: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct BlendArg {
    /// How many frames the two clips overlap, 0 (a cut) to half the shorter
    /// clip as trimmed. Defaults to 8.
    pub frames: Option<u32>,
    /// smooth (the default) or linear.
    pub curve: Option<String>,
    /// What lines up at the seam: hips-and-facing (the default), hips or off.
    pub line_up: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClipArg {
    /// A library clip id, as list_animations returns it. Give this or keep.
    pub clip: Option<String>,
    /// The index of a clip the recipe already has (get_recipe's clips, 0 the
    /// base), kept as it is: its hash, trim and blend, unless trim or blend
    /// is sent. The keys and time edits on it move with it. update_recipe
    /// only.
    pub keep: Option<u32>,
    /// The frames of the clip kept, [in, out], out after in and at most its
    /// frame count ([6, 58] keeps 52). Left out, a kept clip's own trim, or
    /// the whole of a library clip.
    pub trim: Option<[u32; 2]>,
    /// The blend from the clip before; ignored on the first clip. Left out,
    /// a kept clip's own blend, or 8 frames, smooth, hips and facing lined
    /// up.
    pub blend: Option<BlendArg>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct TimeArg {
    /// retime (a range plays faster or slower), freeze (stops on a frame
    /// and adds frames) or hold-over (keeps a pose across a range, the
    /// length unchanged).
    pub kind: String,
    /// The clip it sits on: 0 the base, then the joined clips.
    pub clip: u32,
    /// retime: the first frame of the range, on the clip's own frames
    /// before its trim.
    pub from: Option<u32>,
    /// retime: the frame the range ends at, after from.
    pub to: Option<u32>,
    /// retime: 0.25 to 4 (1.6 plays 14 frames in 9). Ranges on a clip may
    /// not overlap.
    pub speed: Option<f32>,
    /// retime: frames it eases over, at most half the range.
    pub ease: Option<u32>,
    /// freeze and hold-over: the frame of the clip whose pose is held.
    pub at: Option<u32>,
    /// freeze: frames it adds; hold-over: frames from at that keep the
    /// pose, ending within the clip. 1 to 600.
    pub frames: Option<u32>,
    /// freeze and hold-over: breathing so a long hold is not dead still, 0 to 1.
    pub breathe: Option<f32>,
    /// freeze and hold-over: frames it eases in over.
    pub ease_in: Option<u32>,
    /// freeze and hold-over: frames it eases out over.
    pub ease_out: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct KeyArg {
    /// The clip it sits on: 0 the base (always 0 on the rest pose), then
    /// the joined clips. Defaults to 0.
    pub clip: Option<u32>,
    /// The frame of that clip before its trim, from 0 (on the rest pose,
    /// the recipe's own frame).
    pub frame: u32,
    /// How the motion eases into it: smooth (the default), in, out, linear or step.
    pub ease: Option<String>,
    /// Degrees per humanoid bone turned at this frame, [x, y, z], within
    /// 180 each way: {"chest": [0, -18, 0]}. Only the bones you mean, at
    /// most 32.
    pub pose: BTreeMap<String, [f32; 3]>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct GripFromArg {
    /// The library grip's id.
    pub entry: String,
    /// Its revision, from 1.
    pub revision: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateRecipeArgs {
    /// The project id.
    pub project: String,
    /// The library clip it starts from, by id as list_animations returns it.
    /// Leave it out with restPose true to start from the model's rest pose,
    /// or give clips instead to join several.
    pub clip: Option<String>,
    /// Library clips joined in order, the first the base, each with its
    /// trim and blend (keep is not taken here). Up to 8.
    pub clips: Option<Vec<ClipArg>>,
    /// Left and right swapped across the whole recipe, after joining.
    #[serde(default)]
    pub mirror: bool,
    /// True starts the recipe on the model's rest pose (2 seconds, 60
    /// frames at 30 fps) with no base clip; never Mixamo's.
    #[serde(default)]
    pub rest_pose: bool,
    /// The recipe name, one line, up to 120 characters. Defaults to the clip's name.
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct ChangeArg {
    /// Keep it to edit a change; left out, a free one is made (l-1, l-2).
    pub id: Option<String>,
    /// reach (a hand or foot to a blank's target with IK), nudge (a pose
    /// offset on a bone across the clip), look-at (head or neck follows a
    /// blank's target), wave (a layered arm gesture), pin (a hand or foot
    /// stays put), pose (sparse pose keys) or grip (a hand pose copied
    /// from the grip library).
    pub kind: String,
    /// A VRM humanoid bone name. reach and pin: leftHand, rightHand,
    /// leftFoot or rightFoot. look-at: head or neck. wave: leftUpperArm,
    /// rightUpperArm, leftLowerArm, rightLowerArm, leftHand or rightHand.
    /// grip: leftHand or rightHand, the hand that holds. nudge: any
    /// humanoid bone (hips, spine, chest, head, leftUpperArm and so on).
    /// pose: leave it out, each key names its bones.
    pub bone: Option<String>,
    /// How much shows, 0 to 1. Defaults to 1.
    pub weight: Option<f32>,
    /// Where in the clip it starts, as a share of its length from 0 to 1. Defaults to 0.
    pub start: Option<f32>,
    /// Where it ends, 0 to 1 and after start. Defaults to 1.
    pub end: Option<f32>,
    /// reach and look-at: the id of the blank holding the target.
    pub target: Option<String>,
    /// nudge: degrees added to the bone's rotation, [x, y, z], within 180 each way.
    pub rotation: Option<[f32; 3]>,
    /// nudge: metres added to the bone's position, [x, y, z] with y up, within 2 each way.
    pub offset: Option<[f32; 3]>,
    /// wave: how many waves between start and end, more than 0 and at most 50.
    pub cycles: Option<f32>,
    /// wave: how far each wave swings in degrees, more than 0 and at most 90.
    pub amount: Option<f32>,
    /// pin and grip: frames it eases in over.
    pub ease_in: Option<u32>,
    /// pin and grip: frames it eases out over.
    pub ease_out: Option<u32>,
    /// A foot pin: what stays flat, toe-and-heel, heel or off.
    pub flat: Option<String>,
    /// pose and grip: add (on top of the clip, pose's default) or replace
    /// (in place of it, grip's default).
    pub mode: Option<String>,
    /// pose: the keys, at most 64, one per clip and frame.
    pub keys: Option<Vec<KeyArg>>,
    /// grip: the recipe's copy of the grip's pose, degrees per hand and
    /// finger bone, at most 64 bones.
    pub pose: Option<BTreeMap<String, [f32; 3]>>,
    /// grip: the library grip and revision it was copied from, as
    /// get_recipe answers it; send it back unchanged.
    pub from: Option<GripFromArg>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct BlankArg {
    /// Letters, digits, dashes and underscores; changes name it as their target.
    pub id: String,
    /// What fills it, one line, such as "The prop the hand reaches".
    pub label: String,
    /// prop (an asset of the project) or point (a place in the scene).
    pub kind: String,
    /// A prop blank's asset id, as list_assets returns it. Left out, the blank stays open.
    pub prop: Option<String>,
    /// A point blank's point, [x, y, z] in metres with y up. Left out, the blank stays open.
    pub point: Option<[f32; 3]>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdateRecipeArgs {
    /// The project id.
    pub project: String,
    /// The recipe id.
    pub recipe: String,
    /// A new name; the file and id stay.
    pub name: Option<String>,
    /// Every change, in the order they are laid over the clip. Replaces them whole.
    pub changes: Option<Vec<ChangeArg>>,
    /// Every blank. Replaces them whole.
    pub blanks: Option<Vec<BlankArg>>,
    /// Every clip in order, the base first, each a library clip id or the
    /// index of a clip the recipe has (keep), with its trim and blend.
    /// Replaces them whole; up to 8. Keys and time edits not sent with it
    /// follow their kept clip to its new place; changes and time sent with
    /// it count clips in the new order. Dropping or replacing a clip that
    /// kept keys or time edits sit on is refused (clip-in-use).
    pub clips: Option<Vec<ClipArg>>,
    /// Every time edit. Replaces them whole.
    pub time: Option<Vec<TimeArg>>,
    /// Left and right swapped across the whole recipe, or not.
    pub mirror: Option<bool>,
    /// The `version` get_recipe answered with. Needed whenever anything but
    /// the name is sent; a recipe that changed since is refused (moved) and
    /// nothing is written. A name only update may leave it out.
    pub expected: Option<String>,
    /// Prop blank ids to empty for good, even when the asset they name is gone.
    pub clear: Option<Vec<String>>,
}

pub(super) fn layer_kind(given: &str) -> Result<LayerKind, ToolError> {
    match given
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '_'], "-")
        .as_str()
    {
        "reach" => Ok(LayerKind::Reach),
        "nudge" => Ok(LayerKind::Nudge),
        "look-at" | "lookat" => Ok(LayerKind::LookAt),
        "wave" => Ok(LayerKind::Wave),
        "pin" => Ok(LayerKind::Pin),
        "pose" | "pose-keys" => Ok(LayerKind::Pose),
        "grip" => Ok(LayerKind::Grip),
        _ => Err(ToolError::from(format!(
            "{given} is not a kind of change. Use reach, nudge, look-at, wave, pin, pose or grip."
        ))),
    }
}

/// `given` lower case with spaces and underscores as dashes.
pub(super) fn word_of(given: &str) -> String {
    given.trim().to_ascii_lowercase().replace([' ', '_'], "-")
}

pub(super) fn one_of<T: Copy>(
    given: &str,
    what: &str,
    words: &[(&str, T)],
) -> Result<T, ToolError> {
    let word = word_of(given);
    words
        .iter()
        .find(|(w, _)| *w == word)
        .map(|(_, v)| *v)
        .ok_or_else(|| {
            let all: Vec<&str> = words.iter().map(|(w, _)| *w).collect();
            ToolError::from(format!("{given} is not a {what}. Use {}.", all.join(", ")))
        })
}

pub(super) fn blend_of(arg: BlendArg) -> Result<BlendWindow, ToolError> {
    let defaults = BlendWindow::default();
    Ok(BlendWindow {
        frames: arg.frames.unwrap_or(defaults.frames),
        curve: arg
            .curve
            .map(|c| {
                one_of(
                    &c,
                    "blend curve",
                    &[
                        ("smooth", BlendCurve::Smooth),
                        ("linear", BlendCurve::Linear),
                    ],
                )
            })
            .transpose()?
            .unwrap_or(defaults.curve),
        line_up: arg
            .line_up
            .map(|l| {
                one_of(
                    &l,
                    "line up",
                    &[
                        ("hips-and-facing", LineUp::HipsAndFacing),
                        ("hips", LineUp::Hips),
                        ("off", LineUp::Off),
                    ],
                )
            })
            .transpose()?
            .unwrap_or(defaults.line_up),
    })
}

pub(super) fn pick_of(arg: ClipArg) -> Result<ClipPick, ToolError> {
    Ok(ClipPick {
        clip: arg.clip,
        keep: arg.keep,
        trim: arg.trim,
        blend: arg.blend.map(blend_of).transpose()?,
    })
}

pub(super) fn picks_of(args: Vec<ClipArg>) -> Result<Vec<ClipPick>, ToolError> {
    args.into_iter().map(pick_of).collect()
}

pub(super) fn time_of(arg: TimeArg) -> Result<TimeEdit, ToolError> {
    let kind = one_of(
        &arg.kind,
        "kind of time edit",
        &[
            ("retime", TimeKind::Retime),
            ("freeze", TimeKind::Freeze),
            ("hold-over", TimeKind::HoldOver),
        ],
    )?;
    let mut edit = TimeEdit::hold(kind, arg.clip, 0, 0);
    edit.at = arg.at;
    edit.frames = arg.frames;
    edit.from = arg.from;
    edit.to = arg.to;
    edit.speed = arg.speed;
    edit.ease = arg.ease;
    edit.breathe = arg.breathe;
    edit.ease_in = arg.ease_in;
    edit.ease_out = arg.ease_out;
    Ok(edit)
}

pub(super) fn key_of(arg: KeyArg) -> Result<PoseKey, ToolError> {
    Ok(PoseKey {
        clip: arg.clip.unwrap_or(0),
        frame: arg.frame,
        ease: arg
            .ease
            .map(|e| {
                one_of(
                    &e,
                    "key ease",
                    &[
                        ("smooth", KeyEase::Smooth),
                        ("in-out", KeyEase::Smooth),
                        ("in", KeyEase::In),
                        ("out", KeyEase::Out),
                        ("linear", KeyEase::Linear),
                        ("step", KeyEase::Step),
                    ],
                )
            })
            .transpose()?
            .unwrap_or_default(),
        pose: arg.pose,
    })
}

pub(super) fn blank_kind(given: &str) -> Result<BlankKind, ToolError> {
    match given.trim().to_ascii_lowercase().as_str() {
        "prop" => Ok(BlankKind::Prop),
        "point" => Ok(BlankKind::Point),
        _ => Err(ToolError::from(format!(
            "{given} is not a kind of blank. Use prop or point."
        ))),
    }
}

pub(super) fn layer_of(arg: ChangeArg) -> Result<RecipeLayer, ToolError> {
    let defaults = LayerSettings::default();
    let flat = arg
        .flat
        .map(|f| {
            one_of(
                &f,
                "way to keep a foot flat",
                &[
                    ("toe-and-heel", PinFlat::ToeAndHeel),
                    ("heel", PinFlat::Heel),
                    ("off", PinFlat::Off),
                ],
            )
        })
        .transpose()?;
    let mode = arg
        .mode
        .map(|m| {
            one_of(
                &m,
                "mode",
                &[("add", LayerMode::Add), ("replace", LayerMode::Replace)],
            )
        })
        .transpose()?;
    Ok(RecipeLayer {
        id: arg.id.unwrap_or_default(),
        kind: layer_kind(&arg.kind)?,
        bone: arg.bone.unwrap_or_default(),
        settings: LayerSettings {
            weight: arg.weight.unwrap_or(defaults.weight),
            start: arg.start.unwrap_or(defaults.start),
            end: arg.end.unwrap_or(defaults.end),
            target: arg.target,
            rotation: arg.rotation,
            offset: arg.offset,
            cycles: arg.cycles,
            amount: arg.amount,
            ease_in: arg.ease_in,
            ease_out: arg.ease_out,
            flat,
        },
        mode,
        keys: arg
            .keys
            .unwrap_or_default()
            .into_iter()
            .map(key_of)
            .collect::<Result<_, _>>()?,
        pose: arg.pose,
        from: arg.from.map(|f| GripFrom {
            entry: f.entry,
            revision: f.revision,
        }),
    })
}

pub(super) fn blank_of(arg: BlankArg) -> Result<RecipeBlank, ToolError> {
    Ok(RecipeBlank {
        kind: blank_kind(&arg.kind)?,
        id: arg.id,
        label: arg.label,
        prop: arg.prop,
        point: arg.point,
    })
}
