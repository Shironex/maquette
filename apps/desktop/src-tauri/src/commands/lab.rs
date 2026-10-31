//! The Concept lab: the shell side of `maquette_core::concepts`. Jobs run
//! on the shared Codex lane and reach the web view as `ConceptUpdate`
//! events.

use std::path::{Path, PathBuf};
use std::time::Duration;

use maquette_core::concepts::backend::ImageJobKind;
use maquette_core::concepts::jobs::{Lane, Start};

use super::image_api::{self, Backend};
use maquette_core::concepts::{
    self, ConceptJob, Concepts, Direction, MovedDirection, NewConcepts, NewObjectRefs, REFS_DIR,
    SentObjects, SentPicks, StyleBoard, StyleBoardChange, sheet, style,
};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{JobState, ProviderJobs};
use maquette_core::toolchain::ProbeEnv;
use maquette_core::{AssetKind, CoreError, WorkspaceConfig, project};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_specta::Event as _;

use super::CommandResult;
use crate::events::ConceptUpdate;
use crate::session::Session;

/// The service the app shares between commands.
pub struct Lab(pub Concepts);

/// The file in the app's config folder that keeps the Lab's jobs across
/// restarts.
pub const KEPT_JOBS: &str = "lab-jobs.json";

impl Lab {
    /// Brings back the jobs kept in `config_dir` (failed, ready for Retry)
    /// and keeps them there from now on.
    pub fn new(handle: AppHandle, config_dir: &Path) -> Self {
        let concepts = Concepts::new(codex_home(), 4, move |event| {
            if super::codex_usage::settles(&event) {
                super::codex_usage::read_soon(&handle);
            }
            if let Err(err) = ConceptUpdate(event).emit(&handle) {
                tracing::warn!(error = %err, "could not emit a concept event");
            }
        });
        let restored = concepts.keep_jobs_in(config_dir.join(KEPT_JOBS));
        if !restored.is_empty() {
            tracing::info!(
                jobs = restored.len(),
                "brought back the Lab jobs of the last session"
            );
        }
        Self(concepts)
    }
}

/// `$CODEX_HOME`, or `~/.codex`.
pub(crate) fn codex_home() -> Option<PathBuf> {
    let env = ProbeEnv::from_system();
    env.codex_home
        .or_else(|| env.home.map(|home| home.join(".codex")))
}

/// An image in the project's `refs/` folder.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RefImage {
    /// Relative to the project folder.
    pub file: String,
    pub name: String,
    pub bytes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LabView {
    pub project: String,
    /// Absolute, for the image URLs.
    pub project_dir: String,
    /// The most recently changed first.
    pub directions: Vec<Direction>,
    /// Oldest first.
    pub jobs: Vec<ConceptJob>,
    pub refs: Vec<RefImage>,
    pub concurrency: u32,
    /// The project's style board, empty when it has none.
    pub style: StyleBoard,
}

const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

pub(crate) fn list_refs(project_dir: &Path) -> Vec<RefImage> {
    let Ok(entries) = std::fs::read_dir(project_dir.join(REFS_DIR)) else {
        return Vec::new();
    };
    let mut out: Vec<RefImage> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_image(p) && !maquette_core::workspace::sync::is_sync_path(p))
        .filter_map(|p| {
            let name = p.file_name()?.to_string_lossy().into_owned();
            #[expect(clippy::cast_precision_loss, reason = "image sizes stay small")]
            let bytes = p.metadata().map(|m| m.len() as f64).unwrap_or(0.0);
            Some(RefImage {
                file: format!("{REFS_DIR}/{name}"),
                name,
                bytes,
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn project_dir(
    session: &Session,
    project_id: &str,
) -> CommandResult<(PathBuf, WorkspaceConfig, PathBuf)> {
    let (root, config) = session.workspace()?;
    // Loading the manifest checks that the project exists.
    project::load(&root, &config.layout, project_id)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    Ok((root, config, dir))
}

fn cache_dir(root: &Path, config: &WorkspaceConfig) -> PathBuf {
    root.join(&config.layout.cache).join("concepts")
}

pub(crate) fn view(session: &Session, lab: &Concepts, project_id: &str) -> CommandResult<LabView> {
    let (_, config, dir) = project_dir(session, project_id)?;
    Ok(LabView {
        project: project_id.to_string(),
        project_dir: dir.display().to_string(),
        directions: concepts::list_directions(&dir),
        jobs: lab.jobs(Some(project_id)),
        refs: list_refs(&dir),
        concurrency: config.lab.concurrency,
        style: style::load_or_default(&dir),
    })
}

/// Queues one job per image, on Codex or the Image API. Shared with the
/// MCP server.
pub(crate) fn generate(
    session: &Session,
    lab: &Concepts,
    tools: &Toolbox,
    request: &NewConcepts,
    backend: Backend,
) -> CommandResult<(Direction, Vec<ConceptJob>)> {
    let (root, config, dir) = project_dir(session, &request.project)?;
    let request = concepts::clean_request(&dir, request)?;
    lab.set_concurrency(config.lab.concurrency);
    let cache = cache_dir(&root, &config);
    Ok(lab.start(Start {
        project_dir: &dir,
        cache_dir: &cache,
        tools,
        request,
        timeout: Duration::from_secs(u64::from(config.lab.timeout_minutes) * 60),
        api: image_api::plan(&root, &config, ImageJobKind::Concepts, backend),
    })?)
}

/// Queues one Codex job per object, with the source concept attached.
/// Shared with the MCP server.
pub(crate) fn make_objects(
    session: &Session,
    lab: &Concepts,
    tools: &Toolbox,
    request: &NewObjectRefs,
    backend: Backend,
) -> CommandResult<(Direction, Vec<ConceptJob>)> {
    let (root, config, dir) = project_dir(session, &request.project)?;
    let request = concepts::objects::clean(&dir, request)?;
    lab.set_concurrency(config.lab.concurrency);
    let cache = cache_dir(&root, &config);
    let lane = Lane {
        project_dir: &dir,
        cache_dir: &cache,
        tools,
        timeout: Duration::from_secs(u64::from(config.lab.timeout_minutes) * 60),
        api: image_api::plan(&root, &config, ImageJobKind::ObjectRefs, backend),
    };
    Ok(lab.start_objects(&lane, &request)?)
}

#[tauri::command]
#[specta::specta]
pub async fn lab_make_objects(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    request: NewObjectRefs,
) -> CommandResult<LabView> {
    let tools = super::toolchain::toolbox(&session).await?;
    make_objects(&session, &lab.0, &tools, &request, Backend::default())?;
    view(&session, &lab.0, &request.project)
}

/// Send picks for a direction of object refs: one asset per picked object,
/// each with its own ref. `kind` overrides the direction's kind.
#[tauri::command]
#[specta::specta]
pub fn lab_send_object_picks(
    session: State<'_, Session>,
    project: String,
    direction: String,
    kind: Option<AssetKind>,
) -> CommandResult<SentObjects> {
    let (root, config) = session.workspace()?;
    Ok(concepts::send_object_picks(
        &root,
        &config.layout,
        &project,
        &direction,
        kind,
    )?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn lab_get(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    project: String,
) -> CommandResult<LabView> {
    view(&session, &lab.0, &project)
}

#[tauri::command]
#[specta::specta]
pub async fn lab_generate(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    request: NewConcepts,
) -> CommandResult<LabView> {
    let tools = super::toolchain::toolbox(&session).await?;
    generate(&session, &lab.0, &tools, &request, Backend::default())?;
    view(&session, &lab.0, &request.project)
}

/// Stars or picks one image; `None` leaves that mark alone.
#[tauri::command]
#[specta::specta]
pub fn lab_mark(
    session: State<'_, Session>,
    project: String,
    direction: String,
    file: String,
    starred: Option<bool>,
    picked: Option<bool>,
) -> CommandResult<Direction> {
    let (_, _, dir) = project_dir(&session, &project)?;
    Ok(concepts::mark(&dir, &direction, &file, starred, picked)?)
}

/// Tiles the direction's images into `contact_sheet.png` next to them and
/// returns its absolute path.
#[tauri::command]
#[specta::specta]
pub async fn lab_contact_sheet(
    session: State<'_, Session>,
    project: String,
    direction: String,
) -> CommandResult<String> {
    let tools = super::toolchain::toolbox(&session).await?;
    let (root, config, dir) = project_dir(&session, &project)?;
    let found = concepts::load_direction(&dir, &direction)?
        .ok_or_else(|| CoreError::NotFound(format!("the direction {direction}")))?;
    let cache = cache_dir(&root, &config);
    let rel = tauri::async_runtime::spawn_blocking(move || {
        sheet::render(&dir, &found, &cache, &tools).map(|rel| dir.join(rel))
    })
    .await
    .map_err(super::join_failed)??;
    Ok(rel.display().to_string())
}

/// Copies the picks into `refs/` and puts the asset of the same name on
/// the Board at Refs.
#[tauri::command]
#[specta::specta]
pub fn lab_send_picks(
    session: State<'_, Session>,
    project: String,
    direction: String,
    kind: AssetKind,
) -> CommandResult<SentPicks> {
    let (root, config) = session.workspace()?;
    Ok(concepts::send_picks(
        &root,
        &config.layout,
        &project,
        &direction,
        kind,
    )?)
}

/// Moves a direction to another project of the workspace. Refused while
/// its images are still generating; the asset of the same name stays put
/// while a Send to 3D job still needs it there.
pub(crate) fn move_direction(
    session: &Session,
    lab: &Concepts,
    studio: &ProviderJobs,
    project_id: &str,
    direction_id: &str,
    to: &str,
) -> CommandResult<MovedDirection> {
    let (root, config, dir) = project_dir(session, project_id)?;
    let generating = lab
        .jobs(Some(project_id))
        .iter()
        .any(|j| j.direction == direction_id && !j.state.is_finished());
    if generating {
        return Err(super::invalid(
            "Images of this direction are still generating. Move it once they are done or cancelled.",
        ));
    }
    let found = concepts::load_direction(&dir, direction_id)?
        .ok_or_else(|| CoreError::NotFound(format!("the direction {direction_id}")))?;
    let asset = project::slugify(found.name.trim());
    let keep_asset = studio
        .list(Some(project_id))
        .iter()
        .any(|j| j.asset == asset && (j.state.is_active() || j.state == JobState::Inbox));
    Ok(concepts::move_direction(
        &root,
        &config.layout,
        project_id,
        direction_id,
        to,
        keep_asset,
    )?)
}

#[tauri::command]
#[specta::specta]
pub fn lab_move_direction(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    studio: State<'_, super::providers::Studio>,
    project: String,
    direction: String,
    to: String,
) -> CommandResult<MovedDirection> {
    move_direction(&session, &lab.0, &studio.0, &project, &direction, &to)
}

/// Copies images dropped on the composer into `refs/` (skipping ones that
/// are already there) and returns their paths relative to the project.
pub(crate) fn import_refs(
    session: &Session,
    project_id: &str,
    paths: &[String],
) -> CommandResult<Vec<String>> {
    let (_, _, dir) = project_dir(session, project_id)?;
    let refs = dir.join(REFS_DIR);
    std::fs::create_dir_all(&refs).map_err(|e| CoreError::io(&refs, e))?;
    let mut out = Vec::new();
    for raw in paths {
        let from = session.expand(raw);
        if !from.is_file() || !is_image(&from) {
            return Err(super::invalid(&format!(
                "{} is not a PNG, JPEG or WebP image.",
                session.display(&from)
            )));
        }
        let Some(name) = from.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        let inside = from
            .canonicalize()
            .ok()
            .zip(refs.canonicalize().ok())
            .is_some_and(|(f, r)| f.starts_with(r));
        if !inside {
            let to = refs.join(&name);
            std::fs::copy(&from, &to).map_err(|e| CoreError::io(&from, e))?;
        }
        out.push(format!("{REFS_DIR}/{name}"));
    }
    Ok(out)
}

/// Changes the project's style board. Its images may be board images,
/// concepts and refs of the project, or absolute paths: from anywhere for
/// the app (a file a person chose), only inside the workspace for an agent.
/// Every image is copied into the project's `style/` folder.
pub(crate) fn set_style(
    session: &Session,
    project_id: &str,
    change: &StyleBoardChange,
    from_anywhere: bool,
) -> CommandResult<StyleBoard> {
    let (root, _, dir) = project_dir(session, project_id)?;
    let mut change = change.clone();
    if let Some(images) = &mut change.images {
        for entry in images.iter_mut() {
            let path = session.expand(entry);
            if !path.is_absolute() {
                continue;
            }
            let inside = path
                .canonicalize()
                .ok()
                .zip(root.canonicalize().ok())
                .is_some_and(|(p, r)| p.starts_with(r));
            if !from_anywhere && !inside {
                return Err(super::invalid(&format!(
                    "{} is outside the workspace. Give a concept or ref of the project, or a path inside the workspace.",
                    session.display(&path)
                )));
            }
            *entry = path.display().to_string();
        }
    }
    Ok(style::update(&dir, &change)?)
}

#[tauri::command]
#[specta::specta]
pub fn lab_set_style(
    session: State<'_, Session>,
    project: String,
    change: StyleBoardChange,
) -> CommandResult<StyleBoard> {
    set_style(&session, &project, &change, true)
}

#[tauri::command]
#[specta::specta]
pub fn lab_import_refs(
    session: State<'_, Session>,
    project: String,
    paths: Vec<String>,
) -> CommandResult<Vec<String>> {
    import_refs(&session, &project, &paths)
}

pub mod queue;
#[cfg(test)]
mod tests;
