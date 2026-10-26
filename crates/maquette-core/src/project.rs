//! Projects: one folder per scene under the workspace's projects folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::assets::Asset;
use crate::error::{CoreError, CoreResult};
use crate::game::{GameSettings, NewGame, ProjectTarget};
use crate::json::write_json;
use crate::workspace::{FolderLayout, WorkspaceConfig, is_single_segment};

mod import;

pub use import::{
    BlendFile, ImportScan, RepoInspection, SiteFolders, inspect_site_repo, scan_import,
};
pub(crate) use import::{has_ext, sorted_children};

pub const PROJECT_FILE: &str = "project.json";
pub const PROJECT_VERSION: u32 = 1;
pub const NAME_MAX_CHARS: usize = 80;
const SLUG_MAX_CHARS: usize = 60;
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
    /// game.
    pub site_repo: Option<String>,
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

/// `"Moon Island"` becomes `"moon-island"`. Common accented Latin letters are
/// folded to ASCII, anything else is dropped. Never empty.
pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for c in name.chars() {
        let folded = fold_char(c);
        if folded.is_empty() {
            if c.is_whitespace() || c.is_ascii_punctuation() {
                pending_dash = true;
            }
            continue;
        }
        if pending_dash && !slug.is_empty() {
            slug.push('-');
        }
        pending_dash = false;
        slug.push_str(&folded);
    }
    let mut slug: String = slug.chars().take(SLUG_MAX_CHARS).collect();
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "project".to_string()
    } else {
        slug
    }
}

fn fold_char(c: char) -> String {
    if c.is_ascii_alphanumeric() {
        return c.to_ascii_lowercase().to_string();
    }
    let folded = match c.to_lowercase().next().unwrap_or(c) {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ą' | 'ā' | 'ă' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'č' | 'ĉ' => "c",
        'ď' | 'đ' | 'ð' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ę' | 'ē' | 'ě' => "e",
        'ğ' | 'ĝ' => "g",
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'ı' => "i",
        'ł' | 'ľ' | 'ĺ' => "l",
        'ñ' | 'ń' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => "o",
        'œ' => "oe",
        'ř' | 'ŕ' => "r",
        'ś' | 'š' | 'ş' | 'ŝ' => "s",
        'ß' => "ss",
        'ť' | 'ţ' => "t",
        'þ' => "th",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => "u",
        'ý' | 'ÿ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        _ => "",
    };
    folded.to_string()
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
                format: crate::game::GameFormat::Glb,
                models_dir: crate::game::DEFAULT_MODELS_DIR.to_string(),
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

    let manifest = ProjectManifest {
        version: PROJECT_VERSION,
        id,
        name,
        template,
        target: new.target,
        site_repo,
        source_folder: clean_optional(new.source_folder),
        game,
        created_at: now_rfc3339(),
        assets: Vec::new(),
    };
    write_json(&dir.join(PROJECT_FILE), &manifest)?;
    Ok(manifest)
}

pub fn load(workspace_root: &Path, layout: &FolderLayout, id: &str) -> CoreResult<ProjectManifest> {
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

/// Every readable project, oldest first. Folders without a readable
/// manifest are skipped with a warning.
pub fn list(workspace_root: &Path, layout: &FolderLayout) -> CoreResult<Vec<ProjectManifest>> {
    let parent = projects_dir(workspace_root, layout);
    let entries = match fs::read_dir(&parent) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&parent, err)),
    };
    let mut projects = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.join(PROJECT_FILE).is_file() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
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
    pub site_repo: Option<String>,
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
    modify(workspace_root, layout, id, |manifest| {
        manifest.name = name;
        manifest.site_repo = clean_optional(site_repo);
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
    let mut manifest = load(workspace_root, layout, id)?;
    change(&mut manifest)?;
    let path = dir(workspace_root, layout, id)?.join(PROJECT_FILE);
    write_json(&path, &manifest)?;
    Ok(manifest)
}

/// The current time as RFC 3339 UTC with milliseconds.
pub fn now_rfc3339() -> String {
    rfc3339_utc(SystemTime::now())
}

pub(crate) fn rfc3339_utc(time: SystemTime) -> String {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = since.as_secs();
    let millis = since.subsec_millis();
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        rem / 3600,
        (rem / 60) % 60,
        rem % 60
    )
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's
/// algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests;
