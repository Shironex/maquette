//! Undoing a merge. The merged entry's folder waits in `library/.removed/`
//! with `merged.json`, the record of what the merge did; restoring that
//! entry takes the versions the merge added off the kept entry (while they
//! are still its newest and no other project uses them), puts the kept
//! entry's tags and refs back, moves the folder back and points the
//! project assets back at it.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::merge::{LibraryMerge, repoint};
use super::remove::{REMOVED_DIR, move_folder, removed_dir};
use super::{LibraryEntry, LibraryVersion, dir, entry_dir, files, load, save};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::{self, ProjectManifest, now_rfc3339};
use crate::workspace::FolderLayout;

/// The record a merge leaves in the merged entry's removed folder.
pub(super) const MERGE_RECORD: &str = "merged.json";
pub(super) const RECORD_VERSION: u32 = 1;
/// How many recent merges the look-alikes list shows.
const RECENT: usize = 5;

/// One project asset a merge pointed at the kept entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OriginChange {
    pub project: String,
    pub asset: String,
    /// The merged entry's version it was on.
    pub from: u32,
    /// The kept entry's version it moved to.
    pub to: u32,
}

/// `merged.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MergeRecord {
    pub version: u32,
    pub merge: LibraryMerge,
    /// The versions the merge added to the kept entry.
    pub added: Vec<u32>,
    pub tags_before: Vec<String>,
    pub refs_before: Vec<String>,
    pub origins: Vec<OriginChange>,
}

pub(super) fn write(folder: &Path, record: &MergeRecord) -> CoreResult<()> {
    write_json(&folder.join(MERGE_RECORD), record)
}

fn read(folder: &Path) -> CoreResult<MergeRecord> {
    let path = folder.join(MERGE_RECORD);
    let text = fs::read_to_string(&path).map_err(|e| CoreError::io(&path, e))?;
    let record: MergeRecord = serde_json::from_str(&text).map_err(|e| CoreError::json(&path, e))?;
    if record.version > RECORD_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: record.version,
            supported: RECORD_VERSION,
        });
    }
    Ok(record)
}

/// The latest merges still waiting in `.removed/`, newest first.
pub(super) fn recent(root: &Path) -> Vec<LibraryMerge> {
    let parent = dir(root).join(REMOVED_DIR);
    let read_dir = match fs::read_dir(&parent) {
        Ok(found) => found,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            tracing::warn!(dir = %parent.display(), error = %err, "could not read removed entries");
            return Vec::new();
        }
    };
    let mut merges: Vec<LibraryMerge> = read_dir
        .filter_map(Result::ok)
        .filter(|f| f.path().join(MERGE_RECORD).is_file())
        .filter_map(|f| read(&f.path()).ok())
        .map(|r| r.merge)
        .collect();
    merges.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| a.merged.cmp(&b.merged)));
    merges.truncate(RECENT);
    merges
}

/// Removes a version's model file, and its picture when no other version
/// or ref of `others` names it.
pub(super) fn drop_files(folder: &Path, version: &LibraryVersion, others: &LibraryEntry) {
    files::remove_version(folder, &version.file);
    let Some(picture) = &version.thumbnail else {
        return;
    };
    let named = others.refs.contains(picture)
        || others
            .versions
            .iter()
            .any(|v| v.version != version.version && v.thumbnail.as_ref() == Some(picture));
    if !named && let Err(err) = fs::remove_file(folder.join(picture)) {
        tracing::warn!(file = %picture, error = %err, "could not remove a merged picture");
    }
}

/// Why the merge that removed `id` cannot be undone, if it cannot.
///
/// # Errors
///
/// Names the reason: the kept entry is gone, has newer versions, or a
/// project uses a version the merge added.
pub(super) fn check(root: &Path, layout: &FolderLayout, id: &str) -> CoreResult<()> {
    let record = read(&removed_dir(root, id)?)?;
    let keep = load(root, &record.merge.into)?;
    let newest: Vec<u32> = keep
        .versions
        .iter()
        .rev()
        .take(record.added.len())
        .rev()
        .map(|v| v.version)
        .collect();
    if newest != record.added {
        return Err(CoreError::Invalid(format!(
            "{} has newer versions than the ones the merge added, so the merge of {} cannot be \
             undone.",
            keep.name, record.merge.merged_name
        )));
    }
    for manifest in project::list(root, layout)? {
        for asset in &manifest.assets {
            let Some(origin) = asset.origin.as_ref().filter(|o| o.entry == keep.id) else {
                continue;
            };
            let ours = record
                .origins
                .iter()
                .any(|c| c.project == manifest.id && c.asset == asset.id);
            if !ours && record.added.contains(&origin.version) {
                return Err(CoreError::Invalid(format!(
                    "{} in {} uses version {} of {}, which came with the merge. Update or remove \
                     it, then undo the merge.",
                    asset.name, manifest.name, origin.version, keep.name
                )));
            }
        }
    }
    Ok(())
}

/// Undoes the merge that removed `id`. Answers the projects whose origins
/// were pointed back.
///
/// # Errors
///
/// Fails when [`check`] does, or a file cannot be written or moved.
pub(super) fn undo(
    root: &Path,
    layout: &FolderLayout,
    id: &str,
) -> CoreResult<Vec<ProjectManifest>> {
    check(root, layout, id)?;
    let waiting = removed_dir(root, id)?;
    let record = read(&waiting)?;
    let mut keep = load(root, &record.merge.into)?;
    let folder = entry_dir(root, &keep.id)?;
    let (gone, stays): (Vec<LibraryVersion>, Vec<LibraryVersion>) = keep
        .versions
        .iter()
        .cloned()
        .partition(|v| record.added.contains(&v.version));
    keep.versions = stays;
    let added_refs: Vec<String> = keep
        .refs
        .iter()
        .filter(|r| !record.refs_before.contains(r))
        .cloned()
        .collect();
    keep.refs.clone_from(&record.refs_before);
    keep.tags.clone_from(&record.tags_before);
    keep.updated_at = now_rfc3339();
    save(root, &keep)?;
    for version in &gone {
        drop_files(&folder, version, &keep);
    }
    for r in added_refs {
        let named = keep
            .versions
            .iter()
            .any(|v| v.thumbnail.as_ref() == Some(&r));
        if !named && let Err(err) = fs::remove_file(folder.join(&r)) {
            tracing::warn!(file = %r, error = %err, "could not remove a merged ref");
        }
    }
    move_folder(&waiting, &entry_dir(root, id)?)?;
    let back = entry_dir(root, id)?.join(MERGE_RECORD);
    if let Err(err) = fs::remove_file(&back) {
        tracing::warn!(file = %back.display(), error = %err, "could not remove a merge record");
    }
    repoint(root, layout, &record.origins, (&keep.id, id), false)
}
