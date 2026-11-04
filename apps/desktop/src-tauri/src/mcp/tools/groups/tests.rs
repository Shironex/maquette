use maquette_core::WorkspaceConfig;

use super::*;

fn config_off(ids: &[&str]) -> WorkspaceConfig {
    let mut config = WorkspaceConfig::default();
    config.agents.rules.tool_groups_off = ids.iter().map(ToString::to_string).collect();
    config
}

#[test]
fn every_tool_belongs_to_exactly_one_known_group() {
    let names = McpTools::tool_names();
    for name in &names {
        assert!(
            of_tool(name).is_some(),
            "{name} has no group in GROUPS: tag its router in tools.rs with a known id"
        );
    }
    let counted: usize = GROUPS.iter().map(|g| tools_of(g.id).len()).sum();
    assert_eq!(
        counted,
        names.len(),
        "a tool is counted twice or not at all"
    );
}

#[test]
fn every_group_has_tools_and_a_unique_id() {
    for (i, group) in GROUPS.iter().enumerate() {
        assert!(!tools_of(group.id).is_empty(), "{} has no tools", group.id);
        assert!(
            GROUPS[..i].iter().all(|g| g.id != group.id),
            "{} twice",
            group.id
        );
        assert!(!group.title.is_empty() && !group.about.is_empty());
    }
}

#[test]
fn the_core_groups_hold_what_an_agent_needs_to_find_its_way() {
    for tool in [
        "get_started",
        "create_project",
        "list_projects",
        "get_project",
    ] {
        assert!(of_tool(tool).is_some_and(|g| g.core), "{tool}");
    }
    let core: Vec<&str> = GROUPS.iter().filter(|g| g.core).map(|g| g.id).collect();
    assert_eq!(core, ["start", "project"]);
}

#[test]
fn the_core_groups_and_unknown_ids_can_never_be_off() {
    let off = off_in(&config_off(&["start", "project", "nope", "avatar-rig"]));
    let ids: Vec<&str> = off.iter().map(|g| g.id).collect();
    assert_eq!(ids, ["avatar-rig"]);
    assert!(off_in(&WorkspaceConfig::default()).is_empty());
}

#[test]
fn the_avatar_and_rig_group_holds_the_avatar_and_rig_tools() {
    let names = tools_of("avatar-rig");
    assert_eq!(names.len(), 7, "{names:?}");
    assert!(names.iter().any(|n| n == "build_avatar"));
    assert!(names.iter().any(|n| n == "run_rig_doctor"));
}

#[test]
fn summaries_count_every_tool_once() {
    let rows = summaries();
    assert_eq!(rows.len(), GROUPS.len());
    let total: u32 = rows.iter().map(|r| r.tools).sum();
    assert_eq!(total as usize, McpTools::tool_names().len());
    assert!(
        rows.iter()
            .find(|r| r.id == "start")
            .is_some_and(|r| r.always_on)
    );
    assert!(
        rows.iter()
            .find(|r| r.id == "game")
            .is_some_and(|r| !r.always_on)
    );
}

#[test]
fn a_refusal_names_the_group_and_where_a_person_turns_it_on() {
    let group = by_id("game").expect("game");
    let body = super::super::errors::body(&refusal("get_game", group));
    assert_eq!(body["code"], GROUP_OFF);
    let message = body["error"].as_str().expect("message");
    assert!(message.contains("game tool group"), "{message}");
    assert!(message.contains("Settings > Agents"), "{message}");
    assert!(message.contains("Nothing ran"), "{message}");
    assert_eq!(body["personOnly"], true);
    assert_eq!(body["tryTool"], "get_started");
}

#[test]
fn the_off_line_is_short_and_only_there_when_a_group_is_off() {
    assert_eq!(off_line(&[]), None);
    let off = off_in(&config_off(&["game", "avatar-rig"]));
    let line = off_line(&off).expect("line");
    assert!(
        line.contains("game (") && line.contains("avatar-rig (7)"),
        "{line}"
    );
    assert!(line.len() < 200, "{line}");
}

fn answer(body: &serde_json::Value, is_error: bool) -> rmcp::model::CallToolResponse {
    let content = vec![rmcp::model::ContentBlock::text(
        serde_json::to_string_pretty(body).expect("text"),
    )];
    if is_error {
        rmcp::model::CallToolResult::error(content).into()
    } else {
        rmcp::model::CallToolResult::success(content).into()
    }
}

fn body_of(response: rmcp::model::CallToolResponse) -> serde_json::Value {
    let rmcp::model::CallToolResponse::Complete(result) = response else {
        panic!("a complete answer");
    };
    let Some(rmcp::model::ContentBlock::Text(t)) = result.content.first() else {
        panic!("a text block");
    };
    serde_json::from_str(&t.text).expect("json")
}

#[test]
fn a_success_answer_s_next_leaves_out_the_tools_of_a_group_that_is_off() {
    let body = serde_json::json!({
        "project": "moon-island",
        "next": [
            { "tool": "start_concepts", "why": "Make concept images." },
            { "tool": "add_built_asset", "why": "Or a Blender script." },
            { "person": true, "why": "Save a key in Settings." },
        ],
    });
    let off = off_in(&config_off(&["lab"]));
    let scrubbed = body_of(scrub_off_tools(Ok(answer(&body, false)), &off).expect("ok"));
    let tools: Vec<&str> = scrubbed["next"]
        .as_array()
        .expect("next")
        .iter()
        .filter_map(|s| s["tool"].as_str())
        .collect();
    assert_eq!(tools, ["add_built_asset"]);
    assert_eq!(scrubbed["next"].as_array().map(Vec::len), Some(2));
    assert_eq!(scrubbed["project"], "moon-island");

    // Every group on, or no step in an off group: the answer is untouched.
    let all_on = body_of(scrub_off_tools(Ok(answer(&body, false)), &[]).expect("ok"));
    assert_eq!(all_on, body);
    let other = off_in(&config_off(&["game"]));
    let kept = body_of(scrub_off_tools(Ok(answer(&body, false)), &other).expect("ok"));
    assert_eq!(kept, body);
}
