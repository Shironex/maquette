//! Projects: one folder per scene under the workspace's projects folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::assets::Asset;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::settings::game::{GameSettings, NewGame, ProjectTarget};
use crate::workspace::{FolderLayout, WorkspaceConfig, is_single_segment};

mod clock;
mod import;
pub mod repos;
mod slug;

pub use clock::now_rfc3339;
pub(crate) use clock::rfc3339_utc;
pub use import::{
    BlendFile, ImportScan, RepoInspection, SiteFolders, inspect_site_repo, scan_import,
};
pub(crate) use import::{has_ext, sorted_children};
pub use repos::SiteRepoState;
pub use slug::slugify;

pub const PROJECT_FILE: &str = "project.json";
pub const PROJECT_VERSION: u32 = 1;
pub const NAME_MAX_CHARS: usize = 80;
const PROJECT_DIRS: [&str; 6] = ["concepts", "refs", "models", "blender", "web", "tools"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectTemplate {
    FloatingDiorama,
    Room,
    CharacterShowcase,
    Imported,
    /// A game project, started from the models already in the game.
    GameImport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub version: u32,
    /// The folder name. Always matches the folder on load, even if the user
    /// renamed it by hand.
    pub id: String,
    pub name: String,
    pub template: ProjectTemplate,
    /// Where it ships. Older files have none: they are websites.
    #[serde(default)]
    pub target: ProjectTarget,
    /// The repo Ship writes to: the site of a website, the game repo of a
    /// game. This computer's path (see `project::repos`); `project.json` itself
    /// keeps it only from older files.
    pub site_repo: Option<String>,
    /// The repo's folder name, saved so another computer can ask for it.
    #[serde(default)]
    pub site_repo_name: Option<String>,
    /// Whether this computer knows where the repo is. Never saved.
    #[serde(default)]
    pub site_repo_state: SiteRepoState,
    pub source_folder: Option<String>,
    /// A game project's settings; none for a website.
    #[serde(default)]
    pub game: Option<Box<GameSettings>>,
    /// RFC 3339 UTC with milliseconds, for example `2026-10-24T09:30:00.123Z`.
    pub created_at: String,
    /// The Board's assets. Older files have none, so it defaults to empty.
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewProject {
    pub name: String,
    pub template: ProjectTemplate,
    /// Left out, a website.
    #[serde(default)]
    pub target: ProjectTarget,
    pub site_repo: Option<String>,
    pub source_folder: Option<String>,
    /// For a game: the format and the model folder New project found.
    #[serde(default)]
    pub game: Option<NewGame>,
}

fn projects_dir(workspace_root: &Path, layout: &FolderLayout) -> PathBuf {
    workspace_root.join(&layout.projects)
}

/// The folder of project `id`. Fails for anything that is not a plain id.
pub fn dir(workspace_root: &Path, layout: &FolderLayout, id: &str) -> CoreResult<PathBuf> {
    check_id(id)?;
    Ok(projects_dir(workspace_root, layout).join(id))
}

fn clean_name(name: &str) -> CoreResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("Give the project a name.".to_string()));
    }
    if name.chars().count() > NAME_MAX_CHARS {
        return Err(CoreError::Invalid(format!(
            "Keep the project name to {NAME_MAX_CHARS} characters or fewer."
        )));
    }
    Ok(name.to_string())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn check_id(id: &str) -> CoreResult<()> {
    if is_single_segment(id) {
        Ok(())
    } else {
        Err(CoreError::Invalid(format!("{id:?} is not a project id.")))
    }
}

pub fn create(
    workspace_root: &Path,
    layout: &FolderLayout,
    new: NewProject,
) -> CoreResult<ProjectManifest> {
    let name = clean_name(&new.name)?;
    let site_repo = clean_optional(new.site_repo);
    let (template, game) = match new.target {
        ProjectTarget::Website if new.template == ProjectTemplate::GameImport => {
            return Err(CoreError::Invalid(
                "A website project starts from a template or a folder.".to_string(),
            ));
        }
        ProjectTarget::Website => (new.template, None),
        ProjectTarget::Game => {
            let wanted = new.game.unwrap_or(NewGame {
                format: crate::settings::game::GameFormat::Glb,
                models_dir: crate::settings::game::DEFAULT_MODELS_DIR.to_string(),
            });
            let repo = site_repo.as_deref().map(Path::new);
            let settings = crate::game::new_settings(&wanted, repo)?;
            (ProjectTemplate::GameImport, Some(Box::new(settings)))
        }
    };
    let base = slugify(&name);
    let parent = projects_dir(workspace_root, layout);
    fs::create_dir_all(&parent).map_err(|e| CoreError::io(&parent, e))?;

    // create_dir (not create_dir_all) claims the slug atomically, so two
    // creates with the same name cannot share a folder.
    let mut suffix = 1u32;
    let (id, dir) = loop {
        let id = if suffix == 1 {
            base.clone()
        } else {
            format!("{base}-{suffix}")
        };
        let dir = parent.join(&id);
        match fs::create_dir(&dir) {
            Ok(()) => break (id, dir),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => suffix += 1,
            Err(err) => return Err(CoreError::io(&dir, err)),
        }
    };
    for sub in PROJECT_DIRS {
        let path = dir.join(sub);
        fs::create_dir_all(&path).map_err(|e| CoreError::io(&path, e))?;
    }

    repos::set(workspace_root, &id, site_repo.clone());
    let (site_repo, site_repo_state) = repos::resolve(workspace_root, &id, None, None);
    let manifest = ProjectManifest {
        version: PROJECT_VERSION,
        id,
        name,
        template,
        target: new.target,
        site_repo_name: site_repo.as_deref().and_then(repos::folder_name),
        site_repo,
        site_repo_state,
        source_folder: clean_optional(new.source_folder),
        game,
        created_at: now_rfc3339(),
        assets: Vec::new(),
    };
    write_manifest(workspace_root, &dir.join(PROJECT_FILE), &manifest, None)?;
    Ok(manifest)
}

/// Writes `manifest` as `project.json`: without the repo state, and with
/// no repo path but an older file's `legacy` one that has not moved into
/// this computer's map yet.
fn write_manifest(
    workspace_root: &Path,
    path: &Path,
    manifest: &ProjectManifest,
    legacy: Option<String>,
) -> CoreResult<()> {
    let mut value = serde_json::to_value(manifest).map_err(|e| CoreError::json(path, e))?;
    if let Some(object) = value.as_object_mut() {
        object.remove("siteRepoState");
        let kept = repos::legacy_to_keep(workspace_root, &manifest.id, legacy);
        object.insert(
            "siteRepo".into(),
            kept.map_or(serde_json::Value::Null, Into::into),
        );
    }
    write_json(path, &value)
}

/// The manifest with this computer's repo filled in (see [`repos::resolve`]).
pub fn load(workspace_root: &Path, layout: &FolderLayout, id: &str) -> CoreResult<ProjectManifest> {
    let raw = load_raw(workspace_root, layout, id)?;
    Ok(with_repo(workspace_root, raw))
}

/// `raw` with this computer's repo in place of the file's.
fn with_repo(workspace_root: &Path, mut manifest: ProjectManifest) -> ProjectManifest {
    let legacy = manifest.site_repo.take();
    let (path, state) = repos::resolve(
        workspace_root,
        &manifest.id,
        legacy.as_deref(),
        manifest.site_repo_name.as_deref(),
    );
    if manifest.site_repo_name.is_none() {
        manifest.site_repo_name = legacy.as_deref().and_then(repos::folder_name);
    }
    manifest.site_repo = path;
    manifest.site_repo_state = state;
    manifest
}

/// The manifest as `project.json` has it, `siteRepo` and all.
fn load_raw(workspace_root: &Path, layout: &FolderLayout, id: &str) -> CoreResult<ProjectManifest> {
    check_id(id)?;
    let path = projects_dir(workspace_root, layout)
        .join(id)
        .join(PROJECT_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!("the project {id}")));
        }
        Err(err) => return Err(CoreError::io(&path, err)),
    };
    let mut manifest: ProjectManifest =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&path, e))?;
    if manifest.version > PROJECT_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: manifest.version,
            supported: PROJECT_VERSION,
        });
    }
    manifest.version = PROJECT_VERSION;
    manifest.id = id.to_string();
    Ok(manifest)
}

/// The ids of the folders that hold a `project.json`, in no order.
fn ids(workspace_root: &Path, layout: &FolderLayout) -> Vec<String> {
    let parent = projects_dir(workspace_root, layout);
    let Ok(entries) = fs::read_dir(&parent) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join(PROJECT_FILE).is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|id| !crate::workspace::sync::is_sync_name(id))
        .collect()
}

/// Every readable project, oldest first. Folders without a readable
/// manifest are skipped with a warning.
pub fn list(workspace_root: &Path, layout: &FolderLayout) -> CoreResult<Vec<ProjectManifest>> {
    let parent = projects_dir(workspace_root, layout);
    if let Err(err) = fs::read_dir(&parent) {
        if err.kind() == io::ErrorKind::NotFound {
            return Ok(Vec::new());
        }
        return Err(CoreError::io(&parent, err));
    }
    let mut projects = Vec::new();
    for id in ids(workspace_root, layout) {
        match load(workspace_root, layout, &id) {
            Ok(manifest) => projects.push(manifest),
            Err(err) => tracing::warn!(project = %id, error = %err, "skipping unreadable project"),
        }
    }
    projects.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(projects)
}

/// What runs in a project right now. Core leaves it empty; the shell fills
/// it from its runner, the Lab and the provider queue.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectActivity {
    /// The id of the pipeline run going now.
    pub run: Option<String>,
    /// Concept images queued or generating.
    pub concepts: u32,
    /// Send to 3D jobs on their way (By hand ones waiting count too).
    pub sends: u32,
}

/// One row of the project switcher.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub template: ProjectTemplate,
    pub target: ProjectTarget,
    /// This computer's path to the repo.
    pub site_repo: Option<String>,
    pub site_repo_name: Option<String>,
    pub site_repo_state: SiteRepoState,
    pub created_at: String,
    pub asset_count: u32,
    /// Absolute, so the UI can resolve `thumb`.
    pub dir: String,
    /// The picture the switcher shows, relative to the project folder: the
    /// first poster, else a scene's thumbnail, else the newest card's.
    /// `None` when the project has none yet.
    pub thumb: Option<String>,
    pub activity: ProjectActivity,
}

const POSTERS_DIR: &str = "web/posters";
const PICTURE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

fn project_thumb(dir: &Path, manifest: &ProjectManifest) -> Option<String> {
    let poster = sorted_children(&dir.join(POSTERS_DIR))
        .into_iter()
        .find(|(_, path)| path.is_file() && has_ext(path, &PICTURE_EXTENSIONS))
        .map(|(name, _)| format!("{POSTERS_DIR}/{name}"));
    if poster.is_some() {
        return poster;
    }
    let mut pictured: Vec<&Asset> = manifest
        .assets
        .iter()
        .filter(|a| a.thumbnail.as_ref().is_some_and(|t| dir.join(t).is_file()))
        .collect();
    // Scenes first, then the card that changed last.
    pictured.sort_by(|a, b| {
        let scene = |x: &Asset| x.kind != crate::assets::AssetKind::Scene;
        scene(a)
            .cmp(&scene(b))
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });
    pictured.first().and_then(|a| a.thumbnail.clone())
}

pub fn summary(
    workspace_root: &Path,
    layout: &FolderLayout,
    manifest: &ProjectManifest,
) -> ProjectSummary {
    let dir = projects_dir(workspace_root, layout).join(&manifest.id);
    ProjectSummary {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        template: manifest.template,
        target: manifest.target,
        site_repo: manifest.site_repo.clone(),
        site_repo_name: manifest.site_repo_name.clone(),
        site_repo_state: manifest.site_repo_state,
        created_at: manifest.created_at.clone(),
        asset_count: u32::try_from(manifest.assets.len()).unwrap_or(u32::MAX),
        thumb: project_thumb(&dir, manifest),
        dir: dir.display().to_string(),
        activity: ProjectActivity::default(),
    }
}

/// Every readable project as the switcher lists it, oldest first.
pub fn summaries(workspace_root: &Path, layout: &FolderLayout) -> CoreResult<Vec<ProjectSummary>> {
    Ok(list(workspace_root, layout)?
        .iter()
        .map(|m| summary(workspace_root, layout, m))
        .collect())
}

/// Makes `id` the current project and writes that to `maquette.json`, so
/// it opens again after a restart. Fails, and changes nothing, when the
/// project cannot be read.
pub fn switch(
    workspace_root: &Path,
    config: &WorkspaceConfig,
    id: &str,
) -> CoreResult<(WorkspaceConfig, ProjectManifest)> {
    let manifest = load(workspace_root, &config.layout, id)?;
    let mut next = config.clone();
    next.current_project = Some(manifest.id.clone());
    crate::workspace::save(workspace_root, &next)?;
    Ok((next, manifest))
}

pub fn update(
    workspace_root: &Path,
    layout: &FolderLayout,
    id: &str,
    name: String,
    site_repo: Option<String>,
) -> CoreResult<ProjectManifest> {
    let name = clean_name(&name)?;
    let site_repo = clean_optional(site_repo);
    modify(workspace_root, layout, id, |manifest| {
        // A rename on a computer that was never told where the repo is
        // keeps the name, so it is still asked for.
        let changes_repo = site_repo.is_some() || repos::get(workspace_root, id).is_some();
        repos::set(workspace_root, id, site_repo.clone());
        let (site_repo, state) = repos::resolve(workspace_root, id, None, None);
        manifest.name = name;
        if changes_repo {
            manifest.site_repo_name = site_repo.as_deref().and_then(repos::folder_name);
            manifest.site_repo = site_repo;
            manifest.site_repo_state = state;
        }
        Ok(())
    })
}

/// Serialises manifest writes in this process: the UI, the runner and the
/// provider jobs all change `project.json` from their own threads.
static WRITES: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Loads the manifest, applies `change` and writes it back atomically. The
/// file is left alone when `change` fails. Writers in this process take
/// turns, so two changes never overwrite each other.
pub fn modify<T>(
    workspace_root: &Path,
    layout: &FolderLayout,
    id: &str,
    change: impl FnOnce(&mut ProjectManifest) -> CoreResult<T>,
) -> CoreResult<ProjectManifest> {
    let _turn = WRITES.lock().unwrap_or_else(|p| p.into_inner());
    let raw = load_raw(workspace_root, layout, id)?;
    let legacy = raw.site_repo.clone();
    let mut manifest = with_repo(workspace_root, raw);
    change(&mut manifest)?;
    let path = dir(workspace_root, layout, id)?.join(PROJECT_FILE);
    write_manifest(workspace_root, &path, &manifest, legacy)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests;
