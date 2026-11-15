//! What call_tool does with a tool's answer once it ran: every waiting
//! answer gets its pace from when its wait began, and an error body gives
//! its message to the activity list.

use rmcp::ErrorData;
use rmcp::model::{CallToolResponse, ContentBlock};
use serde_json::Value;

use super::{WAITING, hints};
use crate::mcp::hub::Hub;

/// The approval id a waiting answer names, as an object or a bare id.
fn waiting_on(value: &Value) -> Option<&str> {
    ["/approval/id", "/approval", "/jobs/0/approval/id"]
        .iter()
        .find_map(|at| value.pointer(at).and_then(Value::as_str))
}

/// A tool's answer as it leaves: `next` steps of groups that are off are
/// scrubbed, and waits are paced.
pub(super) fn finish(
    result: Result<CallToolResponse, ErrorData>,
    off: &[&'static super::groups::ToolGroup],
    hub: &Hub,
) -> Result<CallToolResponse, ErrorData> {
    pace_waits(super::groups::scrub_off_tools(result, off), hub)
}

/// Gives a waiting answer `waitingSinceMs` from its approval when the tool
/// did not, and paces its polls by it, so every wait carries the same exit
/// rule.
fn pace_waits(
    mut result: Result<CallToolResponse, ErrorData>,
    hub: &Hub,
) -> Result<CallToolResponse, ErrorData> {
    let Ok(CallToolResponse::Complete(answer)) = &mut result else {
        return result;
    };
    if answer.is_error == Some(true) {
        return result;
    }
    for block in &mut answer.content {
        let ContentBlock::Text(t) = block else {
            continue;
        };
        let Ok(mut body) = serde_json::from_str::<Value>(&t.text) else {
            continue;
        };
        if body.get("status").and_then(Value::as_str) != Some(WAITING)
            || body.get("waitingSinceMs").is_some()
        {
            continue;
        }
        let since = waiting_on(&body).and_then(|id| {
            hub.approvals()
                .into_iter()
                .find(|a| a.id == id)
                .map(|a| a.requested_at_ms)
        });
        if let Some(since) = since {
            body["waitingSinceMs"] = serde_json::json!(since);
            hints::add_waiting_fields(&mut body);
            t.text = serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string());
        }
    }
    result
}

/// The message of an error body, for the activity list. Text that is not
/// one stays as it is.
pub(super) fn message_of(text: String) -> String {
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
        .unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_activity_list_reads_the_message_of_a_body() {
        let text = serde_json::json!({ "error": "nope", "code": "refused" }).to_string();
        assert_eq!(message_of(text), "nope");
        assert_eq!(message_of("plain".to_string()), "plain");
    }

    #[test]
    fn a_waiting_answer_names_its_approval_in_any_shape() {
        let object = serde_json::json!({ "approval": { "id": "approval-1" } });
        assert_eq!(waiting_on(&object), Some("approval-1"));
        let bare = serde_json::json!({ "approval": "approval-2" });
        assert_eq!(waiting_on(&bare), Some("approval-2"));
        let batch = serde_json::json!({ "jobs": [{ "approval": { "id": "approval-3" } }] });
        assert_eq!(waiting_on(&batch), Some("approval-3"));
    }
}
