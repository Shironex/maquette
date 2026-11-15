use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::mcp::tools::McpTools;

/// Tools that refuse a game project (`commands/ship.rs` `view` and
/// `check_save`, `plan_camera.rs` `website_dir`).
const REFUSE_A_GAME: &[&str] = &[
    "get_size_report",
    "get_ship_settings",
    "set_ship_settings",
    "get_camera_plan",
    "propose_camera_plan",
    "get_apply_preview",
];

/// Tools that refuse a website project (`game::settings_of`).
const REFUSE_A_WEBSITE: &[&str] = &[
    "get_game",
    "set_game_settings",
    "scan_game_models",
    "import_game_models",
];

fn place(target: ProjectTarget) -> Place<'static> {
    Place {
        project: "p",
        target,
    }
}

fn tools(steps: &[Value]) -> Vec<&str> {
    steps.iter().filter_map(|s| s["tool"].as_str()).collect()
}

/// Every step the table gives for a project of `target`, over every outcome.
fn every_step(target: ProjectTarget) -> Vec<Value> {
    let at = place(target);
    let mut out = Vec::new();
    for has_repo in [true, false] {
        for codex in [true, false] {
            for blender in [true, false] {
                out.extend(start_of(at, has_repo, Can { codex, blender }));
            }
        }
    }
    out.push(repo_step(at));
    for status in ["done", "failed", "cancelled", "running"] {
        for (build_only, no_repo, ship_denied) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let end = RunEnd {
                build_only,
                no_repo,
                ship_denied,
            };
            out.extend(after_run(status, end, at).unwrap_or_default());
        }
    }
    out
}

/// One answer per row of [`after_tool`], each as the tool answers it.
fn answers() -> Vec<(&'static str, Value)> {
    let job = |status: &str, provider: &str| json!({ "status": status, "id": "lamp-1", "project": "p", "asset": "lamp", "provider": provider, "inboxFile": "lamp-1.glb" });
    let mut out = vec![
        ("start_concepts", json!({ "status": "started", "jobs": [] })),
        (
            "pick_concepts",
            json!({ "project": "p", "assets": [{ "id": "lamp" }] }),
        ),
        (
            "pick_concepts",
            json!({ "project": "p", "assets": [{ "id": "lamp" }, { "id": "desk" }] }),
        ),
        ("send_to_3d", job("waiting for approval", "tripo")),
        ("send_to_3d", job("waiting", "by-hand")),
        (
            "send_batch_to_3d",
            json!({ "jobs": [job("waiting", "by-hand")] }),
        ),
        (
            "accept_inbox_model",
            json!({ "asset": { "id": "lamp" }, "version": 2 }),
        ),
        (
            "accept_inbox_models",
            json!({ "results": [{ "ok": true, "project": "p", "asset": { "id": "lamp" } }] }),
        ),
        (
            "import_game_models",
            json!({ "project": "p", "imported": ["crate"] }),
        ),
        (
            "import_game_models",
            json!({ "project": "p", "imported": [] }),
        ),
        ("list_inbox", json!({ "items": [{ "id": "lamp-1.glb" }] })),
    ];
    for status in ["inbox", "accepted", "failed", "cancelled"] {
        out.push(("get_provider_job", job(status, "tripo")));
    }
    out
}

#[test]
fn every_tool_a_step_names_exists_and_takes_the_target_it_is_named_for() {
    let known: BTreeSet<String> = McpTools::tool_names().into_iter().collect();
    for target in [ProjectTarget::Website, ProjectTarget::Game] {
        let refused = match target {
            ProjectTarget::Website => REFUSE_A_WEBSITE,
            ProjectTarget::Game => REFUSE_A_GAME,
        };
        for s in every_step(target) {
            let Some(tool) = s["tool"].as_str() else {
                assert_eq!(s["person"], true, "{s}");
                continue;
            };
            assert!(known.contains(tool), "{tool} is not a tool");
            assert!(!refused.contains(&tool), "{tool} refuses a {target:?}");
        }
    }
    for (tool, answer) in answers() {
        let steps = after_tool(tool, &answer, Some("p")).unwrap_or_else(|| panic!("{tool}"));
        assert!(!steps.is_empty(), "{tool}");
        for name in tools(&steps) {
            assert!(known.contains(name), "{name} after {tool} is not a tool");
            // Only a game imports, so only a game is led to its game tools.
            if tool != "import_game_models" {
                assert!(
                    !REFUSE_A_WEBSITE.contains(&name) && !REFUSE_A_GAME.contains(&name),
                    "{name} after {tool} refuses one of the targets"
                );
            }
        }
    }
}

#[test]
fn a_game_starts_from_its_repo_and_never_from_a_concept_round() {
    let game = place(ProjectTarget::Game);
    assert_eq!(
        tools(&start_of(game, true, Can::BOTH)),
        ["scan_game_models", "import_game_models"]
    );
    assert_eq!(
        tools(&start_of(game, false, Can::BOTH)),
        ["set_game_settings", "import_game_models"]
    );
    let website = place(ProjectTarget::Website);
    assert_eq!(
        tools(&start_of(website, false, Can::BOTH)),
        ["start_concepts", "add_built_asset", "set_ship_settings"]
    );
    // Every step names its project.
    for s in start_of(game, true, Can::BOTH) {
        assert_eq!(s["args"]["project"], "p");
    }
}

#[test]
fn a_done_game_run_points_at_its_ship_plan_and_budgets() {
    let game = place(ProjectTarget::Game);
    let done = after_run("done", RunEnd::default(), game).expect("steps");
    assert_eq!(tools(&done), ["get_ship_plan", "get_game"]);
    let no_repo = RunEnd {
        no_repo: true,
        ..RunEnd::default()
    };
    assert_eq!(
        tools(&after_run("done", no_repo, game).expect("steps")),
        ["set_game_settings", "get_game"]
    );
    let website = place(ProjectTarget::Website);
    assert_eq!(
        tools(&after_run("done", no_repo, website).expect("steps")),
        ["set_ship_settings", "get_size_report"]
    );
}

#[test]
fn a_failed_run_points_at_its_log_and_a_running_one_at_nothing() {
    let at = place(ProjectTarget::Website);
    let failed = after_run("failed", RunEnd::default(), at).expect("steps");
    assert_eq!(failed[0]["tool"], "get_run");
    assert_eq!(failed[0]["args"]["project"], "p");
    assert!(after_run("running", RunEnd::default(), at).is_none());
    // A denied Ship stops at a person; a plain cancel has nothing to add.
    let denied = RunEnd {
        ship_denied: true,
        ..RunEnd::default()
    };
    assert_eq!(
        after_run("cancelled", denied, at).expect("steps")[0]["person"],
        true
    );
    assert!(after_run("cancelled", RunEnd::default(), at).is_none());
    let build_only = RunEnd {
        build_only: true,
        ..RunEnd::default()
    };
    assert_eq!(
        after_run("done", build_only, at).expect("steps")[0]["tool"],
        "get_ship_plan"
    );
}

#[test]
fn a_send_is_followed_and_a_by_hand_one_through_its_handoffs() {
    let paid = json!({ "status": "waiting for approval", "id": "lamp-1", "project": "p", "provider": "tripo" });
    let steps = after_tool("send_to_3d", &paid, None).expect("steps");
    assert_eq!(tools(&steps), ["get_provider_job"]);
    assert_eq!(steps[0]["args"], json!({ "project": "p", "id": "lamp-1" }));
    let by_hand =
        json!({ "status": "waiting", "id": "lamp-1", "project": "p", "provider": "by-hand" });
    assert_eq!(
        tools(&after_tool("send_to_3d", &by_hand, None).expect("steps")),
        ["list_handoffs", "get_provider_job"]
    );
    // A send still in flight adds nothing: the answer already says to poll.
    let running = json!({ "status": "running", "id": "lamp-1", "project": "p", "asset": "lamp" });
    assert!(after_tool("get_provider_job", &running, None).is_none());
    // The list of a project's sends is not one send.
    assert!(
        after_tool(
            "get_provider_job",
            &json!({ "project": "p", "jobs": [] }),
            None
        )
        .is_none()
    );
    // An empty inbox or an unknown tool has no row.
    assert!(after_tool("list_inbox", &json!({ "items": [] }), None).is_none());
    assert!(after_tool("get_layout", &json!({}), Some("p")).is_none());
}

#[test]
fn an_answer_that_has_its_own_next_or_failed_keeps_it() {
    use rmcp::model::{CallToolResponse, CallToolResult, ContentBlock};
    let body = |v: &Value| {
        Ok(CallToolResponse::from(CallToolResult::success(vec![
            ContentBlock::text(v.to_string()),
        ])))
    };
    let read = |r: &Result<CallToolResponse, rmcp::ErrorData>| -> Value {
        let Ok(CallToolResponse::Complete(answer)) = r else {
            panic!("complete")
        };
        let ContentBlock::Text(t) = &answer.content[0] else {
            panic!("text")
        };
        serde_json::from_str(&t.text).expect("json")
    };
    let own = json!({ "status": "started", "next": [{ "tool": "get_run" }] });
    assert_eq!(
        read(&add_next("start_concepts", Some("p"), body(&own)))["next"],
        own["next"]
    );
    let bare = json!({ "status": "started" });
    let added = read(&add_next("start_concepts", Some("p"), body(&bare)));
    assert_eq!(added["next"][0]["tool"], "get_concepts");
    assert_eq!(added["next"][0]["args"]["project"], "p");
}

/// A send of `provider` in `state`, as the studio keeps it.
fn send(provider: &str, state: &str, denied: bool) -> maquette_core::providers::ProviderJob {
    let json = json!({
        "id": "lamp-1", "project": "p", "asset": "lamp", "assetName": "Lamp",
        "provider": provider, "service": "tripo", "variant": "prop",
        "images": ["refs/a.png"], "tripo": null, "meshy": null, "state": state,
        "phase": null, "progress": 0.0, "remoteId": null, "credits": null,
        "createdAtMs": 0.0, "startedAtMs": null, "finishedAtMs": null, "message": null,
        "inboxFile": null, "handoff": null, "card": null, "version": null,
        "viaAgent": true, "cancel": "free", "batch": null, "denied": denied
    });
    serde_json::from_value(json).expect("job")
}

/// Tools whose step may leave project out on purpose: accepting from the
/// inbox files the model into the project it was sent for
/// (`inbox::OTHER_PROJECT`), and discarding one needs only its id.
const PROJECT_LEFT_OUT: &[(&str, &str)] = &[
    ("list_inbox", "accept_inbox_model"),
    ("list_inbox", "discard_inbox_model"),
];

#[test]
fn every_step_to_a_tool_that_takes_a_project_carries_it() {
    let takes: BTreeMap<String, (bool, bool)> = McpTools::router()
        .list_all()
        .into_iter()
        .map(|tool| {
            let props = tool.input_schema.get("properties");
            let has = |key: &str| props.and_then(|p| p.get(key)).is_some();
            (tool.name.to_string(), (has("project"), has("asset")))
        })
        .collect();
    let check = |from: &str, steps: &[Value]| {
        for s in steps {
            let Some(tool) = s["tool"].as_str() else {
                continue;
            };
            if PROJECT_LEFT_OUT.contains(&(from, tool)) {
                continue;
            }
            let (project, _) = takes[tool];
            if project {
                assert_eq!(s["args"]["project"], "p", "{tool} after {from}: {s}");
            }
        }
    };
    for target in [ProjectTarget::Website, ProjectTarget::Game] {
        check("the table", &every_step(target));
    }
    for (tool, answer) in answers() {
        check(tool, &after_tool(tool, &answer, Some("p")).expect("steps"));
    }
    let refs = crate::mcp::tools::refs::refs_next("p", "lamp");
    check("add_refs", &refs);
    for (provider, state, denied) in [
        ("by-hand", "waiting", false),
        ("tripo", "failed", false),
        ("tripo", "cancelled", true),
    ] {
        let next = crate::mcp::tools::sending::next::job_next(&send(provider, state, denied))
            .expect("steps");
        check("send_to_3d", &next);
    }
    // The steps on one asset name it too.
    let by_hand = crate::mcp::tools::sending::next::job_next(&send("by-hand", "waiting", false))
        .expect("steps");
    for s in refs.iter().chain(&by_hand) {
        let tool = s["tool"].as_str().expect("tool");
        if takes[tool].1 {
            assert_eq!(s["args"]["asset"], "lamp", "{s}");
        }
    }
}
