//! Where an inbox model lands on Accept, read from the project it goes to:
//! the version it would become and whether a newer version of that asset
//! was made after it arrived (the old model a redo left behind). The shell
//! calls [`place`] after linking the items to their sends, since a send
//! names the asset too.

use std::collections::HashMap;
use std::path::Path;

use chrono::DateTime;

use super::InboxItem;
use crate::project::{self, ProjectManifest};
use crate::workspace::WorkspaceConfig;

/// The milliseconds an RFC 3339 time stands for, or None when it is unreadable.
fn millis(at: &str) -> Option<f64> {
    DateTime::parse_from_rfc3339(at)
        .ok()
        .map(|t| t.timestamp_millis() as f64)
}

/// Fills each item's `next_version` and `replaced_by` from the project of
/// the asset it goes to (its send's, else its aim's). A project that cannot
/// be read leaves its items as they were; an asset not made yet starts at 1.
pub fn place(root: &Path, config: &WorkspaceConfig, items: &mut [InboxItem]) {
    let mut projects: HashMap<String, Option<ProjectManifest>> = HashMap::new();
    for item in items.iter_mut() {
        let goal = item
            .link
            .as_ref()
            .map(|l| (l.project.clone(), l.asset.clone()))
            .or_else(|| {
                item.aim
                    .as_ref()
                    .map(|a| (a.project.clone(), a.asset.clone()))
            });
        item.next_version = None;
        item.replaced_by = None;
        let Some((project_id, asset_id)) = goal else {
            continue;
        };
        let manifest = projects
            .entry(project_id.clone())
            .or_insert_with(|| project::load(root, &config.layout, &project_id).ok());
        let Some(manifest) = manifest else {
            continue;
        };
        let Some(asset) = manifest.assets.iter().find(|a| a.id == asset_id) else {
            item.next_version = Some(1);
            continue;
        };
        let newest = asset.versions.iter().map(|v| v.version).max().unwrap_or(0);
        item.next_version = Some(newest + 1);
        item.replaced_by = asset
            .versions
            .iter()
            .filter(|v| millis(&v.created_at).is_some_and(|at| at > item.modified_ms))
            .map(|v| v.version)
            .max();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rfc3339_times_and_refuses_others() {
        assert_eq!(millis("1970-01-01T00:00:01Z"), Some(1000.0));
        assert_eq!(millis("yesterday"), None);
    }
}
