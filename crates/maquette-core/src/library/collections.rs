//! Collections: named groups of library entries, kept in
//! `library/collections.json`. A collection only lists entry ids, so an
//! entry can sit in several, and deleting a collection never touches a
//! model. A removed entry keeps its place in them, so restoring it puts it
//! back where it was.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::tags::unique;
use super::{dir, load, turn};
use crate::assets;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::{now_rfc3339, slugify};

pub(super) const COLLECTIONS_FILE: &str = "collections.json";
const COLLECTIONS_VERSION: u32 = 1;

/// One collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryCollection {
    pub id: String,
    pub name: String,
    /// Entry ids, in the order they joined.
    pub entries: Vec<String>,
    pub created_at: String,
}

/// One change to the collections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum CollectionChange {
    Create {
        name: String,
        entries: Vec<String>,
    },
    Add {
        collection: String,
        entries: Vec<String>,
    },
    Remove {
        collection: String,
        entries: Vec<String>,
    },
    Rename {
        collection: String,
        name: String,
    },
    Delete {
        collection: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct CollectionsFile {
    version: u32,
    collections: Vec<LibraryCollection>,
}

fn file(root: &Path) -> PathBuf {
    dir(root).join(COLLECTIONS_FILE)
}

fn read(root: &Path) -> CoreResult<Vec<LibraryCollection>> {
    let path = file(root);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&path, err)),
    };
    let found: CollectionsFile =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&path, e))?;
    if found.version > COLLECTIONS_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: found.version,
            supported: COLLECTIONS_VERSION,
        });
    }
    Ok(found.collections)
}

fn sorted(mut all: Vec<LibraryCollection>) -> Vec<LibraryCollection> {
    all.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    all
}

/// Every collection, by name.
///
/// # Errors
///
/// Fails when `collections.json` cannot be read.
pub fn collections(root: &Path) -> CoreResult<Vec<LibraryCollection>> {
    Ok(sorted(read(root)?))
}

fn find<'a>(all: &'a mut [LibraryCollection], id: &str) -> CoreResult<&'a mut LibraryCollection> {
    all.iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| CoreError::NotFound(format!("the collection {id}")))
}

/// The cleaned name, refused when another collection has it already.
fn free_name(all: &[LibraryCollection], name: &str, except: Option<&str>) -> CoreResult<String> {
    let name =
        assets::clean_name(name).map_err(|_| CoreError::Invalid(collection_name_rule(name)))?;
    let taken = all
        .iter()
        .any(|c| Some(c.id.as_str()) != except && c.name.to_lowercase() == name.to_lowercase());
    if taken {
        return Err(CoreError::Invalid(format!(
            "There is a collection called {name} already."
        )));
    }
    Ok(name)
}

fn collection_name_rule(name: &str) -> String {
    if name.trim().is_empty() {
        "Give the collection a name.".to_string()
    } else {
        format!(
            "Keep the collection name to {} characters or fewer.",
            assets::ASSET_NAME_MAX_CHARS
        )
    }
}

fn free_id(all: &[LibraryCollection], name: &str) -> String {
    let base = slugify(name);
    let taken = |id: &str| all.iter().any(|c| c.id == id);
    if !taken(&base) {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !taken(id))
        .unwrap_or(base)
}

/// Checks that every id names a library entry.
fn known(root: &Path, entries: &[String]) -> CoreResult<Vec<String>> {
    let ids = unique(entries);
    for id in &ids {
        load(root, id)?;
    }
    Ok(ids)
}

/// Applies one change and answers every collection after it, by name.
///
/// # Errors
///
/// Fails for an unknown collection or entry, an empty, too long or taken
/// name, or a file that cannot be written. Nothing is written then.
pub fn change_collection(
    root: &Path,
    change: &CollectionChange,
) -> CoreResult<Vec<LibraryCollection>> {
    let _turn = turn();
    let mut all = read(root)?;
    match change {
        CollectionChange::Create { name, entries } => {
            let name = free_name(&all, name, None)?;
            let entries = known(root, entries)?;
            let id = free_id(&all, &name);
            all.push(LibraryCollection {
                id,
                name,
                entries,
                created_at: now_rfc3339(),
            });
        }
        CollectionChange::Add {
            collection,
            entries,
        } => {
            let entries = known(root, entries)?;
            let found = find(&mut all, collection)?;
            for id in entries {
                if !found.entries.contains(&id) {
                    found.entries.push(id);
                }
            }
        }
        CollectionChange::Remove {
            collection,
            entries,
        } => {
            let found = find(&mut all, collection)?;
            found.entries.retain(|id| !entries.contains(id));
        }
        CollectionChange::Rename { collection, name } => {
            let name = free_name(&all, name, Some(collection))?;
            find(&mut all, collection)?.name = name;
        }
        CollectionChange::Delete { collection } => {
            find(&mut all, collection)?;
            all.retain(|c| &c.id != collection);
        }
    }
    let folder = dir(root);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    let saved = CollectionsFile {
        version: COLLECTIONS_VERSION,
        collections: all,
    };
    write_json(&file(root), &saved)?;
    Ok(sorted(saved.collections))
}
