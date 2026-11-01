use serde_json::Value;

use super::next::{Facts, ProjectFacts, next_steps};
use super::*;

fn facts(project: Option<ProjectFacts>) -> Facts {
    Facts {
        workspace_open: true,
        blender: true,
        codex: true,
        any_key: false,
        inbox: 0,
        project,
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
            .contains("kind game")
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
    };
    assert!(says_no_key(&keys(Some(false), Some(false))));
    assert!(!says_no_key(&keys(None, Some(false))));
    assert!(!says_no_key(&keys(None, None)));
    assert!(!says_no_key(&keys(Some(true), Some(false))));
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
