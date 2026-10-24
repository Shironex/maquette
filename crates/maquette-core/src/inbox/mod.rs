//! The model inbox: `.glb` and `.gltf` files dropped into the workspace's
//! inbox folder, analysed and checked, then accepted into a project's
//! `models/` folder as a new asset version.

pub mod analyse;
pub mod checks;
pub mod fixture;
pub(crate) mod images;
pub(crate) mod math;
pub mod watch;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

pub use analyse::{GlbReport, RigKind, analyse};
pub use checks::{Check, CheckStatus};

use crate::assets::{
    self, Asset, AssetKind, AssetStatus, AssetVersion, ModelStats, NewAsset, Stage,
};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project;
use crate::thumbs::ThumbState;
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
    /// True until someone picks the kind.
    pub kind_guessed: bool,
    pub state: InboxState,
    pub report: Option<GlbReport>,
    pub checks: Vec<Check>,
    pub worst: CheckStatus,
    pub error: Option<String>,
    /// The studio render of the file, once it exists (an absolute path).
    pub thumbnail: Option<String>,
    pub thumbnail_state: ThumbState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AcceptRequest {
    pub item: String,
    pub project: String,
    /// An existing asset to add a version to. Without it, an asset named
    /// `name` (or the item's name) is found or created.
    pub asset: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AcceptResult {
    pub asset: Asset,
    pub version: AssetVersion,
    /// Where the file went.
    pub file: String,
    /// The project after the change, so the Board can update.
    pub project: crate::project::ProjectManifest,
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

fn stamp(meta: &fs::Metadata) -> (u64, u128) {
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
        .map(|r| checks::run(r, kind, config))
        .unwrap_or_default();
    #[expect(
        clippy::cast_precision_loss,
        reason = "sizes and epoch ms stay far below 2^53"
    )]
    let (bytes, modified_ms) = (len as f64, (modified_ns / 1_000_000) as f64);
    Ok(InboxItem {
        name: display_name(&id),
        file: path.display().to_string(),
        bytes,
        modified_ms,
        kind,
        kind_guessed: picked.is_none(),
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
    build_item(&path, &cache, config, &load_kinds(&cache))
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

fn move_file(from: &Path, to: &Path) -> CoreResult<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    // A different volume: copy, then remove the original.
    fs::copy(from, to).map_err(|e| CoreError::io(to, e))?;
    fs::remove_file(from).map_err(|e| CoreError::io(from, e))
}

pub fn stats_of(report: &GlbReport) -> ModelStats {
    ModelStats {
        triangles: report.triangles,
        bytes: report.bytes,
        textures: u32::try_from(report.textures.len()).unwrap_or(u32::MAX),
        max_texture: report.max_texture,
        joints: report.joints,
        animations: u32::try_from(report.animations.len()).unwrap_or(u32::MAX),
    }
}

/// Moves the model into the project's `models/` folder as the next version
/// of its asset and records it, with the asset moved to the Blender column
/// for review.
///
/// # Errors
///
/// Fails for a missing item, a glTF that depends on files next to it, an
/// unknown project or asset, or when the file cannot be moved.
pub fn accept(
    root: &Path,
    config: &WorkspaceConfig,
    request: &AcceptRequest,
) -> CoreResult<AcceptResult> {
    let found = item(root, config, &request.item)?;
    let source = PathBuf::from(&found.file);
    if external_files(&source) {
        return Err(CoreError::Invalid(
            "This glTF uses files next to it. Export it as a single .glb, then accept it."
                .to_string(),
        ));
    }
    let project_dir = project::dir(root, &config.layout, &request.project)?;
    let models = project_dir.join("models");
    let ext = source
        .extension()
        .map_or_else(|| "glb".to_string(), |e| e.to_string_lossy().to_lowercase());
    let stats = found.report.as_ref().map(stats_of);
    let mut result: Option<(Asset, AssetVersion, String)> = None;
    let manifest = project::modify(root, &config.layout, &request.project, |manifest| {
        let id = match &request.asset {
            Some(id) => assets::find(manifest, id)?.id.clone(),
            None => {
                let name = request.name.clone().unwrap_or_else(|| found.name.clone());
                match assets::find_by_name(manifest, &name) {
                    Some(asset) => asset.id.clone(),
                    None => {
                        assets::add(
                            manifest,
                            NewAsset {
                                name,
                                kind: found.kind,
                                stage: Stage::Tripo,
                            },
                        )?
                        .id
                    }
                }
            }
        };
        let number = assets::find(manifest, &id)?.next_version();
        let file_name = format!("{}_v{number}.{ext}", id.replace('-', "_"));
        let target = models.join(&file_name);
        if target.exists() {
            return Err(CoreError::Invalid(format!(
                "models/{file_name} already exists. Rename it or remove it first."
            )));
        }
        fs::create_dir_all(&models).map_err(|e| CoreError::io(&models, e))?;
        move_file(&source, &target)?;
        let version =
            assets::add_version(manifest, &id, format!("models/{file_name}"), stats.clone())?;
        let current = assets::find(manifest, &id)?.stage;
        if current < Stage::Blender {
            assets::move_to(manifest, &id, Stage::Blender)?;
        }
        assets::set_status(manifest, &id, AssetStatus::Review)?;
        result = Some((
            assets::find(manifest, &id)?.clone(),
            version,
            target.display().to_string(),
        ));
        Ok(())
    })?;
    let _ = fs::remove_file(cache_dir(root, config).join(format!("{}.json", request.item)));
    let (asset, version, file) =
        result.ok_or_else(|| CoreError::Invalid("The model was not accepted.".to_string()))?;
    Ok(AcceptResult {
        asset,
        version,
        file,
        project: manifest,
    })
}

/// The arguments that open `file` in Blender's window: a `.blend` opens as
/// is; a model is imported into an empty scene. Only for the explicit Open
/// in Blender action, never for pipeline steps.
pub fn open_in_blender_args(file: &Path) -> Vec<String> {
    let is_blend = file
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("blend"));
    if is_blend {
        return vec![file.display().to_string()];
    }
    vec![
        "--python-expr".to_string(),
        "import bpy, sys; bpy.ops.wm.read_factory_settings(use_empty=True); \
         bpy.ops.import_scene.gltf(filepath=sys.argv[sys.argv.index('--') + 1])"
            .to_string(),
        "--".to_string(),
        file.display().to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{NewProject, ProjectTemplate};

    fn workspace() -> (tempfile::TempDir, WorkspaceConfig) {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = WorkspaceConfig::default();
        fs::create_dir_all(inbox_dir(dir.path(), &config)).expect("inbox");
        (dir, config)
    }

    fn drop_in(root: &Path, config: &WorkspaceConfig, name: &str, bytes: &[u8]) {
        fs::write(inbox_dir(root, config).join(name), bytes).expect("write");
    }

    #[test]
    fn names_and_kinds() {
        assert_eq!(display_name("character_tails_v2.glb"), "Character tails");
        assert_eq!(display_name("Sakura-Tree.glb"), "Sakura tree");
        assert_eq!(display_name("v2.glb"), "V2");
        assert_eq!(guess_kind(None, "moon_island.glb"), AssetKind::Scene);
        assert_eq!(guess_kind(None, "lantern.glb"), AssetKind::Prop);
    }

    #[test]
    fn lists_analyses_and_caches() {
        let (dir, config) = workspace();
        let root = dir.path();
        assert!(
            list(&root.join("nowhere"), &config)
                .expect("missing inbox")
                .is_empty()
        );
        drop_in(root, &config, "crate_clean.glb", &fixture::clean_prop());
        drop_in(
            root,
            &config,
            "character_tails.glb",
            &fixture::character(true),
        );
        drop_in(root, &config, "broken.glb", b"half a download");
        drop_in(root, &config, "readme.txt", b"ignored");
        let items = list(root, &config).expect("list");
        assert_eq!(items.len(), 3);
        let character = items
            .iter()
            .find(|i| i.id == "character_tails.glb")
            .expect("character");
        assert_eq!(character.kind, AssetKind::Character);
        assert!(character.kind_guessed);
        assert_eq!(character.worst, CheckStatus::Fail);
        assert_eq!(character.state, InboxState::Ready);
        let broken = items.iter().find(|i| i.id == "broken.glb").expect("broken");
        assert_eq!(broken.state, InboxState::Failed);
        assert!(broken.error.is_some());
        assert!(
            cache_dir(root, &config)
                .join("character_tails.glb.json")
                .is_file()
        );
        assert!(!cache_dir(root, &config).join("broken.glb.json").exists());

        // The cache is used: a doctored record comes back as is.
        let cache_file = cache_dir(root, &config).join("crate_clean.glb.json");
        let mut record: CachedReport =
            serde_json::from_str(&fs::read_to_string(&cache_file).expect("read")).expect("json");
        record.report.triangles = 7;
        write_json(&cache_file, &record).expect("write");
        assert_eq!(
            item(root, &config, "crate_clean.glb")
                .expect("item")
                .report
                .expect("r")
                .triangles,
            7
        );
        assert_eq!(
            reprocess(root, &config, "crate_clean.glb")
                .expect("again")
                .report
                .expect("r")
                .triangles,
            12
        );

        let prop = set_kind(root, &config, "character_tails.glb", AssetKind::Prop).expect("kind");
        assert!(!prop.kind_guessed);
        assert_eq!(prop.kind, AssetKind::Prop);
        assert!(item(root, &config, "../escape.glb").is_err());
        assert!(item(root, &config, "readme.txt").is_err());
        assert_eq!(
            item(root, &config, "gone.glb").expect_err("missing").code(),
            "not-found"
        );
    }

    #[test]
    fn accept_moves_the_file_and_registers_versions() {
        let (dir, config) = workspace();
        let root = dir.path();
        let manifest = project::create(
            root,
            &config.layout,
            NewProject {
                name: "Moon Island".into(),
                template: ProjectTemplate::FloatingDiorama,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        drop_in(
            root,
            &config,
            "character_tails.glb",
            &fixture::character(true),
        );
        let request = AcceptRequest {
            item: "character_tails.glb".into(),
            project: manifest.id.clone(),
            asset: None,
            name: Some("Moon girl".into()),
        };
        let first = accept(root, &config, &request).expect("accept");
        assert_eq!(first.asset.id, "moon-girl");
        assert_eq!(first.asset.kind, AssetKind::Character);
        assert_eq!(first.asset.stage, Stage::Blender);
        assert_eq!(first.asset.status, AssetStatus::Review);
        assert_eq!(first.version.version, 1);
        assert_eq!(first.version.file, "models/moon_girl_v1.glb");
        assert_eq!(first.version.stats.as_ref().expect("stats").joints, 65);
        assert!(Path::new(&first.file).is_file());
        assert!(
            !inbox_dir(root, &config)
                .join("character_tails.glb")
                .exists()
        );

        drop_in(
            root,
            &config,
            "character_tails.glb",
            &fixture::character(false),
        );
        let second = accept(
            root,
            &config,
            &AcceptRequest {
                asset: Some("moon-girl".into()),
                name: None,
                ..request.clone()
            },
        )
        .expect("again");
        assert_eq!(second.version.version, 2);
        assert_eq!(second.version.file, "models/moon_girl_v2.glb");
        let loaded = project::load(root, &config.layout, &manifest.id).expect("load");
        assert_eq!(loaded.assets.len(), 1);
        assert_eq!(loaded.assets[0].versions.len(), 2);

        assert!(accept(root, &config, &request).is_err(), "the file is gone");
        drop_in(
            root,
            &config,
            "scene.gltf",
            br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"scene.bin","byteLength":4}]}"#,
        );
        let err = accept(
            root,
            &config,
            &AcceptRequest {
                item: "scene.gltf".into(),
                ..request
            },
        )
        .expect_err("external");
        assert!(err.to_string().contains("single .glb"));
    }

    #[test]
    fn open_in_blender_imports_models_and_opens_blends() {
        assert_eq!(
            open_in_blender_args(Path::new("/w/a.blend")),
            vec!["/w/a.blend".to_string()]
        );
        let args = open_in_blender_args(Path::new("/w/inbox/a.glb"));
        assert_eq!(args[0], "--python-expr");
        assert!(args[1].contains("import_scene.gltf"));
        assert_eq!(&args[2..], ["--".to_string(), "/w/inbox/a.glb".to_string()]);
        assert!(
            !args.iter().any(|a| a == "-b"),
            "this one opens the window on purpose"
        );
    }
}
