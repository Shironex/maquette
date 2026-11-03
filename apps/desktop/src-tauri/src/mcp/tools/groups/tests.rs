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
fn the_avatar_and_rig_group_keeps_the_old_switch_s_tools() {
    let names = tools_of(maquette_core::workspace::AVATAR_RIG_GROUP);
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
