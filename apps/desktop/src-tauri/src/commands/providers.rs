//! Send to 3D: the shell side of `maquette_core::providers`. Jobs reach
//! the web view as `ProviderUpdate` events; asset changes go out as
//! `ProjectChanged`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use maquette_core::providers::http::Timing;
use maquette_core::providers::secrets::{self, SecretStore};
use maquette_core::providers::{
    BatchRequest, BatchSent, DownloadsScan, Endpoints, KeyStatus, KeyTest, Options, PresetVariant,
    ProviderEvent, ProviderId, ProviderJob, ProviderJobs, SendRequest, handoff, test_key,
};
use maquette_core::{Asset, AssetKind, CoreError, assets, project};
use serde::Serialize;
use tauri::{AppHandle, Manager as _, State};
use tauri_specta::Event as _;

use super::lab::{RefImage, list_refs};
use super::{CommandResult, blocking};
use crate::events::{ProjectChanged, ProviderUpdate};
use crate::keychain::{KeychainStore, NoKeychain};
use crate::opener;
use crate::session::Session;

/// The service the app shares between commands and the MCP tools.
pub struct Studio(pub ProviderJobs);

impl Studio {
    pub fn new(handle: AppHandle) -> Self {
        let secrets: Arc<dyn SecretStore> = match KeychainStore::new() {
            Ok(store) => Arc::new(store),
            Err(err) => {
                tracing::warn!(error = %err, "no keychain, API keys cannot be saved");
                Arc::new(NoKeychain(err.to_string()))
            }
        };
        Self(ProviderJobs::new(
            secrets,
            options(),
            maquette_core::inbox::send_ports(),
            move |event| {
                emit(&handle, event);
            },
        ))
    }
}

/// Production endpoints. Debug builds can point them at a local fake with
/// `MAQUETTE_TRIPO_URL` and `MAQUETTE_MESHY_URL`.
fn options() -> Options {
    let mut endpoints = Endpoints::default();
    if cfg!(debug_assertions) {
        if let Ok(url) = std::env::var("MAQUETTE_TRIPO_URL") {
            endpoints.tripo = url;
        }
        if let Ok(url) = std::env::var("MAQUETTE_MESHY_URL") {
            endpoints.meshy = url;
        }
    }
    Options {
        endpoints,
        ..Options::default()
    }
}

pub(crate) fn emit(handle: &AppHandle, event: ProviderEvent) {
    let sent = match event {
        ProviderEvent::Project { project } => ProjectChanged { project }.emit(handle),
        event => ProviderUpdate(event).emit(handle),
    };
    if let Err(err) = sent {
        tracing::warn!(error = %err, "could not emit a provider event");
    }
}

/// Everything the Send to 3D panel needs for one asset.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SendOptions {
    pub project: String,
    /// Absolute, for the image URLs.
    pub project_dir: String,
    pub asset: Asset,
    /// Every image in `refs/`.
    pub images: Vec<RefImage>,
    /// The asset's own refs, picked by default, front first.
    pub suggested: Vec<String>,
    /// From the asset's kind.
    pub variant: PresetVariant,
    pub keys: Vec<KeyStatus>,
    /// This asset's sends, newest last.
    pub jobs: Vec<ProviderJob>,
}

pub(crate) fn key_statuses(studio: &ProviderJobs) -> CommandResult<Vec<KeyStatus>> {
    ProviderId::ALL
        .iter()
        .map(|&service| Ok(secrets::status(studio.secrets().as_ref(), service)?))
        .collect()
}

/// The asset's view slots, front first: what a send picks by default. An
/// asset nobody tagged reads them from its refs' order, as sends always
/// did.
fn suggested(project_dir: &Path, asset: &Asset) -> Vec<String> {
    maquette_core::refs::slot_files(&super::refs::asset_refs(project_dir, asset))
}

pub(crate) fn send_options(
    session: &Session,
    studio: &ProviderJobs,
    project_id: &str,
    asset_id: &str,
) -> CommandResult<SendOptions> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?.clone();
    let dir = project::dir(&root, &config.layout, project_id)?;
    let images = list_refs(&dir);
    let suggested = suggested(&dir, &asset);
    let jobs = studio
        .list(Some(project_id))
        .into_iter()
        .filter(|j| j.asset == asset.id)
        .collect();
    Ok(SendOptions {
        project: project_id.to_string(),
        project_dir: dir.display().to_string(),
        variant: variant_for(&asset),
        asset,
        images,
        suggested,
        keys: key_statuses(studio)?,
        jobs,
    })
}

/// One asset of the batch panel: its refs and the preset it starts from.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemOptions {
    pub asset: Asset,
    /// Its own refs, front first; the batch sends these.
    pub suggested: Vec<String>,
    pub variant: PresetVariant,
}

/// Everything the batch Send to 3D panel needs.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchOptions {
    pub project: String,
    /// Absolute, for the image URLs.
    pub project_dir: String,
    /// In the order asked.
    pub items: Vec<BatchItemOptions>,
    pub keys: Vec<KeyStatus>,
}

pub(crate) fn batch_options(
    session: &Session,
    studio: &ProviderJobs,
    project_id: &str,
    asset_ids: &[String],
) -> CommandResult<BatchOptions> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    let mut items = Vec::new();
    for id in asset_ids {
        if items.iter().any(|i: &BatchItemOptions| &i.asset.id == id) {
            continue;
        }
        let asset = assets::find(&manifest, id)?.clone();
        items.push(BatchItemOptions {
            suggested: suggested(&dir, &asset),
            variant: variant_for(&asset),
            asset,
        });
    }
    Ok(BatchOptions {
        project: project_id.to_string(),
        project_dir: dir.display().to_string(),
        items,
        keys: key_statuses(studio)?,
    })
}

fn variant_for(asset: &Asset) -> PresetVariant {
    if asset.kind == AssetKind::Character {
        PresetVariant::Character
    } else {
        PresetVariant::Prop
    }
}

/// Reads the saved keys, which can wait on a Keychain prompt.
#[tauri::command]
#[specta::specta]
pub async fn provider_batch_options(
    app: AppHandle,
    project: String,
    assets: Vec<String>,
) -> CommandResult<BatchOptions> {
    blocking(move || {
        batch_options(
            &app.state::<Session>(),
            &app.state::<Studio>().0,
            &project,
            &assets,
        )
    })
    .await?
}

/// Sends several assets at once from the app. Pressing Send in the panel
/// (after its one confirmation for API providers) is the approval.
#[tauri::command]
#[specta::specta]
pub async fn provider_send_batch(
    studio: State<'_, Studio>,
    request: BatchRequest,
) -> CommandResult<BatchSent> {
    let jobs = studio.0.clone();
    Ok(blocking(move || jobs.send_batch(&request, false, false)).await??)
}

/// Reads the saved keys, which can wait on a Keychain prompt.
#[tauri::command]
#[specta::specta]
pub async fn provider_send_options(
    app: AppHandle,
    project: String,
    asset: String,
) -> CommandResult<SendOptions> {
    blocking(move || {
        send_options(
            &app.state::<Session>(),
            &app.state::<Studio>().0,
            &project,
            &asset,
        )
    })
    .await?
}

/// Starts a send from the app. You are the one pressing Send, so it needs
/// no approval.
#[tauri::command]
#[specta::specta]
pub async fn provider_send(
    studio: State<'_, Studio>,
    request: SendRequest,
) -> CommandResult<ProviderJob> {
    let jobs = studio.0.clone();
    Ok(blocking(move || jobs.send(&request, false, false)).await??)
}

#[tauri::command(async)]
#[specta::specta]
pub fn provider_jobs(
    studio: State<'_, Studio>,
    project: Option<String>,
) -> CommandResult<Vec<ProviderJob>> {
    Ok(studio.0.list(project.as_deref()))
}

/// Meshy is asked over the network, so this runs off the main thread.
#[tauri::command]
#[specta::specta]
pub async fn provider_cancel(studio: State<'_, Studio>, id: String) -> CommandResult<ProviderJob> {
    let jobs = studio.0.clone();
    Ok(blocking(move || jobs.cancel(&id)).await??)
}

/// By hand: a GLB dropped on the asset's card.
#[tauri::command(async)]
#[specta::specta]
pub fn provider_hand_back(
    studio: State<'_, Studio>,
    id: String,
    file: String,
) -> CommandResult<ProviderJob> {
    Ok(studio.0.hand_back(&id, Path::new(&file))?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn provider_clear_finished(
    studio: State<'_, Studio>,
    project: String,
) -> CommandResult<Vec<ProviderJob>> {
    studio.0.clear_finished(&project);
    Ok(studio.0.list(Some(&project)))
}

/// Shows a By hand job's folder in Finder or Explorer.
#[tauri::command(async)]
#[specta::specta]
pub fn provider_open_handoff(studio: State<'_, Studio>, id: String) -> CommandResult<()> {
    let job = studio
        .0
        .get(&id)
        .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
    let folder = job
        .handoff
        .map(PathBuf::from)
        .ok_or_else(|| CoreError::Invalid("This send has no handoff folder.".to_string()))?;
    if !folder.is_dir() {
        return Err(CoreError::NotFound(format!("the folder {}", folder.display())).into());
    }
    opener::open_folder(&folder).map_err(|err| CoreError::io(&folder, err).payload())
}

/// Opens the service's web studio in the system browser. Only the two
/// allowlisted sites open, never an address from the web view.
#[tauri::command(async)]
#[specta::specta]
pub fn provider_open_site(service: ProviderId) -> CommandResult<()> {
    let url = handoff::site(service);
    opener::open_provider_site(url).map_err(|err| {
        CoreError::Unavailable(format!("Could not open {url} in the browser. {err}")).payload()
    })
}

/// The image of a By hand job's folder named `image`, checked against the
/// card so only files the handoff wrote can be shown.
pub(crate) fn handoff_image(job: &ProviderJob, image: &str) -> CommandResult<PathBuf> {
    let card = job
        .card
        .as_ref()
        .ok_or_else(|| CoreError::Invalid("This send has no settings card.".to_string()))?;
    if !card.images.iter().any(|i| i == image) {
        return Err(CoreError::NotFound(format!("the image {image} in {}", job.id)).into());
    }
    let folder = job
        .handoff
        .as_deref()
        .map(PathBuf::from)
        .ok_or_else(|| CoreError::Invalid("This send has no handoff folder.".to_string()))?;
    let path = folder.join(image);
    if !path.is_file() {
        return Err(CoreError::NotFound(format!("the file {}", path.display())).into());
    }
    Ok(path)
}

/// Shows one image of a By hand job selected in Finder or Explorer, ready
/// to drag into the web studio's upload.
#[tauri::command(async)]
#[specta::specta]
pub fn provider_reveal_image(
    studio: State<'_, Studio>,
    id: String,
    image: String,
) -> CommandResult<()> {
    let job = studio
        .0
        .get(&id)
        .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
    let path = handoff_image(&job, &image)?;
    opener::reveal_file(&path).map_err(|err| CoreError::io(&path, err).payload())
}

/// Looks for models in the Downloads folder (the OS known folder) that
/// came after this project's handoffs. Reads only; each match is copied
/// into the inbox once you confirm it with `provider_hand_back`.
#[tauri::command]
#[specta::specta]
pub async fn provider_check_downloads(
    app: AppHandle,
    studio: State<'_, Studio>,
    project: String,
) -> CommandResult<DownloadsScan> {
    let downloads = app.path().download_dir().map_err(|err| {
        CoreError::Unavailable(format!("Could not find the Downloads folder. {err}")).payload()
    })?;
    let jobs = studio.0.clone();
    blocking(move || jobs.check_downloads(&project, &downloads)).await
}

/// Every key's status. A read can wait on a Keychain prompt.
#[tauri::command]
#[specta::specta]
pub async fn provider_keys(studio: State<'_, Studio>) -> CommandResult<Vec<KeyStatus>> {
    let jobs = studio.0.clone();
    blocking(move || key_statuses(&jobs)).await?
}

/// Saves a key in the keychain. Only its last four characters come back.
#[tauri::command]
#[specta::specta]
pub async fn provider_key_save(
    studio: State<'_, Studio>,
    service: ProviderId,
    key: String,
) -> CommandResult<KeyStatus> {
    let key = secrets::clean_key(service, &key)?;
    let store = Arc::clone(studio.0.secrets());
    blocking(move || {
        store.set(service, &key)?;
        Ok(secrets::status(store.as_ref(), service)?)
    })
    .await?
}

#[tauri::command]
#[specta::specta]
pub async fn provider_key_remove(
    studio: State<'_, Studio>,
    service: ProviderId,
) -> CommandResult<KeyStatus> {
    let store = Arc::clone(studio.0.secrets());
    blocking(move || {
        store.remove(service)?;
        Ok(secrets::status(store.as_ref(), service)?)
    })
    .await?
}

/// Tries a key with the free balance call. Without `key`, the saved one.
#[tauri::command]
#[specta::specta]
pub async fn provider_key_test(
    studio: State<'_, Studio>,
    service: ProviderId,
    key: Option<String>,
) -> CommandResult<KeyTest> {
    let key = match key.filter(|k| !k.trim().is_empty()) {
        Some(key) => secrets::clean_key(service, &key)?,
        None => {
            let store = Arc::clone(studio.0.secrets());
            blocking(move || store.get(service))
                .await??
                .ok_or_else(|| {
                    CoreError::Invalid(format!("Save a {} API key first.", service.label()))
                })?
        }
    };
    let endpoints = studio.0.options().endpoints.clone();
    let timing = match service {
        ProviderId::Tripo => Timing::TRIPO,
        ProviderId::Meshy => Timing::MESHY,
    };
    blocking(move || test_key(&endpoints, service, &key, timing)).await
}

#[cfg(test)]
mod tests;
