//! The rules agents work under, and the MCP tool groups they are not
//! offered. Keys this version does not know, such as the single avatar and
//! rig switch from before the groups, are ignored on read and dropped on
//! the next save.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
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

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn read(extra: Value) -> AgentRules {
        let mut rules = json!({
            "blenderHeadlessOnly": true,
            "neverCaptureScreen": true,
            "askBeforeShipping": true,
        });
        if let (Some(rules), Value::Object(extra)) = (rules.as_object_mut(), extra) {
            rules.extend(extra);
        }
        serde_json::from_value(rules).expect("rules")
    }

    #[test]
    fn the_old_avatar_switch_is_ignored() {
        for switch in [false, true] {
            let rules = read(json!({ "avatarRigTools": switch }));
            assert!(rules.tool_groups_off.is_empty(), "{switch}");
        }
        let both = read(json!({ "avatarRigTools": false, "toolGroupsOff": ["game"] }));
        assert_eq!(both.tool_groups_off, vec!["game"]);
        assert!(!both.is_off("avatar-rig"));
    }

    #[test]
    fn the_list_round_trips_without_the_old_switch() {
        let rules = AgentRules {
            tool_groups_off: vec!["game".to_string(), "avatar-rig".to_string()],
            ..AgentRules::default()
        };
        let saved = serde_json::to_value(&rules).expect("value");
        assert!(saved.get("avatarRigTools").is_none());
        let back: AgentRules = serde_json::from_value(saved).expect("rules");
        assert_eq!(back, rules);
        assert!(back.is_off("avatar-rig"));
    }
}
