//! The model inbox: `.glb` and `.gltf` files dropped into the workspace's
//! inbox folder, analysed and checked, then accepted into a project's
//! `models/` folder as a new asset version.

pub mod analyse;
pub mod checks;
pub mod fixture;
pub(crate) mod images;
pub(crate) mod math;
pub mod watch;

mod accept;
mod aim;
mod discard;

pub use accept::{AcceptRequest, AcceptResult, accept, open_in_blender_args, stats_of};
pub use aim::{InboxAim, set_aim};
pub use discard::{DISCARDED_DIR, Discarded, discard};

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

pub use analyse::{GlbReport, RigKind, analyse};
pub use checks::{Check, CheckStatus, SendNotes, SentMaps};

use crate::assets::{
    self, Asset, AssetKind, AssetStatus, AssetVersion, ModelStats, NewAsset, Stage,
};
use crate::error::{CoreError, CoreResult};
use crate::game::FolderBudget;
use crate::json::write_json;
use crate::project;
use crate::thumbs::{Fingerprint, ThumbState};
use crate::workspace::{WorkspaceConfig, is_single_segment};

const KINDS_FILE: &str = "kinds.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum InboxState {
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
    /// The file name, unique within the inbox.
    pub id: String,
    /// A readable name from the file name: "character_tails_v2" becomes
    /// "Character tails".
    pub name: String,
    pub file: String,
    pub bytes: f64,
    pub modified_ms: f64,
    pub kind: AssetKind,
    /// True while the kind is only a guess from the file: nobody picked
    /// it and no send says what it is.
    pub kind_guessed: bool,
    /// True when the kind comes from the send this model was made for.
    pub kind_from_send: bool,
    pub state: InboxState,
    pub report: Option<GlbReport>,
    pub checks: Vec<Check>,
    pub worst: CheckStatus,
    pub error: Option<String>,
    /// The studio render of the file, once it exists (an absolute path).
    pub thumbnail: Option<String>,
    pub thumbnail_state: ThumbState,
    /// The Send to 3D this model came from, when its name matches one.
    pub link: Option<crate::providers::InboxLink>,
    /// The asset it was attached to without a send, which Accept defaults to.
    pub aim: Option<InboxAim>,
}

pub fn inbox_dir(root: &Path, config: &WorkspaceConfig) -> PathBuf {
    root.join(&config.layout.inbox)
}

fn cache_dir(root: &Path, config: &WorkspaceConfig) -> PathBuf {
    root.join(&config.layout.cache).join("inbox")
}

/// The analysis is cached against the file's size and modified time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedReport {
    len: u64,
    modified_ns: u128,
    report: GlbReport,
}

pub(crate) fn stamp(meta: &fs::Metadata) -> (u64, u128) {
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    (meta.len(), modified)
}

/// "character_tails_v2.glb" becomes "Character tails".
pub fn display_name(file: &str) -> String {
    let stem = Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().into_owned());
    let mut words: Vec<&str> = stem
        .split(['_', '-', ' ', '.'])
        .filter(|w| !w.is_empty())
        .collect();
    if words.len() > 1
        && let Some(last) = words.last()
        && last.len() > 1
        && last.starts_with(['v', 'V'])
        && last[1..].chars().all(|c| c.is_ascii_digit())
    {
        words.pop();
    }
    let joined = words.join(" ").to_lowercase();
    let mut chars = joined.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// Takes what the send record says: its kind, unless someone picked one,
/// and the maps and texture size it asked for. The checks run again, so a
/// choice made on purpose does not show as a warning. `folder` is the game
/// folder budget of the asset it was sent for, in a game project.
pub fn apply_send(
    item: &mut InboxItem,
    kind: AssetKind,
    notes: SendNotes,
    config: &WorkspaceConfig,
    folder: Option<&FolderBudget>,
) {
    if item.kind_guessed {
        item.kind = kind;
        item.kind_guessed = false;
        item.kind_from_send = true;
    }
    if let Some(report) = &item.report {
        item.checks = checks::run(report, item.kind, config, notes, folder);
        item.worst = checks::worst(&item.checks);
    }
}

/// A first guess from the analysis and the name.
pub fn guess_kind(report: Option<&GlbReport>, file: &str) -> AssetKind {
    if report.is_some_and(|r| r.skins > 0) {
        return AssetKind::Character;
    }
    let lower = file.to_lowercase();
    if ["scene", "island", "diorama", "room", "level", "environment"]
        .iter()
        .any(|w| lower.contains(w))
    {
        return AssetKind::Scene;
    }
    AssetKind::Prop
}

fn load_kinds(cache: &Path) -> BTreeMap<String, AssetKind> {
    fs::read_to_string(cache.join(KINDS_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn check_id(id: &str) -> CoreResult<()> {
    if is_single_segment(id) && watch::is_model(Path::new(id)) {
        Ok(())
    } else {
        Err(CoreError::Invalid(format!("{id} is not an inbox model.")))
    }
}

fn build_item(
    path: &Path,
    cache: &Path,
    config: &WorkspaceConfig,
    kinds: &BTreeMap<String, AssetKind>,
) -> CoreResult<InboxItem> {
    let id = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let meta = fs::metadata(path).map_err(|e| CoreError::io(path, e))?;
    let (len, modified_ns) = stamp(&meta);
    let cache_file = cache.join(format!("{id}.json"));
    let cached: Option<CachedReport> = fs::read_to_string(&cache_file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(|c: &CachedReport| c.len == len && c.modified_ns == modified_ns);
    let (report, error) = match cached {
        Some(c) => (Some(c.report), None),
        None => match analyse(path) {
            Ok(report) => {
                let record = CachedReport {
                    len,
                    modified_ns,
                    report: report.clone(),
                };
                if fs::create_dir_all(cache).is_ok()
                    && let Err(err) = write_json(&cache_file, &record)
                {
                    tracing::warn!(error = %err, "could not cache the inbox analysis");
                }
                (Some(report), None)
            }
            // Not cached: a file still being written fails now and passes later.
            Err(err) => (None, Some(err.to_string())),
        },
    };
    let picked = kinds.get(&id).copied();
    let kind = picked.unwrap_or_else(|| guess_kind(report.as_ref(), &id));
    let checks = report
        .as_ref()
        .map(|r| checks::run(r, kind, config, SendNotes::default(), None))
        .unwrap_or_default();
    let Fingerprint { bytes, modified_ms } = Fingerprint::new(len, modified_ns);
    Ok(InboxItem {
        name: display_name(&id),
        file: path.display().to_string(),
        bytes,
        modified_ms,
        kind,
        kind_guessed: picked.is_none(),
        kind_from_send: false,
        state: if report.is_some() {
            InboxState::Ready
        } else {
            InboxState::Failed
        },
        worst: if report.is_some() {
            checks::worst(&checks)
        } else {
            CheckStatus::Fail
        },
        report,
        checks,
        error,
        id,
        // The shell asks the thumbnail service; on its own the inbox has none.
        thumbnail: None,
        thumbnail_state: ThumbState::Fallback,
        // The shell asks the provider jobs, which know the send ids.
        link: None,
        aim: None,
    })
}

/// Every model in the inbox, newest first.
///
/// # Errors
///
/// Fails when the inbox folder exists but cannot be read.
pub fn list(root: &Path, config: &WorkspaceConfig) -> CoreResult<Vec<InboxItem>> {
    let dir = inbox_dir(root, config);
    let read = match fs::read_dir(&dir) {
        Ok(read) => read,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CoreError::io(&dir, e)),
    };
    let cache = cache_dir(root, config);
    let kinds = load_kinds(&cache);
    let mut items: Vec<InboxItem> = read
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && watch::is_model(p))
        .filter_map(|p| build_item(&p, &cache, config, &kinds).ok())
        .collect();
    aim::fill(&cache, &mut items);
    items.sort_by(|a, b| {
        b.modified_ms
            .total_cmp(&a.modified_ms)
            .then(a.id.cmp(&b.id))
    });
    Ok(items)
}

/// One model by its file name.
///
/// # Errors
///
/// Fails for a name outside the inbox or a missing file.
pub fn item(root: &Path, config: &WorkspaceConfig, id: &str) -> CoreResult<InboxItem> {
    check_id(id)?;
    let path = inbox_dir(root, config).join(id);
    if !path.is_file() {
        return Err(CoreError::NotFound(format!("{id} in the inbox")));
    }
    let cache = cache_dir(root, config);
    let mut found = build_item(&path, &cache, config, &load_kinds(&cache))?;
    aim::fill(&cache, std::slice::from_mut(&mut found));
    Ok(found)
}

/// Drops the cached analysis and runs it again.
///
/// # Errors
///
/// As [`item`].
pub fn reprocess(root: &Path, config: &WorkspaceConfig, id: &str) -> CoreResult<InboxItem> {
    check_id(id)?;
    let _ = fs::remove_file(cache_dir(root, config).join(format!("{id}.json")));
    item(root, config, id)
}

/// Records the kind someone picked, which changes the rig checks.
///
/// # Errors
///
/// As [`item`], or when the choice cannot be saved.
pub fn set_kind(
    root: &Path,
    config: &WorkspaceConfig,
    id: &str,
    kind: AssetKind,
) -> CoreResult<InboxItem> {
    check_id(id)?;
    let cache = cache_dir(root, config);
    let mut kinds = load_kinds(&cache);
    kinds.insert(id.to_string(), kind);
    fs::create_dir_all(&cache).map_err(|e| CoreError::io(&cache, e))?;
    write_json(&cache.join(KINDS_FILE), &kinds)?;
    item(root, config, id)
}

fn external_files(path: &Path) -> bool {
    let is_gltf = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("gltf"));
    if !is_gltf {
        return false;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return true;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return true;
    };
    ["buffers", "images"].iter().any(|key| {
        json.get(key)
            .and_then(|v| v.as_array())
            .is_some_and(|list| {
                list.iter().any(|entry| {
                    entry
                        .get("uri")
                        .and_then(|u| u.as_str())
                        .is_some_and(|u| !u.starts_with("data:"))
                })
            })
    })
}

#[cfg(test)]
mod tests;

/// Registers this module's IPC types (the GLB report, its checks and the inbox) with the shell's
/// TypeScript export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<analyse::RigKind>()
        .register::<analyse::TextureInfo>()
        .register::<analyse::PbrMaps>()
        .register::<analyse::JointCount>()
        .register::<analyse::FingerWeights>()
        .register::<analyse::Islands>()
        .register::<analyse::GlbReport>()
        .register::<checks::CheckStatus>()
        .register::<checks::Check>()
        .register::<InboxState>()
        .register::<InboxItem>()
        .register::<AcceptRequest>()
        .register::<AcceptResult>()
}
