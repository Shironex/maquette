//! Ship to a game: what the Ship step would write into the game's model
//! folder, and the record of the files Maquette owns there.
//!
//! Game export leaves the files to ship in `web/game/`, laid out as the
//! model folder is (`props/brazier.glb`). Ship (`tools/ship_game.mjs`)
//! copies each one to the same path in the game, but only over a file the
//! ship record lists: a file Maquette wrote there before, or one imported
//! from there. Every other file in the model folder is hand made and left
//! alone, also when an export would land on it. Ship never removes a file
//! and never commits.
//!
//! The record is `web/ship-record.json`, the same file a website project's
//! Ship keeps, with the model folder (relative to the game repo) as the
//! target and the files relative to it:
//!
//! ```json
//! { "version": 1, "targets": { "assets/models": ["props/brazier.glb"] } }
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{GameFormat, GameSettings, check_asset, file_of, obj, settings_of};
use crate::error::{CoreError, CoreResult};
use crate::inbox::analyse;
use crate::json::write_json;
use crate::project::{ProjectManifest, sorted_children};

pub const RECORD_FILE: &str = "web/ship-record.json";
pub const RECORD_VERSION: u32 = 1;
/// Where Game export leaves the files Ship writes.
pub const EXPORT_DIR: &str = "web/game";
/// The extensions a game export must never use.
pub const FORBIDDEN_EXTENSIONS: [&str; 3] = [
    "KHR_texture_basisu",
    "EXT_meshopt_compression",
    "KHR_draco_mesh_compression",
];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShipRecord {
    pub version: u32,
    /// A target folder to the files shipped or imported there.
    pub targets: BTreeMap<String, Vec<String>>,
}

/// The project's ship record, empty when it has none or cannot be read.
pub fn read_record(project_dir: &Path) -> ShipRecord {
    fs::read_to_string(project_dir.join(RECORD_FILE))
        .ok()
        .and_then(|t| serde_json::from_str::<ShipRecord>(&t).ok())
        .filter(|r| r.version <= RECORD_VERSION)
        .unwrap_or(ShipRecord {
            version: RECORD_VERSION,
            targets: BTreeMap::new(),
        })
}

/// Adds `files` to the record's `target`, so Ship may replace them later.
pub fn add_to_record(project_dir: &Path, target: &str, files: &[String]) -> CoreResult<()> {
    let mut record = read_record(project_dir);
    record.version = RECORD_VERSION;
    let list = record.targets.entry(target.to_string()).or_default();
    let mut all: BTreeSet<String> = list.drain(..).collect();
    all.extend(files.iter().cloned());
    *list = all.into_iter().collect();
    let path = project_dir.join(RECORD_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    write_json(&path, &record)
}

/// How one file in the model folder changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ShipChange {
    New,
    /// The record lists it and its bytes differ.
    Replaced,
    Unchanged,
    /// Not in the ship record: Ship never touches it.
    HandMade,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShipFile {
    /// Relative to the model folder, with `/`.
    pub path: String,
    pub change: ShipChange,
    /// The file Ship writes; the game's own for a hand made file.
    pub bytes: f64,
    /// The game's file now, when there is one.
    pub before_bytes: Option<f64>,
    pub triangles: Option<u32>,
    pub before_triangles: Option<u32>,
    /// The asset it comes from, by name.
    pub asset: Option<String>,
    /// An export that would land on a hand made file, so it is held back.
    pub blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShipCheck {
    pub ok: bool,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShipPlan {
    pub repo: Option<String>,
    pub models_dir: String,
    pub branch: Option<String>,
    /// False before the first Game export.
    pub exported: bool,
    /// What the export holds, in path order.
    pub files: Vec<ShipFile>,
    /// The game's files the record does not list and no export touches.
    pub hand_made: Vec<ShipFile>,
    /// How many files the record lists after this Ship.
    pub record_after: u32,
    pub checks: Vec<ShipCheck>,
}

/// Every file under `dir`, relative to it with `/`, sorted.
pub(crate) fn files_under(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        for (name, path) in sorted_children(dir) {
            if name.starts_with('.') {
                continue;
            }
            let rel = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if path.is_dir() {
                walk(&path, &rel, out);
            } else if path.is_file() {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out
}

#[expect(clippy::cast_precision_loss, reason = "file sizes stay far below 2^53")]
fn size(path: &Path) -> Option<f64> {
    fs::metadata(path).ok().map(|m| m.len() as f64)
}

/// Triangles and the extensions a model file uses; none for other files.
fn model_numbers(path: &Path) -> (Option<u32>, Vec<String>) {
    match GameFormat::of(&path.to_string_lossy()) {
        Some(GameFormat::Glb) => {
            analyse(path).map_or((None, Vec::new()), |r| (Some(r.triangles), r.extensions))
        }
        Some(GameFormat::Obj) => (obj::read(path).ok().map(|m| m.stats.triangles), Vec::new()),
        None => (None, Vec::new()),
    }
}

/// The asset an exported file belongs to: the one whose model it is, or
/// whose `.obj` it sits next to under the same name (its `.mtl`).
fn asset_for(settings: &GameSettings, manifest: &ProjectManifest, path: &str) -> Option<String> {
    let stem = |p: &str| p.rsplit_once('.').map_or(p, |(s, _)| s).to_string();
    manifest
        .assets
        .iter()
        .find(|a| {
            let file = file_of(settings, a);
            file == path || stem(&file) == stem(path)
        })
        .map(|a| a.name.clone())
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    match (fs::read(a), fs::read(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// What Ship would do now, from the last Game export. Reads only.
pub fn plan(project_dir: &Path, manifest: &ProjectManifest) -> CoreResult<ShipPlan> {
    let settings = settings_of(manifest)?;
    let export = project_dir.join(EXPORT_DIR);
    let repo = manifest.site_repo.as_deref().map(Path::new);
    let game_dir = repo.map(|r| r.join(&settings.models_dir));
    let record = read_record(project_dir);
    let owned: BTreeSet<String> = record
        .targets
        .get(&settings.models_dir)
        .map(|l| l.iter().cloned().collect())
        .unwrap_or_default();
    let produced = files_under(&export);
    let mut files = Vec::new();
    let mut extensions = BTreeSet::new();
    for path in &produced {
        let from = export.join(path);
        let target = game_dir.as_ref().map(|d| d.join(path));
        let exists = target.as_ref().is_some_and(|t| t.is_file());
        let (triangles, used) = model_numbers(&from);
        extensions.extend(used);
        let change = match (&target, exists) {
            (Some(t), true) if same_bytes(&from, t) => ShipChange::Unchanged,
            (_, true) if owned.contains(path) => ShipChange::Replaced,
            (_, true) => ShipChange::HandMade,
            _ => ShipChange::New,
        };
        let before = target.as_ref().filter(|_| exists);
        files.push(ShipFile {
            path: path.clone(),
            change,
            bytes: size(&from).unwrap_or(0.0),
            before_bytes: before.and_then(|t| size(t)),
            triangles,
            before_triangles: before
                .filter(|_| change == ShipChange::Replaced)
                .and_then(|t| model_numbers(t).0),
            asset: asset_for(settings, manifest, path),
            blocked: change == ShipChange::HandMade,
        });
    }
    let hand_made = game_dir
        .as_ref()
        .map(|d| files_under(d))
        .unwrap_or_default()
        .into_iter()
        .filter(|p| !owned.contains(p) && !produced.contains(p))
        .map(|path| ShipFile {
            bytes: game_dir
                .as_ref()
                .and_then(|d| size(&d.join(&path)))
                .unwrap_or(0.0),
            path,
            change: ShipChange::HandMade,
            before_bytes: None,
            triangles: None,
            before_triangles: None,
            asset: None,
            blocked: false,
        })
        .collect();
    let mut after: BTreeSet<String> = owned;
    after.extend(files.iter().filter(|f| !f.blocked).map(|f| f.path.clone()));
    let over = manifest
        .assets
        .iter()
        .filter(|a| check_asset(settings, a).is_some_and(|c| c.over()))
        .count();
    let forbidden: Vec<&str> = FORBIDDEN_EXTENSIONS
        .iter()
        .copied()
        .filter(|e| extensions.contains(*e))
        .collect();
    let checks = vec![
        ShipCheck {
            ok: over == 0,
            text: if over == 0 {
                "Every model fits its budget".to_string()
            } else {
                format!("{over} over budget")
            },
        },
        ShipCheck {
            ok: forbidden.is_empty(),
            text: if forbidden.is_empty() {
                "No KTX2 or meshopt in any file".to_string()
            } else {
                format!("Uses {}", forbidden.join(", "))
            },
        },
    ];
    Ok(ShipPlan {
        repo: manifest.site_repo.clone(),
        models_dir: settings.models_dir.clone(),
        branch: repo.and_then(super::detect::branch),
        exported: export.is_dir(),
        files,
        hand_made,
        record_after: u32::try_from(after.len()).unwrap_or(u32::MAX),
        checks,
    })
}
