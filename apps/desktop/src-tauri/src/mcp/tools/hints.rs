//! What an answer adds so an agent is never left at a dead end: `next` steps
//! on empty answers, and how to wait on the answers that wait for a person.
//! The steps of the main path come from one table, [`table`].

use serde_json::{Value, json};

use super::WAITING;

pub(super) mod table;

/// Seconds between polls of something a person has to answer.
pub(super) const POLL_EVERY_SECONDS: u32 = 5;

pub(super) const APPROVE_IN_APP: &str = "The approval prompt in the Maquette app, also listed under Settings > Agents. No tool can approve.";
const APPROVE_IN_AVATAR: &str = "The Avatar view in the Maquette app, also listed under Settings > Agents. No tool can approve.";

/// One step of a `next` list.
pub(super) fn step(tool: &str, why: &str) -> Value {
    json!({ "tool": tool, "why": why })
}

/// One step only a person can take.
pub(super) fn person_step(why: &str) -> Value {
    json!({ "person": true, "why": why })
}

/// Adds a `next` list to an answer.
pub(super) fn with_next(mut value: Value, next: Vec<Value>) -> Value {
    value["next"] = Value::Array(next);
    value
}

/// A waiting answer says how often to poll and where a person approves.
pub(super) fn add_waiting_fields(value: &mut Value) {
    if value.get("status").and_then(Value::as_str) != Some(WAITING) {
        return;
    }
    // Only the avatar's approvals name a step and read get_avatar.
    let avatar = value.get("step").is_some();
    value["pollEverySeconds"] = json!(POLL_EVERY_SECONDS);
    // A tool that knows a closer place (a review in a view) names it.
    if value.get("whereToApprove").is_some_and(Value::is_string) {
        return;
    }
    value["whereToApprove"] = json!(if avatar {
        APPROVE_IN_AVATAR
    } else {
        APPROVE_IN_APP
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_waiting_answer_gets_the_poll_and_the_place() {
        let mut waiting = json!({ "status": WAITING });
        add_waiting_fields(&mut waiting);
        assert_eq!(waiting["pollEverySeconds"], POLL_EVERY_SECONDS);
        assert!(
            waiting["whereToApprove"]
                .as_str()
                .expect("place")
                .contains("approval prompt")
        );

        let mut avatar = json!({ "status": WAITING, "step": "rest pose" });
        add_waiting_fields(&mut avatar);
        assert!(
            avatar["whereToApprove"]
                .as_str()
                .expect("place")
                .contains("Avatar view")
        );

        let mut review = json!({ "status": WAITING, "whereToApprove": "The Motion view" });
        add_waiting_fields(&mut review);
        assert_eq!(
            review["whereToApprove"], "The Motion view",
            "a closer place stays"
        );
        assert_eq!(review["pollEverySeconds"], POLL_EVERY_SECONDS);

        let mut done = json!({ "status": "done" });
        add_waiting_fields(&mut done);
        assert!(done.get("pollEverySeconds").is_none());
    }
}
