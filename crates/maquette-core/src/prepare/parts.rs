//! An asset's part names: the contract between the export, the object
//! states and the explode tracks. A split (or a provider) gives the parts
//! plain names such as `part_1` or `tripo_part_0`; the Inspector renames
//! them, joins two into one and hides one while looking. The fix-up script
//! applies the joins and the names after the fix-ups, so the exported
//! nodes carry them; hiding never changes the export.
//!
//! ```json
//! "parts": {
//!   "names": { "part_1": "lid", "part_2": "cage" },
//!   "merge": [["part_3", "part_4"]],
//!   "hidden": ["part_5"]
//! }
//! ```
//!
//! Keys of `names`, `merge` and `hidden` are the names the model gives its
//! parts. A joined group keeps the name of its first part (and that part's
//! new name, if it has one).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// The longest part name, in characters.
pub const MAX_PART_NAME: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PartsPlan {
    /// The name in the model to the name it ships as.
    #[serde(default)]
    pub names: BTreeMap<String, String>,
    /// Groups of parts joined into one mesh, named after the first.
    #[serde(default)]
    pub merge: Vec<Vec<String>>,
    /// Parts drawn as a ghost in the Inspector. They still ship.
    #[serde(default)]
    pub hidden: Vec<String>,
}

impl PartsPlan {
    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.merge.is_empty() && self.hidden.is_empty()
    }

    /// Whether it changes the exported model: names or joins, not hiding.
    pub fn changes_export(&self) -> bool {
        !self.names.is_empty() || !self.merge.is_empty()
    }

    /// Names trimmed, no rename to itself, no single part groups and the
    /// hidden list sorted without repeats.
    pub fn normalized(&self) -> Self {
        let names = self
            .names
            .iter()
            .map(|(from, to)| (from.trim().to_string(), to.trim().to_string()))
            .filter(|(from, to)| from != to)
            .collect();
        let merge = self
            .merge
            .iter()
            .map(|group| group.iter().map(|p| p.trim().to_string()).collect())
            .filter(|group: &Vec<String>| group.len() >= 2)
            .collect();
        let hidden: BTreeSet<String> = self.hidden.iter().map(|p| p.trim().to_string()).collect();
        Self {
            names,
            merge,
            hidden: hidden.into_iter().collect(),
        }
    }

    /// The name `part` ships as.
    pub fn name_of<'a>(&'a self, part: &'a str) -> &'a str {
        self.names.get(part).map_or(part, String::as_str)
    }

    /// The group `part` was joined into, by its first part, when it was.
    pub fn joined_into(&self, part: &str) -> Option<&str> {
        self.merge
            .iter()
            .find(|group| group.iter().any(|p| p == part))
            .and_then(|group| group.first())
            .map(String::as_str)
    }
}

fn invalid(message: String) -> CoreError {
    CoreError::Invalid(message)
}

/// Checks one asset's part names: none empty or too long, no two parts
/// shipping under one name, every joined group of two or more and no part
/// in two groups.
///
/// # Errors
///
/// Returns the first problem as a sentence.
pub fn check_parts(what: &str, plan: &PartsPlan) -> CoreResult<()> {
    let mut shipped: BTreeMap<&str, &str> = BTreeMap::new();
    for (from, to) in &plan.names {
        let name = to.trim();
        if from.trim().is_empty() || name.is_empty() {
            return Err(invalid(format!("{what}: a part name cannot be empty.")));
        }
        if name.chars().count() > MAX_PART_NAME {
            return Err(invalid(format!(
                "{what}: the part name {name:?} is longer than {MAX_PART_NAME} characters."
            )));
        }
        if let Some(other) = shipped.insert(name, from) {
            return Err(invalid(format!(
                "{what}: {other} and {from} would both ship as {name}. Give each part its own name."
            )));
        }
    }
    let mut seen = BTreeSet::new();
    for group in &plan.merge {
        if group.len() < 2 {
            return Err(invalid(format!("{what}: join at least two parts.")));
        }
        for part in group {
            if part.trim().is_empty() {
                return Err(invalid(format!("{what}: a joined part has no name.")));
            }
            if !seen.insert(part.trim()) {
                return Err(invalid(format!(
                    "{what}: {part} is joined twice. Keep each part in one group."
                )));
            }
        }
    }
    Ok(())
}
