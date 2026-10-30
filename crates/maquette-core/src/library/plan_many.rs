//! The plan for adding a whole project to the library: what each asset
//! with a model would become (a new entry, an entry's next version,
//! nothing because the library has the file) and what it meets on the way
//! (an entry with its name, one that looks like it, a model that cannot be
//! read). It reads only.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::add::entry_of;
use super::look_alike::{picture_of, settings};
use super::names::same_name;
use super::{LibraryEntry, files, list};
use crate::assets::{Asset, AssetKind};
use crate::error::CoreResult;
use crate::project::{self, slugify};
use crate::refs::picture::{self, Object};
use crate::workspace::FolderLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PlannedState {
    New,
    NextVersion,
    SameFile,
    NameClash,
    LooksLike,
    Unreadable,
}

/// One asset in the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlannedAdd {
    pub asset: String,
    pub name: String,
    pub kind: AssetKind,
    pub state: PlannedState,
    /// The library entry it meets: the one it would join, has the same file, the same name or the same look.
    pub entry: Option<String>,
    pub entry_name: Option<String>,
    /// How alike the two pictures are, when both read.
    pub looks: Option<f64>,
    /// The model file, relative to the project folder.
    pub file: String,
    pub triangles: Option<u32>,
    /// Why it cannot be added, for unreadable.
    pub reason: Option<String>,
}

/// What adding a project's assets to the library would do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AddManyPlan {
    pub project: String,
    pub project_name: String,
    /// The suggested project tag: the project name slugified.
    pub tag: String,
    pub check_on_add: bool,
    pub assets: Vec<PlannedAdd>,
}

/// The glTF binary header: the magic, then the declared length at byte 8.
const GLB_MAGIC: &[u8; 4] = b"glTF";

/// Why a model file's bytes cannot be added, if they cannot.
fn unreadable(file: &str, bytes: &[u8]) -> Option<String> {
    let glb = Path::new(file)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("glb"));
    if !glb {
        return None;
    }
    if bytes.get(..4) != Some(GLB_MAGIC.as_slice()) {
        return Some("The file is not a .glb model.".to_string());
    }
    let declared = bytes
        .get(8..12)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes);
    let whole = u32::try_from(bytes.len()).ok();
    if declared.is_none() || declared != whole {
        return Some("The file is cut short: it is smaller than its header says.".to_string());
    }
    None
}

/// What one asset meets in the library.
struct Library<'a> {
    entries: &'a [LibraryEntry],
    pictures: Vec<Option<Object>>,
    threshold: f64,
    check_on_add: bool,
}

impl Library<'_> {
    fn plan(&self, project_id: &str, project_dir: &Path, asset: &Asset) -> Option<PlannedAdd> {
        let latest = asset.latest()?;
        let mut out = PlannedAdd {
            asset: asset.id.clone(),
            name: asset.name.clone(),
            kind: asset.kind,
            state: PlannedState::New,
            entry: None,
            entry_name: None,
            looks: None,
            file: latest.file.clone(),
            triangles: latest.stats.as_ref().map(|s| s.triangles),
            reason: None,
        };
        let Ok(bytes) = fs::read(project_dir.join(&latest.file)) else {
            out.state = PlannedState::Unreadable;
            out.reason = Some("The model file is missing.".to_string());
            return Some(out);
        };
        if let Some(why) = unreadable(&latest.file, &bytes) {
            out.state = PlannedState::Unreadable;
            out.reason = Some(why);
            return Some(out);
        }
        let hash = files::hash_bytes(&bytes);
        let seen = asset
            .thumbnail
            .as_deref()
            .and_then(|t| picture::object_in(&project_dir.join(t)));
        let looks = |i: usize| {
            let (a, b) = (seen.as_ref()?, self.pictures[i].as_ref()?);
            Some(picture::score(a, b).total())
        };
        let mut meet = |state: PlannedState, i: usize, score: Option<f64>| {
            out.state = state;
            out.entry = Some(self.entries[i].id.clone());
            out.entry_name = Some(self.entries[i].name.clone());
            out.looks = score;
        };
        let own = entry_of(self.entries, project_id, asset);
        let index_of = |e: &LibraryEntry| self.entries.iter().position(|x| x.id == e.id);
        if let Some(i) = self
            .entries
            .iter()
            .position(|e| e.versions.iter().any(|v| v.hash == hash))
        {
            meet(PlannedState::SameFile, i, None);
        } else if let Some(i) = own.and_then(index_of) {
            meet(PlannedState::NextVersion, i, None);
        } else if let Some(i) = self
            .entries
            .iter()
            .position(|e| same_name(&e.name, &asset.name))
        {
            meet(PlannedState::NameClash, i, looks(i));
        } else if self.check_on_add
            && let Some((i, score)) = (0..self.entries.len())
                .filter_map(|i| looks(i).map(|s| (i, s)))
                .max_by(|a, b| a.1.total_cmp(&b.1))
            && score >= self.threshold
        {
            meet(PlannedState::LooksLike, i, Some(score));
        }
        Some(out)
    }
}

/// What adding every asset of `project_id` that has a model would do, in
/// the project's order.
///
/// # Errors
///
/// Fails for an unknown project or a library that cannot be read.
pub fn plan_add_many(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
) -> CoreResult<AddManyPlan> {
    let manifest = project::load(root, layout, project_id)?;
    let project_dir = project::dir(root, layout, project_id)?;
    let entries = list(root)?;
    let found = settings(root)?;
    let library = Library {
        pictures: entries.iter().map(|e| picture_of(root, e)).collect(),
        entries: &entries,
        threshold: found.threshold,
        check_on_add: found.check_on_add,
    };
    let assets = manifest
        .assets
        .iter()
        .filter_map(|a| library.plan(&manifest.id, &project_dir, a))
        .collect();
    Ok(AddManyPlan {
        project: manifest.id.clone(),
        tag: slugify(&manifest.name),
        project_name: manifest.name,
        check_on_add: found.check_on_add,
        assets,
    })
}
