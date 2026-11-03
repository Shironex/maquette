//! The rules agents work under, and the MCP tool groups they are not
//! offered. Files from before the groups carried one switch for the avatar
//! and rig tools. Reading a file turns that switch into its group, and for
//! one release a save still writes the switch beside the list, so a copy of
//! the workspace synced to a computer that runs the version before the
//! groups keeps its meaning.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The group the old `avatarRigTools` switch stood for.
pub const AVATAR_RIG_GROUP: &str = "avatar-rig";

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

const SWITCH: &str = "avatarRigTools";
const LIST: &str = "toolGroupsOff";

fn rules_of(config: &mut Value) -> Option<&mut serde_json::Map<String, Value>> {
    config.get_mut("agents")?.get_mut("rules")?.as_object_mut()
}

/// Reading: a file with the list uses the list and ignores the switch. A
/// file with only the switch (from before the groups) gets the list the
/// switch stood for: false means the avatar and rig group is off.
pub(super) fn read_legacy_switch(config: &mut Value) {
    let Some(rules) = rules_of(config) else {
        return;
    };
    let switch = rules.remove(SWITCH);
    if !rules.contains_key(LIST) && switch == Some(Value::Bool(false)) {
        rules.insert(LIST.to_string(), Value::from(vec![AVATAR_RIG_GROUP]));
    }
}

/// Writing: the switch beside the list, false when the group is off.
pub(super) fn write_legacy_switch(config: &mut Value, rules: &AgentRules) {
    if let Some(map) = rules_of(config) {
        map.insert(
            SWITCH.to_string(),
            Value::Bool(!rules.is_off(AVATAR_RIG_GROUP)),
        );
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn file(rules: Value) -> Value {
        let mut rules = rules;
        for key in [
            "blenderHeadlessOnly",
            "neverCaptureScreen",
            "askBeforeShipping",
        ] {
            rules[key] = json!(true);
        }
        json!({ "agents": { "rules": rules } })
    }

    fn read(rules: Value) -> AgentRules {
        let mut config = file(rules);
        read_legacy_switch(&mut config);
        serde_json::from_value(config["agents"]["rules"].clone()).expect("rules")
    }

    #[test]
    fn the_old_switch_off_turns_the_avatar_and_rig_group_off() {
        let rules = read(json!({ "avatarRigTools": false }));
        assert_eq!(rules.tool_groups_off, vec![AVATAR_RIG_GROUP]);
        assert!(rules.is_off(AVATAR_RIG_GROUP));
    }

    #[test]
    fn the_old_switch_on_or_missing_offers_every_group() {
        for extra in [json!({ "avatarRigTools": true }), json!({})] {
            assert!(read(extra).tool_groups_off.is_empty());
        }
    }

    #[test]
    fn the_list_wins_when_both_keys_are_present() {
        let both = read(json!({ "avatarRigTools": false, "toolGroupsOff": ["game"] }));
        assert_eq!(both.tool_groups_off, vec!["game"]);
        assert!(!both.is_off(AVATAR_RIG_GROUP));
        let empty = read(json!({ "avatarRigTools": false, "toolGroupsOff": [] }));
        assert!(empty.tool_groups_off.is_empty());
        let on = read(json!({ "avatarRigTools": true, "toolGroupsOff": ["avatar-rig"] }));
        assert_eq!(on.tool_groups_off, vec![AVATAR_RIG_GROUP]);
    }

    #[test]
    fn a_save_writes_the_switch_beside_the_list_and_it_reads_back() {
        for off in [
            vec![],
            vec!["game".to_string()],
            vec!["game".to_string(), "avatar-rig".to_string()],
        ] {
            let rules = AgentRules {
                tool_groups_off: off,
                ..AgentRules::default()
            };
            let mut saved =
                json!({ "agents": { "rules": serde_json::to_value(&rules).expect("value") } });
            assert!(saved["agents"]["rules"].get(SWITCH).is_none());
            write_legacy_switch(&mut saved, &rules);
            assert_eq!(
                saved["agents"]["rules"][SWITCH],
                json!(!rules.is_off(AVATAR_RIG_GROUP))
            );
            read_legacy_switch(&mut saved);
            let back: AgentRules =
                serde_json::from_value(saved["agents"]["rules"].clone()).expect("rules");
            assert_eq!(back, rules);
        }
    }
}
