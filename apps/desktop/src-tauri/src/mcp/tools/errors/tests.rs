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
            include_str!(concat!("../../../../../../../", $path))
        };
    }
    let sent: &[(&str, &str)] = &[
        (
            "would pass the monthly OpenAI image cap",
            source!("crates/maquette-core/src/concepts/jobs/api.rs"),
        ),
        (
            "cap can only be changed by a person",
            include_str!("../image_api.rs"),
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
            include_str!("../../../commands/image_api.rs"),
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
        ("has not run yet", include_str!("../runs.rs")),
        (
            "has no game repo",
            include_str!("../../../commands/game.rs"),
        ),
        (
            "A game project ships through Game export",
            include_str!("../../../commands/ship.rs"),
        ),
        (
            "is already on its way to",
            source!("crates/maquette-providers/src/jobs/send.rs"),
        ),
        (
            "is not a game project",
            source!("crates/maquette-model/src/settings/game/lookup.rs"),
        ),
        ("There is no send ", include_str!("../sending/by_id.rs")),
        ("is already a project", include_str!("../start.rs")),
    ];
    for (phrase, source) in sent {
        assert!(source.contains(phrase), "no source says {phrase:?}");
        assert!(BY_MESSAGE.iter().any(|(p, _)| p == phrase), "{phrase:?}");
    }
    // These two take the provider's name.
    let providers = include_str!("../../../commands/providers.rs");
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
fn the_common_game_and_send_refusals_say_what_to_do() {
    for (message, tool) in [
        (
            "Moon has no game repo. Pick one in Settings.",
            "set_game_settings",
        ),
        ("A game project ships through Game export.", "run_pipeline"),
        (
            "Lamp is already on its way to Tripo (lamp-1).",
            "get_provider_job",
        ),
        ("Site is not a game project.", "list_projects"),
        ("There is no send lantern-1 in moon.", "get_provider_job"),
    ] {
        let out = err(message);
        assert_eq!(out["tryTool"], tool, "{message}");
        assert!(out.get("personOnly").is_none(), "{message}");
        assert!(out["fix"].as_str().is_some(), "{message}");
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
fn a_try_tool_in_a_group_that_is_off_is_dropped() {
    let off: Vec<_> = ["send", "lab"]
        .iter()
        .filter_map(|id| super::super::groups::by_id(id))
        .collect();
    let key = without_off_tool(err("Save a Tripo API key first"), &off);
    assert!(key.get("tryTool").is_none(), "{key}");
    let fix = key["fix"].as_str().expect("fix");
    assert!(
        fix.contains("send_to_3d is in the send tool group"),
        "{fix}"
    );
    // A tool of a group that is on stays.
    let list = without_off_tool(err("No workspace is open"), &off);
    assert_eq!(list["tryTool"], "get_started");
    assert_eq!(without_off_tool(err("plain"), &off), err("plain"));
}

#[test]
fn the_activity_list_reads_the_message_of_a_body() {
    let text = err("nope").to_string();
    assert_eq!(message_of(text), "nope");
    assert_eq!(message_of("plain".to_string()), "plain");
}
