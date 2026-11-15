//! Work on a recipe that must not cross its bake: removing the recipe is
//! refused while it bakes or its bake is being added as a version, and no
//! bake starts or add begins while the work runs.

use std::path::Path;

use super::RecipeBakes;
use super::service::{key, record_of};
use crate::error::{CoreError, CoreResult};
use crate::workspace::WorkspaceConfig;

impl RecipeBakes {
    /// Runs `work` while `recipe` of `project` neither bakes nor is being
    /// added, holding both off until it is done.
    ///
    /// # Errors
    ///
    /// `invalid` while the recipe bakes or its bake is being added; else
    /// what `work` answers.
    pub fn while_idle<T>(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        project: &str,
        recipe: &str,
        work: impl FnOnce() -> CoreResult<T>,
    ) -> CoreResult<T> {
        let name = key(project, recipe);
        // The same order as a start and an add take them, so none cross.
        let active = self.active();
        let adding = self.adding();
        // A bake saves its record as finished just before it leaves the
        // running ones, so only one still running on disk is baking.
        let baking = active.contains_key(&name)
            && record_of(root, config, project, recipe)?.is_none_or(|r| r.running());
        if baking {
            return Err(CoreError::Invalid(
                "This recipe is baking. Wait for the bake or cancel it first.".to_string(),
            ));
        }
        if adding.contains(&name) {
            return Err(CoreError::Invalid(
                "This recipe's bake is being added as a version now. Try again once that is done."
                    .to_string(),
            ));
        }
        work()
    }
}
