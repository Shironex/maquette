//! The Library view: every entry with where it is used, read from the
//! projects' asset origins, and what reuse saved.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{LibraryEntry, dir, list};
use crate::error::CoreResult;
use crate::game::{GameFormat, ProjectTarget};
use crate::project;
use crate::workspace::FolderLayout;

/// One project asset that came from an entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryUse {
    pub project: String,
    pub project_name: String,
    pub target: ProjectTarget,
    pub asset: String,
    pub asset_name: String,
    /// The library version it is on.
    pub version: u32,
    /// True when the entry has a newer version.
    pub update: bool,
}

/// One entry as the Library view shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub entry: LibraryEntry,
    /// The entry's folder, absolute, for its pictures.
    pub dir: String,
    /// The newest version's format, when Maquette knows it.
    pub format: Option<GameFormat>,
    pub uses: Vec<LibraryUse>,
    /// Credits its versions cost to make, as far as the app knows.
    pub spent: f64,
    /// Credits the copies saved: what the version each one uses cost, for
    /// every use that is not the asset the version came from.
    pub saved: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryView {
    /// The library folder, absolute.
    pub dir: String,
    pub items: Vec<LibraryItem>,
}

fn item(root: &Path, entry: LibraryEntry, uses: Vec<LibraryUse>) -> LibraryItem {
    let credits_of = |v: u32| entry.find(v).and_then(|v| v.credits).unwrap_or(0.0);
    let spent = entry.versions.iter().filter_map(|v| v.credits).sum();
    let saved = uses
        .iter()
        .filter(|u| {
            !entry
                .versions
                .iter()
                .any(|v| v.from.project == u.project && v.from.asset == u.asset)
        })
        .map(|u| credits_of(u.version))
        .sum();
    let format = entry.latest().and_then(|v| GameFormat::of(&v.file));
    LibraryItem {
        dir: dir(root).join(&entry.id).display().to_string(),
        entry,
        format,
        uses,
        spent,
        saved,
    }
}

/// The whole library with every project's uses.
///
/// # Errors
///
/// Fails when the library or projects folder cannot be read.
pub fn view(root: &Path, layout: &FolderLayout) -> CoreResult<LibraryView> {
    let entries = list(root)?;
    let projects = project::list(root, layout)?;
    let items = entries
        .into_iter()
        .map(|entry| {
            let latest = entry.latest().map_or(0, |v| v.version);
            let uses = projects
                .iter()
                .flat_map(|p| p.assets.iter().map(move |a| (p, a)))
                .filter_map(|(p, a)| {
                    let origin = a.origin.as_ref().filter(|o| o.entry == entry.id)?;
                    Some(LibraryUse {
                        project: p.id.clone(),
                        project_name: p.name.clone(),
                        target: p.target,
                        asset: a.id.clone(),
                        asset_name: a.name.clone(),
                        version: origin.version,
                        update: origin.version < latest,
                    })
                })
                .collect();
            item(root, entry, uses)
        })
        .collect();
    Ok(LibraryView {
        dir: dir(root).display().to_string(),
        items,
    })
}
