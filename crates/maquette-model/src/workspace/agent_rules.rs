//! The rules agents work under, and the MCP tool groups they are not
//! offered. Files from before the groups carried one switch for the avatar
//! and rig tools; reading one turns that switch into its group.

use serde::{Deserialize, Deserializer, Serialize};

/// The group the old `avatarRigTools` switch stood for.
pub const AVATAR_RIG_GROUP: &str = "avatar-rig";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentRules {
    pub blender_headless_only: bool,
    pub never_capture_screen: bool,
    pub ask_before_shipping: bool,
    /// The MCP tool groups agents are not offered, by id. Empty offers every
    /// group; the server ignores ids it does not know and the groups that
    /// are always on.
    #[serde(default)]
    pub tool_groups_off: Vec<String>,
}

impl Default for AgentRules {
    fn default() -> Self {
        Self {
            blender_headless_only: true,
            never_capture_screen: true,
            ask_before_shipping: true,
            tool_groups_off: Vec::new(),
        }
    }
}

impl AgentRules {
    /// Whether the setting turned this group off.
    pub fn is_off(&self, group: &str) -> bool {
        self.tool_groups_off.iter().any(|id| id == group)
    }
}

/// The rules as a file holds them, old or new.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Saved {
    blender_headless_only: bool,
    never_capture_screen: bool,
    ask_before_shipping: bool,
    #[serde(default)]
    tool_groups_off: Vec<String>,
    /// The switch before the groups: false left the avatar and rig tools out.
    #[serde(default)]
    avatar_rig_tools: Option<bool>,
}

impl<'de> Deserialize<'de> for AgentRules {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let saved = Saved::deserialize(deserializer)?;
        let mut off = saved.tool_groups_off;
        if saved.avatar_rig_tools == Some(false) && !off.iter().any(|id| id == AVATAR_RIG_GROUP) {
            off.push(AVATAR_RIG_GROUP.to_string());
        }
        Ok(Self {
            blender_headless_only: saved.blender_headless_only,
            never_capture_screen: saved.never_capture_screen,
            ask_before_shipping: saved.ask_before_shipping,
            tool_groups_off: off,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn read(value: serde_json::Value) -> AgentRules {
        serde_json::from_value(value).expect("rules")
    }

    const SAFETY: [(&str, bool); 3] = [
        ("blenderHeadlessOnly", true),
        ("neverCaptureScreen", true),
        ("askBeforeShipping", true),
    ];

    fn with(extra: serde_json::Value) -> serde_json::Value {
        let mut value = json!({});
        for (key, on) in SAFETY {
            value[key] = json!(on);
        }
        for (key, v) in extra.as_object().expect("object") {
            value[key] = v.clone();
        }
        value
    }

    #[test]
    fn the_old_switch_off_turns_the_avatar_and_rig_group_off() {
        let rules = read(with(json!({ "avatarRigTools": false })));
        assert_eq!(rules.tool_groups_off, vec![AVATAR_RIG_GROUP]);
        assert!(rules.is_off(AVATAR_RIG_GROUP));
    }

    #[test]
    fn the_old_switch_on_or_missing_offers_every_group() {
        for extra in [json!({ "avatarRigTools": true }), json!({})] {
            assert!(read(with(extra)).tool_groups_off.is_empty());
        }
    }

    #[test]
    fn the_new_list_wins_and_the_group_is_named_once() {
        let rules = read(with(json!({
            "avatarRigTools": false,
            "toolGroupsOff": ["game", "avatar-rig"],
        })));
        assert_eq!(rules.tool_groups_off, vec!["game", "avatar-rig"]);
    }

    #[test]
    fn a_save_writes_the_list_and_never_the_old_switch() {
        let migrated = read(with(json!({ "avatarRigTools": false })));
        let saved = serde_json::to_value(&migrated).expect("value");
        assert!(saved.get("avatarRigTools").is_none(), "{saved}");
        assert_eq!(saved["toolGroupsOff"], json!(["avatar-rig"]));
        assert_eq!(read(saved), migrated);
    }
}
