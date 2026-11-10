//! Saving a grip to the library, as a new grip or as the next revision of
//! one. It never overwrites: each save adds a revision. The person asks for
//! the plan first (where it goes, how big, what changes against the newest
//! revision) and confirms it; the save sends the entry version the plan
//! read, so a grip that moved since is refused (code `moved`). Agents only
//! propose a save; the app's confirm writes.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    ENTRY_VERSION, LibraryGripEntry, LibraryGripRevision, check_expected, free_entry_id,
    load_entry, names, read_revision, revision_file, revision_path, turn, write_revision,
};
use crate::animations::recipes::library::link::new_uid;
use crate::error::{CoreError, CoreResult};
use crate::project::now_rfc3339;
use crate::recipes::grips::{self, GripFile};
use crate::recipes::unique_name;

/// Where a grip was saved from: a pose key change of a project's recipe,
/// at one frame of its bake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GripSource {
    pub project: String,
    pub recipe: String,
    pub layer: String,
    pub frame: u32,
}

/// What a save would do, for the save sheet and an agent's proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GripSavePlan {
    /// The library grip it goes to; none for a new one.
    pub entry: Option<String>,
    /// The library grip's name, or the name a new one gets.
    pub entry_name: String,
    /// The library grip's version as read; the save sends it back.
    pub entry_version: Option<String>,
    /// Its newest revision now; none for a new one.
    pub latest: Option<u32>,
    /// The revision the save writes.
    pub revision: u32,
    /// Where the file goes: `library/grips/<id>/r<n>.grip.json`. A new
    /// grip's id is the one free now, which the save may still change.
    pub file: String,
    /// How big the file is, bytes.
    pub bytes: u32,
    /// The grip as it would be written.
    pub grip: GripFile,
    /// What changes against the newest revision, a plain line each.
    pub changes: Vec<String>,
    /// The grip holds what the newest revision holds: nothing to save.
    pub same: bool,
}

/// A confirmed save.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GripSave {
    /// The grip to save. Its revision is set by the save; saved as a
    /// revision of `entry`, it takes that grip's name.
    pub grip: GripFile,
    /// The library grip to add a revision to; none makes a new one.
    pub entry: Option<String>,
    /// The entry version the plan read. Needed with `entry`.
    pub expected: Option<String>,
    /// Where it was saved from, kept with the revision.
    pub source: Option<GripSource>,
}

/// What a save wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GripSaved {
    pub entry: String,
    pub name: String,
    pub revision: u32,
    /// The library grip's version after the save.
    pub entry_version: String,
}

fn plural(n: usize) -> String {
    if n == 1 {
        "1 bone".to_string()
    } else {
        format!("{n} bones")
    }
}

/// What `after` changes against `before`, a line each; for no `before` (a
/// new grip), one line saying what it holds. Empty when they pose the same
/// hands the same way and hold the same weapon.
pub fn changes(before: Option<&GripFile>, after: &GripFile) -> Vec<String> {
    let Some(before) = before else {
        return vec![format!(
            "A new grip for {}: {}.",
            grips::hands_words(after),
            plural(after.pose.len())
        )];
    };
    let mut out = Vec::new();
    if before.weapon != after.weapon {
        out.push(format!(
            "Holds {} instead of {}.",
            after.weapon, before.weapon
        ));
    }
    if before.hands != after.hands {
        out.push(format!(
            "Poses {}, where it posed {}.",
            grips::hands_words(after),
            grips::hands_words(before)
        ));
    }
    let added = after
        .pose
        .keys()
        .filter(|b| !before.pose.contains_key(*b))
        .count();
    let removed = before
        .pose
        .keys()
        .filter(|b| !after.pose.contains_key(*b))
        .count();
    let turned = after
        .pose
        .iter()
        .filter(|(bone, turn)| before.pose.get(*bone).is_some_and(|old| old != *turn))
        .count();
    if added > 0 {
        out.push(format!("Poses {} more.", plural(added)));
    }
    if removed > 0 {
        out.push(format!("Leaves {} out.", plural(removed)));
    }
    if turned > 0 {
        out.push(format!("Turns {} another way.", plural(turned)));
    }
    out
}

fn size_of(grip: &GripFile) -> CoreResult<u32> {
    Ok(u32::try_from(grips::text(grip)?.len()).unwrap_or(u32::MAX))
}

/// What saving `grip` to `entry` (a new library grip when none) would do.
/// Saved as a revision, it takes the entry's name.
///
/// # Errors
///
/// Fails for a grip the file checks refuse, a weapon no grip holds, an
/// unknown entry or unreadable files.
pub fn plan_save(root: &Path, grip: &GripFile, entry: Option<&str>) -> CoreResult<GripSavePlan> {
    let mut grip = grips::normalized(grip);
    grips::check_weapon(&grip.weapon)?;
    let Some(id) = entry else {
        grip.revision = 1;
        grip.name = unique_name(&grip.name, &names(root));
        grips::validate(&grip)?;
        return Ok(GripSavePlan {
            entry: None,
            entry_name: grip.name.clone(),
            entry_version: None,
            latest: None,
            revision: 1,
            file: revision_path(&free_entry_id(root, &grip.name)?, 1),
            bytes: size_of(&grip)?,
            changes: changes(None, &grip),
            same: false,
            grip,
        });
    };
    let (found, version) = load_entry(root, id)?;
    let newest = read_revision(root, &found, None)?;
    grip.name.clone_from(&found.name);
    grip.revision = found.last_revision.max(newest.revision) + 1;
    grips::validate(&grip)?;
    Ok(GripSavePlan {
        entry: Some(found.id.clone()),
        entry_name: found.name.clone(),
        entry_version: Some(version),
        latest: Some(newest.revision),
        revision: grip.revision,
        file: revision_path(&found.id, grip.revision),
        bytes: size_of(&grip)?,
        changes: changes(Some(&newest), &grip),
        same: grips::same_pose(&newest, &grip),
        grip,
    })
}

/// Saves the grip as the person confirmed: a new library grip, or the next
/// revision of `save.entry`.
///
/// # Errors
///
/// Fails for a grip the file checks refuse, a weapon no grip holds, an
/// unknown entry, one that moved since the plan (code `moved`), an entry
/// save without `expected`, a grip that holds what the newest revision
/// holds, or files that cannot be written.
pub fn save_grip(root: &Path, save: &GripSave) -> CoreResult<GripSaved> {
    let mut grip = grips::normalized(&save.grip);
    grips::check_weapon(&grip.weapon)?;
    let now = now_rfc3339();
    let source = |revision: u32| LibraryGripRevision {
        revision,
        file: revision_file(revision),
        saved_at: now.clone(),
        project: save.source.as_ref().map(|s| s.project.clone()),
        recipe: save.source.as_ref().map(|s| s.recipe.clone()),
        layer: save.source.as_ref().map(|s| s.layer.clone()),
        frame: save.source.as_ref().map(|s| s.frame),
    };
    let entry = {
        let _turn = turn();
        let entry = match &save.entry {
            Some(id) => {
                let (mut found, version) = load_entry(root, id)?;
                let Some(expected) = save.expected.as_deref() else {
                    return Err(CoreError::Invalid(format!(
                        "Saving a revision of {} needs the version its plan read.",
                        found.name
                    )));
                };
                check_expected(id, &version, Some(expected))?;
                let newest = read_revision(root, &found, None)?;
                if grips::same_pose(&newest, &grip) {
                    return Err(CoreError::Invalid(format!(
                        "The grip holds what r{} of {} holds already, so there is nothing to save.",
                        newest.revision, found.name
                    )));
                }
                let next = found.last_revision.max(newest.revision) + 1;
                found.revisions.push(source(next));
                found.last_revision = next;
                found.updated_at.clone_from(&now);
                grip.name.clone_from(&found.name);
                found
            }
            None => {
                // A taken name reads `name (2)`, in the list and in the file.
                grip.name = unique_name(&grip.name, &names(root));
                LibraryGripEntry {
                    version: ENTRY_VERSION,
                    id: free_entry_id(root, &grip.name)?,
                    uid: new_uid(&grip.name),
                    name: grip.name.clone(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    last_revision: 1,
                    revisions: vec![source(1)],
                }
            }
        };
        grip.revision = entry.last_revision;
        write_revision(root, &entry, &grip)?;
        entry
    };
    Ok(GripSaved {
        entry_version: load_entry(root, &entry.id)?.1,
        name: entry.name,
        revision: entry.last_revision,
        entry: entry.id,
    })
}
