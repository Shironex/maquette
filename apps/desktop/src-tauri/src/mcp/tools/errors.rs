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
    "A person saves the key in Maquette (Settings > Image generation). No tool can. The free routes: the codex backend (set_image_backend), or a person's own images added as refs.",
    Some("set_image_backend"),
);
const CODEX: Advice = person(
    "A person installs Codex CLI and runs codex login, then checks the tools in Settings > Toolchain. The other routes: the openai-images backend (needs an OpenAI key and a person's approval), or a person's own images added as refs.",
    Some("get_started"),
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

/// Message parts to the advice they call for, first match wins. A test keeps
/// them in step with the messages the core really sends.
const BY_MESSAGE: &[(&str, Advice)] = &[
    ("Tripo API key", NEW_KEY),
    ("Meshy API key", NEW_KEY),
    ("Settings > 3D providers", NEW_KEY),
    ("OpenAI API key", OPENAI_KEY),
    ("Settings > Image generation", OPENAI_KEY),
    ("Codex CLI is not installed", CODEX),
    ("not signed in", CODEX),
    ("Blender is not installed", BLENDER),
    ("Blender was not found", BLENDER),
    ("ffmpeg", FFMPEG),
    ("No workspace is open", NO_WORKSPACE),
    (
        "has not run yet",
        agent(
            "Start a run; buildOnly true stops before Ship and asks no approval.",
            "run_pipeline",
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

/// Advice by code, when the message itself names nothing.
fn by_code(code: &str, message: &str) -> Option<Advice> {
    match code {
        "not-found" if message.contains("project") => Some(agent(
            "Check the project id; list_projects gives every id.",
            "list_projects",
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

/// The message of an error body, for the activity list. Text that is not
/// one stays as it is.
pub(super) fn message_of(text: String) -> String {
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
        .unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(message: &str) -> Value {
        body(&ToolError::from(message))
    }

    #[test]
    fn the_code_of_a_core_error_comes_through() {
        let out = body(&CoreError::NotFound("project nope".into()).into());
        assert_eq!(out["code"], "not-found");
        assert_eq!(out["error"], "Could not find project nope.");
        assert_eq!(out["tryTool"], "list_projects");
        assert!(out.get("personOnly").is_none());
    }

    #[test]
    fn a_plain_refusal_says_refused_and_adds_nothing() {
        let out = err("mesh and rig only go with by-hand.");
        assert_eq!(out["code"], REFUSED);
        assert!(out.get("fix").is_none());
        assert!(out.get("tryTool").is_none());
    }

    #[test]
    fn person_only_gaps_are_named_and_offer_by_hand_as_the_free_route() {
        let key = err("Add a Tripo API key in Settings > 3D providers first.");
        assert_eq!(key["personOnly"], true);
        assert_eq!(key["tryTool"], "send_to_3d");
        assert!(key["fix"].as_str().expect("fix").contains("by-hand"));

        for message in [
            "Codex CLI is not installed or did not pass the tool check. Install it, run codex login, then check the tools in Settings.",
            "Blender is not installed, and a pack needs it.",
            "Blender was not found. Set its path in Settings, Toolchain, then go on.",
            "Save an OpenAI API key first.",
            "No workspace is open yet.",
        ] {
            assert_eq!(err(message)["personOnly"], true, "{message}");
        }
    }

    #[test]
    fn the_activity_list_reads_the_message_of_a_body() {
        let text = err("nope").to_string();
        assert_eq!(message_of(text), "nope");
        assert_eq!(message_of("plain".to_string()), "plain");
    }
}
