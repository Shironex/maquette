use serde_json::Value;

use super::next::{Facts, ProjectFacts, Sends, next_steps};
use super::*;

fn facts(project: Option<ProjectFacts>) -> Facts {
    Facts {
        workspace_open: true,
        blender: true,
        codex: true,
        any_key: false,
        inbox: 0,
        project,
        off: Vec::new(),
    }
}

fn empty() -> ProjectFacts {
    ProjectFacts {
        id: "p".into(),
        target: ProjectTarget::Website,
        has_repo: true,
        assets: 0,
        at_refs: 0,
        with_model: 0,
        sends: Sends::default(),
    }
}

fn tools(next: &[Value]) -> Vec<&str> {
    next.iter().filter_map(|s| s["tool"].as_str()).collect()
}

#[test]
fn no_workspace_only_asks_a_person() {
    let next = next_steps(&Facts::no_workspace());
    assert_eq!(next.len(), 1);
    assert_eq!(next[0]["person"], true);
}

#[test]
fn no_project_says_create_project() {
    assert_eq!(tools(&next_steps(&facts(None))), ["create_project"]);
}

#[test]
fn an_empty_project_offers_concepts_and_a_built_asset() {
    assert_eq!(
        tools(&next_steps(&facts(Some(empty())))),
        ["start_concepts", "add_built_asset"]
    );
}

#[test]
fn without_codex_a_person_is_named_and_the_built_route_stays() {
    let mut f = facts(Some(empty()));
    f.codex = false;
    let next = next_steps(&f);
    assert_eq!(next[0]["person"], true);
    assert_eq!(tools(&next), ["add_built_asset"]);
}

#[test]
fn refs_without_a_key_offer_by_hand_and_no_repo_says_set_it() {
    let project = ProjectFacts {
        has_repo: false,
        assets: 1,
        at_refs: 1,
        ..empty()
    };
    let next = next_steps(&facts(Some(project)));
    assert_eq!(tools(&next), ["send_to_3d", "set_ship_settings"]);
    assert!(next[0]["why"].as_str().expect("why").contains("by-hand"));
}

#[test]
fn a_game_without_a_repo_points_at_the_game_settings() {
    let project = ProjectFacts {
        target: ProjectTarget::Game,
        has_repo: false,
        assets: 1,
        with_model: 1,
        ..empty()
    };
    assert_eq!(
        tools(&next_steps(&facts(Some(project)))),
        ["run_pipeline", "set_game_settings"]
    );
}

#[test]
fn templates_any_known_id_and_refuses_the_rest() {
    assert_eq!(website_template(None), Ok(WEBSITE_DEFAULT));
    assert_eq!(website_template(Some("Room")), Ok(ProjectTemplate::Room));
    assert!(
        website_template(Some("imported"))
            .expect_err("folder")
            .contains("New project")
    );
    assert!(
        website_template(Some("game-import"))
            .expect_err("game")
            .contains("target game")
    );
    assert!(
        website_template(Some("castle"))
            .expect_err("bad")
            .contains("not a website template")
    );
    assert_eq!(
        game_template(Some("prop-set")),
        Ok(Some(GameTemplateId::PropSet))
    );
    assert!(game_template(Some("room")).is_err());
}

#[test]
fn a_taken_name_is_found_by_name_and_by_id() {
    let rows = [("moon-island", "Moon Island"), ("old", "Renamed later")];
    assert!(taken(" moon island ", rows).is_some());
    assert!(taken("Moon-Island", rows).is_some());
    assert!(taken("renamed later", rows).is_some());
    assert!(taken("Sun Island", rows).is_none());
}

#[test]
fn a_second_name_with_no_latin_letters_is_not_taken() {
    assert_eq!(project::slugify("月の島"), "project");
    assert!(taken("星の島", [("project", "月の島")]).is_none());
    assert!(taken("月の島", [("project", "月の島")]).is_some());
    assert!(taken("Project", [("project", "Project")]).is_some());
}

#[test]
fn a_key_the_store_could_not_read_is_not_called_missing() {
    let says_no_key = |keys: &Keys| {
        gates(&[], keys, true)
            .iter()
            .any(|g| g["gate"].as_str().is_some_and(|w| w.contains("API key")))
    };
    let keys = |tripo, meshy| Keys {
        tripo,
        meshy,
        openai: None,
        prompt: false,
    };
    assert!(says_no_key(&keys(Some(false), Some(false))));
    assert!(!says_no_key(&keys(None, Some(false))));
    assert!(!says_no_key(&keys(None, None)));
    assert!(!says_no_key(&keys(Some(true), Some(false))));
}

#[test]
fn an_unreadable_key_store_is_not_a_saved_key() {
    let keys = |tripo, meshy| Keys {
        tripo,
        meshy,
        openai: None,
        prompt: false,
    };
    assert!(keys(Some(true), None).any_saved());
    assert!(keys(None, Some(true)).any_saved());
    assert!(!keys(None, None).any_saved());
    assert!(!keys(None, Some(false)).any_saved());
    assert!(keys(None, Some(false)).unreadable());
    let all_read = Keys {
        tripo: Some(false),
        meshy: Some(false),
        openai: Some(false),
        prompt: false,
    };
    assert!(!all_read.unreadable());
    assert!(all_read.value().get("note").is_none());
    // A read given up on while a Keychain prompt waits says to answer it.
    let waiting = Keys {
        tripo: None,
        prompt: true,
        ..all_read
    };
    let value = waiting.value();
    assert_eq!(value["unreadable"], true);
    assert_eq!(value["tripo"], serde_json::Value::Null);
    assert!(
        value["note"]
            .as_str()
            .expect("note")
            .contains("Keychain prompt")
    );
}

fn found(id: ToolId, warning: &str) -> ToolReport {
    ToolReport {
        id,
        label: id.label().to_string(),
        required: true,
        status: ToolStatus::Found,
        source: None,
        path: None,
        version: None,
        detail: None,
        hint: None,
        warning: Some(warning.to_string()),
    }
}

#[test]
fn a_found_blender_with_a_warning_works_and_a_codex_not_signed_in_does_not() {
    let keys = Keys {
        tripo: Some(true),
        meshy: Some(true),
        openai: None,
        prompt: false,
    };
    let says = |reports: &[ToolReport], what: &str| {
        gates(reports, &keys, true)
            .iter()
            .any(|g| g["gate"].as_str().is_some_and(|w| w.contains(what)))
    };
    let blender = [found(ToolId::Blender, "The saved path is gone.")];
    assert!(blender_works(&blender));
    assert!(!says(&blender, "locate Blender"));
    assert!(says(&[], "locate Blender"));
    let codex = [found(ToolId::Codex, "Codex CLI is not signed in.")];
    assert!(!tool_works(&codex, ToolId::Codex));
    assert!(says(&codex, "Codex CLI"));
}

#[test]
fn next_leaves_out_the_tools_of_a_group_that_is_off() {
    let project = ProjectFacts {
        assets: 2,
        at_refs: 1,
        with_model: 1,
        ..empty()
    };
    let mut f = facts(Some(project));
    let all = tools(&next_steps(&f))
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    assert!(all.contains(&"send_to_3d".to_string()) && all.contains(&"run_pipeline".to_string()));
    f.off = ["send", "pipeline"]
        .iter()
        .filter_map(|id| super::super::groups::by_id(id))
        .collect();
    let next = next_steps(&f);
    let left = tools(&next);
    assert!(
        !left.contains(&"send_to_3d") && !left.contains(&"run_pipeline"),
        "{left:?}"
    );
}

#[test]
fn create_project_takes_target_and_still_reads_kind() {
    let read = |args: Value| serde_json::from_value::<CreateProjectArgs>(args);
    let by_target = read(json!({ "name": "Crypt", "target": "game" })).expect("target");
    assert_eq!(by_target.target, TargetArg::Game);
    let by_kind = read(json!({ "name": "Web", "kind": "website" })).expect("kind");
    assert_eq!(by_kind.target, TargetArg::Website);
    assert!(
        read(json!({ "name": "Nothing" })).is_err(),
        "one of them is needed"
    );
}

#[test]
fn every_step_of_a_project_names_it() {
    let project = ProjectFacts {
        id: "moon".into(),
        has_repo: false,
        assets: 2,
        at_refs: 1,
        with_model: 1,
        ..empty()
    };
    for s in next_steps(&facts(Some(project))) {
        assert_eq!(s["args"]["project"], "moon", "{s}");
    }
}

#[test]
fn a_send_in_flight_is_followed_and_never_sends_back_to_the_concepts() {
    // The send moved the only asset to stage Tripo: nothing at refs, no model.
    let project = ProjectFacts {
        assets: 1,
        sends: Sends {
            approval: 1,
            ..Sends::default()
        },
        ..empty()
    };
    let next = next_steps(&facts(Some(project)));
    assert_eq!(tools(&next), ["get_provider_job"]);
    assert_eq!(next[0]["args"]["project"], "p");
    let by_hand = ProjectFacts {
        assets: 1,
        sends: Sends {
            by_hand: 1,
            ..Sends::default()
        },
        ..empty()
    };
    assert_eq!(tools(&next_steps(&facts(Some(by_hand)))), ["list_handoffs"]);
    // With nothing in flight the same project is led to its concepts.
    let idle = ProjectFacts {
        assets: 1,
        ..empty()
    };
    assert_eq!(tools(&next_steps(&facts(Some(idle)))), ["get_concepts"]);
}

#[test]
fn an_empty_game_starts_from_its_repo_and_never_from_concepts() {
    let with_repo = ProjectFacts {
        target: ProjectTarget::Game,
        ..empty()
    };
    assert_eq!(
        tools(&next_steps(&facts(Some(with_repo)))),
        ["scan_game_models", "import_game_models"]
    );
    let no_repo = ProjectFacts {
        target: ProjectTarget::Game,
        has_repo: false,
        ..empty()
    };
    // The repo step comes once, first.
    assert_eq!(
        tools(&next_steps(&facts(Some(no_repo)))),
        ["set_game_settings", "import_game_models"]
    );
}

#[test]
fn sends_count_by_state_and_only_by_hand_waits_for_a_person() {
    use maquette_core::providers::{JobState, Provider};
    let sends = Sends::count([
        (JobState::Approval, Provider::Tripo),
        (JobState::Running, Provider::Meshy),
        (JobState::Queued, Provider::Tripo),
        (JobState::Waiting, Provider::ByHand),
        (JobState::Accepted, Provider::Tripo),
        (JobState::Failed, Provider::Tripo),
        (JobState::Inbox, Provider::ByHand),
    ]);
    assert_eq!(
        sends,
        Sends {
            approval: 1,
            running: 2,
            by_hand: 1
        }
    );
    let listed = super::waiting::listed(&[], &[("p", sends), ("q", Sends::default())], 0.0);
    let kinds: Vec<&str> = listed.iter().filter_map(|w| w["kind"].as_str()).collect();
    assert_eq!(
        kinds,
        ["sends", "by-hand"],
        "a project with nothing in flight lists nothing"
    );
    assert_eq!(listed[0]["inFlight"], 2);
    assert_eq!(listed[0]["poll"]["tool"], "get_provider_job");
    assert_eq!(listed[0]["poll"]["args"]["project"], "p");
    assert_eq!(listed[1]["poll"]["tool"], "list_handoffs");
}

#[test]
fn every_waiting_approval_names_the_tool_that_follows_it_and_its_age() {
    use crate::mcp::hub::Approval;
    let approval = |kind: &str, at: f64| -> Approval {
        serde_json::from_value(json!({
            "id": format!("a-{kind}"), "kind": kind, "project": "p", "runId": "p-run-3",
            "job": "lamp-1", "assetName": "Lamp", "items": [], "requestedAtMs": at,
            "asset": "lamp", "recipeSave": { "recipe": "wave", "revision": 1 },
        }))
        .expect(kind)
    };
    let kinds = [
        ("ship", "get_run"),
        ("send-to-3d", "get_provider_job"),
        ("image-round", "get_concepts"),
        ("rest-pose", "get_avatar"),
        ("reweight", "get_avatar"),
        ("springs", "get_avatar"),
        ("rig-doctor", "get_rig_doctor"),
        ("recipe-bake", "get_recipe_bake"),
        ("recipe-save", "get_recipe"),
        ("grip-save", "list_grips"),
        ("page-preview", "get_page_preview"),
    ];
    // Newest first in, oldest first out.
    let approvals: Vec<Approval> = kinds
        .iter()
        .enumerate()
        .map(|(i, (kind, _))| approval(kind, 1000.0 * (20 - i) as f64))
        .collect();
    let listed = super::waiting::listed(&approvals, &[], 30_000.0);
    assert_eq!(listed.len(), kinds.len());
    let known = McpTools::tool_names();
    for (row, (kind, tool)) in listed.iter().rev().zip(kinds) {
        assert_eq!(row["kind"], kind);
        assert_eq!(row["poll"]["tool"], tool, "{kind}");
        assert!(known.contains(&tool.to_string()), "{tool}");
        assert_eq!(row["project"], "p");
        assert_eq!(row["approval"], format!("a-{kind}"));
    }
    assert_eq!(listed[0]["kind"], "page-preview");
    assert_eq!(listed[0]["ageSeconds"], 20);
    let ship = listed.iter().find(|r| r["kind"] == "ship").expect("ship");
    assert_eq!(
        ship["poll"]["args"],
        json!({ "project": "p", "runId": "p-run-3" })
    );
    let send = listed
        .iter()
        .find(|r| r["kind"] == "send-to-3d")
        .expect("send");
    assert_eq!(
        send["poll"]["args"],
        json!({ "project": "p", "id": "lamp-1" })
    );
    let save = listed
        .iter()
        .find(|r| r["kind"] == "recipe-save")
        .expect("save");
    assert_eq!(save["poll"]["args"]["recipe"], "wave");
}
