//! The next steps table: for a tool and how it answered, and for where the
//! project ships, the tools that come next. The answers on the main path
//! (get_started, create_project, an empty asset list, runs, sends, the
//! inbox, picks and imports) take their `next` from here, so the website and
//! game journeys cannot drift apart file by file.

use maquette_core::ProjectTarget;
use serde_json::{Value, json};

use super::{person_step, step};

/// The project a step is about and where it ships.
#[derive(Debug, Clone, Copy)]
pub(in crate::mcp::tools) struct Place<'a> {
    pub project: &'a str,
    pub target: ProjectTarget,
}

/// What this computer can do for the start of a project.
#[derive(Debug, Clone, Copy)]
pub(in crate::mcp::tools) struct Can {
    pub codex: bool,
    pub blender: bool,
}

impl Can {
    /// Nothing known against either, as right after create_project.
    pub(in crate::mcp::tools) const BOTH: Self = Self {
        codex: true,
        blender: true,
    };
}

/// A step with the arguments to call it with.
pub(in crate::mcp::tools) fn step_with(tool: &str, why: &str, args: Value) -> Value {
    json!({ "tool": tool, "why": why, "args": args })
}

fn in_project(tool: &str, why: &str, project: &str) -> Value {
    step_with(tool, why, json!({ "project": project }))
}

fn build_only(project: &str, why: &str) -> Value {
    step_with(
        "run_pipeline",
        why,
        json!({ "project": project, "buildOnly": true }),
    )
}

/// The step that gives a project its repo.
pub(in crate::mcp::tools) fn repo_step(place: Place<'_>) -> Value {
    match place.target {
        ProjectTarget::Website => in_project(
            "set_ship_settings",
            "Set siteRepo: without a repo a run leaves Ship out and answers shipped false.",
            place.project,
        ),
        ProjectTarget::Game => in_project(
            "set_game_settings",
            "Set repo: without a repo a run leaves Ship out and answers shipped false.",
            place.project,
        ),
    }
}

/// How a project with no assets starts. A website starts from concepts or a
/// built model; a game starts from the models already in its repo.
pub(in crate::mcp::tools) fn start_of(place: Place<'_>, has_repo: bool, can: Can) -> Vec<Value> {
    let mut out = Vec::new();
    match place.target {
        ProjectTarget::Website => {
            out.push(if can.codex {
                in_project(
                    "start_concepts",
                    "Make concept images on Codex usage, then pick_concepts puts them on the Board (the maquette-lab-round skill walks it).",
                    place.project,
                )
            } else {
                person_step(
                    "Sign in to Codex (codex login) for concept images, or bring your own images as refs.",
                )
            });
            if can.blender {
                out.push(in_project(
                    "add_built_asset",
                    "Or make a model with a Blender script: no credits, no approval.",
                    place.project,
                ));
            }
            if !has_repo {
                out.push(repo_step(place));
            }
        }
        ProjectTarget::Game => {
            out.push(if has_repo {
                in_project(
                    "scan_game_models",
                    "A game starts from its own models: see what the model folder of the game repo holds.",
                    place.project,
                )
            } else {
                in_project(
                    "set_game_settings",
                    "A game starts from its own models: set repo to the game repo first.",
                    place.project,
                )
            });
            out.push(in_project(
                "import_game_models",
                "Import the models to work on: each becomes an asset whose version 1 is the repo file.",
                place.project,
            ));
        }
    }
    out
}

/// What a finished run says about Ship.
#[derive(Debug, Clone, Copy, Default)]
pub(in crate::mcp::tools) struct RunEnd {
    pub build_only: bool,
    /// The run left Ship out, for want of a repo.
    pub no_repo: bool,
    pub ship_denied: bool,
}

/// What to do after a run ended.
pub(in crate::mcp::tools) fn after_run(
    status: &str,
    end: RunEnd,
    place: Place<'_>,
) -> Option<Vec<Value>> {
    let RunEnd {
        build_only,
        no_repo,
        ship_denied,
    } = end;
    let p = place.project;
    let game = place.target == ProjectTarget::Game;
    match status {
        // A denied Ship ends the run as cancelled, not failed, and everything
        // before it is built: only a person takes this further.
        "cancelled" if ship_denied => Some(vec![
            person_step(
                "A person denied Ship. Ask them before running it again; do not retry on your own.",
            ),
            in_project("get_ship_plan", "See what Ship would have written.", p),
        ]),
        "failed" => Some(vec![
            in_project(
                "get_run",
                "Read the log with a larger logLines to see why the step failed.",
                p,
            ),
            in_project(
                "run_pipeline",
                "Run again with from set to the failed step once it is fixed.",
                p,
            ),
        ]),
        "done" if build_only => Some(vec![
            in_project("get_ship_plan", "See what Ship would write.", p),
            in_project(
                "run_pipeline",
                "Run without buildOnly to ship; a person may have to approve.",
                p,
            ),
        ]),
        "done" if no_repo => Some(vec![
            repo_step(place),
            if game {
                in_project(
                    "get_game",
                    "Check each model against its folder's budget.",
                    p,
                )
            } else {
                in_project(
                    "get_size_report",
                    "Compare what came in with what the build made.",
                    p,
                )
            },
        ]),
        "done" if game => Some(vec![
            in_project(
                "get_ship_plan",
                "See what Ship wrote into the model folder.",
                p,
            ),
            in_project(
                "get_game",
                "Check each model against its folder's budget.",
                p,
            ),
        ]),
        "done" => Some(vec![in_project(
            "get_size_report",
            "Compare what came in with what shipped.",
            p,
        )]),
        _ => None,
    }
}

fn text<'a>(answer: &'a Value, key: &str) -> Option<&'a str> {
    answer.get(key).and_then(Value::as_str)
}

/// The steps after a send to 3D, by its provider.
fn follow_sends(project: &str, id: Option<&str>, by_hand: bool) -> Vec<Value> {
    let poll = match id {
        Some(id) => step_with(
            "get_provider_job",
            "Poll the send until it reaches the inbox; do not send again.",
            json!({ "project": project, "id": id }),
        ),
        None => in_project(
            "get_provider_job",
            "Poll the sends until they reach the inbox; do not send again.",
            project,
        ),
    };
    if by_hand {
        vec![
            in_project(
                "list_handoffs",
                "A person makes the model on the provider's site and brings it back; see what still waits.",
                project,
            ),
            poll,
        ]
    } else {
        vec![poll]
    }
}

/// The steps after a model was accepted into `project`.
fn after_accept(project: &str, asset: Option<&str>) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(asset) = asset {
        let args = json!({ "project": project, "asset": asset });
        out.push(step_with(
            "get_real_size",
            "Check the model's size in metres.",
            args.clone(),
        ));
        out.push(step_with(
            "preview_fixups",
            "See the fix-ups the model would get before it ships.",
            args,
        ));
    }
    out.push(build_only(
        project,
        "Build the project: no approval, nothing written to a repo.",
    ));
    out
}

/// The `next` of a tool's answer, when the table has a row for it and the
/// answer carries none of its own. `project` is the call's argument.
pub(in crate::mcp::tools) fn after_tool(
    tool: &str,
    answer: &Value,
    project: Option<&str>,
) -> Option<Vec<Value>> {
    let project = text(answer, "project").or(project);
    match tool {
        "start_concepts" => Some(vec![in_project(
            "get_concepts",
            "Follow the round, then pick the good images onto the Board with pick_concepts.",
            project?,
        )]),
        "pick_concepts" => {
            let project = project?;
            let assets: Vec<&str> = answer
                .get("assets")?
                .as_array()?
                .iter()
                .filter_map(|a| text(a, "id"))
                .collect();
            let first = *assets.first()?;
            let views = step_with(
                "get_ref_views",
                if assets.len() == 1 {
                    "Check the asset's views: front, left, back and right go to 3D in that order."
                } else {
                    "Check each asset's views: front, left, back and right go to 3D in that order."
                },
                json!({ "project": project, "asset": first }),
            );
            let send = if assets.len() == 1 {
                step_with(
                    "send_to_3d",
                    "Send it to 3D: tripo or meshy wait for a person to approve the credits, by-hand is free.",
                    json!({ "project": project, "asset": first }),
                )
            } else {
                in_project(
                    "send_batch_to_3d",
                    "Send them to 3D at once: tripo or meshy wait for one approval, by-hand is free.",
                    project,
                )
            };
            Some(vec![views, send])
        }
        "send_to_3d" => Some(follow_sends(
            project?,
            text(answer, "id"),
            text(answer, "provider") == Some("by-hand"),
        )),
        "send_batch_to_3d" => {
            let jobs = answer.get("jobs")?.as_array()?;
            let first = jobs.first()?;
            Some(follow_sends(
                text(first, "project").or(project)?,
                None,
                text(first, "provider") == Some("by-hand"),
            ))
        }
        // One send, not the list of a project's sends.
        "get_provider_job" if answer.get("jobs").is_none() => {
            let project = project?;
            let asset = text(answer, "asset")?;
            match text(answer, "status")? {
                "inbox" => {
                    let file = text(answer, "inboxFile")?;
                    Some(vec![
                        step_with(
                            "get_inbox_checks",
                            "The model is in the inbox: check it.",
                            json!({ "id": file }),
                        ),
                        step_with(
                            "accept_inbox_model",
                            "Accept it as the asset's next version, or discard_inbox_model and redo it.",
                            json!({ "id": file, "project": project, "asset": asset }),
                        ),
                    ])
                }
                "accepted" => Some(after_accept(project, Some(asset))),
                "failed" | "cancelled" => Some(vec![step_with(
                    "send_to_3d",
                    "Send again once the cause in message is fixed; tripo or meshy wait for a person again.",
                    json!({ "project": project, "asset": asset }),
                )]),
                _ => None,
            }
        }
        "accept_inbox_model" => {
            let asset = answer.pointer("/asset/id").and_then(Value::as_str);
            Some(after_accept(project?, asset))
        }
        "accept_inbox_models" => {
            let first = answer
                .get("results")?
                .as_array()?
                .iter()
                .find(|r| r["ok"] == json!(true))?;
            let asset = first.pointer("/asset/id").and_then(Value::as_str);
            Some(after_accept(text(first, "project")?, asset))
        }
        "import_game_models" => {
            let project = project?;
            let imported = answer
                .get("imported")
                .and_then(Value::as_array)
                .is_some_and(|i| !i.is_empty());
            if !imported {
                return Some(vec![in_project(
                    "scan_game_models",
                    "Nothing was imported: see which files the model folder holds.",
                    project,
                )]);
            }
            Some(vec![
                in_project(
                    "get_game",
                    "Check each imported model against its folder's budget.",
                    project,
                ),
                build_only(
                    project,
                    "Build the game's models: no approval, nothing written to the repo.",
                ),
                in_project("get_ship_plan", "See what Ship would write.", project),
            ])
        }
        "list_inbox" => {
            let first = answer.get("items")?.as_array()?.first()?;
            let id = text(first, "id")?;
            Some(vec![
                step_with(
                    "get_inbox_checks",
                    "Check each model before accepting it.",
                    json!({ "id": id }),
                ),
                step_with(
                    "accept_inbox_model",
                    "Accept the good ones with the project they go to (sentFor or aimedAt names it).",
                    json!({ "id": id }),
                ),
                step(
                    "discard_inbox_model",
                    "Discard the bad ones; restore_inbox_model puts one back for 30 days.",
                ),
            ])
        }
        _ => None,
    }
}

/// Adds the table's `next` to a successful answer that has none.
pub(in crate::mcp::tools) fn add_next(
    tool: &str,
    project: Option<&str>,
    mut result: Result<rmcp::model::CallToolResponse, rmcp::ErrorData>,
) -> Result<rmcp::model::CallToolResponse, rmcp::ErrorData> {
    if let Ok(rmcp::model::CallToolResponse::Complete(answer)) = &mut result {
        if answer.is_error == Some(true) {
            return result;
        }
        for block in &mut answer.content {
            if let rmcp::model::ContentBlock::Text(t) = block
                && let Ok(mut body) = serde_json::from_str::<Value>(&t.text)
                && body.is_object()
                && body.get("next").is_none()
                && let Some(next) = after_tool(tool, &body, project)
            {
                body["next"] = Value::Array(next);
                t.text = serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string());
            }
        }
    }
    result
}

#[cfg(test)]
mod tests;
