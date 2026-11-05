//! Adding a staged bake as the model's next version, after its review. The
//! shell brings the staged model in through the same route the rig
//! doctor's Add as vN takes (the Inbox for the asset, then Accept), so the
//! version, its checks, thumbnail and events are the Inbox's. Here: whether
//! a bake can be added at all, a claim that holds the recipe while the
//! shell adds it (no bake starts and no second add runs meanwhile), and
//! what is left to do once it is a version.

use std::path::{Path, PathBuf};

use super::service::{clear, folder, key, record_of, save};
use super::{RecipeBakeRecord, RecipeBakeState, RecipeBakes, STAGED_FILE, mark_for};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::now_ms;
use crate::project;
use crate::recipes::baked::{self, RecipeBakeUse};
use crate::workspace::WorkspaceConfig;

/// How a refusal of an out of date bake starts; the reason follows.
pub const OUT_OF_DATE: &str = "Not added: this bake is out of date.";
/// How a refusal of a bake other than the one the person looked at starts.
pub const BAKE_CHANGED: &str = "Not added: the bake changed since you looked at it.";

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

/// The recipe held for one add: made by [`RecipeBakes::claim_add`], ended
/// by [`AddClaim::added`] once the version is in, or let go when dropped.
pub struct AddClaim {
    held: Held,
    /// The bake being added, as checked under the claim.
    pub record: RecipeBakeRecord,
    /// Its staged model.
    pub staged: PathBuf,
}

/// A recipe in the adding set, taken out again when dropped.
struct Held {
    bakes: RecipeBakes,
    key: String,
}

impl Drop for Held {
    fn drop(&mut self) {
        self.bakes.adding().remove(&self.key);
    }
}

impl RecipeBakes {
    /// Holds `recipe` of `project` for an add of the bake the person looked
    /// at, the one that finished at `viewed` (its `finishedMs`): checked as
    /// [`ready_to_add`] does, under the claim. While the claim lives, a bake
    /// of the recipe is refused and so is a second add.
    ///
    /// # Errors
    ///
    /// As [`ready_to_add`]; [`BAKE_CHANGED`] when the staged bake is another
    /// than the one at `viewed`; `invalid` while it bakes or is being added.
    pub fn claim_add(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        project: &str,
        recipe: &str,
        viewed: Option<f64>,
    ) -> CoreResult<AddClaim> {
        let name = key(project, recipe);
        {
            // The same order as a start takes them, so the two never cross.
            let active = self.active();
            let mut adding = self.adding();
            // A bake saves its record as finished just before it leaves the
            // running ones, so only one still running on disk is baking.
            let baking = active.contains_key(&name)
                && record_of(root, config, project, recipe)?.is_some_and(|r| r.running());
            if baking {
                return Err(CoreError::Invalid(
                    "This recipe is baking again. Add it once the bake is done.".to_string(),
                ));
            }
            if !adding.insert(name.clone()) {
                return Err(CoreError::Invalid(
                    "This bake is being added already.".to_string(),
                ));
            }
        }
        let held = Held {
            bakes: self.clone(),
            key: name,
        };
        let (record, staged) = ready_to_add(root, config, project, recipe)?;
        if record.finished_ms != viewed {
            return Err(CoreError::Invalid(format!(
                "{BAKE_CHANGED} {} was baked again since. Look at the new bake, then add it.",
                record.recipe_name
            )));
        }
        Ok(AddClaim {
            held,
            record,
            staged,
        })
    }
}

impl AddClaim {
    /// Ends the add once the shell brought the staged model in as `version`
    /// of its model. It cannot fail, since the version is in by then: the
    /// record is saved as added first (so the bake is never added twice),
    /// then the bake is recorded on that version in `recipe-bakes.json`
    /// (the Inbox's accept records it from the mark; this makes sure, for a
    /// Mixamo bake above all, and names the recipe and its version that
    /// made it) and the staged model is cleared. A step that cannot be
    /// written is logged; the new version then reads the bake as out of
    /// date, so it is not added again either.
    pub fn added(self, root: &Path, config: &WorkspaceConfig, version: u32) -> RecipeBakeRecord {
        let mut record = self.record.clone();
        record.state = RecipeBakeState::Added;
        record.added_version = Some(version);
        record.stale = None;
        record.updated_ms = now_ms();
        let project = record.project.clone();
        let recipe = record.recipe.clone();
        let warn = |what: &str, err: &CoreError| {
            tracing::warn!(project = %project, recipe = %recipe, "recipe bake added as v{version}, but {what}: {err}");
        };
        match folder(root, config, &project, &recipe) {
            Ok(work) => {
                if let Err(err) = save(&work, &record) {
                    warn("its record could not be saved", &err);
                }
                clear(&work.join(STAGED_FILE));
            }
            Err(err) => warn("its folder is not a recipe's", &err),
        }
        if let Some(result) = &record.result {
            let this = RecipeBakeUse::of(&mark_for(result), version);
            let recorded = project::dir(root, &config.layout, &project).and_then(|dir| {
                if baked::of_asset(&dir, &result.asset)?.contains(&this) {
                    return Ok(());
                }
                baked::record(&dir, &result.asset, this)
            });
            if let Err(err) = recorded {
                warn("recipe-bakes.json could not be written", &err);
            }
        }
        self.held.bakes.tell(&record);
        record
    }
}
