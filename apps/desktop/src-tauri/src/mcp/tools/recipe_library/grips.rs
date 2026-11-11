//! The grip library for agents: listing and reading library grips with
//! their revisions, laying one on a recipe (a copy of the pose and a link
//! back) or taking a newer revision into a grip change, and proposing that
//! the hand pose of a recipe's pose keys be saved as a grip, which only
//! raises an approval: the person saves in the app. Agents never write the
//! grip library and never remove from it. Nothing here bakes, runs Blender
//! or spends anything.

use std::collections::BTreeMap;

use maquette_core::animations::grips::{
    ApplyGrip, GripFromKeys, LibraryGripDoc, LibraryGripRevision, LibraryGripSummary,
};
use maquette_core::recipes::LayerMode;
use maquette_core::recipes::grips::{GripHands, GripSide, WEAPONS};
use rmcp::RoleServer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Peer, schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::super::hints::{step, with_next};
use super::super::plans::agent_name;
use super::super::recipes::change_view;
use super::super::{McpTools, Out, ToolError, WAITING, respond, to_short_value, to_value};
use crate::commands::grip_library as commands;

/// Where a person saves a grip an agent proposed.
/// The code of a grip save refused while another grip from the same pose
/// key change waits for a person.
pub(super) const SAVE_WAITING: &str = "grip-save-waiting";
const SAVE_IN_APP: &str = "The approval prompt in the Maquette app: Review opens the recipe in the Motion view with the save a grip sheet, where the person looks at the hand and saves. Also listed under Settings > Agents. No tool can approve.";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct GripArgs {
    /// The library grip's id, as list_grips returns it.
    pub grip: String,
    /// A revision number; the newest when left out.
    pub revision: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct ApplyGripArgs {
    /// The project id.
    pub project: String,
    /// The recipe id.
    pub recipe: String,
    /// The library grip's id, as list_grips returns it.
    pub grip: String,
    /// A revision number; the newest when left out.
    pub revision: Option<u32>,
    /// The hands to take: both, right or left. A grip that poses both hands
    /// gives either alone; one that poses one hand gives the other as its
    /// mirror, or both. Left out, a new change takes the grip's own hands
    /// and a change that is there keeps the hands it posed.
    pub hands: Option<String>,
    /// The grip change to take it into, by id: its place, eases, weight and
    /// target stay, which is how a newer revision is taken. Left out, or an
    /// id the recipe has not got, adds a grip change at the end (with that
    /// id, or the lowest free l-N).
    pub layer: Option<String>,
    /// The version get_recipe (or the last write) answered with; a recipe
    /// changed since is refused (moved).
    pub expected: String,
    /// Where it starts and ends, as shares of the clips' frames end to end,
    /// 0 to 1. A new change covers the whole recipe.
    pub start: Option<f32>,
    pub end: Option<f32>,
    /// How much of it shows, 0 to 1 (1 for a new change).
    pub weight: Option<f32>,
    /// Frames it eases in and out over (4 each for a new change, 0 holds at once).
    pub ease_in: Option<u32>,
    pub ease_out: Option<u32>,
    /// replace (in place of the clip's turns on its bones, the default) or
    /// add (on top of them).
    pub mode: Option<String>,
    /// The blank that holds what the hand holds, by id.
    pub target: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProposeGripSaveArgs {
    /// The project id.
    pub project: String,
    /// The recipe id.
    pub recipe: String,
    /// The pose key change that holds the hand pose, by id.
    pub layer: String,
    /// The frame of the bake to read (get_recipe's frames); left out, the
    /// frame its first key on a hand stands on.
    pub frame: Option<u32>,
    /// The library grip to add a revision to, by id. Left out, a new grip.
    pub entry: Option<String>,
    /// The new grip's name. A revision keeps its grip's name.
    pub name: Option<String>,
    /// What it holds: one of list_grips' weapons (longsword, scythe, dagger,
    /// shield, empty and so on).
    pub weapon: String,
    /// The hands to keep: both, right or left. Left out, the sides the keys turn.
    pub hands: Option<String>,
    /// For both hands: the hand that holds, right (the default) or left.
    pub holds: Option<String>,
    /// Only these bones, by VRM name; left out, every hand, finger and arm
    /// bone the keys turn.
    pub bones: Option<Vec<String>>,
}

fn hands_of(given: &str) -> Result<GripHands, ToolError> {
    match given.trim().to_ascii_lowercase().as_str() {
        "both" => Ok(GripHands::Both),
        "right" => Ok(GripHands::Right),
        "left" => Ok(GripHands::Left),
        _ => Err(ToolError::from(format!(
            "{given} is not which hands to take. Use both, right or left."
        ))),
    }
}

fn side_of(given: &str) -> Result<GripSide, ToolError> {
    match given.trim().to_ascii_lowercase().as_str() {
        "right" => Ok(GripSide::Right),
        "left" => Ok(GripSide::Left),
        _ => Err(ToolError::from(format!(
            "{given} is not a hand. Use right or left."
        ))),
    }
}

fn mode_of(given: &str) -> Result<LayerMode, ToolError> {
    match given.trim().to_ascii_lowercase().as_str() {
        "add" => Ok(LayerMode::Add),
        "replace" => Ok(LayerMode::Replace),
        _ => Err(ToolError::from(format!(
            "{given} is not how a grip is laid on. Use replace or add."
        ))),
    }
}

/// A revision with where it was saved from, the fields it has none of left out.
fn revision_view(rev: &LibraryGripRevision) -> Value {
    let mut out = json!({ "revision": rev.revision, "savedAt": rev.saved_at });
    for (key, value) in [
        ("project", rev.project.as_ref().map(|v| json!(v))),
        ("recipe", rev.recipe.as_ref().map(|v| json!(v))),
        ("layer", rev.layer.as_ref().map(|v| json!(v))),
        ("frame", rev.frame.map(|v| json!(v))),
    ] {
        if let Some(value) = value {
            out[key] = value;
        }
    }
    out
}

/// A line of the grip list as agents read it, without the pose.
fn summary_view(grip: &LibraryGripSummary) -> Value {
    if let Some(problem) = &grip.problem
        && grip.latest == 0
    {
        return json!({ "id": grip.id, "name": grip.name, "problem": problem });
    }
    json!({
        "id": grip.id, "name": grip.name, "weapon": grip.weapon, "group": grip.group,
        "hands": grip.hands_words, "holds": grip.hands.map(|h| to_value(h.holds)),
        "supports": grip.hands.and_then(|h| h.supports).map(to_value),
        "bones": grip.pose.len(), "latest": grip.latest, "version": grip.version,
        "revisions": grip.revisions.iter().map(revision_view).collect::<Vec<_>>(),
    })
}

/// One revision of a grip as agents read it, with its pose.
fn grip_view(doc: &LibraryGripDoc) -> Value {
    let grip = &doc.grip;
    json!({
        "grip": doc.entry.id, "name": doc.entry.name, "uid": doc.entry.uid,
        "version": doc.version, "revision": doc.revision,
        "latest": doc.entry.latest().map(|r| r.revision),
        "weapon": grip.weapon, "holds": to_value(grip.hands.holds),
        "supports": grip.hands.supports.map(to_value),
        "bones": grip.pose.len(), "pose": to_short_value(&grip.pose),
        "revisions": doc.entry.revisions.iter().map(revision_view).collect::<Vec<_>>(),
    })
}

#[tool_router(router = grips_router, vis = "pub(in crate::mcp::tools)")]
impl McpTools {
    #[tool(
        description = "List the grip library: hand poses kept outside any project (library/grips at the workspace root), built as the recipe library is. Each grip has its id, name, weapon and the group it sorts under, hands (both hands, right hand, left hand, or either hand for an empty hand), which hand holds and which supports, how many bones its pose turns, its newest revision, every revision with the project, recipe, pose key change and frame it was saved from, and its version. A grip that cannot be read lists with its problem. weapons lists what a grip can hold. Lay one on a recipe with apply_grip. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn list_grips(&self) -> Out {
        let found = self
            .blocking(|host, _| {
                let listed = commands::list(host.session())?;
                let out = json!({
                    "grips": listed.iter().map(summary_view).collect::<Vec<_>>(),
                    "weapons": WEAPONS.iter().map(|(id, _, _)| *id).collect::<Vec<_>>(),
                });
                Ok(if listed.is_empty() {
                    with_next(
                        out,
                        vec![step(
                            "propose_grip_save",
                            "Propose saving the hand pose of a recipe's pose keys as a grip; a person saves it.",
                        )],
                    )
                } else {
                    with_next(out, vec![step("apply_grip", "Lay one on a recipe.")])
                })
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read a library grip: one revision (the newest unless revision is given) with its pose, degrees [x, y, z] per hand, finger and arm bone about the model's own axes (X to its left, Y up, Z ahead) as each stands under its parent, with no frames; its weapon, the hand that holds and the one that supports, its uid, and every revision with where it was saved from. A revision is never changed once saved. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn get_grip(&self, Parameters(args): Parameters<GripArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let doc = commands::get(host.session(), args.grip.trim(), args.revision)?;
                Ok(with_next(
                    grip_view(&doc),
                    vec![step("apply_grip", "Lay this revision on a recipe.")],
                ))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Lay a library grip on an animation recipe, as the Motion view's Add a grip does: a grip change that holds its own copy of the revision's pose (the newest unless revision is given) and a link back (from: the grip's id, uid and revision), so the recipe stays whole when exported and get_recipe says when the library has a newer revision. hands takes both, right or left: a grip that poses both hands gives either alone, one that poses one hand gives the other as its mirror, or both. layer names a grip change that is there to take the grip into, which is how a newer revision is taken: its place, eases, weight and target stay. Left out, a change is added at the end. It is laid in place of the clip's turns on its bones (mode add lays it on top), across the whole recipe unless start and end say, easing in and out over 4 frames unless told. Needs expected, the version get_recipe answered with; a recipe changed since is refused (moved). Checked like update_recipe (32 changes, 64 bones a grip, 1024 bone turns a recipe). The link and its revision do not make a bake out of date; the pose does. Grips bake. Nothing is baked or spent."
    )]
    async fn apply_grip(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<ApplyGripArgs>,
    ) -> Out {
        let by = agent_name(None, &peer);
        let found = self
            .blocking(move |host, _| {
                let request = ApplyGrip {
                    project: args.project.clone(),
                    recipe: args.recipe,
                    grip: args.grip.trim().to_string(),
                    revision: args.revision,
                    hands: args.hands.as_deref().map(hands_of).transpose()?,
                    layer: args.layer,
                    expected: Some(args.expected),
                    start: args.start,
                    end: args.end,
                    weight: args.weight,
                    ease_in: args.ease_in,
                    ease_out: args.ease_out,
                    mode: args.mode.as_deref().map(mode_of).transpose()?,
                    target: args.target,
                };
                let before =
                    crate::commands::recipes::get(host.session(), &args.project, &request.recipe)?
                        .recipe;
                let mut applied = commands::apply(host.session(), &request)?;
                crate::commands::recipes::drafted(
                    host.session(),
                    &args.project,
                    Some(&before),
                    &mut applied.doc,
                    &by,
                );
                host.recipe_changed(&args.project);
                let doc = &applied.doc;
                let layer = doc
                    .recipe
                    .layers
                    .iter()
                    .find(|l| l.id == applied.layer)
                    .ok_or_else(|| ToolError::from("The grip was written but not read back."))?;
                let mut out = json!({
                    "recipe": doc.id, "version": doc.version, "motionVersion": doc.motion_version,
                    "layer": change_view(layer), "updated": applied.updated,
                    "grip": applied.entry, "name": applied.name,
                    "revision": applied.revision, "latest": applied.latest,
                    "newer": applied.latest > applied.revision,
                    "bones": layer.pose.as_ref().map_or(0, BTreeMap::len),
                });
                if let Some(drafts) = &doc.drafts {
                    out["drafts"] = super::super::recipes::drafts_view(drafts);
                }
                Ok(with_next(
                    out,
                    vec![step(
                        "bake_recipe",
                        "Bake it onto a rigged model to see the hand exactly.",
                    )],
                ))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Propose saving the hand pose a recipe's pose key change holds at one frame as a grip in the grip library: a new grip (name), or the next revision of entry. Nothing is saved by this tool: it answers status waiting for approval with the approval id, the grip it read (hands, bones, pose) and what would change against the newest revision, in plain lines, and the person looks at the hand in the app's save a grip sheet and saves there. Only hand, finger and arm bones are taken, as their keys blend at that frame of the bake (the first key on a hand when frame is left out); hands keeps both, right or left. Keys that add to the clip are read as they are, and notes says a grip shows them from the rest pose. A save always adds a revision, never overwrites one. A pose that holds what the newest revision holds is refused. Asking twice for the same change returns the same approval; asking for another grip from that pose key change while one waits is refused (grip-save-waiting). Poll get_grip or list_grips: once saved, the revision is there. No tool removes a library grip or a revision. Spends nothing."
    )]
    async fn propose_grip_save(&self, Parameters(args): Parameters<ProposeGripSaveArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                let entry = args.entry.as_deref().map(str::trim).filter(|e| !e.is_empty());
                let name = match (entry, args.name.as_deref().map(str::trim)) {
                    // A revision keeps its grip's name, whatever is sent.
                    (Some(id), _) => commands::get(host.session(), id, None)?.entry.name,
                    (None, Some(name)) if !name.is_empty() => name.to_string(),
                    (None, _) => {
                        return Err(ToolError::from(
                            "Give the new grip a name, or entry to add a revision to a grip that is there.",
                        ));
                    }
                };
                let request = GripFromKeys {
                    project: args.project,
                    recipe: args.recipe,
                    layer: args.layer,
                    frame: args.frame,
                    name,
                    weapon: args.weapon,
                    hands: args.hands.as_deref().map(hands_of).transpose()?,
                    holds: args.holds.as_deref().map(side_of).transpose()?,
                    bones: args.bones,
                };
                let keyed = commands::keyed(host.session(), &request)?;
                let plan = commands::plan_save(host.session(), &keyed.grip, entry)?;
                if plan.same {
                    return Err(ToolError::from(format!(
                        "The pose holds what r{} of {} holds already, so there is nothing to save.",
                        plan.latest.unwrap_or_default(),
                        plan.entry_name
                    )));
                }
                let approval = hub
                    .request_grip_save(&plan, &keyed.source, &keyed.recipe_name)
                    .map_err(|waiting| ToolError {
                        message: format!(
                            "A grip save from the pose keys {} of {} waits for a person already ({waiting}) with another grip. Poll get_grip or list_grips until the person saves or closes it, then propose again.",
                            keyed.source.layer, keyed.recipe_name
                        ),
                        code: SAVE_WAITING.into(),
                    })?;
                let target = match &plan.entry {
                    Some(id) => format!("r{} of the library grip {id}", plan.revision),
                    None => format!("a new library grip, {}", plan.entry_name),
                };
                let mut answer = json!({
                    "status": WAITING,
                    "approval": approval,
                    "whereToApprove": SAVE_IN_APP,
                    "message": format!("Saving the hand pose of {} as {target} waits for a person to look at it and save it in Maquette. Nothing is saved until then.", keyed.recipe_name),
                    "plan": {
                        "entry": plan.entry, "name": plan.entry_name, "revision": plan.revision,
                        "latest": plan.latest, "file": plan.file, "bytes": plan.bytes,
                        "changes": plan.changes, "layer": keyed.source.layer,
                        "frame": keyed.source.frame, "weapon": plan.grip.weapon,
                        "holds": to_value(plan.grip.hands.holds),
                        "supports": plan.grip.hands.supports.map(to_value),
                        "bones": plan.grip.pose.len(), "pose": to_short_value(&plan.grip.pose),
                    },
                });
                if !keyed.notes.is_empty() {
                    answer["notes"] = json!(keyed.notes);
                }
                Ok(with_next(
                    answer,
                    vec![step("list_grips", "Follow the save: the revision lists once the person saves.")],
                ))
            })
            .await;
        respond(found)
    }
}
