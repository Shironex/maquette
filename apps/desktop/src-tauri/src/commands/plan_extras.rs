//! The person's side of a canvas's extras: answering an agent's removal
//! proposals (only here; the MCP server has no tool to answer one),
//! starting a canvas from a node, the "Auto layout after agents" switch
//! and marking a mechanics overlap as fine. Each is a logged change the
//! person can undo from the Changes panel. Like a save, each answer and
//! setting names the opened canvas's `created`, so it never lands on
//! another canvas that took the id.

use maquette_core::plans::removal::{self, RemovalProposal};
use maquette_core::plans::{self, PlanDoc};
use tauri::State;

use super::CommandResult;
use super::plans::project_dir;
use crate::session::Session;

/// The removal proposals waiting for the person, oldest first, each marked
/// stale when a node it names changed or went since the agent asked.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_proposals(
    session: State<'_, Session>,
    project: String,
    id: String,
) -> CommandResult<Vec<RemovalProposal>> {
    Ok(removal::open(&project_dir(&session, &project)?, &id)?)
}

/// Answers a removal proposal as the person: `remove` takes the nodes off
/// (with their links) as the person's undoable change, refused when the
/// proposal went stale; otherwise "Keep it" leaves the canvas as it is.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_answer_removal(
    session: State<'_, Session>,
    project: String,
    id: String,
    created: String,
    proposal: String,
    remove: bool,
) -> CommandResult<PlanDoc> {
    let dir = project_dir(&session, &project)?;
    if remove {
        return Ok(removal::accept(&dir, &id, &proposal, Some(&created))?.doc);
    }
    removal::dismiss(&dir, &id, &proposal, Some(&created))?;
    Ok(plans::get(&dir, &id)?)
}

/// Makes a canvas from the node `node` of `source` and the nodes linked to
/// it, as the person's first change there. The source does not change.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_start_from(
    session: State<'_, Session>,
    project: String,
    source: String,
    node: String,
    name: Option<String>,
) -> CommandResult<PlanDoc> {
    let dir = project_dir(&session, &project)?;
    let started = plans::start_from(&dir, &source, &node, name.as_deref(), None)?;
    Ok(started.changed.doc)
}

/// Turns "Auto layout after agents" on or off for the canvas.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_set_auto_layout(
    session: State<'_, Session>,
    project: String,
    id: String,
    created: String,
    on: bool,
) -> CommandResult<PlanDoc> {
    let dir = project_dir(&session, &project)?;
    Ok(plans::set_auto_layout(&dir, &id, on, Some(&created))?.doc)
}

/// Marks the overlap of the cards `a` and `b` as fine until either card's
/// words change.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_mark_fine(
    session: State<'_, Session>,
    project: String,
    id: String,
    created: String,
    a: String,
    b: String,
) -> CommandResult<PlanDoc> {
    let dir = project_dir(&session, &project)?;
    Ok(plans::mark_fine(&dir, &id, &a, &b, Some(&created))?.doc)
}
