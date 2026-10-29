//! The asset an inbox model is aimed at when no send stands behind it: a
//! download someone attached to an asset with no By hand send waiting.
//! Kept next to the picked kinds in the inbox cache, by file name, and
//! dropped once the model is accepted. Accept goes to that asset by default.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::{InboxItem, cache_dir, check_id};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::workspace::WorkspaceConfig;

// The aim is shared with the provider sends, so it lives in the sends
// module; this keeps the old path.
pub use crate::sends::InboxAim;

const AIMS_FILE: &str = "aims.json";

fn load(cache: &Path) -> BTreeMap<String, InboxAim> {
    fs::read_to_string(cache.join(AIMS_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(cache: &Path, aims: &BTreeMap<String, InboxAim>) -> CoreResult<()> {
    fs::create_dir_all(cache).map_err(|e| CoreError::io(cache, e))?;
    write_json(&cache.join(AIMS_FILE), aims)
}

/// Gives each item the asset it is aimed at, if any.
pub(super) fn fill(cache: &Path, items: &mut [InboxItem]) {
    let aims = load(cache);
    for item in items {
        item.aim = aims.get(&item.id).cloned();
    }
}

/// Aims the inbox model `id` at an asset, or forgets its aim with `None`.
///
/// # Errors
///
/// Fails for a name outside the inbox, or when the choice cannot be saved.
pub fn set_aim(
    root: &Path,
    config: &WorkspaceConfig,
    id: &str,
    aim: Option<InboxAim>,
) -> CoreResult<()> {
    check_id(id)?;
    let cache = cache_dir(root, config);
    let mut aims = load(&cache);
    let before = aims.len();
    match aim {
        Some(aim) => {
            aims.insert(id.to_string(), aim);
        }
        None => {
            aims.remove(id);
            if aims.len() == before {
                return Ok(());
            }
        }
    }
    save(&cache, &aims)
}
