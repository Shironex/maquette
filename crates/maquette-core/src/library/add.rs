//! Add to library: copies a project asset's newest model, its picture, its
//! refs and its settings into the library. The same asset added again with
//! a model the entry does not have yet becomes the entry's next version.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    LibraryEntry, LibraryVersion, VersionFrom, clean_tags, entry_dir, files, list, load, save,
    source, turn,
};
use crate::assets::copy::{key, refs_of};
use crate::assets::{self, Asset, AssetOrigin};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest, now_rfc3339, slugify};
use crate::providers::ProviderJob;
use crate::workspace::FolderLayout;
use crate::{materials, prepare};

/// What Add to library asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AddToLibrary {
    pub project: String,
    pub asset: String,
    /// The entry's name; left out, the asset's (or the entry's, when it has one).
    pub name: Option<String>,
    /// Replaces the entry's tags; left out, they stay.
    pub tags: Option<Vec<String>>,
    /// Adds the model as the next version of this entry instead of the one
    /// the asset belongs to.
    pub into: Option<String>,
}

/// What Add to library did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryAdded {
    pub entry: LibraryEntry,
    /// The library version the asset's newest model is now.
    pub version: u32,
    /// False when the library already had this exact model.
    pub added: bool,
    /// The project, with the asset's origin set.
    pub project: ProjectManifest,
}

/// The entry this asset belongs to: the one it came from, else the one an
/// earlier add of it made.
pub(super) fn entry_of<'a>(
    entries: &'a [LibraryEntry],
    project: &str,
    asset: &Asset,
) -> Option<&'a LibraryEntry> {
    let by_origin = asset
        .origin
        .as_ref()
        .and_then(|o| entries.iter().find(|e| e.id == o.entry));
    by_origin.or_else(|| {
        entries.iter().find(|e| {
            e.versions
                .iter()
                .any(|v| v.from.project == project && v.from.asset == asset.id)
        })
    })
}

fn free_id(entries: &[LibraryEntry], root: &Path, name: &str) -> String {
    let base = slugify(name);
    let taken = |id: &str| {
        entries.iter().any(|e| e.id == id) || entry_dir(root, id).is_ok_and(|d| d.exists())
    };
    if !taken(&base) {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !taken(id))
        .unwrap_or(base)
}

fn new_entry(entries: &[LibraryEntry], root: &Path, asset: &Asset, name: &str) -> LibraryEntry {
    let now = now_rfc3339();
    LibraryEntry {
        version: super::ENTRY_VERSION,
        id: free_id(entries, root, name),
        name: name.to_string(),
        kind: asset.kind,
        tags: Vec::new(),
        refs: Vec::new(),
        versions: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
    }
}

/// Adds the newest model of `request.asset` to the library. `jobs` are the
/// project's sends: they say what made the model, what it cost and which
/// refs it was sent with. With `request.into` the model joins that entry,
/// whichever entry the asset belonged to, and the asset's origin follows.
///
/// # Errors
///
/// Fails for an unknown project or asset, an asset with no model yet, a
/// missing model file, bad tags or a file that cannot be copied. Nothing
/// is left half written.
pub fn add(
    root: &Path,
    layout: &FolderLayout,
    request: &AddToLibrary,
    jobs: &[ProviderJob],
) -> CoreResult<LibraryAdded> {
    let manifest = project::load(root, layout, &request.project)?;
    let project_dir = project::dir(root, layout, &request.project)?;
    let asset = assets::find(&manifest, &request.asset)?.clone();
    let latest = asset.latest().cloned().ok_or_else(|| {
        CoreError::Invalid(format!(
            "{} has no model yet. Accept or build one first, then add it to the library.",
            asset.name
        ))
    })?;
    let model_hash = files::hash(&project_dir.join(&latest.file))?;
    let tags = request.tags.as_deref().map(clean_tags).transpose()?;
    let name = request
        .name
        .as_deref()
        .map(assets::clean_name)
        .transpose()?;

    let _turn = turn();
    let entries = list(root)?;
    let chosen = match &request.into {
        Some(id) => Some(load(root, id)?),
        None => entry_of(&entries, &request.project, &asset).cloned(),
    };
    let mut entry = chosen.unwrap_or_else(|| {
        new_entry(
            &entries,
            root,
            &asset,
            name.as_deref().unwrap_or(&asset.name),
        )
    });
    if let Some(name) = name {
        entry.name = name;
    }
    if let Some(tags) = tags {
        entry.tags = tags;
    }
    let known = entry
        .versions
        .iter()
        .find(|v| v.hash == model_hash)
        .map(|v| v.version);
    let (version, added) = match known {
        Some(version) => (version, false),
        None => {
            let version = add_version(
                root,
                &request.project,
                &project_dir,
                &mut entry,
                &asset,
                jobs,
                model_hash,
            )?;
            (version, true)
        }
    };
    entry.updated_at = now_rfc3339();
    save(root, &entry)?;

    let origin = AssetOrigin {
        entry: entry.id.clone(),
        version,
    };
    let project = project::modify(root, layout, &request.project, |m| {
        if let Some(a) = m.assets.iter_mut().find(|a| a.id == asset.id) {
            a.origin = Some(origin.clone());
        }
        Ok(())
    })?;
    Ok(LibraryAdded {
        entry,
        version,
        added,
        project,
    })
}

/// Copies the asset's newest model, picture and refs into the entry and
/// records them as its next version, with the asset's settings.
fn add_version(
    root: &Path,
    project_id: &str,
    project_dir: &Path,
    entry: &mut LibraryEntry,
    asset: &Asset,
    jobs: &[ProviderJob],
    model_hash: String,
) -> CoreResult<u32> {
    let latest = asset
        .latest()
        .ok_or_else(|| CoreError::Invalid(format!("{} has no model yet.", asset.name)))?;
    let folder = entry_dir(root, &entry.id)?;
    let number = entry.next_version();
    let entry_key = key(&entry.id);
    let file = files::copy_version(project_dir, &latest.file, &folder, &entry_key, number)?;
    let copied = (|| {
        let thumbnail = match &asset.thumbnail {
            Some(picture) => {
                files::copy_picture(project_dir, picture, &folder, &entry_key, number)?
            }
            None => None,
        };
        let sent: Vec<String> = jobs
            .iter()
            .filter(|j| j.asset == asset.id)
            .flat_map(|j| j.images.clone())
            .collect();
        let refs = files::copy_refs(project_dir, &refs_of(project_dir, asset, &sent), &folder)?;
        CoreResult::Ok((thumbnail, refs))
    })();
    let (thumbnail, refs) = match copied {
        Ok(found) => found,
        Err(err) => {
            files::remove_version(&folder, &file);
            return Err(err);
        }
    };
    for r in refs {
        if !entry.refs.contains(&r) {
            entry.refs.push(r);
        }
    }
    let (made_with, credits) = source::detect(project_dir, asset, latest.version, jobs);
    let prep = prepare::load(project_dir)
        .ok()
        .and_then(|p| p.models.get(&asset.id).cloned())
        .filter(|p| !p.is_empty());
    let looks = materials::load(project_dir)
        .ok()
        .and_then(|m| m.models.get(&asset.id).cloned())
        .filter(|m| !m.is_empty());
    entry.versions.push(LibraryVersion {
        version: number,
        file,
        created_at: now_rfc3339(),
        stats: latest.stats.clone(),
        source: made_with,
        credits,
        hash: model_hash,
        from: VersionFrom {
            project: project_id.to_string(),
            asset: asset.id.clone(),
            version: latest.version,
        },
        thumbnail,
        prep,
        materials: looks,
    });
    Ok(number)
}
