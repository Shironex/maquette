//! Whether a finished bake is out of date: the recipe changed since (its
//! version is a hash of the file), or the model has another newest version
//! than the one it was baked onto. Read each time a bake is read, never
//! saved, so it always says how things stand now.

use std::path::Path;

use super::RecipeBakeRecord;
use crate::error::CoreError;
use crate::workspace::WorkspaceConfig;
use crate::{assets, project, recipes};

/// Why the recipe's bake is out of date.
pub const RECIPE_CHANGED: &str = "The recipe changed since this bake. Bake it again.";
const RECIPE_GONE: &str = "The recipe is gone, so this bake is out of date.";

/// Why `record`'s bake is out of date, or none when it still matches the
/// recipe and the model (or is no finished bake). What cannot be read
/// says nothing.
pub fn stale_of(
    root: &Path,
    config: &WorkspaceConfig,
    record: &RecipeBakeRecord,
) -> Option<String> {
    // Only a staged bake can go out of date; an added one is a version now.
    if record.state != super::RecipeBakeState::Done {
        return None;
    }
    let result = record.result.as_ref()?;
    let dir = project::dir(root, &config.layout, &record.project).ok()?;
    match recipes::version(&dir, &record.recipe) {
        Err(CoreError::NotFound(_)) => return Some(RECIPE_GONE.to_string()),
        Err(_) => return None,
        Ok(now) if now != result.recipe_version => return Some(RECIPE_CHANGED.to_string()),
        Ok(_) => {}
    }
    let manifest = project::load(root, &config.layout, &record.project).ok()?;
    let Ok(asset) = assets::find(&manifest, &result.asset) else {
        return Some(format!(
            "{} is no longer in the project, so this bake is out of date.",
            result.asset_name
        ));
    };
    let newest = asset.latest()?.version;
    (newest != result.from_version).then(|| {
        format!(
            "{} is at v{newest} now, and this bake went onto v{}. Bake it again.",
            asset.name, result.from_version
        )
    })
}
