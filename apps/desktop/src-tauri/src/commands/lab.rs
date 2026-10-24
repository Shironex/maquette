//! The Concept lab: the shell side of `maquette_core::concepts`. Jobs run
//! on the shared Codex lane and reach the web view as `ConceptUpdate`
//! events.

use std::path::{Path, PathBuf};
use std::time::Duration;

use maquette_core::concepts::jobs::{Lane, Start};
use maquette_core::concepts::{
    self, ConceptJob, Concepts, Direction, MovedDirection, NewConcepts, NewObjectRefs, REFS_DIR,
    SentObjects, SentPicks, sheet,
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

impl Lab {
    pub fn new(handle: AppHandle) -> Self {
        Self(Concepts::new(codex_home(), 4, move |event| {
            if let Err(err) = ConceptUpdate(event).emit(&handle) {
                tracing::warn!(error = %err, "could not emit a concept event");
            }
        }))
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
        .filter(|p| p.is_file() && is_image(p))
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
    })
}

/// Queues one Codex job per image. Shared with the MCP server.
pub(crate) fn generate(
    session: &Session,
    lab: &Concepts,
    tools: &Toolbox,
    request: &NewConcepts,
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
    })?)
}

/// Queues one Codex job per object, with the source concept attached.
/// Shared with the MCP server.
pub(crate) fn make_objects(
    session: &Session,
    lab: &Concepts,
    tools: &Toolbox,
    request: &NewObjectRefs,
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
    make_objects(&session, &lab.0, &tools, &request)?;
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

#[tauri::command]
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
    generate(&session, &lab.0, &tools, &request)?;
    view(&session, &lab.0, &request.project)
}

#[tauri::command]
#[specta::specta]
pub fn lab_cancel(lab: State<'_, Lab>, job_id: String) -> CommandResult<()> {
    Ok(lab.0.cancel(&job_id)?)
}

/// Stops every queued and running job of the project.
#[tauri::command]
#[specta::specta]
pub fn lab_cancel_all(lab: State<'_, Lab>, project: String) -> u32 {
    u32::try_from(lab.0.cancel_all(&project)).unwrap_or(u32::MAX)
}

#[tauri::command]
#[specta::specta]
pub fn lab_clear_finished(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    project: String,
) -> CommandResult<LabView> {
    lab.0.clear_finished(&project);
    view(&session, &lab.0, &project)
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

#[tauri::command]
#[specta::specta]
pub fn lab_import_refs(
    session: State<'_, Session>,
    project: String,
    paths: Vec<String>,
) -> CommandResult<Vec<String>> {
    import_refs(&session, &project, &paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::{AppState, NewProject, ProjectTemplate, workspace};

    #[test]
    fn dropped_images_land_in_refs_and_show_in_the_view() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let manifest = project::create(
            &root,
            &config.layout,
            NewProject {
                name: "Moon".into(),
                template: ProjectTemplate::Room,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        let outside = dir.path().join("sketch.png");
        std::fs::write(&outside, b"png").expect("png");
        let got =
            import_refs(&session, &manifest.id, &[outside.display().to_string()]).expect("import");
        assert_eq!(got, vec!["refs/sketch.png".to_string()]);
        let text = dir.path().join("notes.txt");
        std::fs::write(&text, b"x").expect("txt");
        assert!(import_refs(&session, &manifest.id, &[text.display().to_string()]).is_err());
        let lab = Concepts::new(None, 2, |_| {});
        let view = view(&session, &lab, &manifest.id).expect("view");
        assert_eq!(view.refs.len(), 1);
        assert_eq!(view.concurrency, 4);
        assert!(view.directions.is_empty() && view.jobs.is_empty());
    }

    #[test]
    fn a_direction_moves_to_another_project() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        for name in ["Moon Island", "Maquette landing"] {
            project::create(
                &root,
                &config.layout,
                NewProject {
                    name: name.into(),
                    template: ProjectTemplate::FloatingDiorama,
                    site_repo: None,
                    source_folder: None,
                },
            )
            .expect("project");
        }
        let moon = project::dir(&root, &config.layout, "moon-island").expect("dir");
        let request = NewConcepts {
            project: "moon-island".into(),
            direction: "Maquette landing studio".into(),
            prompt: "A studio desk".into(),
            refs: Vec::new(),
            aspect: concepts::Aspect::Wide,
            count: 1,
        };
        let (direction, files) = concepts::prepare(&moon, &request).expect("prepare");
        std::fs::write(moon.join(&files[0].1), b"png").expect("png");
        concepts::add_image(&moon, &direction.id, 1, &files[0].1, None).expect("add");

        let lab = Concepts::new(None, 2, |_| {});
        let studio = ProviderJobs::new(
            std::sync::Arc::new(maquette_core::providers::MemoryStore::default()),
            maquette_core::providers::Options::default(),
            |_| {},
        );
        let moved = move_direction(
            &session,
            &lab,
            &studio,
            "moon-island",
            "maquette-landing-studio",
            "maquette-landing",
        )
        .expect("move");
        assert_eq!(moved.to.id, "maquette-landing");
        assert!(
            view(&session, &lab, "moon-island")
                .expect("moon")
                .directions
                .is_empty()
        );
        assert_eq!(
            view(&session, &lab, "maquette-landing")
                .expect("landing")
                .directions
                .len(),
            1
        );
        assert!(
            move_direction(
                &session,
                &lab,
                &studio,
                "moon-island",
                "nope",
                "maquette-landing"
            )
            .is_err()
        );
    }

    #[test]
    fn object_refs_are_checked_and_need_codex() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let manifest = project::create(
            &root,
            &config.layout,
            NewProject {
                name: "Landing".into(),
                template: ProjectTemplate::Room,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
        std::fs::write(project_dir.join("refs/workbench.png"), b"png").expect("png");
        let lab = Concepts::new(None, 2, |_| {});
        let mut request = NewObjectRefs {
            project: manifest.id.clone(),
            source: "refs/workbench.png".into(),
            source_name: "Workbench".into(),
            objects: vec!["desk lamp".into()],
            notes: Default::default(),
            aspect: concepts::Aspect::Square,
            kind: AssetKind::Prop,
        };
        let err = make_objects(&session, &lab, &Toolbox::default(), &request).expect_err("codex");
        assert!(err.message.contains("Codex CLI"), "{}", err.message);
        assert!(
            view(&session, &lab, &manifest.id)
                .expect("view")
                .directions
                .is_empty(),
            "nothing written without Codex"
        );
        request.objects.clear();
        let err = make_objects(&session, &lab, &Toolbox::default(), &request).expect_err("none");
        assert!(
            err.message.contains("at least one object"),
            "{}",
            err.message
        );
        request.objects = vec!["lamp".into()];
        request.source = "../outside.png".into();
        assert!(make_objects(&session, &lab, &Toolbox::default(), &request).is_err());
        request.project = "nope".into();
        assert!(make_objects(&session, &lab, &Toolbox::default(), &request).is_err());
    }
}
