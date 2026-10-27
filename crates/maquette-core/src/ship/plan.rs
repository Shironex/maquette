//! What Ship would write into a website's models folder, read only. It
//! follows the same rules as the Ship starter (`tools/ship.mjs`), which
//! decides again when it runs:
//!
//! - a file the site does not have is new;
//! - a file whose bytes match is unchanged;
//! - a file the ship record lists is Maquette's and is replaced, unless the
//!   models style is on and the new file is larger than the site's;
//! - any other file the site has is not Maquette's and is held back;
//! - `never` holds the asset back and `always` writes it whatever is there.
//!
//! A held file is neither copied nor removed. A file the record lists that
//! this Ship no longer makes or holds is removed from the site.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{ShipChoice, ShipSettings, ShipStyle, TextureFormat, load_or_default};
use crate::error::CoreResult;
use crate::game::ship::{ShipChange, ShipCheck, read_record};
use crate::pipeline::PipelineDef;
use crate::project::ProjectManifest;
use crate::site::{self, CONFIG_FILE, IndexEntry};

/// What the Ship starter must contain to honour the choices and holds.
pub const SHIP_SCRIPT_MARKER: &str = "sizeGuard";
/// What Export GLB must contain to export at the model's own origin.
pub const EXPORT_ORIGIN_MARKER: &str = "--origin";
/// What Build scene must contain for Export GLB to undo the placement.
pub const BUILD_ORIGIN_MARKER: &str = "maquette_moved";

/// Where Ship writes in the site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShipTarget {
    /// The site repo on this computer, when the project has one.
    pub repo: Option<String>,
    /// The site's folders, from `maquette.site.json` or the legacy ones.
    pub models: String,
    pub posters: Option<String>,
    pub data: Option<String>,
    /// True when the site has a `maquette.site.json`.
    pub configured: bool,
    /// True when the models folder is already in the site.
    pub models_exist: bool,
    /// Why the site config could not be read.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SitePlanFile {
    /// The file name in the models folder, `desk.glb`.
    pub file: String,
    /// The asset it comes from.
    pub asset: Option<String>,
    pub asset_name: Option<String>,
    pub change: ShipChange,
    pub choice: ShipChoice,
    /// False when Ship holds it back; `held` says why.
    pub ships: bool,
    pub held: Option<String>,
    /// The file Ship writes.
    pub bytes: f64,
    /// The site's file now.
    pub before_bytes: Option<f64>,
    pub triangles: Option<u32>,
    pub before_triangles: Option<u32>,
}

/// A site file this Ship does not write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SiteLeftFile {
    pub file: String,
    pub bytes: f64,
    /// True when Maquette shipped it before, so this Ship removes it.
    pub removed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SiteShipPlan {
    pub target: ShipTarget,
    pub style: ShipStyle,
    pub textures: TextureFormat,
    /// False before Compress first wrote anything.
    pub exported: bool,
    /// What Compress made, in file order.
    pub files: Vec<SitePlanFile>,
    /// The site's other `.glb` files.
    pub left: Vec<SiteLeftFile>,
    pub checks: Vec<ShipCheck>,
    /// True when `tools/ship.mjs` honours the choices; an older copy ships
    /// every file it finds.
    pub script_ready: bool,
    /// True when Build scene and Export GLB can export at the own origin.
    pub origin_ready: bool,
}

#[expect(clippy::cast_precision_loss, reason = "file sizes stay far below 2^53")]
pub(super) fn size(path: &Path) -> Option<f64> {
    fs::metadata(path)
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.len() as f64)
}

pub(super) fn triangles(path: &Path) -> Option<u32> {
    let report = crate::inbox::analyse(path).ok()?;
    Some(report.triangles)
}

fn glbs(dir: &Path) -> Vec<String> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<String> = read
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_lowercase().ends_with(".glb") && !n.starts_with('.'))
        .collect();
    out.sort();
    out
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    match (fs::read(a), fs::read(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// `site/models` as the Ship starter's `path.resolve` spells it, the key
/// of the ship record.
fn record_key(site: &Path, models: &str) -> String {
    let mut out = PathBuf::new();
    for c in site.join(models).components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out.display().to_string()
}

fn contains(root: &Path, script: &str, marker: &str) -> bool {
    fs::read_to_string(root.join(script)).is_ok_and(|t| t.contains(marker))
}

/// True when the project's own starters can export at the model's origin.
pub fn origin_ready(root: &Path) -> bool {
    contains(root, "tools/export_web.py", EXPORT_ORIGIN_MARKER)
        && contains(root, "tools/build_scene.py", BUILD_ORIGIN_MARKER)
}

/// True when the project's Ship starter honours the choices and holds.
pub fn script_ready(root: &Path) -> bool {
    contains(root, "tools/ship.mjs", SHIP_SCRIPT_MARKER)
}

fn target_of(manifest: &ProjectManifest) -> ShipTarget {
    let repo = manifest.site_repo.clone();
    let site = repo.as_deref().map(Path::new);
    let (config, error) = match site.map(site::load_config) {
        Some(Ok(c)) => (c, None),
        Some(Err(e)) => (site::SiteConfig::legacy(), Some(e.to_string())),
        None => (site::SiteConfig::legacy(), None),
    };
    ShipTarget {
        configured: site.is_some_and(|s| s.join(CONFIG_FILE).is_file()),
        models_exist: site.is_some_and(|s| s.join(&config.models).is_dir()),
        repo,
        models: config.models,
        posters: config.posters,
        data: config.data,
        error,
    }
}

/// Why `choice` holds one file back, or `None` when it ships.
fn held(choice: ShipChoice, change: ShipChange, grows: bool, guard: bool) -> Option<String> {
    match (choice, change) {
        (ShipChoice::Never, _) => Some("Kept off: this asset is set to never ship.".to_string()),
        (ShipChoice::Always, _) => None,
        (_, ShipChange::HandMade) => Some(
            "The site's file was not made by Maquette. Set the asset to always ship to replace it."
                .to_string(),
        ),
        (_, ShipChange::Replaced) if guard && grows => Some(
            "The site's file is smaller. Set the asset to always ship to replace it.".to_string(),
        ),
        _ => None,
    }
}

/// What the next Ship would do, from what Compress wrote last.
///
/// # Errors
///
/// Never at the moment; kept fallible for reads that can fail later.
pub fn plan(
    project_dir: &Path,
    manifest: &ProjectManifest,
    def: &PipelineDef,
) -> CoreResult<SiteShipPlan> {
    let ship: ShipSettings = load_or_default(project_dir);
    let root = def.root_dir(project_dir);
    let (_, dist) = crate::inspect::folders(project_dir, def);
    let target = target_of(manifest);
    let index = site::ship_index(manifest, &ship);
    let by_file: BTreeMap<String, (&String, &IndexEntry)> = index
        .models
        .iter()
        .map(|(id, e)| (e.file.to_lowercase(), (id, e)))
        .collect();
    let built = glbs(&dist);
    let candidates: Vec<String> = if target.data.is_some() {
        built
            .iter()
            .filter(|f| by_file.contains_key(&f.to_lowercase()))
            .cloned()
            .collect()
    } else {
        built.clone()
    };
    let site_dir = target
        .repo
        .as_deref()
        .map(|r| Path::new(r).join(&target.models));
    let owned: BTreeSet<String> = target
        .repo
        .as_deref()
        .and_then(|r| {
            read_record(&root)
                .targets
                .remove(&record_key(Path::new(r), &target.models))
        })
        .unwrap_or_default()
        .into_iter()
        .collect();
    let mut files = Vec::new();
    for file in &candidates {
        let from = dist.join(file);
        let to = site_dir.as_ref().map(|d| d.join(file));
        let before = to.as_ref().and_then(|t| size(t));
        let bytes = size(&from).unwrap_or(0.0);
        let change = match (&to, before) {
            (Some(t), Some(_)) if same_bytes(&from, t) => ShipChange::Unchanged,
            (_, Some(_)) if owned.contains(file) => ShipChange::Replaced,
            (_, Some(_)) => ShipChange::HandMade,
            _ => ShipChange::New,
        };
        let entry = by_file.get(&file.to_lowercase());
        let choice = entry.map(|(_, e)| e.choice).unwrap_or_default();
        let grows = before.is_some_and(|b| bytes > b);
        let why = held(choice, change, grows, ship.size_guard());
        files.push(SitePlanFile {
            file: file.clone(),
            asset: entry.map(|(id, _)| (*id).clone()),
            asset_name: entry.map(|(_, e)| e.name.clone()),
            change,
            choice,
            ships: why.is_none(),
            held: why,
            bytes,
            before_bytes: before,
            triangles: triangles(&from),
            before_triangles: to
                .as_ref()
                .filter(|_| before.is_some())
                .and_then(|t| triangles(t)),
        });
    }
    let held_files: BTreeSet<&str> = files
        .iter()
        .filter(|f| !f.ships)
        .map(|f| f.file.as_str())
        .collect();
    let left: Vec<SiteLeftFile> = site_dir
        .as_ref()
        .map(|d| glbs(d))
        .unwrap_or_default()
        .into_iter()
        .filter(|f| !candidates.contains(f))
        .map(|file| SiteLeftFile {
            bytes: site_dir
                .as_ref()
                .and_then(|d| size(&d.join(&file)))
                .unwrap_or(0.0),
            removed: owned.contains(&file) && !held_files.contains(file.as_str()),
            file,
        })
        .collect();
    let checks = checks(&target, &files, &ship, &root);
    Ok(SiteShipPlan {
        style: ship.style,
        textures: ship.textures,
        exported: !built.is_empty(),
        files,
        left,
        checks,
        script_ready: script_ready(&root),
        origin_ready: origin_ready(&root),
        target,
    })
}

fn check(ok: bool, yes: &str, no: String) -> ShipCheck {
    ShipCheck {
        ok,
        text: if ok { yes.to_string() } else { no },
    }
}

fn checks(
    target: &ShipTarget,
    files: &[SitePlanFile],
    ship: &ShipSettings,
    root: &Path,
) -> Vec<ShipCheck> {
    let mut out = vec![check(
        target.repo.is_some(),
        "The project has a site repo",
        "No site repo yet: Ship copies nothing. Pick one in Settings.".to_string(),
    )];
    if let Some(error) = &target.error {
        out.push(check(
            false,
            "",
            format!("{CONFIG_FILE} could not be read: {error}"),
        ));
    }
    if target.repo.is_some() {
        out.push(check(
            target.models_exist,
            "The models folder is in the site",
            format!(
                "{} is not in the site yet, Ship creates it. Set the models folder if the site keeps them elsewhere.",
                target.models
            ),
        ));
    }
    if ship.style == ShipStyle::Models {
        out.push(check(
            target.data.is_none(),
            "No scene data: the site places the models itself",
            "The site config writes scene data. A hand-built site usually wants none.".to_string(),
        ));
        out.push(check(
            origin_ready(root),
            "Models export at their own origin",
            "The starters are older and export the models where the scene puts them. Update Build scene and Export GLB in the Pipeline view.".to_string(),
        ));
    }
    let custom = !ship.assets.values().all(|a| a.choice == ShipChoice::Auto) || ship.size_guard();
    if custom {
        out.push(check(
            script_ready(root),
            "Ship follows the choices below",
            "tools/ship.mjs is older and copies every file. Update the Ship starter in the Pipeline view.".to_string(),
        ));
    }
    let held = files.iter().filter(|f| !f.ships).count();
    if held > 0 {
        out.push(check(
            false,
            "",
            format!(
                "{held} {} held back",
                if held == 1 { "file is" } else { "files are" }
            ),
        ));
    }
    out
}
