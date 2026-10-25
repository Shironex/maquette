//! Built assets: models a Blender script inside the project makes, the
//! fourth model source next to Tripo, Meshy and By hand.
//!
//! Each built asset names a script and the helper files it imports (or
//! reads), all kept flat in the project's `built/` folder, so assets that
//! share a helper share one copy of it. `built.json` next to `project.json`
//! records them, keyed by asset id like `prepare.json`:
//!
//! ```json
//! {
//!   "version": 1,
//!   "assets": {
//!     "laptop": {
//!       "script": "built/laptop.py",
//!       "helpers": ["built/common.py"],
//!       "root": null,
//!       "last": { "key": "3f1c...", "at": "2026-10-25T13:30:00.000Z", "version": 1, "error": null,
//!                 "objects": ["laptop", "laptop_body", "lid", "screen"], "materials": ["palette", "screen"],
//!                 "triangles": 1170, "warnings": [] }
//!     }
//!   }
//! }
//! ```
//!
//! A build runs the script headless (`blender -b`) through
//! [`BUILD_SCRIPT`] in a scratch copy under the workspace cache, so a
//! script that writes next to itself never writes into the project. The GLB
//! it exports becomes the asset's next version in `models/` straight away:
//! no credits are spent and the script is the source of truth, so a built
//! version skips the Inbox. Its checks still run and their warnings stay on
//! the build record. A build's key hashes the script, every helper (name
//! and content), the root and the runner script; the same key as the last
//! good build is up to date, and a build whose GLB matches the newest
//! version byte for byte adds no version.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::assets::Asset;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::ProjectManifest;

mod add;
mod blender;
mod build;

pub use add::{NewBuilt, add};
pub use blender::{LANE, work_dir};
pub use build::{BuildContext, BuildOutcome, BuildResult, Builder};

pub const BUILT_FILE: &str = "built.json";
pub const BUILT_VERSION: u32 = 1;
/// The project folder that holds every built asset's script and helpers.
pub const BUILT_DIR: &str = "built";
/// The script Maquette runs Blender with: it imports the asset's script,
/// calls its `build()` and exports the result.
pub const BUILD_SCRIPT: &str = include_str!("build_asset.py");
/// Blender keeps object names to 63 bytes.
pub const ROOT_MAX_CHARS: usize = 63;
pub const MAX_HELPERS: usize = 64;
/// The largest script or helper a built asset takes, 50 MB.
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

/// What the last build of an asset did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BuildRecord {
    /// The build key of the files it ran on.
    pub key: String,
    /// RFC 3339 UTC.
    pub at: String,
    /// The version it made, or the newest one when the GLB matched it.
    pub version: Option<u32>,
    /// Why it failed, when it did.
    pub error: Option<String>,
    /// The exported object names, parents first.
    pub objects: Vec<String>,
    pub materials: Vec<String>,
    pub triangles: u32,
    /// The titles of the Inbox checks that did not pass.
    pub warnings: Vec<String>,
}

/// One built asset's script and helpers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BuiltSource {
    /// Relative to the project folder, for example `built/laptop.py`.
    pub script: String,
    /// The same, for the files the script imports or reads.
    pub helpers: Vec<String>,
    /// The object to export with its children. Without it, the object
    /// `build()` returns, else the whole scene.
    pub root: Option<String>,
    pub last: Option<BuildRecord>,
}

/// `built.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltFile {
    pub version: u32,
    pub assets: BTreeMap<String, BuiltSource>,
}

impl Default for BuiltFile {
    fn default() -> Self {
        Self {
            version: BUILT_VERSION,
            assets: BTreeMap::new(),
        }
    }
}

/// How a built asset stands against its files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltState {
    /// Never built.
    New,
    /// The newest version comes from these exact files.
    Current,
    /// The script, a helper or the root changed since the last build.
    Stale,
    /// The last build of these exact files failed.
    Failed,
    /// The script or a helper is gone from `built/`.
    Missing,
    /// Building now.
    Building,
}

/// One built asset as the Board and agents read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BuiltAsset {
    pub asset: String,
    pub name: String,
    pub source: BuiltSource,
    pub state: BuiltState,
    /// The files of `source` that are not in the project.
    pub missing: Vec<String>,
}

pub fn path(project_dir: &Path) -> PathBuf {
    project_dir.join(BUILT_FILE)
}

/// Reads `built.json`, or no built assets when the project has none.
///
/// # Errors
///
/// Fails when the file exists but cannot be read or comes from a newer
/// Maquette, so it is never overwritten.
pub fn load(project_dir: &Path) -> CoreResult<BuiltFile> {
    let file = path(project_dir);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(BuiltFile::default()),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    let version = value.get("version").and_then(serde_json::Value::as_u64);
    if let Some(found) = version.filter(|v| *v > u64::from(BUILT_VERSION)) {
        return Err(CoreError::UnsupportedVersion {
            found: u32::try_from(found).unwrap_or(u32::MAX),
            supported: BUILT_VERSION,
        });
    }
    serde_json::from_value(value).map_err(|e| CoreError::json(&file, e))
}

/// One read, change and write at a time.
static WRITES: Mutex<()> = Mutex::new(());

/// Reads `built.json` from disk, applies `change` and writes it back.
///
/// # Errors
///
/// Refuses, writing nothing, when the file cannot be read or `change`
/// fails.
pub fn update<T>(
    project_dir: &Path,
    change: impl FnOnce(&mut BuiltFile) -> CoreResult<T>,
) -> CoreResult<T> {
    let _turn = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let mut file = load(project_dir)?;
    let out = change(&mut file)?;
    file.version = BUILT_VERSION;
    write_json(&path(project_dir), &file)?;
    Ok(out)
}

impl BuiltSource {
    /// The script first, then the helpers.
    pub fn files(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.script).chain(self.helpers.iter())
    }
}

fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The content hash a built version is compared by.
pub(crate) fn file_hash(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|bytes| sha(&bytes))
}

/// The build key of `source` in `project_dir`, or the files that are
/// missing. Line endings are folded, so a CRLF checkout keys the same.
pub fn key(project_dir: &Path, source: &BuiltSource) -> Result<String, Vec<String>> {
    let mut missing = Vec::new();
    let mut parts = Vec::new();
    for rel in source.files() {
        match fs::read(project_dir.join(rel)) {
            Ok(bytes) => parts.push((rel.clone(), sha(&fold(&bytes)))),
            Err(_) => missing.push(rel.clone()),
        }
    }
    if !missing.is_empty() {
        return Err(missing);
    }
    // The script keeps its place; helpers count by name, in any order.
    parts[1..].sort();
    let mut hasher = Sha256::new();
    hasher.update(b"maquette built v1\n");
    hasher.update(sha(BUILD_SCRIPT.replace("\r\n", "\n").as_bytes()));
    hasher.update(b"\n");
    hasher.update(source.root.as_deref().unwrap_or("").as_bytes());
    for (rel, hash) in &parts {
        hasher.update(b"\n");
        hasher.update(rel.as_bytes());
        hasher.update(b"\0");
        hasher.update(hash.as_bytes());
    }
    Ok(hex::encode(hasher.finalize())[..32].to_string())
}

fn fold(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// How `source` stands for `asset`.
pub fn state_of(
    project_dir: &Path,
    asset: &Asset,
    source: &BuiltSource,
    building: bool,
) -> (BuiltState, Vec<String>) {
    if building {
        return (BuiltState::Building, Vec::new());
    }
    let key = match key(project_dir, source) {
        Ok(key) => key,
        Err(missing) => return (BuiltState::Missing, missing),
    };
    let state = match &source.last {
        None => BuiltState::New,
        Some(last) if last.key != key => BuiltState::Stale,
        Some(last) if last.error.is_some() => BuiltState::Failed,
        Some(_) if asset.versions.is_empty() => BuiltState::New,
        Some(_) => BuiltState::Current,
    };
    (state, Vec::new())
}

/// Every built asset of the project with how it stands, in Board order.
/// `busy` names the assets building now ([`Builder::busy`]).
///
/// # Errors
///
/// Fails when `built.json` cannot be read.
pub fn list(
    project_dir: &Path,
    manifest: &ProjectManifest,
    busy: &[String],
) -> CoreResult<Vec<BuiltAsset>> {
    let file = load(project_dir)?;
    Ok(manifest
        .assets
        .iter()
        .filter_map(|asset| {
            let source = file.assets.get(&asset.id)?;
            let (state, missing) = state_of(project_dir, asset, source, busy.contains(&asset.id));
            Some(BuiltAsset {
                asset: asset.id.clone(),
                name: asset.name.clone(),
                source: source.clone(),
                state,
                missing,
            })
        })
        .collect())
}

/// The built assets a full pipeline run rebuilds first: new and stale ones.
///
/// # Errors
///
/// Fails when `built.json` cannot be read.
pub fn to_rebuild(project_dir: &Path, manifest: &ProjectManifest) -> CoreResult<Vec<BuiltAsset>> {
    Ok(list(project_dir, manifest, &[])?
        .into_iter()
        .filter(|b| matches!(b.state, BuiltState::New | BuiltState::Stale))
        .collect())
}

#[cfg(test)]
mod tests;
