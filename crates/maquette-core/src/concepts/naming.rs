//! Which folder under `concepts/` a direction's name goes to.

use std::path::Path;

use super::list_directions;

/// The folder name for a direction called `name`. A name with nothing a
/// slug keeps gets its own stable id from its text, since every such name
/// would otherwise share `project`.
pub fn direction_id(name: &str) -> String {
    crate::project::latin_slug(name).map_or_else(
        || {
            let hash = name.trim().bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
            });
            format!("direction-{hash:016x}")
        },
        crate::project::windows_safe,
    )
}

/// The folder of a direction already called `name`, which an older id rule
/// may have put elsewhere (`con`, or `project` for a name in another script).
pub(super) fn existing_id(project_dir: &Path, name: &str) -> Option<String> {
    let name = name.trim();
    list_directions(project_dir)
        .into_iter()
        .find(|d| d.name.trim() == name)
        .map(|d| d.id)
}
