//! Merging two library entries that are one model. Every version of the
//! merged entry the kept entry lacks (by file hash) is copied in as the
//! kept entry's next version, its refs come along and, when asked, its
//! tags. Project assets that came from the merged entry are pointed at the
//! kept entry's matching version, so every project keeps the model it
//! uses. The merged entry then moves to `library/.removed/` with a record
//! of what the merge did, and restoring it undoes the merge.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::merge_undo::{self, MergeRecord, OriginChange};
use super::remove::{move_folder, removed_dir};
use super::{
    LibraryEntry, LibraryVersion, MAX_TAGS, clean_tags, entry_dir, files, load, save, turn,
};
use crate::assets::AssetOrigin;
use crate::assets::copy::key;
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest, now_rfc3339};
use crate::workspace::FolderLayout;

/// What a merge asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MergeEntries {
    /// The entry that stays.
    pub keep: String,
    /// The entry that goes into it.
    pub merge: String,
    /// Whether the merged entry's tags join the kept entry's.
    pub copy_tags: bool,
}

/// One merge, as the look-alikes list shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMerge {
    pub merged: String,
    pub merged_name: String,
    pub into: String,
    pub into_name: String,
    /// The versions of `into` the merged entry's versions became.
    pub versions: Vec<u32>,
    pub at: String,
}

/// What a merge did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMerged {
    pub entry: LibraryEntry,
    pub merge: LibraryMerge,
    /// Projects whose assets now point at the kept entry.
    pub projects: Vec<ProjectManifest>,
}

/// The two entries' tags together, refused when there are too many.
fn joined_tags(keep: &LibraryEntry, gone: &LibraryEntry) -> CoreResult<Vec<String>> {
    let all: Vec<String> = keep.tags.iter().chain(&gone.tags).cloned().collect();
    clean_tags(&all).map_err(|_| {
        CoreError::Invalid(format!(
            "Together {} and {} have more than {MAX_TAGS} tags. Take some off first, or merge \
             without copying tags.",
            keep.name, gone.name
        ))
    })
}

/// Copies every version of `gone` that `keep` lacks into `keep`. Answers
/// the version of `keep` each of `gone`'s became and the ones added. On a
/// failure the files already copied are removed.
fn carry_versions(
    root: &Path,
    gone: &LibraryEntry,
    keep: &mut LibraryEntry,
) -> CoreResult<(Vec<u32>, Vec<u32>)> {
    let from = entry_dir(root, &gone.id)?;
    let to = entry_dir(root, &keep.id)?;
    let entry_key = key(&keep.id);
    let mut mapped = Vec::new();
    let mut added: Vec<LibraryVersion> = Vec::new();
    for version in &gone.versions {
        if let Some(have) = keep.versions.iter().find(|v| v.hash == version.hash) {
            mapped.push(have.version);
            continue;
        }
        let number = keep.next_version();
        let copied =
            files::copy_version(&from, &version.file, &to, &entry_key, number).and_then(|file| {
                let thumbnail = match &version.thumbnail {
                    Some(picture) => {
                        match files::copy_picture(&from, picture, &to, &entry_key, number) {
                            Ok(found) => found,
                            Err(err) => {
                                files::remove_version(&to, &file);
                                return Err(err);
                            }
                        }
                    }
                    None => None,
                };
                Ok(LibraryVersion {
                    version: number,
                    file,
                    thumbnail,
                    ..version.clone()
                })
            });
        match copied {
            Ok(next) => {
                mapped.push(number);
                added.push(next.clone());
                keep.versions.push(next);
            }
            Err(err) => {
                for done in &added {
                    merge_undo::drop_files(&to, done, keep);
                }
                return Err(err);
            }
        }
    }
    Ok((mapped, added.iter().map(|v| v.version).collect()))
}

/// The project assets that came from `gone`, each with the version of the
/// kept entry it moves to.
fn origin_changes(
    root: &Path,
    layout: &FolderLayout,
    gone: &LibraryEntry,
    mapped: &[u32],
) -> CoreResult<Vec<OriginChange>> {
    let newest = mapped.last().copied().unwrap_or(1);
    let mut changes = Vec::new();
    for manifest in project::list(root, layout)? {
        for asset in &manifest.assets {
            let Some(origin) = asset.origin.as_ref().filter(|o| o.entry == gone.id) else {
                continue;
            };
            let to = gone
                .versions
                .iter()
                .position(|v| v.version == origin.version)
                .and_then(|i| mapped.get(i).copied())
                .unwrap_or(newest);
            changes.push(OriginChange {
                project: manifest.id.clone(),
                asset: asset.id.clone(),
                from: origin.version,
                to,
            });
        }
    }
    Ok(changes)
}

/// Points each changed asset's origin at `entry`, `to` when `forward`,
/// else back at `entry` and `from`. Only assets whose origin is still the
/// other side's are changed. Answers the projects written.
pub(super) fn repoint(
    root: &Path,
    layout: &FolderLayout,
    changes: &[OriginChange],
    (from_entry, to_entry): (&str, &str),
    forward: bool,
) -> CoreResult<Vec<ProjectManifest>> {
    let mut by_project: BTreeMap<&str, Vec<&OriginChange>> = BTreeMap::new();
    for change in changes {
        by_project.entry(&change.project).or_default().push(change);
    }
    let mut written = Vec::new();
    for (id, list) in by_project {
        let manifest = project::modify(root, layout, id, |m| {
            for change in &list {
                let (was, now) = if forward {
                    (change.from, change.to)
                } else {
                    (change.to, change.from)
                };
                let Some(asset) = m.assets.iter_mut().find(|a| a.id == change.asset) else {
                    continue;
                };
                let current = AssetOrigin {
                    entry: from_entry.to_string(),
                    version: was,
                };
                if asset.origin.as_ref() == Some(&current) {
                    asset.origin = Some(AssetOrigin {
                        entry: to_entry.to_string(),
                        version: now,
                    });
                }
            }
            Ok(())
        });
        match manifest {
            Ok(m) => written.push(m),
            Err(err) => {
                tracing::warn!(project = id, error = %err, "could not repoint library origins")
            }
        }
    }
    Ok(written)
}

/// Merges `request.merge` into `request.keep`.
///
/// # Errors
///
/// Fails for an unknown entry, an entry merged into itself, too many tags
/// together, or a file that cannot be copied. A failure before the kept
/// entry is saved leaves the library as it was.
pub fn merge(
    root: &Path,
    layout: &FolderLayout,
    request: &MergeEntries,
) -> CoreResult<LibraryMerged> {
    if request.keep == request.merge {
        return Err(CoreError::Invalid(
            "Pick two different models to merge.".to_string(),
        ));
    }
    let _turn = turn();
    let mut keep = load(root, &request.keep)?;
    let gone = load(root, &request.merge)?;
    let before = keep.clone();
    if request.copy_tags {
        keep.tags = joined_tags(&keep, &gone)?;
    }
    let (mapped, added) = carry_versions(root, &gone, &mut keep)?;
    let to = entry_dir(root, &keep.id)?;
    let saved = (|| {
        let from = entry_dir(root, &gone.id)?;
        for r in files::copy_refs(&from, &gone.refs, &to)? {
            if !keep.refs.contains(&r) {
                keep.refs.push(r);
            }
        }
        keep.updated_at = now_rfc3339();
        save(root, &keep)
    })();
    if let Err(err) = saved {
        for version in keep.versions.iter().filter(|v| added.contains(&v.version)) {
            merge_undo::drop_files(&to, version, &before);
        }
        return Err(err);
    }
    let origins = origin_changes(root, layout, &gone, &mapped)?;
    let record = MergeRecord {
        version: merge_undo::RECORD_VERSION,
        merge: LibraryMerge {
            merged: gone.id.clone(),
            merged_name: gone.name.clone(),
            into: keep.id.clone(),
            into_name: keep.name.clone(),
            versions: mapped,
            at: now_rfc3339(),
        },
        added,
        tags_before: before.tags,
        refs_before: before.refs,
        origins,
    };
    let waiting = removed_dir(root, &gone.id)?;
    move_folder(&entry_dir(root, &gone.id)?, &waiting)?;
    merge_undo::write(&waiting, &record)?;
    let projects = repoint(root, layout, &record.origins, (&gone.id, &keep.id), true)?;
    Ok(LibraryMerged {
        entry: keep,
        merge: record.merge,
        projects,
    })
}
