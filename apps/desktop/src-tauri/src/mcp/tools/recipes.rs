//! Animation recipes for agents: a project's recipes, each a base clip from
//! the animation library (or several joined) with changes laid over it on
//! humanoid bones. Listing, reading, making one from library clips and
//! changing its clips, time edits, mirror, changes, blanks or name. Nothing
//! here bakes, runs Blender or spends anything.

use std::collections::BTreeMap;

use maquette_core::animations::recipes::{BaseMatch, BaseStatus, ClipPick, RecipeDoc};
use maquette_core::recipes::{
    BlankKind, BlendCurve, BlendWindow, GripFrom, KeyEase, LayerKind, LayerMode, LayerSettings,
    LineUp, PinFlat, PoseKey, RecipeBase, RecipeBlank, RecipeChange, RecipeLayer, TimeEdit,
    TimeKind,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use maquette_core::animations::recipes::bake::not_bakeable;

use super::hints::{step, with_next};
use super::{McpTools, Out, ProjectArgs, ToolError, respond, to_short_value, to_value};
use crate::commands::recipes as commands;

/// The code of an update_recipe that sends lists without the version it read.
pub(super) const NEEDS_VERSION: &str = "needs-version";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecipeArgs {
    /// The project id.
    pub project: String,
    /// The recipe id, as list_recipes returns it (its file name without .recipe.json).
    pub recipe: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BlendArg {
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
pub struct ClipArg {
    /// A library clip id, as list_animations returns it. Give this or keep.
    pub clip: Option<String>,
    /// The index of a clip the recipe already has (get_recipe's clips, 0 the
    /// base), kept as it is, its hash included. update_recipe only.
    pub keep: Option<u32>,
    /// The frames of the clip kept, [in, out], out after in and at most its
    /// frame count ([6, 58] keeps 52). Left out, the whole clip.
    pub trim: Option<[u32; 2]>,
    /// The blend from the clip before; ignored on the first clip. Left out:
    /// 8 frames, smooth, hips and facing lined up.
    pub blend: Option<BlendArg>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimeArg {
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
pub struct KeyArg {
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
pub struct GripFromArg {
    /// The library grip's id.
    pub entry: String,
    /// Its revision, from 1.
    pub revision: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecipeArgs {
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
pub struct ChangeArg {
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
pub struct BlankArg {
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
pub struct UpdateRecipeArgs {
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
    /// Replaces them whole; up to 8.
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

fn layer_kind(given: &str) -> Result<LayerKind, ToolError> {
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
fn word_of(given: &str) -> String {
    given.trim().to_ascii_lowercase().replace([' ', '_'], "-")
}

fn one_of<T: Copy>(given: &str, what: &str, words: &[(&str, T)]) -> Result<T, ToolError> {
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

fn blend_of(arg: BlendArg) -> Result<BlendWindow, ToolError> {
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

fn pick_of(arg: ClipArg) -> Result<ClipPick, ToolError> {
    Ok(ClipPick {
        clip: arg.clip,
        keep: arg.keep,
        trim: arg.trim,
        blend: arg.blend.map(blend_of).transpose()?,
    })
}

fn picks_of(args: Vec<ClipArg>) -> Result<Vec<ClipPick>, ToolError> {
    args.into_iter().map(pick_of).collect()
}

fn time_of(arg: TimeArg) -> Result<TimeEdit, ToolError> {
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

fn key_of(arg: KeyArg) -> Result<PoseKey, ToolError> {
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

fn blank_kind(given: &str) -> Result<BlankKind, ToolError> {
    match given.trim().to_ascii_lowercase().as_str() {
        "prop" => Ok(BlankKind::Prop),
        "point" => Ok(BlankKind::Point),
        _ => Err(ToolError::from(format!(
            "{given} is not a kind of blank. Use prop or point."
        ))),
    }
}

fn layer_of(arg: ChangeArg) -> Result<RecipeLayer, ToolError> {
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

fn blank_of(arg: BlankArg) -> Result<RecipeBlank, ToolError> {
    Ok(RecipeBlank {
        kind: blank_kind(&arg.kind)?,
        id: arg.id,
        label: arg.label,
        prop: arg.prop,
        point: arg.point,
    })
}

/// One change in a few fields, as update_recipe takes it back; settings
/// and fields its kind does not use are left out.
pub(super) fn change_view(layer: &RecipeLayer) -> Value {
    let mut out = Map::new();
    out.insert("id".into(), json!(layer.id));
    out.insert("kind".into(), to_value(layer.kind));
    if !layer.bone.is_empty() {
        out.insert("bone".into(), json!(layer.bone));
    }
    let settings = to_short_value(&layer.settings);
    if let Value::Object(fields) = settings {
        out.extend(fields.into_iter().filter(|(_, v)| !v.is_null()));
    }
    if let Some(mode) = layer.mode {
        out.insert("mode".into(), to_value(mode));
    }
    if layer.kind == LayerKind::Pose {
        out.insert("keys".into(), to_short_value(&layer.keys));
    }
    if let Some(pose) = &layer.pose {
        out.insert("pose".into(), to_short_value(pose));
    }
    if let Some(from) = &layer.from {
        out.insert("from".into(), to_value(from));
    }
    Value::Object(out)
}

/// One clip as agents read it, with how it stands in the library.
fn clip_view(clip: &RecipeBase, found: &BaseMatch) -> Value {
    let mut out = json!({
        "name": clip.name, "frames": clip.frames,
        "seconds": to_short_value(clip.length), "hash": clip.hash,
        "library": to_value(found.status),
    });
    if clip.rest {
        out["rest"] = json!(true);
    }
    if let Some(id) = &found.clip {
        out["clip"] = json!(id);
    }
    if let Some(warning) = &found.warning {
        out["warning"] = json!(warning);
    }
    if let Some(trim) = clip.trim {
        out["trim"] = json!(trim);
    }
    if let Some(blend) = &clip.blend {
        out["blend"] = to_value(blend);
    }
    out
}

/// A recipe as agents read it.
pub(super) fn recipe_view(doc: &RecipeDoc) -> Value {
    let recipe = &doc.recipe;
    let base = clip_view(&recipe.base, &doc.base);
    let blanks: Vec<Value> = recipe
        .blanks
        .iter()
        .map(|b| {
            json!({
                "id": b.id, "label": b.label, "kind": to_value(b.kind),
                "prop": b.prop, "point": b.point.map(to_short_value), "filled": b.filled(),
            })
        })
        .collect();
    let mut out = json!({
        "id": doc.id, "file": doc.file, "name": recipe.name, "revision": recipe.revision,
        "edited": recipe.edited, "version": doc.version, "mixamo": doc.mixamo, "base": base,
        "blanks": blanks, "changes": recipe.layers.iter().map(change_view).collect::<Vec<_>>(),
    });
    if recipe.version > 1 {
        out["format"] = json!(recipe.version);
        let clips: Vec<Value> = std::iter::once(clip_view(&recipe.base, &doc.base))
            .chain(
                recipe
                    .joined
                    .iter()
                    .zip(&doc.joined)
                    .map(|(clip, found)| clip_view(clip, found)),
            )
            .collect();
        out["clips"] = json!(clips);
        out["mirror"] = json!(recipe.mirror);
        out["time"] = to_short_value(&recipe.time);
        if let Some(refusal) = not_bakeable(doc) {
            out["bakeable"] = json!(false);
            out["notBakeable"] = json!(refusal);
        }
    }
    if !doc.warnings.is_empty() {
        out["warnings"] = json!(doc.warnings);
    }
    out
}

/// What to do next with a recipe: fill its blanks, add changes, or import
/// its base clip first.
fn after(doc: &RecipeDoc) -> Vec<Value> {
    let mut next = Vec::new();
    if doc.base.status == BaseStatus::Missing
        || doc.joined.iter().any(|j| j.status == BaseStatus::Missing)
    {
        next.push(step(
            "import_animations",
            "Import the base clip into the library before the recipe can be baked.",
        ));
    }
    if doc.recipe.blanks.iter().any(|b| !b.filled()) {
        next.push(step(
            "update_recipe",
            "Fill the open blanks: a prop's asset id or a point.",
        ));
    } else if doc.recipe.layers.is_empty() {
        next.push(step(
            "update_recipe",
            "Add changes: reach, nudge, look-at or wave on a humanoid bone.",
        ));
    }
    next
}

#[tool_router(router = recipes_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List a project's animation recipes: each id (for the other recipe tools), name, base clip name and frame count, whether the base clip is Mixamo's, its change and blank counts with how many blanks are still open, and its version to send as expected (a prop blank whose asset the project no longer has counts as open, with a warning). A recipe file that cannot be read lists with its problem. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn list_recipes(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let listed = commands::list(host.session(), &args.project)?;
                let recipes: Vec<Value> = listed
                    .iter()
                    .map(|r| match &r.problem {
                        Some(problem) => json!({ "id": r.id, "problem": problem }),
                        None => {
                            let mut line = json!({
                                "id": r.id, "name": r.name, "base": r.base, "frames": r.frames,
                                "mixamo": r.mixamo, "changes": r.layers, "blanks": r.blanks,
                                "open": r.open, "edited": r.edited, "version": r.version,
                            });
                            if !r.warnings.is_empty() {
                                line["warnings"] = json!(r.warnings);
                            }
                            line
                        }
                    })
                    .collect();
                let out = json!({ "recipes": recipes });
                Ok(if listed.is_empty() {
                    with_next(
                        out,
                        vec![
                            step("list_animations", "Pick a base clip from the library."),
                            step("create_recipe", "Make a recipe on that clip."),
                        ],
                    )
                } else {
                    out
                })
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read an animation recipe: its name, revision, base clip (name, frames, seconds, hash, rest when it runs on the model's rest pose with no clip, and library: found, hash-differs, missing or rest, as matched in this computer's animation library by name and frame count, or by name alone when the hash is empty (a starter's clip), with the clip id and a warning when not exact), mixamo (a bake counts as Mixamo's when any clip does: kept out of export packs and never saved to the clip library), its blanks (targets filled per project: a prop's asset id or a point; a prop the project no longer has reads as an open blank, with a warning) and its changes in order (kind, humanoid bone, weight, start and end as shares of the clip, and the settings its kind uses). Format 2 recipes also answer format, clips (base first, with library, trim, blend), mirror, time and, while this Maquette cannot bake them, bakeable false with notBakeable; changes may be pin, pose (mode, keys) or grip (pose, mode, from). library names the library recipe and revision it came from or was saved as, the library's newest revision and newer when that is later (or gone); saveWaiting is the approval of a save a person has not answered yet. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn get_recipe(&self, Parameters(args): Parameters<RecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                let doc = commands::get(host.session(), &args.project, &args.recipe)?;
                let mut out = recipe_view(&doc);
                if let Some(from) = &doc.recipe.from {
                    let linked = crate::commands::recipe_library::linked(host.session(), from)
                        .ok()
                        .flatten()
                        .and_then(|entry| entry.latest().map(|r| r.revision));
                    out["library"] = match linked {
                        Some(latest) => json!({
                            "entry": from.entry, "revision": from.revision,
                            "latest": latest, "newer": latest > from.revision,
                        }),
                        None => {
                            json!({ "entry": from.entry, "revision": from.revision, "gone": true })
                        }
                    };
                }
                if let Some(waiting) = hub.recipe_save_for(&args.project, &args.recipe) {
                    out["saveWaiting"] = json!(waiting.id);
                }
                Ok(with_next(out, after(&doc)))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Make an animation recipe in a project on a clip of the animation library (clip), on several library clips joined in order (clips: each a clip id with its trim and blend window from the clip before, 8 frames smooth with hips and facing lined up unless told, up to 8 clips), or on the model's rest pose with no base clip (restPose true: 60 frames at 30 fps, never Mixamo's), as the Motion view's Recipes tab does; mirror true swaps left and right across the whole recipe. Writes one file, recipes/<id>.recipe.json, holding each clip's name, frames, length and hash, never keys or a mesh; it stays format 1, which an older Maquette opens, unless it joins clips, trims or mirrors. It starts with no changes or blanks; add them with update_recipe. A starter recipe (list_library_recipes) can be used instead with use_recipe. A recipe on any Mixamo clip is marked mixamo: its bake will count as Mixamo's. Joined clips, trims and mirror cannot be baked by this Maquette yet. Website and game projects both take recipes. Nothing is baked and nothing is spent."
    )]
    async fn create_recipe(&self, Parameters(args): Parameters<CreateRecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let given = [args.clip.is_some(), args.rest_pose, args.clips.is_some()];
                if given.iter().filter(|g| **g).count() != 1 {
                    return Err(ToolError::from(
                        "Give clip (a library clip id from list_animations), clips (several to join) or restPose true, one of the three.",
                    ));
                }
                let doc = match args.clips {
                    Some(clips) => {
                        let picks = picks_of(clips)?;
                        if picks.iter().any(|p| p.keep.is_some()) {
                            return Err(ToolError::from(
                                "A new recipe has no clips to keep yet. Give each clip as a library clip id.",
                            ));
                        }
                        commands::create_joined(
                            host.session(),
                            &args.project,
                            &picks,
                            args.mirror,
                            args.name.as_deref(),
                        )?
                    }
                    None if args.mirror => {
                        let picks = vec![ClipPick {
                            clip: args.clip.clone(),
                            ..ClipPick::default()
                        }];
                        if args.rest_pose {
                            return Err(ToolError::from(
                                "Mirror a recipe on a clip; the rest pose has no sides to swap.",
                            ));
                        }
                        commands::create_joined(
                            host.session(),
                            &args.project,
                            &picks,
                            true,
                            args.name.as_deref(),
                        )?
                    }
                    None => commands::create(
                        host.session(),
                        &args.project,
                        args.clip.as_deref(),
                        args.name.as_deref(),
                    )?,
                };
                host.recipe_changed(&args.project);
                let next = vec![
                    step(
                        "update_recipe",
                        "Add blanks for the targets, then the changes: reach, nudge, look-at or wave.",
                    ),
                    step("get_recipe", "Read it back."),
                ];
                Ok(with_next(recipe_view(&doc), next))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Change an animation recipe: its name (the file and id stay), changes, blanks, clips, time edits, mirror, or any of them. changes, blanks, clips and time each replace the list whole, so read get_recipe first and send every entry you keep. Everything is checked before anything is written: each kind's bones, weight 0 to 1, start before end, targets naming blanks the recipe has, a nudge's rotation or offset, a wave's cycles and amount, pose keys, grips, clips and time edits within their limits, and prop blanks naming assets of the project; keys and time edits left on a clip no longer sent are refused. A recipe using nothing new stays format 1, which an older Maquette opens; joined clips, trims, mirror, time edits, pins, pose keys or grips make it format 2, which this Maquette cannot bake yet. Left out, blanks stay as they are: a prop blank whose asset is gone keeps its prop in the file, reads as open, and warnings says which; one sent open keeps it the same way unless clear names it. Sending anything but the name needs expected, the version get_recipe answered with: a recipe changed since is refused (moved), never overwritten and nothing is written; the revision does not move. Answers as get_recipe does. Nothing is baked."
    )]
    async fn update_recipe(&self, Parameters(args): Parameters<UpdateRecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let lists = args.changes.is_some()
                    || args.blanks.is_some()
                    || args.clear.is_some()
                    || args.clips.is_some()
                    || args.time.is_some()
                    || args.mirror.is_some();
                if lists && args.expected.is_none() {
                    return Err(ToolError {
                        message: format!(
                            "Sending changes, blanks, clips, time, mirror or clear needs expected: the version get_recipe answered with for {}, so a person's save since is never overwritten.",
                            args.recipe
                        ),
                        code: NEEDS_VERSION.to_string(),
                    });
                }
                let clips = args
                    .clips
                    .map(|c| -> Result<_, ToolError> {
                        let picks = picks_of(c)?;
                        Ok(commands::picked_clips(
                            host.session(),
                            &args.project,
                            &args.recipe,
                            &picks,
                        )?)
                    })
                    .transpose()?;
                let change = RecipeChange {
                    name: args.name,
                    layers: args
                        .changes
                        .map(|c| c.into_iter().map(layer_of).collect::<Result<Vec<_>, _>>())
                        .transpose()?,
                    blanks: args
                        .blanks
                        .map(|b| b.into_iter().map(blank_of).collect::<Result<Vec<_>, _>>())
                        .transpose()?,
                    clips,
                    time: args
                        .time
                        .map(|t| t.into_iter().map(time_of).collect::<Result<Vec<_>, _>>())
                        .transpose()?,
                    mirror: args.mirror,
                    expected: args.expected,
                    clear: args.clear.unwrap_or_default(),
                };
                let doc = commands::update(host.session(), &args.project, &args.recipe, change)?;
                host.recipe_changed(&args.project);
                Ok(with_next(recipe_view(&doc), after(&doc)))
            })
            .await;
        respond(found)
    }
}
