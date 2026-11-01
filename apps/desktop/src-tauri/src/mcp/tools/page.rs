//! Tools for the site's dev server the Page view shows: start it, stop it
//! and read how it stands, for a project by id. It is the same server the
//! Page view and Preview on site use, so the app shows an agent's start
//! live, and a server an agent started stays until it is stopped or the
//! app quits.

use std::path::Path;
use std::time::Duration;

use maquette_core::page::PageView;
use maquette_core::site::preview::{PreviewAsk, START_TIMEOUT, ServerOwner, SitePreview};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::commands::page as page_commands;
use crate::commands::site::gate::{self, StartGate, Starter};
use crate::mcp::host::Host;
use crate::mcp::hub::Hub;

use super::hints::APPROVE_IN_APP;
use super::{McpTools, Out, ProjectArgs, WAITING, respond, to_value};

/// How long start_page_preview waits by default, in seconds.
const DEFAULT_WAIT: u64 = 30;
/// Log lines a tool answers with.
const LOG_TAIL: usize = 20;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartPageArgs {
    /// The project id, as list_projects returns it.
    pub project: String,
    /// Seconds to wait for the server to answer, 0 to 90 (default 30). 0
    /// returns at once with the server starting; poll get_page_preview.
    pub wait_seconds: Option<u64>,
}

const NOTE: &str = "The server runs the project's dev command in its site repo: the command set in Settings > Project, else the preview command of the site's maquette.site.json, else the package manager of the nearest lockfile running the dev, start or serve script with a free port. It is the server the Page view and Preview on site share; Maquette stops it when the app quits and never deploys anything.";

const APPROVAL_NOTE: &str = "A person reads this folder and command and approves or denies in the Maquette app; nothing runs until then, and a yes starts the server. Poll get_page_preview or call start_page_preview again; neither raises a second card.";
const REFUSED_FIX: &str = "Do not retry: this answer stands until the app restarts. Ask the person; they can start the server from the Page view, which shows the card again. A changed path or command is a new question.";

fn server(host: &dyn Host) -> Result<Arc<SitePreview>, String> {
    host.site_preview()
        .ok_or_else(|| "This Maquette has no dev server for the Page view.".to_string())
}

fn page_value(view: &PageView) -> Value {
    let status = &view.server;
    let tail: Vec<&String> = status
        .log
        .iter()
        .skip(status.log.len().saturating_sub(LOG_TAIL))
        .collect();
    json!({
        "project": view.project,
        "siteRepo": view.site_repo,
        "command": view.command.as_ref().map(|c| json!({
            "line": c.line(),
            "source": to_value(c.source),
            "manager": c.manager,
            "script": c.script,
            "lockfile": c.lockfile,
            "url": c.url,
        })),
        "commandError": view.command_error,
        "server": {
            "state": to_value(status.state),
            "url": status.url,
            "command": status.command,
            "owner": status.owner.map(to_value),
            "external": status.external,
            "startedAt": status.started_at,
            "message": status.message,
            "log": tail,
        },
        "busyWith": view.busy_with,
        "settings": to_value(&view.settings),
        "note": NOTE,
    })
}

fn approval_value(id: &str, ask: &PreviewAsk) -> Value {
    json!({
        "id": id,
        "repo": ask.repo,
        "command": ask.command,
        "scriptRuns": ask.script_runs,
        "note": APPROVAL_NOTE,
    })
}

/// What a get_page_preview adds while the start waits on a person, or after
/// a person said no.
fn add_approval(value: &mut Value, hub: &Hub, view: &PageView) {
    if let Some(card) = hub.preview_for(&view.project) {
        value["status"] = json!(WAITING);
        if let Some(ask) = &card.preview {
            value["approval"] = approval_value(&card.id, ask);
        }
        return;
    }
    let (Some(repo), Some(command)) = (&view.site_repo, &view.command) else {
        return;
    };
    let ask = PreviewAsk::of(Path::new(repo), command);
    if hub.preview_refused(&view.project, &ask) {
        value["approval"] = json!({ "refused": true, "note": REFUSED_FIX });
    }
}

/// The error of a start a person turned down.
fn refused_body(ask: &PreviewAsk) -> Value {
    json!({
        "error": format!(
            "A person refused to run {} in {}. The server stays stopped.",
            ask.command, ask.repo
        ),
        "code": "preview-refused",
        "fix": REFUSED_FIX,
        "personOnly": true,
        "whereToApprove": APPROVE_IN_APP,
    })
}

#[tool_router(router = page_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get how a project's site dev server stands, as the Page view shows it: idle, starting, running or failed, its address, the command it runs and where that command came from (Settings, the site's maquette.site.json, or package.json and the lockfile), who started it, why it failed and its last log lines. busyWith names another project whose site the one server serves now.",
        annotations(read_only_hint = true)
    )]
    async fn get_page_preview(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                let server = server(host)?;
                let view = page_commands::view(host.session(), &server, &args.project)?;
                let mut value = page_value(&view);
                add_approval(&mut value, hub, &view);
                Ok(value)
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Start a project's site dev server, the one the Page view shows, or reuse it when it already serves that site. This runs the site repo's dev command on this computer. The first start of a repo an agent set waits for a person, who reads the folder and the command in the app: it answers waiting for approval, a yes starts the server, and any changed path or command asks again. A no is an error with code preview-refused. A repo a person set never asks. A server already answering at the site's configured address is reused and never stopped. Starting another project's site stops the one before. Waits up to waitSeconds (default 30) for it to answer and returns its state and address like get_page_preview; a failed start says why with the last log lines. The server stays until stop_page_preview or the app quits. Nothing is built or deployed.",
        annotations(idempotent_hint = true)
    )]
    async fn start_page_preview(&self, Parameters(args): Parameters<StartPageArgs>) -> Out {
        let (host, hub) = (Arc::clone(&self.host), Arc::clone(&self.hub));
        let found = self
            .blocking(move |_, _| {
                let server = server(host.as_ref())?;
                let project = &args.project;
                let waiting = match gate::check(&host, &hub, project, Starter::Agent)? {
                    StartGate::Clear(plan) => {
                        server.begin(&plan, project, ServerOwner::Agent);
                        let wait = args
                            .wait_seconds
                            .unwrap_or(DEFAULT_WAIT)
                            .min(START_TIMEOUT.as_secs());
                        server.wait(Duration::from_secs(wait));
                        None
                    }
                    StartGate::Waiting { id, ask } => Some((id, ask)),
                    StartGate::Refused(ask) => return Ok(Err(refused_body(&ask))),
                };
                let view = page_commands::view(host.session(), &server, project)?;
                let mut value = page_value(&view);
                if let Some((id, ask)) = waiting {
                    value["status"] = json!(WAITING);
                    value["approval"] = approval_value(&id, &ask);
                }
                Ok(Ok(value))
            })
            .await;
        match found {
            Ok(Err(body)) => Ok(CallToolResult::error(vec![ContentBlock::text(
                body.to_string(),
            )])),
            Ok(Ok(value)) => respond(Ok(value)),
            Err(err) => respond(Err(err)),
        }
    }

    #[tool(
        description = "Stop a project's site dev server, whoever started it (the Page view, Preview on site or an agent). It does nothing when the server serves another project or is not running; a server Maquette did not start is only forgotten, never killed. Returns the state like get_page_preview.",
        annotations(idempotent_hint = true)
    )]
    async fn stop_page_preview(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let server = server(host)?;
                let view = page_commands::stop(host.session(), &server, &args.project)?;
                Ok(page_value(&view))
            })
            .await;
        respond(found)
    }
}
