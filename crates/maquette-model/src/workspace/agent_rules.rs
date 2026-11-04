//! The rules agents work under, and the MCP tool groups they are not
//! offered. Files from before the groups carried one `avatarRigTools`
//! switch. Reading a file folds a saved `false` into its group; nothing
//! writes the switch, so the next save drops it.

use serde::{Deserialize, Serialize};
use serde_json::Value;

const AVATAR_RIG_GROUP: &str = "avatar-rig";
const SWITCH: &str = "avatarRigTools";
const LIST: &str = "toolGroupsOff";

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

/// Reading: a file with the list uses the list. A file with only the switch
/// set to false gets the list the switch stood for. The switch is removed
/// either way.
pub(super) fn fold_legacy_switch(config: &mut Value) {
    let Some(rules) = config
        .get_mut("agents")
        .and_then(|a| a.get_mut("rules"))
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    let switch = rules.remove(SWITCH);
    if !rules.contains_key(LIST) && switch == Some(Value::Bool(false)) {
        rules.insert(LIST.to_string(), Value::from(vec![AVATAR_RIG_GROUP]));
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
        let mut config = json!({ "agents": { "rules": rules } });
        fold_legacy_switch(&mut config);
        serde_json::from_value(config["agents"]["rules"].clone()).expect("rules")
    }

    #[test]
    fn the_old_switch_off_without_a_list_turns_the_avatar_rig_group_off() {
        let rules = read(json!({ "avatarRigTools": false }));
        assert_eq!(rules.tool_groups_off, vec!["avatar-rig"]);
        assert!(rules.is_off("avatar-rig"));
    }

    #[test]
    fn the_old_switch_off_keeps_an_existing_list_unchanged() {
        let both = read(json!({ "avatarRigTools": false, "toolGroupsOff": ["game"] }));
        assert_eq!(both.tool_groups_off, vec!["game"]);
        assert!(!both.is_off("avatar-rig"));
        let empty = read(json!({ "avatarRigTools": false, "toolGroupsOff": [] }));
        assert!(empty.tool_groups_off.is_empty());
    }

    #[test]
    fn the_old_switch_on_or_missing_changes_nothing() {
        for extra in [json!({ "avatarRigTools": true }), json!({})] {
            assert!(read(extra).tool_groups_off.is_empty());
        }
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
