//! The built in MCP server, so Claude Code and Codex can drive Maquette.
//! It is off until Settings > Agents turns it on, listens on 127.0.0.1
//! only, and wants the bearer token kept in the app state.

pub mod gate;
pub mod host;
pub mod hub;
pub mod server;
pub mod tools;

use std::sync::{Arc, Mutex, MutexGuard};

use maquette_core::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager as _};
use tauri_specta::Event as _;

use self::host::Host;
use self::hub::{Approval, Hub, McpCall};
use crate::events::McpUpdate;
use crate::session::Session;

/// Everything Settings > Agents shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    /// The workspace setting.
    pub enabled: bool,
    pub running: bool,
    pub port: u32,
    pub url: String,
    /// Made on first start and kept in the app state.
    pub token: Option<String>,
    /// Why the server is not running although it is on.
    pub error: Option<String>,
    pub ask_before_shipping: bool,
    /// Newest first.
    pub calls: Vec<McpCall>,
    pub approvals: Vec<Approval>,
}

pub fn url(port: u32) -> String {
    format!("http://127.0.0.1:{port}{}", server::PATH)
}

#[derive(Default)]
struct ServerState {
    running: Option<server::Running>,
    error: Option<String>,
}

/// The app's MCP server and what it shares with the windows.
pub struct Agents {
    pub hub: Arc<Hub>,
    host: Arc<dyn Host>,
    state: Mutex<ServerState>,
}

/// 32 random bytes as hex.
fn new_token() -> CoreResult<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|e| CoreError::Unavailable(format!("Could not make a token. {e}")))?;
    Ok(hex::encode(bytes))
}

/// The token in the app state, made the first time it is needed.
pub fn ensure_token(session: &Session) -> CoreResult<String> {
    if let Some(token) = session.app_state().mcp_token {
        return Ok(token);
    }
    let token = new_token()?;
    let saved = token.clone();
    session.update_app_state(move |state| state.mcp_token = Some(saved))?;
    Ok(token)
}

impl Agents {
    pub fn new(host: Arc<dyn Host>, hub: Arc<Hub>) -> Self {
        Self {
            hub,
            host,
            state: Mutex::new(ServerState::default()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, ServerState> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn status(&self, session: &Session) -> McpStatus {
        let config = session.workspace().ok().map(|(_, c)| c.agents);
        let (running, port, error) = {
            let state = self.lock();
            (
                state.running.as_ref().map(server::Running::port),
                config.as_ref().map_or(4791, |a| a.mcp_port),
                state.error.clone(),
            )
        };
        let port = running.map_or(port, u32::from);
        McpStatus {
            enabled: config.as_ref().is_some_and(|a| a.mcp_enabled),
            running: running.is_some(),
            port,
            url: url(port),
            token: session.app_state().mcp_token,
            error,
            ask_before_shipping: config.as_ref().is_none_or(|a| a.rules.ask_before_shipping),
            calls: self.hub.calls(),
            approvals: self.hub.approvals(),
        }
    }

    /// Starts, restarts or stops the server to match the open workspace's
    /// setting.
    pub fn sync(&self, session: &Session) {
        let wanted = session
            .workspace()
            .ok()
            .map(|(_, c)| c.agents)
            .filter(|a| a.mcp_enabled)
            .and_then(|a| u16::try_from(a.mcp_port).ok());
        let mut state = self.lock();
        let current = state.running.as_ref().map(server::Running::port);
        if current == wanted && (wanted.is_some() || state.error.is_none()) {
            return;
        }
        if let Some(running) = state.running.take() {
            running.stop();
        }
        state.error = None;
        if let Some(port) = wanted {
            match ensure_token(session).and_then(|token| {
                server::start(port, token, Arc::clone(&self.host), Arc::clone(&self.hub)).map_err(
                    |e| {
                        CoreError::Unavailable(format!("Could not listen on 127.0.0.1:{port}. {e}"))
                    },
                )
            }) {
                Ok(running) => {
                    tracing::info!(port, "the MCP server is listening");
                    state.running = Some(running);
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }

    /// Replaces the token. A running server restarts with the new one.
    pub fn renew_token(&self, session: &Session) -> CoreResult<()> {
        let token = new_token()?;
        session.update_app_state(move |state| state.mcp_token = Some(token))?;
        let had = self.lock().running.take();
        if let Some(running) = had {
            running.stop();
        }
        self.sync(session);
        Ok(())
    }

    pub fn stop(&self) {
        if let Some(running) = self.lock().running.take() {
            running.stop();
        }
    }
}

/// Sends the current status to every window. Called by the hub after a
/// change, so it must not run while the server state is locked.
pub fn emit_status(handle: &AppHandle) {
    let (Some(agents), Some(session)) =
        (handle.try_state::<Agents>(), handle.try_state::<Session>())
    else {
        return;
    };
    if let Err(err) = McpUpdate(agents.status(&session)).emit(handle) {
        tracing::warn!(error = %err, "could not emit an MCP event");
    }
}
