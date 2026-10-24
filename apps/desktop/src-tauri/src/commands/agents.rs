//! Settings > Agents: the MCP server's status, its activity list and the
//! shipping approvals for runs an agent started.

use tauri::{AppHandle, State};

use super::{CommandResult, invalid};
use crate::mcp::{self, Agents, McpStatus};
use crate::session::Session;

#[tauri::command]
#[specta::specta]
pub fn mcp_status(session: State<'_, Session>, agents: State<'_, Agents>) -> McpStatus {
    agents.status(&session)
}

/// Approves or denies the Ship step of a run an agent started.
#[tauri::command]
#[specta::specta]
pub fn mcp_decide(
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    approval_id: String,
    approve: bool,
) -> CommandResult<McpStatus> {
    if !agents.hub.decide(&approval_id, approve) {
        return Err(invalid("That run is no longer waiting for approval."));
    }
    Ok(agents.status(&session))
}

#[tauri::command]
#[specta::specta]
pub fn mcp_clear_activity(session: State<'_, Session>, agents: State<'_, Agents>) -> McpStatus {
    agents.hub.clear_calls();
    agents.status(&session)
}

/// Makes a new token. Agents set up with the old one must be set up again.
#[tauri::command]
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
