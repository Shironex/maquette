//! What a person decides about a case before it runs on: the route, and
//! whether the chains read from the bone tree are right. No agent tool
//! reaches these. A change that makes the sheets stale starts the case over;
//! the version it came from stays as it was.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{RigChain, RigDoctors, RigRecord, RigRoute, RigState, RigStep, read};
use crate::error::{CoreError, CoreResult};
use crate::workspace::WorkspaceConfig;

/// A person's correction of one chain, named by its first bone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigChainEdit {
    pub key: String,
    /// `chain` or `junk`.
    pub read_as: Option<String>,
    /// `left` or `right`, for a leg.
    pub side: Option<String>,
}

/// The edits as `autoconfig.py` reads them.
pub(super) fn by_first_bone(edits: &[RigChainEdit]) -> BTreeMap<&str, Value> {
    edits
        .iter()
        .map(|e| {
            (
                e.key.as_str(),
                json!({ "readAs": e.read_as, "side": e.side }),
            )
        })
        .collect()
}

/// Drops what the runs made, so the case waits for its pose test again.
fn start_over(case: &mut RigRecord) {
    if let Some(file) = case.draft.take() {
        let _ = fs::remove_file(file);
    }
    case.state = RigState::Ready;
    case.step = if case.source.version.is_some() {
        RigStep::Before
    } else {
        RigStep::Accept
    };
    case.before = None;
    case.after = None;
    case.defects.clear();
    case.stats.clear();
    case.fix_steps.clear();
    case.checks.clear();
    case.joint_move = None;
    case.round = 0;
    case.progress = None;
    case.stages.clear();
    case.log.clear();
    case.stop = None;
    case.walk_proof = None;
    case.error = None;
}

fn check_edit(edit: &RigChainEdit, chains: &[RigChain]) -> CoreResult<()> {
    let refuse = |why: String| Err(CoreError::Invalid(why));
    if edit.key.is_empty() || !chains.iter().any(|c| c.key == edit.key) {
        return refuse(format!("{} is not a chain of this model.", edit.key));
    }
    if let Some(word) = edit
        .read_as
        .as_deref()
        .filter(|w| !matches!(*w, "chain" | "junk"))
    {
        return refuse(format!("A chain cannot read as {word}."));
    }
    if let Some(word) = edit
        .side
        .as_deref()
        .filter(|w| !matches!(*w, "left" | "right"))
    {
        return refuse(format!("A leg cannot sit on {word}."));
    }
    let leg = chains
        .iter()
        .any(|c| c.key == edit.key && c.chain.starts_with("leg_"));
    if edit.side.is_some() && !leg {
        return refuse("Only a leg has a side to choose.".to_string());
    }
    Ok(())
}

/// Whether `edit` says anything the table does not say already.
fn changes(edit: &RigChainEdit, chains: &[RigChain]) -> bool {
    chains.iter().filter(|c| c.key == edit.key).any(|c| {
        let junk = c.read_as == "junk";
        let read = edit
            .read_as
            .as_deref()
            .is_some_and(|w| (w == "junk") != junk);
        let side = edit.side.as_deref().is_some_and(|s| !c.side.starts_with(s));
        read || side
    })
}

fn apply(edit: &RigChainEdit, chains: &mut [RigChain]) {
    for row in chains.iter_mut().filter(|c| c.key == edit.key) {
        if let Some(word) = &edit.read_as {
            row.read_as.clone_from(word);
            row.note = None;
        }
        if let Some(side) = &edit.side {
            row.side = format!("{side}, by hand");
            row.read_as = "chain".to_string();
            row.note = None;
        }
    }
}

impl RigDoctors {
    /// A person's word on a creature's chains. With nothing changed, the
    /// sheets stand and only the chains count as looked at; a change starts
    /// the case over from its pose test, draft included, with the edits kept.
    ///
    /// # Errors
    ///
    /// Fails for an unknown or running case, one with no chains read, one
    /// already added, and an edit that does not fit the table.
    pub fn confirm_chains(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        id: &str,
        edits: Vec<RigChainEdit>,
    ) -> CoreResult<RigRecord> {
        let mut case = self.get(root, config, id)?;
        if case.state.running() || self.active().contains_key(id) {
            return Err(CoreError::Invalid("Cancel the run first.".to_string()));
        }
        if case.state == RigState::Landed {
            return Err(CoreError::Invalid(
                "This draft is already a version.".to_string(),
            ));
        }
        let Some(creature) = case.creature.as_mut() else {
            return Err(CoreError::Invalid("No chains were read yet.".to_string()));
        };
        for edit in &edits {
            check_edit(edit, &creature.chains)?;
        }
        let changed = edits.iter().any(|e| changes(e, &creature.chains));
        for edit in &edits {
            apply(edit, &mut creature.chains);
        }
        case.chains_confirmed = true;
        if changed {
            for edit in edits {
                case.chain_edits.retain(|e| e.key != edit.key);
                case.chain_edits.push(edit);
            }
            start_over(&mut case);
        }
        self.publish(&mut case)?;
        Ok(case)
    }

    /// A person picks the route the bone names did not. Humanoid needs Mixamo
    /// names. The case starts over; a draft has to be discarded first.
    ///
    /// # Errors
    ///
    /// Fails for an unknown, running, refused or added case, one with a
    /// draft, and the humanoid route on a model with its own names.
    pub fn set_route(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        id: &str,
        route: RigRoute,
    ) -> CoreResult<RigRecord> {
        let mut case = self.get(root, config, id)?;
        if case.route == Some(route) {
            return Ok(case);
        }
        let refuse = |why: &str| Err(CoreError::Invalid(why.to_string()));
        if case.state.running() || self.active().contains_key(id) {
            return refuse("Cancel the run first.");
        }
        if matches!(case.state, RigState::Refused | RigState::Landed) {
            return refuse("This case has no route left to pick.");
        }
        if case.draft.is_some() {
            return refuse("Discard the draft first: it came from the other route.");
        }
        if route == RigRoute::Humanoid
            && read::read_model(Path::new(&case.input))?.1 != Some(RigRoute::Humanoid)
        {
            return refuse("The humanoid route needs Mixamo bone names.");
        }
        start_over(&mut case);
        case.route = Some(route);
        case.creature = None;
        case.chains_confirmed = false;
        case.chain_edits.clear();
        self.publish(&mut case)?;
        Ok(case)
    }
}

/// Why a case's draft cannot be added yet, when a person still has to look
/// at the chains the doctor read.
pub fn waits_for_chains(case: &RigRecord) -> Option<&'static str> {
    (case.route == Some(RigRoute::Creature) && !case.chains_confirmed)
        .then_some("Look at the chains first: they were only read from the bone tree.")
}
