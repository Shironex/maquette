//! Draft marks: what an agent changed in a recipe since a person last saved
//! it. An agent's write lands in the recipe file at once and bakes as it
//! is; the marks only tell the editor which changes and keys to show as
//! drafted, until a person saves the recipe there. They live beside the
//! recipe in the project, `recipes/<id>.drafts.json`, never in the recipe
//! file, so an export or a library save never carries them.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{RecipeFile, is_recipe_id};
use crate::error::{CoreError, CoreResult};

/// What follows a recipe's id in its marks file.
pub const DRAFTS_SUFFIX: &str = ".drafts.json";

/// A pose key an agent added or changed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DraftKey {
    /// The change it sits on.
    pub layer: String,
    pub clip: u32,
    pub frame: u32,
}

/// What agents changed in a recipe since a person saved it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct RecipeDrafts {
    /// The agent of the newest write, as people see its name.
    pub by: String,
    /// When it wrote, RFC 3339.
    pub at: String,
    /// How many agent writes the marks gather.
    pub writes: u32,
    /// An agent made the recipe.
    pub made: bool,
    /// Changes an agent added or changed, by id.
    pub layers: Vec<String>,
    /// Pose keys an agent added or changed.
    pub keys: Vec<DraftKey>,
    /// An agent changed the clips (which, their trims or blends).
    pub clips: bool,
    /// An agent changed the time edits.
    pub time: bool,
    /// An agent changed the blanks.
    pub blanks: bool,
    /// An agent turned the mirror on or off.
    pub mirror: bool,
}

impl RecipeDrafts {
    /// Nothing is marked.
    pub fn is_empty(&self) -> bool {
        !self.made
            && self.layers.is_empty()
            && self.keys.is_empty()
            && !(self.clips || self.time || self.blanks || self.mirror)
    }
}

/// What changed from `before` (none for a recipe just made) to `after`,
/// with no name or time.
pub fn diff(before: Option<&RecipeFile>, after: &RecipeFile) -> RecipeDrafts {
    let Some(before) = before else {
        // Everything in a recipe an agent made is its, but the clips it
        // was made on.
        let mut made = diff(Some(&RecipeFile::new("", after.base.clone())), after);
        made.made = true;
        made.clips = false;
        return made;
    };
    let mut out = RecipeDrafts::default();
    for layer in &after.layers {
        let old = before.layers.iter().find(|l| l.id == layer.id);
        if old != Some(layer) {
            out.layers.push(layer.id.clone());
        }
        for key in &layer.keys {
            let same = old.is_some_and(|o| o.keys.contains(key));
            if !same {
                out.keys.push(DraftKey {
                    layer: layer.id.clone(),
                    clip: key.clip,
                    frame: key.frame,
                });
            }
        }
    }
    out.clips = !before.clips().eq(after.clips());
    out.time = before.time != after.time;
    out.blanks = before.blanks != after.blanks;
    out.mirror = before.mirror != after.mirror;
    out
}

fn union<T: Ord + Clone>(a: &mut Vec<T>, b: &[T]) {
    a.extend(b.iter().cloned());
    a.sort();
    a.dedup();
}

/// `old` with `new` gathered in: the newest agent and time, every mark.
pub fn merged(old: Option<RecipeDrafts>, new: RecipeDrafts) -> RecipeDrafts {
    let Some(mut out) = old else {
        return RecipeDrafts { writes: 1, ..new };
    };
    out.by = new.by;
    out.at = new.at;
    out.writes = out.writes.saturating_add(1);
    out.made |= new.made;
    union(&mut out.layers, &new.layers);
    union(&mut out.keys, &new.keys);
    out.clips |= new.clips;
    out.time |= new.time;
    out.blanks |= new.blanks;
    out.mirror |= new.mirror;
    out
}

/// `drafts` without the changes and keys `recipe` no longer has; none
/// when nothing is left.
pub fn pruned(mut drafts: RecipeDrafts, recipe: &RecipeFile) -> Option<RecipeDrafts> {
    drafts
        .layers
        .retain(|id| recipe.layers.iter().any(|l| &l.id == id));
    drafts.keys.retain(|k| {
        recipe.layers.iter().any(|l| {
            l.id == k.layer
                && l.keys
                    .iter()
                    .any(|p| p.clip == k.clip && p.frame == k.frame)
        })
    });
    (!drafts.is_empty()).then_some(drafts)
}

fn file(project_dir: &Path, id: &str) -> CoreResult<PathBuf> {
    if !is_recipe_id(id) {
        return Err(CoreError::NotFound(format!("the recipe {id}")));
    }
    Ok(super::dir(project_dir).join(format!("{id}{DRAFTS_SUFFIX}")))
}

/// The marks on the recipe `id`, as `recipe` still has them; none when
/// there are none or the file cannot be read (marks are a hint, never a
/// reason to refuse a recipe).
pub fn read(project_dir: &Path, id: &str, recipe: &RecipeFile) -> Option<RecipeDrafts> {
    let text = fs::read_to_string(file(project_dir, id).ok()?).ok()?;
    let drafts: RecipeDrafts = serde_json::from_str(&text).ok()?;
    pruned(drafts, recipe)
}

/// Marks what an agent called `by` changed in the recipe `id`, from
/// `before` (none when it made it) to `after`, gathered with the marks
/// already there. A write that changed nothing leaves them as they are.
///
/// # Errors
///
/// Fails for an id no recipe could have or when the file cannot be
/// written.
pub fn mark(
    project_dir: &Path,
    id: &str,
    before: Option<&RecipeFile>,
    after: &RecipeFile,
    by: &str,
    at: &str,
) -> CoreResult<Option<RecipeDrafts>> {
    let path = file(project_dir, id)?;
    let old = read(project_dir, id, after);
    let new = diff(before, after);
    if new.is_empty() {
        return Ok(old);
    }
    let new = RecipeDrafts {
        by: by.to_string(),
        at: at.to_string(),
        ..new
    };
    let Some(drafts) = pruned(merged(old, new), after) else {
        return Ok(None);
    };
    let mut text = serde_json::to_string_pretty(&drafts).map_err(|e| CoreError::json(&path, e))?;
    text.push('\n');
    maquette_base::fs::write_atomic(&path, text.as_bytes()).map_err(|e| CoreError::io(&path, e))?;
    Ok(Some(drafts))
}

/// Clears the marks on the recipe `id`: a person saved it, or it went.
///
/// # Errors
///
/// Fails for an id no recipe could have or when the file is there and
/// cannot be removed.
pub fn clear(project_dir: &Path, id: &str) -> CoreResult<()> {
    let path = file(project_dir, id)?;
    match fs::remove_file(&path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(CoreError::io(&path, e)),
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "tests_drafts.rs"]
mod tests;
