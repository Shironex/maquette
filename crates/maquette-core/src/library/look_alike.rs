//! Look-alikes: library entries that may be the same model twice. Two kinds
//! are found:
//!
//! - **Same look:** each entry's newest picture that reads (its version
//!   thumbnail, a PNG) is compared with every other's by outline, colours
//!   and proportions ([`picture::score`]); a score at or over the threshold
//!   pairs them.
//! - **Same name:** names equal once lowercased, trimmed and without a
//!   trailing number, when the pictures did not already pair them.
//!
//! A person decides each pair (keep both, not alike, keep the names) and
//! decided pairs stay hidden; the decisions and the settings live in
//! `library/look-alikes.json`. Nothing here changes an entry: merging is
//! [`super::merge`].

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::merge::LibraryMerge;
use super::names::name_key;
use super::{LibraryEntry, dir, entry_dir, list, merge_undo, turn};
use crate::error::{CoreError, CoreResult};
use crate::json::{from_json_with_defaults, write_json};
use crate::project::now_rfc3339;
use crate::refs::picture::{self, Object};

pub(super) const LOOK_ALIKES_FILE: &str = "look-alikes.json";
const LOOK_ALIKES_VERSION: u32 = 1;
const LEAST_THRESHOLD: f64 = 0.5;
const MOST_THRESHOLD: f64 = 0.99;

/// How look-alikes are found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LookAlikeSettings {
    /// The least picture score, 0.5 to 0.99, that pairs two entries.
    pub threshold: f64,
    /// Whether adding many assets checks each against the library's pictures.
    pub check_on_add: bool,
    /// Whether a merge copies the merged entry's tags by default.
    pub copy_tags: bool,
}

impl Default for LookAlikeSettings {
    fn default() -> Self {
        Self {
            threshold: 0.9,
            check_on_add: true,
            copy_tags: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum LookAlikeKind {
    SameLook,
    SameName,
}

/// What a person said about a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum LookAlikeVerdict {
    KeepBoth,
    NotAlike,
    KeepNames,
}

/// Two entries that may be one model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LookAlikePair {
    pub kind: LookAlikeKind,
    /// The older entry.
    pub a: String,
    pub b: String,
    /// How alike the pictures are, 0 to 1, when both read.
    pub score: Option<f64>,
    pub shape: Option<f64>,
    pub colour: Option<f64>,
    pub ratio: Option<f64>,
}

/// What the look-alike check found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LookAlikes {
    pub pairs: Vec<LookAlikePair>,
    /// Entries with no readable picture, skipped by the picture match.
    pub unseen: Vec<String>,
    /// Recent merges, newest first (at most 5), each undoable with restore.
    pub merges: Vec<LibraryMerge>,
    pub settings: LookAlikeSettings,
    pub scanned_at: String,
    pub took_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Decision {
    a: String,
    b: String,
    verdict: LookAlikeVerdict,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LookAlikesFile {
    version: u32,
    settings: LookAlikeSettings,
    decisions: Vec<Decision>,
}

impl Default for LookAlikesFile {
    fn default() -> Self {
        Self {
            version: LOOK_ALIKES_VERSION,
            settings: LookAlikeSettings::default(),
            decisions: Vec::new(),
        }
    }
}

fn file(root: &Path) -> PathBuf {
    dir(root).join(LOOK_ALIKES_FILE)
}

fn read(root: &Path) -> CoreResult<LookAlikesFile> {
    let path = file(root);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(LookAlikesFile::default()),
        Err(err) => return Err(CoreError::io(&path, err)),
    };
    let found: LookAlikesFile =
        from_json_with_defaults(&text).map_err(|e| CoreError::json(&path, e))?;
    if found.version > LOOK_ALIKES_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: found.version,
            supported: LOOK_ALIKES_VERSION,
        });
    }
    Ok(found)
}

fn write(root: &Path, saved: &LookAlikesFile) -> CoreResult<()> {
    let folder = dir(root);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    write_json(&file(root), saved)
}

/// The saved settings, the defaults when there are none.
///
/// # Errors
///
/// Fails when `look-alikes.json` cannot be read.
pub fn settings(root: &Path) -> CoreResult<LookAlikeSettings> {
    Ok(read(root)?.settings)
}

/// The object in an entry's newest picture that reads, if any.
pub(super) fn picture_of(root: &Path, entry: &LibraryEntry) -> Option<Object> {
    let folder = entry_dir(root, &entry.id).ok()?;
    entry
        .versions
        .iter()
        .rev()
        .filter_map(|v| v.thumbnail.as_deref())
        .find_map(|thumb| picture::object_in(&folder.join(thumb)))
}

fn same_pair(d: &Decision, a: &str, b: &str) -> bool {
    (d.a == a && d.b == b) || (d.a == b && d.b == a)
}

fn pair(kind: LookAlikeKind, a: &LibraryEntry, b: &LibraryEntry) -> LookAlikePair {
    let (older, newer) =
        if (b.created_at.as_str(), b.id.as_str()) < (a.created_at.as_str(), a.id.as_str()) {
            (b, a)
        } else {
            (a, b)
        };
    LookAlikePair {
        kind,
        a: older.id.clone(),
        b: newer.id.clone(),
        score: None,
        shape: None,
        colour: None,
        ratio: None,
    }
}

/// Every undecided look-alike pair in the library.
///
/// # Errors
///
/// Fails when the library or `look-alikes.json` cannot be read.
pub fn look_alikes(root: &Path) -> CoreResult<LookAlikes> {
    let started = Instant::now();
    let saved = read(root)?;
    let entries = list(root)?;
    let pictures: Vec<Option<Object>> = entries.iter().map(|e| picture_of(root, e)).collect();
    let keys: Vec<String> = entries.iter().map(|e| name_key(&e.name)).collect();
    let mut same_look = Vec::new();
    let mut same_names = Vec::new();
    for i in 0..entries.len() {
        for j in i + 1..entries.len() {
            let (a, b) = (&entries[i], &entries[j]);
            if saved.decisions.iter().any(|d| same_pair(d, &a.id, &b.id)) {
                continue;
            }
            let found = match (&pictures[i], &pictures[j]) {
                (Some(x), Some(y)) => Some(picture::score(x, y)),
                _ => None,
            };
            let looks = found.is_some_and(|s| s.total() >= saved.settings.threshold);
            if !looks && keys[i] != keys[j] {
                continue;
            }
            let kind = if looks {
                LookAlikeKind::SameLook
            } else {
                LookAlikeKind::SameName
            };
            let mut found_pair = pair(kind, a, b);
            if let Some(s) = found {
                found_pair.score = Some(s.total());
                found_pair.shape = Some(s.shape);
                found_pair.colour = Some(s.colour);
                found_pair.ratio = Some(s.ratio);
            }
            if looks {
                same_look.push(found_pair);
            } else {
                same_names.push((keys[i].clone(), found_pair));
            }
        }
    }
    same_look.sort_by(|x, y| y.score.unwrap_or(0.0).total_cmp(&x.score.unwrap_or(0.0)));
    same_names.sort_by(|x, y| x.0.cmp(&y.0).then_with(|| x.1.a.cmp(&y.1.a)));
    let mut pairs = same_look;
    pairs.extend(same_names.into_iter().map(|(_, p)| p));
    let unseen = entries
        .iter()
        .zip(&pictures)
        .filter(|(_, p)| p.is_none())
        .map(|(e, _)| e.id.clone())
        .collect();
    Ok(LookAlikes {
        pairs,
        unseen,
        merges: merge_undo::recent(root),
        settings: saved.settings,
        scanned_at: now_rfc3339(),
        took_ms: u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX),
    })
}

/// Records a person's verdict on a pair, so it is no longer listed. The
/// pair's order does not matter.
///
/// # Errors
///
/// Fails for a pair of one entry with itself, an id that is not an entry
/// id, or a file that cannot be written.
pub fn decide(root: &Path, a: &str, b: &str, verdict: LookAlikeVerdict) -> CoreResult<()> {
    entry_dir(root, a)?;
    entry_dir(root, b)?;
    if a == b {
        return Err(CoreError::Invalid(
            "A pair needs two different models.".to_string(),
        ));
    }
    let _turn = turn();
    let mut saved = read(root)?;
    saved.decisions.retain(|d| !same_pair(d, a, b));
    saved.decisions.push(Decision {
        a: a.to_string(),
        b: b.to_string(),
        verdict,
    });
    write(root, &saved)
}

/// Saves the settings, the threshold kept within 0.5 to 0.99. Answers
/// what was saved.
///
/// # Errors
///
/// Fails when the file cannot be read or written.
pub fn save_settings(root: &Path, settings: &LookAlikeSettings) -> CoreResult<LookAlikeSettings> {
    let threshold = if settings.threshold.is_finite() {
        settings.threshold.clamp(LEAST_THRESHOLD, MOST_THRESHOLD)
    } else {
        LookAlikeSettings::default().threshold
    };
    let next = LookAlikeSettings {
        threshold,
        ..settings.clone()
    };
    let _turn = turn();
    let mut saved = read(root)?;
    saved.settings = next.clone();
    write(root, &saved)?;
    Ok(next)
}
