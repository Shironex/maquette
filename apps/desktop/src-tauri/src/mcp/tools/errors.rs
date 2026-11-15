//! The errors agents read: the message, a stable code and, where one helps,
//! what fixes it, the tool to try next and whether only a person can.

use maquette_core::{CoreError, ErrorPayload};
use serde_json::{Value, json};

/// The code of a refusal no [`CoreError`] stands behind.
const REFUSED: &str = "refused";

/// Why a tool could not do the job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError {
    pub message: String,
    pub code: String,
}

impl From<String> for ToolError {
    fn from(message: String) -> Self {
        Self {
            message,
            code: REFUSED.to_string(),
        }
    }
}

impl From<&str> for ToolError {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}

impl From<ErrorPayload> for ToolError {
    fn from(err: ErrorPayload) -> Self {
        Self {
            message: err.message,
            code: err.code,
        }
    }
}

impl From<CoreError> for ToolError {
    fn from(err: CoreError) -> Self {
        err.payload().into()
    }
}

/// What to do about an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Advice {
    fix: &'static str,
    try_tool: Option<&'static str>,
    /// An agent cannot do this: a person has to, in the app or on the computer.
    person_only: bool,
}

const fn person(fix: &'static str, try_tool: Option<&'static str>) -> Advice {
    Advice {
        fix,
        try_tool,
        person_only: true,
    }
}

const fn agent(fix: &'static str, try_tool: &'static str) -> Advice {
    Advice {
        fix,
        try_tool: Some(try_tool),
        person_only: false,
    }
}

const NEW_KEY: Advice = person(
    "A person saves the key in Maquette (Settings > 3D providers). No tool can. The free route: send_to_3d with provider by-hand writes a handoff folder for the provider's site, or add_built_asset makes the model with a Blender script.",
    Some("send_to_3d"),
);
const OPENAI_KEY: Advice = person(
    "A person saves the key in Maquette (Settings > Image generation). No tool can. The free routes: the codex backend (set_image_backend), or your own images added as refs with add_refs.",
    Some("set_image_backend"),
);
const CODEX: Advice = person(
    "A person installs Codex CLI and runs codex login, then checks the tools in Settings > Toolchain. The other routes: the openai-images backend (needs an OpenAI key and a person's approval), or your own images added as refs with add_refs.",
    Some("add_refs"),
);
const BLENDER: Advice = person(
    "A person installs Blender from blender.org or locates it in Settings > Toolchain. No tool can. get_started reads the tools again.",
    Some("get_started"),
);
const FFMPEG: Advice = person(
    "A person installs ffmpeg or locates it in Settings > Toolchain. Without it, render the WebP loop, which needs only Blender.",
    Some("get_started"),
);
const NO_WORKSPACE: Advice = person(
    "A person opens or creates a workspace in the Maquette app. No tool can.",
    Some("get_started"),
);

const CAP: Advice = agent(
    "Ask for fewer images or use the codex backend (set_image_backend). Only a person can raise the cap, in Settings > Image generation.",
    "set_image_backend",
);

/// Message parts to the advice they call for, first match wins. Whole phrases,
/// because a bare word can sit in an asset's name or a path. A test reads the
/// sources these come from, so a reworded message fails it.
const BY_MESSAGE: &[(&str, Advice)] = &[
    ("would pass the monthly OpenAI image cap", CAP),
    ("cap can only be changed by a person", CAP),
    ("API key in Settings > 3D providers first", NEW_KEY),
    ("Save a Tripo API key first", NEW_KEY),
    ("Save a Meshy API key first", NEW_KEY),
    (
        "did not accept the API key. Check it in Settings > 3D providers",
        NEW_KEY,
    ),
    ("Save an OpenAI API key first", OPENAI_KEY),
    (
        "did not accept the API key. Check it in Settings > Image generation",
        OPENAI_KEY,
    ),
    ("Codex CLI is not installed", CODEX),
    ("Codex CLI is not signed in", CODEX),
    ("Blender is not installed", BLENDER),
    ("Blender was not found", BLENDER),
    ("needs ffmpeg, and it was not found", FFMPEG),
    ("ffmpeg is not installed", FFMPEG),
    ("No workspace is open", NO_WORKSPACE),
    (
        "has not run yet",
        agent(
            "Start a run; buildOnly true stops before Ship and asks no approval.",
            "run_pipeline",
        ),
    ),
    (
        "has no game repo",
        agent(
            "Set the game's repo with set_game_settings (repo), no need to open Settings.",
            "set_game_settings",
        ),
    ),
    (
        "A game project ships through Game export",
        agent(
            "Game export is the pipeline's last step: run_pipeline does it (it waits for a person to approve), and get_ship_plan shows what it would write.",
            "run_pipeline",
        ),
    ),
    (
        "is already on its way to",
        agent(
            "Wait for that send: get_provider_job with project and the job id shows where it is.",
            "get_provider_job",
        ),
    ),
    (
        "is not a game project",
        agent(
            "The tool needs a game project. list_projects shows each project's target; create_project with target game makes one.",
            "list_projects",
        ),
    ),
    (
        "There is no send ",
        agent(
            "get_provider_job with project and no id lists that project's sends and their ids.",
            "get_provider_job",
        ),
    ),
    (
        "is already a project",
        agent(
            "Use that project, or call create_project with another name.",
            "list_projects",
        ),
    ),
];

/// How a missing project reads, from `CoreError::NotFound`; a repo or file
/// whose path holds the word "project" must not match.
const NO_SUCH_PROJECT: &str = "Could not find the project ";

/// Advice by code, when the message itself names nothing.
fn by_code(code: &str, message: &str) -> Option<Advice> {
    match code {
        "not-found" if message.starts_with(NO_SUCH_PROJECT) => Some(agent(
            "Check the project id; list_projects gives every id.",
            "list_projects",
        )),
        "moved" => Some(agent(
            "Read the recipe again with get_recipe, keep what changed there, and send your change with its version as expected.",
            "get_recipe",
        )),
        super::recipe_bake::OPEN_BLANKS => Some(agent(
            "Fill the open blanks with update_recipe (a prop's asset id or a point), then bake again.",
            "update_recipe",
        )),
        super::sending::SEVERAL_PROJECTS => Some(agent(
            "Call again with project set to the project the send is in: every send answer carries it, and get_provider_job with project and no id lists that project's sends.",
            "get_provider_job",
        )),
        super::sending::BY_HAND_WAITING => Some(agent(
            "Poll the waiting send with get_provider_job (the message names it) and follow its next. To send with other settings, stop it with cancel_provider_jobs first.",
            "get_provider_job",
        )),
        super::inbox::OTHER_PROJECT => Some(agent(
            "Leave project out to accept the model into the project it was sent for (list_inbox shows sentFor and aimedAt), or give asset to file it in this project on purpose.",
            "list_inbox",
        )),
        super::handoffs::STAGED_BAKE => Some(agent(
            "Call add_recipe_bake: a person reviews the bake in the Motion view and adds it as a version there, which keeps whether it counts as Mixamo's.",
            "add_recipe_bake",
        )),
        super::recipe_bake::STALE_BAKE => Some(agent(
            "Bake the recipe again with bake_recipe, then ask to add the new bake.",
            "bake_recipe",
        )),
        super::animations::BAKES_UNREADABLE => Some(person(
            "A person fixes or removes the project's recipe-bakes.json (the message names it). No tool can. Until then no clip of that model is saved.",
            None,
        )),
        crate::commands::recipes::TOO_BIG => Some(agent(
            "Take out some pose keys, grips or changes and send the change again.",
            "update_recipe",
        )),
        crate::commands::recipes::CLIP_IN_USE => Some(agent(
            "Keep the clip the message names (clips with keep) and the clips a change runs across together in order, or send changes and time without what sits on it, then send the clips again.",
            "get_recipe",
        )),
        super::recipes::NEEDS_VERSION => Some(agent(
            "Call get_recipe, then send the change with its version as expected.",
            "get_recipe",
        )),
        super::recipe_library::LIBRARY_IMPORT => Some(agent(
            "Import the file into a project (project), then call propose_recipe_save: the person saves it to the library from the approval card.",
            "propose_recipe_save",
        )),
        super::groups::GROUP_OFF => Some(person(
            "Ask the person to turn the group on in Settings > Agents in the Maquette app. No tool can. get_started names the groups that are off.",
            Some("get_started"),
        )),
        _ => None,
    }
}

fn advice(err: &ToolError) -> Option<Advice> {
    BY_MESSAGE
        .iter()
        .find(|(part, _)| err.message.contains(part))
        .map(|(_, advice)| *advice)
        .or_else(|| by_code(&err.code, &err.message))
}

/// The error as an agent reads it: `{ error, code, fix?, tryTool?, personOnly? }`.
pub(super) fn body(err: &ToolError) -> Value {
    let mut out = json!({ "error": err.message, "code": err.code });
    if let Some(advice) = advice(err) {
        out["fix"] = json!(advice.fix);
        if let Some(tool) = advice.try_tool {
            out["tryTool"] = json!(tool);
        }
        if advice.person_only {
            out["personOnly"] = json!(true);
        }
    }
    out
}

/// An error body without a `tryTool` that names a tool of a group that is
/// off: that tool would only answer `group-off`. The `fix` says so in its
/// place, since its own text may name the tool too.
pub(super) fn without_off_tool(mut body: Value, off: &[&super::groups::ToolGroup]) -> Value {
    let named = body
        .get("tryTool")
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(tool) = named else { return body };
    let Some(group) = super::groups::of_tool(&tool).filter(|g| off.contains(g)) else {
        return body;
    };
    let note = format!(
        "{tool} is in the {} tool group, which is off in Settings > Agents.",
        group.id
    );
    if let Some(map) = body.as_object_mut() {
        map.remove("tryTool");
        let fix = map
            .get("fix")
            .and_then(Value::as_str)
            .map_or(note.clone(), |f| format!("{f} {note}"));
        map.insert("fix".to_string(), Value::String(fix));
    }
    body
}

#[cfg(test)]
mod tests;
