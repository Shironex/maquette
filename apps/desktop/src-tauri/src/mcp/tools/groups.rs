//! The tool groups. Every tool belongs to exactly one, named by the bracket
//! tag its description starts with. Settings > Agents turns groups off so
//! agents get a shorter tool list; the start and project groups stay on,
//! because an agent cannot find its way without them.

use std::collections::HashMap;
use std::sync::LazyLock;

use maquette_core::WorkspaceConfig;
use serde::Serialize;

use super::McpTools;

/// One group, as Settings > Agents lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolGroup {
    /// The tag on its tools' descriptions, and the id the setting keeps.
    pub id: &'static str,
    pub title: &'static str,
    pub about: &'static str,
    /// Always on: the setting cannot turn it off.
    pub core: bool,
}

const fn group(id: &'static str, title: &'static str, about: &'static str) -> ToolGroup {
    ToolGroup {
        id,
        title,
        about,
        core: false,
    }
}

const fn core(id: &'static str, title: &'static str, about: &'static str) -> ToolGroup {
    ToolGroup {
        id,
        title,
        about,
        core: true,
    }
}

/// Every group, in the order the router builds them. Ids never change: the
/// setting keeps them.
pub const GROUPS: &[ToolGroup] = &[
    core(
        "start",
        "Start",
        "get_started and create_project: where things stand and a first project",
    ),
    core(
        "project",
        "Projects",
        "Projects, assets, the scene layout, copying assets and the asset trash",
    ),
    group(
        "lab",
        "Lab",
        "Concept images and their jobs, the style board, picks, image backends and Codex usage",
    ),
    group(
        "refs",
        "Refs",
        "An asset's reference views: tagging, drawing missing ones, cleaning backgrounds",
    ),
    group(
        "plan",
        "Planning canvases",
        "Canvases and their nodes, the change log and a website's camera plan",
    ),
    group(
        "send",
        "Send to 3D",
        "Tripo, Meshy and by hand sends, redos, splits, credits, handoffs and downloads",
    ),
    group(
        "inbox",
        "Inbox",
        "Models waiting to be accepted: checks, accept, discard and restore",
    ),
    group(
        "prepare",
        "Prepare",
        "Real size, fix-ups, parts, materials, assembling props and comparing versions",
    ),
    group(
        "built",
        "Built models",
        "Models a Blender script in the project makes, with no credits",
    ),
    group(
        "pipeline",
        "Pipeline and ship",
        "Runs, presets, bake settings, starter scripts, shipping and the site's dev server",
    ),
    group(
        "game",
        "Game projects",
        "A game project's repo, format, budgets, imports and ship plan",
    ),
    group(
        "library",
        "Library",
        "Models kept outside any project, copied in and out",
    ),
    group(
        "animation",
        "Animation",
        "The clip library and baking clips onto a rigged model",
    ),
    group(
        "export",
        "Export packs",
        "Export packs for other engines and render styles",
    ),
    group(
        "avatar-rig",
        "Avatar and rig",
        "VRM avatars and the rig doctor",
    ),
];

/// The group with this id.
pub fn by_id(id: &str) -> Option<&'static ToolGroup> {
    GROUPS.iter().find(|g| g.id == id)
}

/// Each tool's group id, read once from the tags on the descriptions.
static GROUP_OF: LazyLock<HashMap<String, &'static str>> = LazyLock::new(|| {
    McpTools::descriptions()
        .into_iter()
        .filter_map(|(name, description)| {
            let tag = description.strip_prefix('[')?.split_once("] ")?.0;
            Some((name, by_id(tag)?.id))
        })
        .collect()
});

/// The group a tool belongs to; None for a name no tool has.
pub fn of_tool(name: &str) -> Option<&'static ToolGroup> {
    GROUP_OF.get(name).and_then(|id| by_id(id))
}

/// The names of a group's tools, sorted.
pub fn tools_of(id: &str) -> Vec<String> {
    let mut names: Vec<String> = GROUP_OF
        .iter()
        .filter(|(_, group)| **group == id)
        .map(|(name, _)| name.clone())
        .collect();
    names.sort();
    names
}

/// The groups a workspace turned off. Unknown ids and the core groups are
/// left out, so a hand edited file cannot hide `get_started`.
pub fn off_in(config: &WorkspaceConfig) -> Vec<&'static ToolGroup> {
    let off = &config.agents.rules.tool_groups_off;
    GROUPS
        .iter()
        .filter(|g| !g.core && off.iter().any(|id| id == g.id))
        .collect()
}

/// A group as Settings > Agents shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct McpToolGroup {
    pub id: String,
    pub title: String,
    pub about: String,
    /// How many tools it holds.
    pub tools: u32,
    /// The setting cannot turn it off.
    pub always_on: bool,
}

/// Every group with its tool count, in order.
pub fn summaries() -> Vec<McpToolGroup> {
    GROUPS
        .iter()
        .map(|g| McpToolGroup {
            id: g.id.to_string(),
            title: g.title.to_string(),
            about: g.about.to_string(),
            tools: u32::try_from(tools_of(g.id).len()).unwrap_or(u32::MAX),
            always_on: g.core,
        })
        .collect()
}

/// The code a tool of a turned off group answers with.
pub const GROUP_OFF: &str = "group-off";

/// Why a tool of a turned off group did not run.
pub fn refusal(tool: &str, group: &ToolGroup) -> super::ToolError {
    super::ToolError {
        message: format!(
            "{tool} is in the {} tool group ({}), which the person turned off in Settings > Agents. Nothing ran.",
            group.id, group.title
        ),
        code: GROUP_OFF.to_string(),
    }
}

/// get_started's line about the groups that are off, or None when all are on.
pub fn off_line(off: &[&ToolGroup]) -> Option<String> {
    if off.is_empty() {
        return None;
    }
    let names: Vec<String> = off
        .iter()
        .map(|g| format!("{} ({})", g.id, tools_of(g.id).len()))
        .collect();
    Some(format!(
        "Off in Settings > Agents: {}. Their tools are not listed; ask the person to turn a group on when you need it.",
        names.join(", ")
    ))
}

/// The groups Settings > Agents turned off now; none without a workspace.
pub(super) fn off_now(host: &dyn super::Host) -> Vec<&'static ToolGroup> {
    host.session()
        .workspace()
        .map(|(_, config)| off_in(&config))
        .unwrap_or_default()
}

/// The tools whose group is on.
pub(super) fn listed(
    tools: Vec<rmcp::model::Tool>,
    off: &[&'static ToolGroup],
) -> Vec<rmcp::model::Tool> {
    tools
        .into_iter()
        .filter(|t| of_tool(&t.name).is_none_or(|g| !off.contains(&g)))
        .collect()
}

/// The group of `tool` when it is turned off now.
pub(super) fn off_group_of(tool: &str, host: &dyn super::Host) -> Option<&'static ToolGroup> {
    of_tool(tool).filter(|g| off_now(host).contains(g))
}

/// get_started's answer with `toolGroupsOff`, the line about the groups
/// that are off, when any are.
pub(super) fn with_off_line(
    mut answer: serde_json::Value,
    host: &dyn super::Host,
) -> serde_json::Value {
    let off = off_now(host);
    if let (Some(line), Some(map)) = (off_line(&off), answer.as_object_mut()) {
        map.insert("toolGroupsOff".to_string(), serde_json::Value::String(line));
    }
    answer
}

/// A tools/list answer, with the cache hints rmcp's own list gives.
pub(super) fn list_result(
    tools: Vec<rmcp::model::Tool>,
    context: &rmcp::service::RequestContext<rmcp::RoleServer>,
) -> rmcp::model::ListToolsResult {
    use rmcp::model::{CacheScope, ListToolsResult, ProtocolVersion, ResultType};
    let hints = context
        .protocol_version()
        .is_some_and(|version| version >= ProtocolVersion::V_2026_07_28);
    // The list follows the setting, so a client should not keep it.
    ListToolsResult {
        result_type: Some(ResultType::COMPLETE),
        tools,
        ttl_ms: hints.then_some(0),
        cache_scope: hints.then_some(CacheScope::Private),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests;
