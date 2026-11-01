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
        let out = body(&CoreError::NotFound("the project nope".into()).into());
        assert_eq!(out["code"], "not-found");
        assert_eq!(out["error"], "Could not find the project nope.");
        assert_eq!(out["tryTool"], "list_projects");
        assert!(out.get("personOnly").is_none());
        let repo = body(&CoreError::NotFound("the site repo /code/project-x".into()).into());
        assert!(repo.get("tryTool").is_none(), "{repo}");
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
    fn the_table_matches_what_the_sources_really_say() {
        // Up from src/mcp/tools to the repo root.
        macro_rules! source {
            ($path:literal) => {
                include_str!(concat!("../../../../../../", $path))
            };
        }
        let sent: &[(&str, &str)] = &[
            (
                "would pass the monthly OpenAI image cap",
                source!("crates/maquette-core/src/concepts/jobs/api.rs"),
            ),
            (
                "cap can only be changed by a person",
                include_str!("image_api.rs"),
            ),
            (
                "API key in Settings > 3D providers first",
                source!("crates/maquette-providers/src/jobs/mod.rs"),
            ),
            (
                "did not accept the API key. Check it in Settings > 3D providers",
                source!("crates/maquette-providers/src/http.rs"),
            ),
            (
                "did not accept the API key. Check it in Settings > Image generation",
                source!("crates/maquette-providers/src/openai_images.rs"),
            ),
            (
                "Save an OpenAI API key first",
                include_str!("../../commands/image_api.rs"),
            ),
            (
                "Codex CLI is not installed",
                source!("crates/maquette-core/src/concepts/jobs.rs"),
            ),
            (
                "Codex CLI is not signed in",
                source!("crates/maquette-base/src/toolchain/detection.rs"),
            ),
            (
                "Blender was not found",
                source!("crates/maquette-core/src/renders/job.rs"),
            ),
            (
                "needs ffmpeg, and it was not found",
                source!("crates/maquette-core/src/renders/run.rs"),
            ),
            (
                "ffmpeg is not installed",
                source!("crates/maquette-core/src/concepts/sheet.rs"),
            ),
            (
                "No workspace is open",
                source!("apps/desktop/src-tauri/src/session.rs"),
            ),
            (
                "Blender is not installed",
                source!("crates/maquette-core/src/thumbs/picture.rs"),
            ),
            ("has not run yet", include_str!("runs.rs")),
            ("is already a project", include_str!("start.rs")),
        ];
        for (phrase, source) in sent {
            assert!(source.contains(phrase), "no source says {phrase:?}");
            assert!(BY_MESSAGE.iter().any(|(p, _)| p == phrase), "{phrase:?}");
        }
        // These two take the provider's name.
        let providers = include_str!("../../commands/providers.rs");
        assert!(providers.contains("Save a {} API key first"));
        let named = ["Save a Tripo API key first", "Save a Meshy API key first"];
        // Every row is pinned: a phrase no source says never matches.
        for (phrase, _) in BY_MESSAGE {
            assert!(
                named.contains(phrase) || sent.iter().any(|(p, _)| p == phrase),
                "{phrase:?} is not pinned to a source"
            );
        }
    }

    #[test]
    fn a_word_in_a_name_or_a_path_earns_no_advice() {
        for message in [
            "Could not write /site/ffmpeg-demo/loop.mp4: disk full.",
            "ffmpeg could not write the loop: disk full.",
            "Could not find an asset named ffmpeg.",
            "Settings > 3D providers has no such field.",
            "The asset Tripo API key was renamed.",
        ] {
            let out = err(message);
            assert!(out.get("fix").is_none(), "{message}");
            assert!(out.get("personOnly").is_none(), "{message}");
        }
    }

    #[test]
    fn a_cap_error_says_the_cap_and_an_agent_can_still_go_on() {
        for message in [
            "This round of 4 would pass the monthly OpenAI image cap: $9.00 spent this month.",
            "Not sent: this image (about $0.20) would pass the monthly OpenAI image cap, $9.90 of $10.",
            super::super::image_api::CAP_REFUSED,
        ] {
            let out = err(message);
            assert_eq!(out["tryTool"], "set_image_backend", "{message}");
            assert!(out.get("personOnly").is_none(), "{message}");
            assert!(
                out["fix"].as_str().expect("fix").contains("codex"),
                "{message}"
            );
            assert!(
                !out["fix"].as_str().expect("fix").contains("saves the key"),
                "{message}"
            );
        }
    }

    #[test]
    fn the_activity_list_reads_the_message_of_a_body() {
        let text = err("nope").to_string();
        assert_eq!(message_of(text), "nope");
        assert_eq!(message_of("plain".to_string()), "plain");
    }
}
