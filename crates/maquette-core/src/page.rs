//! The Page view's settings for one project, in `page.json` in the project
//! folder: the dev command typed in Settings > Project (none uses the
//! site's own, see [`crate::site::preview::command`]), an HTML page concept
//! the view can show instead of the site, which of the two it shows and the
//! path on the site it opens.
//!
//! ```json
//! { "version": 1, "devCommand": "pnpm dev --port {port}", "concept": null,
//!   "source": "site", "path": "/" }
//! ```

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::{from_json_with_defaults, write_json};
use crate::site::preview::command::{self, parse_line};
use crate::site::preview::{DevCommand, ServerStatus};
use crate::site::{self, CONFIG_FILE};

pub const PAGE_FILE: &str = "page.json";
pub const PAGE_VERSION: u32 = 1;
/// The longest path on the site the view opens.
pub const PATH_MAX_CHARS: usize = 300;

/// What the Page view shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PageSource {
    /// The site's own dev server.
    #[default]
    Site,
    /// A local HTML page concept.
    Concept,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PageSettings {
    pub version: u32,
    /// The dev command as typed, `{port}` for the port Maquette picks;
    /// `None` runs the site's own.
    pub dev_command: Option<String>,
    /// An HTML page concept inside the workspace, as an absolute path.
    pub concept: Option<String>,
    pub source: PageSource,
    /// The path on the site, from `/`.
    pub path: String,
}

impl Default for PageSettings {
    fn default() -> Self {
        Self {
            version: PAGE_VERSION,
            dev_command: None,
            concept: None,
            source: PageSource::Site,
            path: "/".to_string(),
        }
    }
}

/// The project's page settings, or the defaults without `page.json`.
///
/// # Errors
///
/// Fails when the file cannot be read, is not JSON or comes from a newer
/// Maquette.
pub fn load(project_dir: &Path) -> CoreResult<PageSettings> {
    let file = project_dir.join(PAGE_FILE);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(PageSettings::default()),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let settings: PageSettings =
        from_json_with_defaults(&text).map_err(|e| CoreError::json(&file, e))?;
    if settings.version > PAGE_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: settings.version,
            supported: PAGE_VERSION,
        });
    }
    Ok(settings)
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// `settings` tidied and checked: the dev command must parse, the concept
/// must be an `.html` file inside `workspace_root` (the app only serves
/// files from there) and the path must start with `/`.
///
/// # Errors
///
/// Returns the first problem as a sentence.
pub fn validate(settings: &PageSettings, workspace_root: &Path) -> CoreResult<PageSettings> {
    let mut next = settings.clone();
    next.version = PAGE_VERSION;
    next.dev_command = clean(settings.dev_command.as_deref());
    if let Some(command) = next.dev_command.as_deref() {
        parse_line(command)?;
    }
    next.concept = clean(settings.concept.as_deref());
    if let Some(concept) = next.concept.as_deref() {
        check_concept(Path::new(concept), workspace_root)?;
    }
    if next.source == PageSource::Concept && next.concept.is_none() {
        return Err(CoreError::Invalid(
            "Pick an HTML page concept before showing it.".into(),
        ));
    }
    let path = settings.path.trim();
    next.path = if path.is_empty() {
        "/".to_string()
    } else {
        path.to_string()
    };
    if !next.path.starts_with('/')
        || next.path.starts_with("//")
        || next
            .path
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
        || next.path.chars().count() > PATH_MAX_CHARS
    {
        return Err(CoreError::Invalid(format!(
            "The page path must start with / and have no spaces, like /pricing, not {:?}.",
            next.path
        )));
    }
    Ok(next)
}

fn check_concept(file: &Path, workspace_root: &Path) -> CoreResult<()> {
    let html = file
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"));
    if !html {
        return Err(CoreError::Invalid(format!(
            "A page concept is an .html file, not {}.",
            file.display()
        )));
    }
    let real = file
        .canonicalize()
        .map_err(|_| CoreError::NotFound(format!("the page concept {}", file.display())))?;
    let root = workspace_root
        .canonicalize()
        .map_err(|e| CoreError::io(workspace_root, e))?;
    if !real.starts_with(&root) || !real.is_file() {
        return Err(CoreError::Invalid(format!(
            "Keep the page concept inside the workspace ({}), for example in the project's concepts folder, so Maquette can show it.",
            workspace_root.display()
        )));
    }
    Ok(())
}

/// Checks `settings` and writes them to `page.json`.
///
/// # Errors
///
/// Fails when [`validate`] refuses them or the file cannot be written.
pub fn save(
    project_dir: &Path,
    workspace_root: &Path,
    settings: &PageSettings,
) -> CoreResult<PageSettings> {
    let next = validate(settings, workspace_root)?;
    write_json(&project_dir.join(PAGE_FILE), &next)?;
    Ok(next)
}

/// What the Page view and the agents read for one project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PageView {
    pub project: String,
    pub site_repo: Option<String>,
    pub settings: PageSettings,
    /// The command that starts the site's dev server, before the port.
    pub command: Option<DevCommand>,
    /// Why there is none: no site repo, no package.json, no dev script.
    pub command_error: Option<String>,
    /// The site config file, when the site has one.
    pub site_config: Option<String>,
    /// The app's dev server, when it serves this project; idle otherwise.
    pub server: ServerStatus,
    /// The project the dev server serves now, when it is another one.
    pub busy_with: Option<String>,
}

/// The dev command for the project's site, or why there is none.
///
/// # Errors
///
/// Fails when there is no site repo, it is gone, its config is invalid or
/// it has nothing to run.
pub fn dev_command(site_repo: Option<&str>, settings: &PageSettings) -> CoreResult<DevCommand> {
    let repo = site_repo.map(Path::new).ok_or_else(|| {
        CoreError::Invalid(
            "This project has no site repo yet. Pick one in Settings > Project.".into(),
        )
    })?;
    if !repo.is_dir() {
        return Err(CoreError::NotFound(format!(
            "the site repo {}",
            repo.display()
        )));
    }
    let config = site::load_config(repo)?;
    command::choose(repo, &config, settings.dev_command.as_deref())
}

/// The project's page view with the app's dev server `status`.
pub fn view(
    project: &str,
    site_repo: Option<&str>,
    settings: PageSettings,
    status: &ServerStatus,
) -> PageView {
    let (command, command_error) = match dev_command(site_repo, &settings) {
        Ok(c) => (Some(c), None),
        Err(e) => (None, Some(e.to_string())),
    };
    let site_config = site_repo
        .map(|r| Path::new(r).join(CONFIG_FILE))
        .filter(|f| f.is_file())
        .map(|f| f.to_string_lossy().into_owned());
    let mine = status.project.as_deref() == Some(project);
    let busy_with = status.project.clone().filter(|p| {
        !mine && status.state != crate::site::preview::ServerState::Idle && p != project
    });
    PageView {
        project: project.to_string(),
        site_repo: site_repo.map(str::to_string),
        settings,
        command,
        command_error,
        site_config,
        server: if mine {
            status.clone()
        } else {
            ServerStatus::default()
        },
        busy_with,
    }
}

#[cfg(test)]
mod tests;
