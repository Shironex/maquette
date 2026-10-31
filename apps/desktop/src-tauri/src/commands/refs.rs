//! An asset's refs and their views: the shell side of `maquette_core::refs`
//! and of the Lab jobs that draw the other views and clean a ref. The jobs
//! run on the Lab's Codex lane and report through `ConceptUpdate` events.

use std::path::Path;
use std::time::Duration;

use maquette_core::concepts::backend::{ApiPlan, ImageJobKind};
use maquette_core::concepts::jobs::Lane;
use maquette_core::concepts::{
    self, ConceptJob, Concepts, Direction, NewCleanRef, NewOtherViews, RefsJobKind,
};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::ProviderJobs;
use maquette_core::refs::{self, AssetRef, OldView, RefView, RefWarning, Replaced};
use maquette_core::{Asset, ProjectManifest, assets, project};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::CommandResult;
use super::image_api::Backend;
use super::lab::{Lab, RefImage, list_refs};
use super::providers::Studio;
use crate::session::Session;

/// Everything the refs sheet shows for one asset.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RefsView {
    /// The project, with the asset as it stands.
    pub project: ProjectManifest,
    /// Absolute, for the image URLs.
    pub project_dir: String,
    pub asset: Asset,
    /// Its refs with their views, front first among the slots.
    pub refs: Vec<AssetRef>,
    /// False while the views are read from the refs' order.
    pub tagged: bool,
    /// Every image in `refs/`, for Add refs.
    pub images: Vec<RefImage>,
    /// The asset's views and clean directions in the Lab.
    pub directions: Vec<Direction>,
    /// Their jobs, oldest first.
    pub jobs: Vec<ConceptJob>,
}

/// The asset's refs as they stand: its tags, or the views read from the
/// order its refs always went in.
pub(crate) fn asset_refs(project_dir: &Path, asset: &Asset) -> Vec<AssetRef> {
    let files = refs::ref_files(project_dir);
    refs::views_of(
        project_dir,
        asset,
        &refs::suggested(project_dir, asset, &files),
    )
}

pub(crate) fn view(
    session: &Session,
    lab: &Concepts,
    project_id: &str,
    asset_id: &str,
) -> CommandResult<RefsView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?.clone();
    let dir = project::dir(&root, &config.layout, project_id)?;
    let directions: Vec<Direction> = concepts::list_directions(&dir)
        .into_iter()
        .filter(|d| d.refs_job.as_ref().is_some_and(|j| j.asset == asset.id))
        .collect();
    let jobs = lab
        .jobs(Some(project_id))
        .into_iter()
        .filter(|j| directions.iter().any(|d| d.id == j.direction))
        .collect();
    Ok(RefsView {
        project_dir: dir.display().to_string(),
        refs: asset_refs(&dir, &asset),
        tagged: !asset.refs.is_empty(),
        images: list_refs(&dir),
        directions,
        jobs,
        asset,
        project: manifest,
    })
}

/// The project folder, cache, timeout, asset and Image API plan of a
/// refs job of `kind`.
fn lane_parts(
    session: &Session,
    lab: &Concepts,
    project_id: &str,
    asset_id: &str,
    kind: ImageJobKind,
    backend: Backend,
) -> CommandResult<(
    std::path::PathBuf,
    std::path::PathBuf,
    Duration,
    Asset,
    Option<ApiPlan>,
)> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?.clone();
    let dir = project::dir(&root, &config.layout, project_id)?;
    lab.set_concurrency(config.lab.concurrency);
    let cache = root.join(&config.layout.cache).join("concepts");
    let timeout = Duration::from_secs(u64::from(config.lab.timeout_minutes) * 60);
    let api = super::image_api::plan(&root, &config, kind, backend);
    Ok((dir, cache, timeout, asset, api))
}

/// Queues the candidates for an asset's missing (or named) views. Shared
/// with the MCP server.
pub(crate) fn make_views(
    session: &Session,
    lab: &Concepts,
    tools: &Toolbox,
    request: &NewOtherViews,
    backend: Backend,
) -> CommandResult<(Direction, Vec<ConceptJob>)> {
    let (dir, cache, timeout, asset, api) = lane_parts(
        session,
        lab,
        &request.project,
        &request.asset,
        ImageJobKind::OtherViews,
        backend,
    )?;
    let lane = Lane {
        project_dir: &dir,
        cache_dir: &cache,
        tools,
        timeout,
        api,
    };
    let current = asset_refs(&dir, &asset);
    Ok(lab.start_views(&lane, &asset, &current, request)?)
}

/// Queues a clean copy of one of an asset's refs. Shared with the MCP
/// server.
pub(crate) fn clean(
    session: &Session,
    lab: &Concepts,
    tools: &Toolbox,
    request: &NewCleanRef,
    backend: Backend,
) -> CommandResult<(Direction, Vec<ConceptJob>)> {
    let (dir, cache, timeout, asset, api) = lane_parts(
        session,
        lab,
        &request.project,
        &request.asset,
        ImageJobKind::CleanBackground,
        backend,
    )?;
    let lane = Lane {
        project_dir: &dir,
        cache_dir: &cache,
        tools,
        timeout,
        api,
    };
    let current = asset_refs(&dir, &asset);
    Ok(lab.start_clean(&lane, &asset, &current, request)?)
}

/// Takes a candidate of the asset's refs work into its refs: a view into
/// its slot, a clean copy into the place of the ref it cleans. With
/// `replace`, a view replaces the ref that held its slot (see
/// [`OldView::Replace`]; the project's sends keep their files in place),
/// else that ref stays as other. Shared with the MCP server.
pub(crate) fn use_candidate(
    session: &Session,
    jobs: &ProviderJobs,
    project_id: &str,
    asset_id: &str,
    file: &str,
    view: Option<RefView>,
    replace: bool,
) -> CommandResult<(ProjectManifest, Option<Replaced>)> {
    let (root, config) = session.workspace()?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    let id = file.split('/').nth(1).unwrap_or_default();
    let kind = concepts::load_direction(&dir, id)
        .ok()
        .flatten()
        .and_then(|d| d.refs_job)
        .map(|j| j.kind);
    if kind == Some(RefsJobKind::Clean) {
        let manifest = refs::use_clean_copy(&root, &config.layout, project_id, asset_id, file)?;
        return Ok((manifest, None));
    }
    let sent: Vec<String> = jobs
        .list(Some(project_id))
        .into_iter()
        .flat_map(|j| j.images)
        .collect();
    let old = if replace {
        OldView::Replace { sent: &sent }
    } else {
        OldView::KeepAsOther
    };
    let used = refs::use_view_image(&root, &config.layout, project_id, asset_id, file, view, old)?;
    Ok((used.manifest, used.replaced))
}

/// One asset of a warnings request: the images as picked and whether its
/// preset sends several as views.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WarningsItem {
    pub asset: String,
    pub images: Vec<String>,
    pub multi_view: bool,
}

/// The weak ref warnings of each item, in the order asked. Shared with
/// the MCP send tools.
pub(crate) fn warnings(
    session: &Session,
    project_id: &str,
    items: &[WarningsItem],
) -> CommandResult<Vec<Vec<RefWarning>>> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    items
        .iter()
        .map(|item| {
            let asset = assets::find(&manifest, &item.asset)?;
            Ok(refs::send_warnings(
                &dir,
                asset,
                &item.images,
                item.multi_view,
            ))
        })
        .collect()
}

#[tauri::command(async)]
#[specta::specta]
pub fn refs_get(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    project: String,
    asset: String,
) -> CommandResult<RefsView> {
    view(&session, &lab.0, &project, &asset)
}

/// Saves the views of every ref of the asset at once.
#[tauri::command]
#[specta::specta]
pub fn refs_set_views(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    project: String,
    asset: String,
    refs: Vec<AssetRef>,
) -> CommandResult<RefsView> {
    let (root, config) = session.workspace()?;
    refs::set_views(&root, &config.layout, &project, &asset, &refs)?;
    view(&session, &lab.0, &project, &asset)
}

#[tauri::command]
#[specta::specta]
pub async fn refs_make_views(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    request: NewOtherViews,
) -> CommandResult<RefsView> {
    let tools = super::toolchain::toolbox(&session).await?;
    make_views(&session, &lab.0, &tools, &request, Backend::default())?;
    view(&session, &lab.0, &request.project, &request.asset)
}

#[tauri::command]
#[specta::specta]
pub async fn refs_clean(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    request: NewCleanRef,
) -> CommandResult<RefsView> {
    let tools = super::toolchain::toolbox(&session).await?;
    clean(&session, &lab.0, &tools, &request, Backend::default())?;
    view(&session, &lab.0, &request.project, &request.asset)
}

/// "Use as Left" or "Use the clean copy". `view` overrides the view a
/// drawn candidate was made for. A view replaces the one in its slot.
#[tauri::command]
#[specta::specta]
pub fn refs_use(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    studio: State<'_, Studio>,
    project: String,
    asset: String,
    file: String,
    view: Option<RefView>,
) -> CommandResult<RefsView> {
    use_candidate(&session, &studio.0, &project, &asset, &file, view, true)?;
    self::view(&session, &lab.0, &project, &asset)
}

/// Weak ref warnings for the Send to 3D panels, one list per item. Off
/// the main thread: the checks read the images.
#[tauri::command]
#[specta::specta]
pub async fn refs_warnings(
    session: State<'_, Session>,
    project: String,
    items: Vec<WarningsItem>,
) -> CommandResult<Vec<Vec<RefWarning>>> {
    warnings(&session, &project, &items)
}
