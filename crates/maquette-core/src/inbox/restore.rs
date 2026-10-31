//! Discarded inbox models and putting one back. A discard moves the file
//! into `<inbox>/.discarded/`; next to the files sits `discarded.json`, a
//! note per file of when it left, the inbox name it had and the asset it
//! was going to, so the Inbox can list them newest first and a restore can
//! aim the model at that asset again. A file discarded before the notes
//! existed is still listed and restored, by its modified time and name.
//!
//! A discarded model is kept [`DISCARD_KEEP_DAYS`] days, counted from when it
//! left the inbox, then [`prune_discarded`] removes it for good. Nothing else
//! in the workspace is touched by that.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{DISCARDED_DIR, Discarded, InboxAim, check_id, inbox_dir, set_aim, stamp, watch};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::thumbs::Fingerprint;
use crate::workspace::{WorkspaceConfig, is_single_segment};

const NOTES_FILE: &str = "discarded.json";
/// How long a discarded model waits before it goes for good.
pub const DISCARD_KEEP_DAYS: u32 = 30;
const DAY_MS: f64 = 24.0 * 3600.0 * 1000.0;

fn expires_at(discarded_ms: f64) -> f64 {
    discarded_ms + f64::from(DISCARD_KEEP_DAYS) * DAY_MS
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Note {
    /// The inbox name the model had.
    id: String,
    at_ms: f64,
    goal: Option<InboxAim>,
    replaced_by: Option<u32>,
}

/// A model waiting in `<inbox>/.discarded/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DiscardedModel {
    /// Its file name in the discarded folder, which restore takes.
    pub name: String,
    /// The inbox name it had (the same, unless a number was added).
    pub id: String,
    /// The file, an absolute path.
    pub file: String,
    pub bytes: f64,
    /// When it was discarded; the file's modified time for an older discard.
    pub discarded_ms: f64,
    /// When it goes for good.
    pub expires_ms: f64,
    /// The asset it was going to, when it was going somewhere.
    pub asset_name: Option<String>,
    /// The version that had replaced it, when a redo had.
    pub replaced_by: Option<u32>,
}

/// Where a restored model went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Restored {
    /// Its inbox id now.
    pub id: String,
    /// The file, an absolute path.
    pub file: String,
}

fn folder(root: &Path, config: &WorkspaceConfig) -> std::path::PathBuf {
    inbox_dir(root, config).join(DISCARDED_DIR)
}

fn load(dir: &Path) -> BTreeMap<String, Note> {
    fs::read_to_string(dir.join(NOTES_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(dir: &Path, notes: &BTreeMap<String, Note>) {
    if let Err(err) = write_json(&dir.join(NOTES_FILE), notes) {
        tracing::warn!(error = %err, "could not save the discarded models' notes");
    }
}

/// Notes when `done` left the inbox and the asset it was going to, for the
/// list and a later restore. A note that cannot be written is only logged:
/// the file is already safe in the folder.
pub fn remember(
    root: &Path,
    config: &WorkspaceConfig,
    done: &Discarded,
    goal: Option<InboxAim>,
    replaced_by: Option<u32>,
    at_ms: f64,
) {
    let dir = folder(root, config);
    let Some(name) = Path::new(&done.file).file_name() else {
        return;
    };
    let mut notes = load(&dir);
    notes.insert(
        name.to_string_lossy().into_owned(),
        Note {
            id: done.id.clone(),
            at_ms,
            goal,
            replaced_by,
        },
    );
    save(&dir, &notes);
    super::round::note_discarded(root, config, &done.id, at_ms);
}

/// Removes the discarded models whose days are up, with their notes, and
/// answers their names. A file without a note has no known discard time (a
/// move keeps its old modified date), so it gets a note now and its full days
/// from here. Nothing is removed when the folder cannot be read.
pub fn prune_discarded(root: &Path, config: &WorkspaceConfig, now_ms: f64) -> Vec<String> {
    let dir = folder(root, config);
    let Ok(read) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut notes = load(&dir);
    let before = notes.clone();
    let mut gone = Vec::new();
    for path in read.filter_map(Result::ok).map(|e| e.path()) {
        if !path.is_file() || !watch::is_model(&path) {
            continue;
        }
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let note = notes.entry(name.clone()).or_default();
        if note.at_ms <= 0.0 {
            note.at_ms = now_ms;
            continue;
        }
        if now_ms < expires_at(note.at_ms) {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => {
                notes.remove(&name);
                gone.push(name);
            }
            Err(err) => tracing::warn!(error = %err, "could not prune a discarded model"),
        }
    }
    notes.retain(|name, _| dir.join(name).is_file());
    if notes != before {
        save(&dir, &notes);
    }
    gone
}

/// Every discarded model, the most recently discarded first.
///
/// # Errors
///
/// Fails when the folder exists but cannot be read.
pub fn list_discarded(root: &Path, config: &WorkspaceConfig) -> CoreResult<Vec<DiscardedModel>> {
    let dir = folder(root, config);
    let read = match fs::read_dir(&dir) {
        Ok(read) => read,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CoreError::io(&dir, e)),
    };
    let notes = load(&dir);
    let mut found: Vec<DiscardedModel> = read
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && watch::is_model(p))
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy().into_owned();
            let meta = fs::metadata(&path).ok()?;
            let (len, modified_ns) = stamp(&meta);
            let Fingerprint { bytes, modified_ms } = Fingerprint::new(len, modified_ns);
            let note = notes.get(&name).cloned().unwrap_or_default();
            let discarded_ms = if note.at_ms > 0.0 {
                note.at_ms
            } else {
                modified_ms
            };
            Some(DiscardedModel {
                id: if note.id.is_empty() {
                    name.clone()
                } else {
                    note.id
                },
                file: path.display().to_string(),
                bytes,
                discarded_ms,
                expires_ms: expires_at(discarded_ms),
                asset_name: note.goal.map(|g| g.asset_name),
                replaced_by: note.replaced_by,
                name,
            })
        })
        .collect();
    found.sort_by(|a, b| {
        b.discarded_ms
            .total_cmp(&a.discarded_ms)
            .then(a.name.cmp(&b.name))
    });
    Ok(found)
}

/// Moves the discarded model `name` back into the inbox under the name it
/// had (a name taken since gets a number) and aims it at the asset it was
/// going to, so Accept goes there again. The send its discard closed stays
/// closed.
///
/// # Errors
///
/// Fails for a name that is not a model in the discarded folder, or when it
/// cannot be moved.
pub fn restore(root: &Path, config: &WorkspaceConfig, name: &str) -> CoreResult<Restored> {
    if !is_single_segment(name) || !watch::is_model(Path::new(name)) {
        return Err(CoreError::Invalid(format!(
            "{name} is not a discarded model."
        )));
    }
    let dir = folder(root, config);
    let source = dir.join(name);
    if !source.is_file() {
        return Err(CoreError::NotFound(format!(
            "{name} among the discarded models"
        )));
    }
    let mut notes = load(&dir);
    let note = notes.remove(name).unwrap_or_default();
    let wanted = if note.id.is_empty() || check_id(&note.id).is_err() {
        name.to_string()
    } else {
        note.id.clone()
    };
    let inbox = inbox_dir(root, config);
    let target = super::discard::free_name(&inbox, &wanted);
    if fs::rename(&source, &target).is_err() {
        fs::copy(&source, &target).map_err(|e| CoreError::io(&target, e))?;
        fs::remove_file(&source).map_err(|e| CoreError::io(&source, e))?;
    }
    save(&dir, &notes);
    super::round::note_restored(root, config, &wanted);
    let id = target
        .file_name()
        .map_or_else(|| wanted.clone(), |n| n.to_string_lossy().into_owned());
    if let Some(goal) = note.goal
        && let Err(err) = set_aim(root, config, &id, Some(goal))
    {
        tracing::warn!(error = %err, "could not aim the restored model at its asset");
    }
    Ok(Restored {
        id,
        file: target.display().to_string(),
    })
}
