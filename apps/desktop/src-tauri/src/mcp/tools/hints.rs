//! What an answer adds so an agent is never left at a dead end: `next` steps
//! on empty answers, and how to wait on the answers that wait for a person.

use serde_json::{Value, json};

use super::WAITING;

/// Seconds between polls of something a person has to answer.
pub(super) const POLL_EVERY_SECONDS: u32 = 5;

const APPROVE_IN_APP: &str = "The approval prompt in the Maquette app, also listed under Settings > Agents. No tool can approve.";
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
    value["whereToApprove"] = json!(if avatar {
        APPROVE_IN_AVATAR
    } else {
        APPROVE_IN_APP
    });
}

/// What to do after a run ended.
pub(super) fn after_run(status: &str, build_only: bool) -> Option<Vec<Value>> {
    match status {
        "failed" => Some(vec![
            step(
                "get_run",
                "Read the log with a larger logLines to see why the step failed.",
            ),
            step(
                "run_pipeline",
                "Run again with from set to the failed step once it is fixed.",
            ),
        ]),
        "done" if build_only => Some(vec![
            step("get_ship_plan", "See what Ship would write."),
            step(
                "run_pipeline",
                "Run without buildOnly to ship; a person may have to approve.",
            ),
        ]),
        "done" => Some(vec![step(
            "get_size_report",
            "Compare what came in with what shipped.",
        )]),
        _ => None,
    }
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

        let mut done = json!({ "status": "done" });
        add_waiting_fields(&mut done);
        assert!(done.get("pollEverySeconds").is_none());
    }

    #[test]
    fn a_failed_run_points_at_its_log_and_a_running_one_at_nothing() {
        let failed = after_run("failed", false).expect("steps");
        assert_eq!(failed[0]["tool"], "get_run");
        assert!(after_run("running", false).is_none());
        assert_eq!(
            after_run("done", true).expect("steps")[0]["tool"],
            "get_ship_plan"
        );
    }
}
