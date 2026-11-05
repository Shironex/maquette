//! Adding a staged bake as the model's next version, after its review. The
//! shell brings the staged model in through the same route the rig
//! doctor's Add as vN takes (the Inbox for the asset, then Accept), so the
//! version, its checks, thumbnail and events are the Inbox's. Here: whether
//! a bake can be added at all, and what is left to do once it is a version.

use std::path::{Path, PathBuf};

use super::service::{clear, folder, record_of, save};
use super::{RecipeBakeRecord, RecipeBakeState, RecipeBakes, STAGED_FILE, mark_for};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::now_ms;
use crate::project;
use crate::recipes::baked::{self, RecipeBakeUse};
use crate::workspace::WorkspaceConfig;

/// How a refusal of an out of date bake starts; the reason follows.
pub const OUT_OF_DATE: &str = "Not added: this bake is out of date.";

/// The staged bake of `recipe` in `project`, ready to add: finished, not
/// out of date (the recipe and the model are as it was baked from), its
/// staged model there. Answers the record and the staged model.
///
/// # Errors
///
/// [`OUT_OF_DATE`] and the reason for a stale bake (bake it again);
/// `invalid` for no bake, one still running, failed, cancelled or added
/// already, or a staged model that is gone.
pub fn ready_to_add(
    root: &Path,
    config: &WorkspaceConfig,
    project: &str,
    recipe: &str,
) -> CoreResult<(RecipeBakeRecord, PathBuf)> {
    let Some(record) = record_of(root, config, project, recipe)? else {
        return Err(CoreError::Invalid(
            "This recipe has no bake to add. Apply it first.".to_string(),
        ));
    };
    match record.state {
        RecipeBakeState::Done => {}
        RecipeBakeState::Running => {
            return Err(CoreError::Invalid(format!(
                "{} is still baking. Add it once the bake is done.",
                record.recipe_name
            )));
        }
        RecipeBakeState::Added => {
            return Err(CoreError::Invalid(format!(
                "This bake was added already, as v{}.",
                record.added_version.unwrap_or_default()
            )));
        }
        RecipeBakeState::Failed | RecipeBakeState::Cancelled => {
            return Err(CoreError::Invalid(
                "The last bake did not finish, so there is nothing to add. Bake it again."
                    .to_string(),
            ));
        }
    }
    if let Some(why) = &record.stale {
        return Err(CoreError::Invalid(format!("{OUT_OF_DATE} {why}")));
    }
    let staged = folder(root, config, project, recipe)?.join(STAGED_FILE);
    if record.result.is_none() || !staged.is_file() {
        return Err(CoreError::Invalid(
            "The staged model is gone. Bake it again.".to_string(),
        ));
    }
    Ok((record, staged))
}

impl RecipeBakes {
    /// Ends a bake the shell added as `version` of its model: the bake is
    /// recorded on that version in `recipe-bakes.json` (the Inbox's accept
    /// records it from the mark; this makes sure, for a Mixamo bake above
    /// all, and names the recipe and its version that made it), the staged
    /// model is cleared and the record is saved as added.
    ///
    /// # Errors
    ///
    /// Fails when there is no finished bake, or a record cannot be read or
    /// written.
    pub fn added(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        project: &str,
        recipe: &str,
        version: u32,
    ) -> CoreResult<RecipeBakeRecord> {
        let mut record = record_of(root, config, project, recipe)?
            .filter(|r| r.state == RecipeBakeState::Done && !self.baking(project, recipe))
            .ok_or_else(|| CoreError::Invalid("There is no finished bake to add.".to_string()))?;
        let result = record
            .result
            .clone()
            .ok_or_else(|| CoreError::Invalid("There is no finished bake to add.".to_string()))?;
        let dir = project::dir(root, &config.layout, project)?;
        let this = RecipeBakeUse::of(&mark_for(&result), version);
        if !baked::of_asset(&dir, &result.asset)?.contains(&this) {
            baked::record(&dir, &result.asset, this)?;
        }
        let work = folder(root, config, project, recipe)?;
        clear(&work.join(STAGED_FILE));
        record.state = RecipeBakeState::Added;
        record.added_version = Some(version);
        record.stale = None;
        record.updated_ms = now_ms();
        save(&work, &record)?;
        self.tell(&record);
        Ok(record)
    }
}
