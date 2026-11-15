use super::*;

/// The longest a tool's description may grow. Detail goes on the arguments.
const MAX_DESCRIPTION: usize = 1250;

#[test]
fn every_tool_has_a_group_and_a_description_of_reasonable_length() {
    for (name, description) in McpTools::descriptions() {
        assert!(description.starts_with('['), "{name} has no group");
        let (tag, rest) = description.split_once("] ").expect("group");
        assert!(!tag.is_empty() && !rest.is_empty(), "{name}");
        assert!(
            description.len() <= MAX_DESCRIPTION,
            "{name} has {} characters",
            description.len()
        );
    }
}
