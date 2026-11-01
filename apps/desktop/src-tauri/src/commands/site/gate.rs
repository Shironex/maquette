//! Page preview: ask first. Every start of the site's dev server, from an
//! agent, the Page view or Preview on site, gets its plan here and only here
//! (`dev_plan` is private to this module's parent), so a repo an agent set
//! never runs its dev command before a person read that command on a card.
//! A repo a person set never asks.
//!
//! The app's own starts show the same card instead of counting as a yes: the
//! Page view starts the server just by opening, so that click says nothing
//! about the command.

use std::path::Path;
use std::sync::Arc;

use maquette_core::app_state::RepoSetBy;
use maquette_core::project::repos;
use maquette_core::site::preview::{PreviewAsk, PreviewPlan, ServerOwner};

use super::super::CommandResult;
use crate::mcp::host::Host;
use crate::mcp::hub::Hub;
use crate::session::Session;

/// Who wants the server started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Starter {
    Agent,
    /// A person in the app. Asking from here takes back an earlier no.
    Person,
}

impl Starter {
    /// Who owns a server a yes starts. A person's stays until stopped, as
    /// Preview on site's does: the Page view may have closed by then, and
    /// nothing would let go of it.
    fn owner(self) -> ServerOwner {
        match self {
            Self::Agent => ServerOwner::Agent,
            Self::Person => ServerOwner::Pipeline,
        }
    }
}

/// How a start stands.
#[derive(Debug)]
pub enum StartGate {
    /// Nothing to ask: this is what to run.
    Clear(Box<PreviewPlan>),
    /// A card waits for a person. A yes starts the server.
    Waiting { id: String, ask: PreviewAsk },
    /// A person said no to this very question.
    Refused(PreviewAsk),
}

/// Records who set a project's repo, on this computer.
pub(crate) fn mark_repo(
    session: &Session,
    root: &Path,
    project: &str,
    by: RepoSetBy,
) -> CommandResult<()> {
    let key = repos::root_key(root);
    if session.app_state().repo_set_by_agent(&key, project) == (by == RepoSetBy::Agent) {
        return Ok(());
    }
    session.update_app_state(|state| state.mark_repo(&key, project, by))?;
    Ok(())
}

/// True when an agent set `project`'s repo and no person has set it since.
pub(crate) fn repo_set_by_agent(session: &Session, root: &Path, project: &str) -> bool {
    session
        .app_state()
        .repo_set_by_agent(&repos::root_key(root), project)
}

/// True when `ask` needs a person's yes before it runs.
fn must_ask(session: &Session, project: &str, ask: &PreviewAsk) -> CommandResult<bool> {
    let (root, _) = session.workspace()?;
    Ok(repo_set_by_agent(session, &root, project)
        && !session.app_state().preview_approved(&ask.repo, &ask.key()))
}

/// Decides whether `project`'s dev server may start now, raising the card
/// when it may not. Blocks briefly to find the program.
///
/// # Errors
///
/// Fails like any start: no repo, nothing to run, the program not found.
pub fn check(
    host: &Arc<dyn Host>,
    hub: &Arc<Hub>,
    project: &str,
    starter: Starter,
) -> CommandResult<StartGate> {
    let (plan, ask) = super::dev_plan(host.session(), project)?;
    if !must_ask(host.session(), project, &ask)? {
        return Ok(StartGate::Clear(Box::new(plan)));
    }
    match starter {
        Starter::Person => hub.forget_preview_refusal(project),
        Starter::Agent if hub.preview_refused(project, &ask) => {
            return Ok(StartGate::Refused(ask));
        }
        Starter::Agent => {}
    }
    let (id, raised) = hub.request_preview(project, &ask);
    if raised {
        await_answer(host, hub, &id, project, &ask, starter);
    }
    Ok(StartGate::Waiting { id, ask })
}

/// Waits for the person on its own thread. A yes is remembered for this very
/// question, then the start goes through [`check`] again, so a command that
/// changed behind the card asks again instead of running.
fn await_answer(
    host: &Arc<dyn Host>,
    hub: &Arc<Hub>,
    id: &str,
    project: &str,
    ask: &PreviewAsk,
    starter: Starter,
) {
    let (host, hub) = (Arc::clone(host), Arc::clone(hub));
    let (id, project, ask) = (id.to_string(), project.to_string(), ask.clone());
    let spawned = std::thread::Builder::new()
        .name("page-preview-approval".into())
        .spawn(move || match hub.wait_until(&id, || false) {
            Some(true) => answered_yes(&host, &hub, &project, &ask, starter),
            Some(false) if !hub.take_preview_withdrawn(&id) => hub.refuse_preview(&project, &ask),
            _ => {}
        });
    if let Err(err) = spawned {
        tracing::warn!(error = %err, "could not wait for a page preview approval");
    }
}

fn answered_yes(
    host: &Arc<dyn Host>,
    hub: &Arc<Hub>,
    project: &str,
    ask: &PreviewAsk,
    starter: Starter,
) {
    if let Err(err) = host
        .session()
        .update_app_state(|state| state.approve_preview(&ask.repo, ask.key()))
    {
        tracing::warn!(error = %err, "could not remember a page preview approval");
        return;
    }
    let Some(server) = host.site_preview() else {
        return;
    };
    match check(host, hub, project, starter) {
        Ok(StartGate::Clear(plan)) => {
            server.begin(&plan, project, starter.owner());
        }
        Ok(_) => {}
        Err(err) => tracing::warn!(error = %err.message, "could not start an approved server"),
    }
}

#[cfg(test)]
mod tests;
