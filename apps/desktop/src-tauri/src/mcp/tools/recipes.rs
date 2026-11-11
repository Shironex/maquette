//! Animation recipes for agents: a project's recipes, each a base clip from
//! the animation library (or several joined) with changes laid over it on
//! humanoid bones. Listing, reading, making one from library clips and
//! changing its clips, time edits, mirror, changes, blanks or name. Nothing
//! here bakes, runs Blender or spends anything.

use maquette_core::animations::recipes::{ClipPick, kept_of};
use maquette_core::recipes::RecipeChange;
use rmcp::RoleServer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Peer, tool, tool_router};
use serde_json::{Value, json};

use super::hints::{step, with_next};
use super::plans::agent_name;
use super::{McpTools, Out, ProjectArgs, ToolError, respond};
use crate::commands::recipes as commands;

mod args;
mod views;

pub(super) use args::RecipeArgs;
use args::{CreateRecipeArgs, UpdateRecipeArgs, blank_of, layer_of, picks_of, time_of};
use views::{after, grip_offer, with_grip_links};
pub(super) use views::{change_view, drafts_view, recipe_view};

/// The code of an update_recipe that sends lists without the version it read.
pub(super) const NEEDS_VERSION: &str = "needs-version";

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
        description = "Read an animation recipe: its name, revision, base clip (name, frames, seconds, hash, rest when it runs on the model's rest pose with no clip, and library: found, hash-differs, missing or rest, as matched in this computer's library by name and frame count, or by name alone when the hash is empty, with the clip id and a warning when not exact), mixamo (a bake counts as Mixamo's when any clip does: kept out of packs and the clip library), its blanks (per project, a prop's asset id or a point; a lost prop reads as open) and its changes in order (kind, humanoid bone, weight, start and end as shares of the clips' frames end to end, and its kind's settings). Format 2 also answers format, clips (base first, with library, trim, blend), mirror, time and frames (the bake's length); changes may be pin, pose (mode, keys) or grip (pose, mode, from). library names the library recipe and revision it came from or was saved as, the library's newest revision and newer when that is later (or gone); a grip change's library says the same of its grip, taken by apply_grip. saveWaiting and gripSavesWaiting are saves a person has yet to answer. drafts: what agents changed since a person saved, drafted until then. Reads only.",
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
                let links = crate::commands::grip_library::links(host.session(), &doc.recipe)
                    .unwrap_or_default();
                with_grip_links(&mut out, &links);
                let waiting: Vec<Value> = doc
                    .recipe
                    .layers
                    .iter()
                    .filter_map(|l| {
                        let ask = hub.grip_save_for(&args.project, &args.recipe, &l.id)?;
                        Some(json!({ "layer": l.id, "approval": ask.id }))
                    })
                    .collect();
                if !waiting.is_empty() {
                    out["gripSavesWaiting"] = json!(waiting);
                }
                let mut next = after(&doc);
                next.extend(grip_offer(&links));
                Ok(with_next(out, next))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Make an animation recipe in a project on a clip of the animation library (clip), on several library clips joined in order (clips: each a clip id with its trim and blend window from the clip before, 8 frames smooth with hips and facing lined up unless told, up to 8 clips), or on the model's rest pose with no base clip (restPose true: 60 frames at 30 fps, never Mixamo's), as the Motion view's Recipes tab does; mirror true swaps left and right across the whole recipe. Writes one file, recipes/<id>.recipe.json, holding each clip's name, frames, length and hash, never keys or a mesh; it stays format 1, which an older Maquette opens, unless it joins clips, trims or mirrors. It starts with no changes or blanks; add them with update_recipe. A starter recipe (list_library_recipes) can be used instead with use_recipe. A recipe on any Mixamo clip is marked mixamo: its bake will count as Mixamo's. Joined clips, trims and mirror bake like any recipe. Website and game projects both take recipes. Nothing is baked and nothing is spent."
    )]
    async fn create_recipe(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<CreateRecipeArgs>,
    ) -> Out {
        let by = agent_name(None, &peer);
        let found = self
            .blocking(move |host, _| {
                let given = [args.clip.is_some(), args.rest_pose, args.clips.is_some()];
                if given.iter().filter(|g| **g).count() != 1 {
                    return Err(ToolError::from(
                        "Give clip (a library clip id from list_animations), clips (several to join) or restPose true, one of the three.",
                    ));
                }
                let mut doc = match args.clips {
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
                commands::drafted(host.session(), &args.project, None, &mut doc, &by);
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
        description = "Change an animation recipe: its name (the file and id stay), changes, blanks, clips, time edits, mirror, or any of them. changes, blanks, clips and time each replace the list whole, so read get_recipe first and send every entry you keep. All is checked first: bones, weights, spans, targets naming blanks, nudge, wave, pose key, grip, clip and time limits, and prop blanks naming project assets. A clip sent with keep stays as it was (trim and blend too unless sent), and keys, changes and time edits not sent move with it; dropping a clip they sit on, or parting clips a change spans, is refused (clip-in-use). A file too big to open again is refused (recipe-too-big). Nothing new keeps format 1, which an older Maquette opens; clips, trims, mirror, time, pins, pose keys or grips make format 2, which bakes too. Blanks left out stay as they are: one whose asset is gone keeps its prop, reads as open, and warnings says which; one sent open keeps it too unless clear names it. Sending anything but the name needs expected, the version get_recipe answered with: a recipe changed since is refused (moved) and nothing is written. Answers as get_recipe does; what you changed shows in drafts until a person saves. Nothing is baked."
    )]
    async fn update_recipe(
        &self,
        peer: Peer<RoleServer>,
        Parameters(args): Parameters<UpdateRecipeArgs>,
    ) -> Out {
        let by = agent_name(None, &peer);
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
                let picked = args
                    .clips
                    .map(|c| -> Result<_, ToolError> {
                        let picks = picks_of(c)?;
                        let clips = commands::picked_clips(
                            host.session(),
                            &args.project,
                            &args.recipe,
                            &picks,
                        )?;
                        Ok((clips, kept_of(&picks)))
                    })
                    .transpose()?;
                let (clips, kept) = picked.unzip();
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
                    kept,
                    time: args
                        .time
                        .map(|t| t.into_iter().map(time_of).collect::<Result<Vec<_>, _>>())
                        .transpose()?,
                    mirror: args.mirror,
                    expected: args.expected,
                    clear: args.clear.unwrap_or_default(),
                };
                let before = commands::get(host.session(), &args.project, &args.recipe)?.recipe;
                let mut doc =
                    commands::update(host.session(), &args.project, &args.recipe, change)?;
                commands::drafted(host.session(), &args.project, Some(&before), &mut doc, &by);
                host.recipe_changed(&args.project);
                Ok(with_next(recipe_view(&doc), after(&doc)))
            })
            .await;
        respond(found)
    }
}
