//! Settings > Agents: the MCP server's status, its activity list and the
//! approvals for what an agent asked (shipping, sends to 3D).

use maquette_core::providers::ledger::DecidedBy;
use tauri::{AppHandle, State, WebviewWindow};

use super::{CommandResult, invalid};
use crate::mcp::{self, Agents, McpStatus};
use crate::session::Session;

#[tauri::command(async)]
#[specta::specta]
pub fn mcp_status(session: State<'_, Session>, agents: State<'_, Agents>) -> McpStatus {
    agents.status(&session)
}

/// Approves or denies what an agent asked: a run's Ship step, a send to 3D
/// that spends credits, an Image API round or a page preview. Only the
/// app's windows call it, never an MCP tool. `by` says which card and what
/// pressed it; the window's focus and visibility are read here from the
/// operating system, and the answer is written to `approvals.jsonl`.
#[tauri::command(async)]
#[specta::specta]
pub fn mcp_decide(
    window: WebviewWindow,
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    approval_id: String,
    approve: bool,
    by: DecidedBy,
) -> CommandResult<McpStatus> {
    let os = [
        window.is_focused().ok(),
        window.is_visible().ok(),
        window.is_minimized().ok(),
    ];
    let by = from_card(by, os)?;
    if !agents.hub.decide_by(&approval_id, approve, by) {
        return Err(invalid("That request is no longer waiting for approval."));
    }
    Ok(agents.status(&session))
}

/// An answer from an approval card, with the window as the operating
/// system reports it (focused, visible, minimised) in place of anything the
/// page claimed about it. Refused from anything but a card.
pub(crate) fn from_card(by: DecidedBy, os: [Option<bool>; 3]) -> CommandResult<DecidedBy> {
    if !by.source.is_card() {
        return Err(invalid("Answer from an approval card."));
    }
    let [window_focused, window_visible, window_minimized] = os;
    Ok(DecidedBy {
        window_focused,
        window_visible,
        window_minimized,
        ..by
    })
}

#[tauri::command(async)]
#[specta::specta]
pub fn mcp_clear_activity(session: State<'_, Session>, agents: State<'_, Agents>) -> McpStatus {
    agents.hub.clear_calls();
    agents.status(&session)
}

/// Makes a new token. Agents set up with the old one must be set up again.
#[tauri::command(async)]
#[specta::specta]
pub fn mcp_renew_token(
    app: AppHandle,
    session: State<'_, Session>,
    agents: State<'_, Agents>,
) -> CommandResult<McpStatus> {
    agents.renew_token(&session)?;
    mcp::emit_status(&app);
    Ok(agents.status(&session))
}

/// Matches the server to the workspace setting and tells the windows.
pub(crate) fn sync(app: &AppHandle, session: &Session, agents: &Agents) {
    agents.sync(session);
    mcp::emit_status(app);
}

#[cfg(test)]
mod tests {
    use maquette_core::providers::ledger::{DecisionInput, DecisionSource};

    use super::*;

    #[test]
    fn only_a_card_answers_and_the_window_comes_from_the_system() {
        for source in [
            DecisionSource::FloatingCard,
            DecisionSource::AgentsPanel,
            DecisionSource::ShipView,
        ] {
            let claimed = DecidedBy {
                input: Some(DecisionInput::Keyboard),
                window_focused: Some(true),
                window_visible: Some(true),
                ..DecidedBy::from(source)
            };
            let kept = from_card(claimed, [Some(false), Some(true), None]).expect("a card");
            assert_eq!(kept.source, source);
            assert_eq!(kept.input, Some(DecisionInput::Keyboard));
            assert_eq!(kept.window_focused, Some(false), "the system's word wins");
            assert_eq!(kept.window_minimized, None);
        }
        for source in [
            DecisionSource::Unknown,
            DecisionSource::Avatar,
            DecisionSource::RigDoctor,
            DecisionSource::JobsCancelled,
            DecisionSource::Quit,
        ] {
            assert!(
                from_card(DecidedBy::from(source), [None; 3]).is_err(),
                "{source:?}"
            );
        }
    }
}
