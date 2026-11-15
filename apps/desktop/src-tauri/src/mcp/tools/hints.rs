//! What an answer adds so an agent is never left at a dead end: `next` steps
//! on empty answers, and how to wait on the answers that wait for a person.
//! The steps of the main path come from one table, [`table`].

use maquette_core::pipeline::history::now_ms;
use serde_json::{Value, json};

use super::WAITING;

/// The status of a By hand send while it waits for a person to make and
/// bring back its model. Nothing is spent, but it waits like an approval.
pub(super) const WAITING_FOR_MODEL: &str = "waiting for model";

pub(super) mod table;

/// Seconds between polls of something a person has to answer, while the
/// wait is new.
pub(super) const POLL_EVERY_SECONDS: u32 = 5;

/// How long an agent polls one wait before it stops and tells the person.
pub(super) const STOP_POLLING_AFTER_SECONDS: u32 = 600;

const APPROVAL_RULE: &str = "Poll every pollEverySeconds. After stopPollingAfterSeconds of waiting (waitingSinceMs), tell the person where to approve and stop polling: nothing is lost and the wait stays open; poll the same tool again later to pick it up. Never start the work again while it waits.";
const MODEL_RULE: &str = "Poll every pollEverySeconds. After stopPollingAfterSeconds of waiting (waitingSinceMs), tell the person which model to bring back and where, and stop polling: the send stays open until a model comes in or someone stops it; follow next to pick it up later. Never send the asset again while it waits.";
const BRING_BACK: &str = "The person makes the model on the provider's site with the settings card in the handoff folder; Maquette picks up the export from Downloads, or they drop the GLB on the asset's card. No tool makes it.";

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

/// Seconds between polls after waiting `waited` seconds: often at first,
/// then less, so a long wait does not cost a call every few seconds.
pub(super) fn poll_every(waited: f64) -> u32 {
    if waited < 60.0 {
        POLL_EVERY_SECONDS
    } else if waited < 300.0 {
        15
    } else {
        60
    }
}

/// A waiting answer says how often to poll, when to stop, and where a
/// person approves or brings the model back. `waitingSinceMs`, when the
/// answer has it, sets the pace.
pub(super) fn add_waiting_fields(value: &mut Value) {
    let status = value.get("status").and_then(Value::as_str);
    let by_hand = status == Some(WAITING_FOR_MODEL);
    if status != Some(WAITING) && !by_hand {
        return;
    }
    let waited = value
        .get("waitingSinceMs")
        .and_then(Value::as_f64)
        .map_or(0.0, |since| ((now_ms() - since) / 1000.0).max(0.0));
    value["pollEverySeconds"] = json!(poll_every(waited));
    value["stopPollingAfterSeconds"] = json!(STOP_POLLING_AFTER_SECONDS);
    value["stopPolling"] = json!(waited >= f64::from(STOP_POLLING_AFTER_SECONDS));
    value["waitRule"] = json!(if by_hand { MODEL_RULE } else { APPROVAL_RULE });
    if by_hand {
        if !value.get("whereToFinish").is_some_and(Value::is_string) {
            value["whereToFinish"] = json!(BRING_BACK);
        }
        return;
    }
    // Only the avatar's approvals name a step and read get_avatar.
    let avatar = value.get("step").is_some();
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

    #[test]
    fn a_long_wait_polls_less_and_then_says_to_stop() {
        assert_eq!(poll_every(0.0), POLL_EVERY_SECONDS);
        assert_eq!(poll_every(120.0), 15);
        assert_eq!(poll_every(900.0), 60);

        let mut fresh = json!({ "status": WAITING, "waitingSinceMs": now_ms() });
        add_waiting_fields(&mut fresh);
        assert_eq!(fresh["pollEverySeconds"], POLL_EVERY_SECONDS);
        assert_eq!(fresh["stopPolling"], false);
        assert_eq!(fresh["stopPollingAfterSeconds"], STOP_POLLING_AFTER_SECONDS);
        assert!(
            fresh["waitRule"]
                .as_str()
                .expect("rule")
                .contains("stop polling")
        );

        let mut old = json!({ "status": WAITING, "waitingSinceMs": now_ms() - 700_000.0 });
        add_waiting_fields(&mut old);
        assert_eq!(old["pollEverySeconds"], 60);
        assert_eq!(old["stopPolling"], true);
    }

    #[test]
    fn a_by_hand_wait_gets_the_pace_and_where_the_model_comes_from() {
        let mut wait = json!({ "status": WAITING_FOR_MODEL, "waitingSinceMs": now_ms() });
        add_waiting_fields(&mut wait);
        assert_eq!(wait["pollEverySeconds"], POLL_EVERY_SECONDS);
        assert!(wait.get("whereToApprove").is_none(), "nothing to approve");
        assert!(
            wait["whereToFinish"]
                .as_str()
                .expect("place")
                .contains("handoff folder")
        );
    }
}
