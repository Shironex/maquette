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
use maquette_core::{Asset, AssetKind, CoreError, assets, concepts, project};
use serde::Serialize;
use tauri::{AppHandle, Manager as _, State};
use tauri_specta::Event as _;

use super::CommandResult;
use super::lab::{RefImage, list_refs};
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
        Self(ProviderJobs::new(secrets, options(), move |event| {
            emit(&handle, event);
        }))
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

async fn blocking<T: Send + 'static>(job: impl FnOnce() -> T + Send + 'static) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(super::join_failed)
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

/// The refs an asset came with: images sent from the Lab direction of the
/// same name (or the name it was first made with, which its id keeps) or
/// object refs titled like it, else files in `refs/` named after the asset.
fn suggested(project_dir: &Path, asset: &Asset, images: &[RefImage]) -> Vec<String> {
    let named = |text: &str| {
        text.trim().eq_ignore_ascii_case(asset.name.trim())
            || project::slugify(text.trim()) == asset.id
    };
    let mut from_lab: Vec<String> = Vec::new();
    for d in concepts::list_directions(project_dir) {
        let whole = named(&d.name);
        for image in d.images {
            let mine = whole || image.title.as_deref().is_some_and(named);
            if let Some(file) = image.ref_file.filter(|_| mine)
                && images.iter().any(|i| i.file == file)
                && !from_lab.contains(&file)
            {
                from_lab.push(file);
            }
        }
    }
    if !from_lab.is_empty() {
        return from_lab.into_iter().take(4).collect();
    }
    let slug = asset.id.replace('-', "_");
    images
        .iter()
        .filter(|i| {
            let name = i.name.to_lowercase().replace('-', "_");
            name.starts_with(&slug)
        })
        .map(|i| i.file.clone())
        .take(4)
        .collect()
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
    let suggested = suggested(&dir, &asset, &images);
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
    let images = list_refs(&dir);
    let mut items = Vec::new();
    for id in asset_ids {
        if items.iter().any(|i: &BatchItemOptions| &i.asset.id == id) {
            continue;
        }
        let asset = assets::find(&manifest, id)?.clone();
        items.push(BatchItemOptions {
            suggested: suggested(&dir, &asset, &images),
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

#[tauri::command]
#[specta::specta]
pub fn provider_batch_options(
    session: State<'_, Session>,
    studio: State<'_, Studio>,
    project: String,
    assets: Vec<String>,
) -> CommandResult<BatchOptions> {
    batch_options(&session, &studio.0, &project, &assets)
}

/// Sends several assets at once from the app. Pressing Send in the panel
/// (after its one confirmation for API providers) is the approval.
#[tauri::command]
#[specta::specta]
pub fn provider_send_batch(
    studio: State<'_, Studio>,
    request: BatchRequest,
) -> CommandResult<BatchSent> {
    Ok(studio.0.send_batch(&request, false, false)?)
}

#[tauri::command]
#[specta::specta]
pub fn provider_send_options(
    session: State<'_, Session>,
    studio: State<'_, Studio>,
    project: String,
    asset: String,
) -> CommandResult<SendOptions> {
    send_options(&session, &studio.0, &project, &asset)
}

/// Starts a send from the app. You are the one pressing Send, so it needs
/// no approval.
#[tauri::command]
#[specta::specta]
pub fn provider_send(
    studio: State<'_, Studio>,
    request: SendRequest,
) -> CommandResult<ProviderJob> {
    Ok(studio.0.send(&request, false, false)?)
}

#[tauri::command]
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
#[tauri::command]
#[specta::specta]
pub fn provider_hand_back(
    studio: State<'_, Studio>,
    id: String,
    file: String,
) -> CommandResult<ProviderJob> {
    Ok(studio.0.hand_back(&id, Path::new(&file))?)
}

#[tauri::command]
#[specta::specta]
pub fn provider_clear_finished(
    studio: State<'_, Studio>,
    project: String,
) -> CommandResult<Vec<ProviderJob>> {
    studio.0.clear_finished(&project);
    Ok(studio.0.list(Some(&project)))
}

/// Shows a By hand job's folder in Finder or Explorer.
#[tauri::command]
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
/// known sites open, never an address from the web view.
#[tauri::command]
#[specta::specta]
pub fn provider_open_site(service: ProviderId) -> CommandResult<()> {
    let url = handoff::site(service);
    opener::open_url(url).map_err(|err| {
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
#[tauri::command]
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

#[tauri::command]
#[specta::specta]
pub fn provider_keys(studio: State<'_, Studio>) -> CommandResult<Vec<KeyStatus>> {
    key_statuses(&studio.0)
}

/// Saves a key in the keychain. Only its last four characters come back.
#[tauri::command]
#[specta::specta]
pub fn provider_key_save(
    studio: State<'_, Studio>,
    service: ProviderId,
    key: String,
) -> CommandResult<KeyStatus> {
    let key = secrets::clean_key(service, &key)?;
    studio.0.secrets().set(service, &key)?;
    Ok(secrets::status(studio.0.secrets().as_ref(), service)?)
}

#[tauri::command]
#[specta::specta]
pub fn provider_key_remove(
    studio: State<'_, Studio>,
    service: ProviderId,
) -> CommandResult<KeyStatus> {
    studio.0.secrets().remove(service)?;
    Ok(secrets::status(studio.0.secrets().as_ref(), service)?)
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
        None => studio.0.secrets().get(service)?.ok_or_else(|| {
            CoreError::Invalid(format!("Save a {} API key first.", service.label()))
        })?,
    };
    let endpoints = studio.0.options().endpoints.clone();
    let timing = match service {
        ProviderId::Tripo => Timing::TRIPO,
        ProviderId::Meshy => Timing::MESHY,
    };
    blocking(move || test_key(&endpoints, service, &key, timing)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::concepts::REFS_DIR;
    use maquette_core::providers::MemoryStore;
    use maquette_core::{
        AppState, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, workspace,
    };

    #[test]
    fn send_options_pick_the_assets_own_refs() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let manifest = project::create(
            &root,
            &config.layout,
            NewProject {
                name: "Moon".into(),
                template: ProjectTemplate::FloatingDiorama,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        project::modify(&root, &config.layout, &manifest.id, |m| {
            assets::add(
                m,
                NewAsset {
                    name: "Fox spirit".into(),
                    kind: AssetKind::Character,
                    stage: Stage::Refs,
                },
            )
            .map(|_| ())
        })
        .expect("asset");
        let refs = project::dir(&root, &config.layout, &manifest.id)
            .expect("dir")
            .join(REFS_DIR);
        for name in ["fox_spirit_front.png", "fox_spirit_side.png", "lantern.png"] {
            std::fs::write(refs.join(name), b"png").expect("ref");
        }
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let jobs = ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {});
        jobs.open_workspace(&root, &config);
        let options = send_options(&session, &jobs, &manifest.id, "fox-spirit").expect("options");
        assert_eq!(options.variant, PresetVariant::Character);
        assert_eq!(options.images.len(), 3);
        assert_eq!(
            options.suggested,
            vec!["refs/fox_spirit_front.png", "refs/fox_spirit_side.png"]
        );
        assert!(options.keys.iter().all(|k| !k.saved));
    }

    #[test]
    fn batch_options_find_object_refs_by_title() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let manifest = project::create(
            &root,
            &config.layout,
            NewProject {
                name: "Landing".into(),
                template: ProjectTemplate::FloatingDiorama,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
        let request = concepts::objects::clean(&project_dir, &{
            let src = project_dir.join("refs/workbench.png");
            std::fs::write(&src, b"png").expect("src");
            concepts::NewObjectRefs {
                project: manifest.id.clone(),
                source: "refs/workbench.png".into(),
                source_name: "Workbench".into(),
                objects: vec!["Desk lamp".into(), "Clay figure".into()],
                notes: Default::default(),
                aspect: concepts::Aspect::Square,
                kind: AssetKind::Prop,
            }
        })
        .expect("clean");
        let (direction, planned) =
            concepts::objects::prepare(&project_dir, &request).expect("prepare");
        for p in &planned {
            std::fs::write(project_dir.join(&p.file), b"png").expect("png");
            concepts::add_image(
                &project_dir,
                &direction.id,
                p.index,
                &p.file,
                Some(&p.object),
            )
            .expect("add");
            concepts::mark(&project_dir, &direction.id, &p.file, None, Some(true)).expect("pick");
        }
        let sent =
            concepts::send_object_picks(&root, &config.layout, &manifest.id, &direction.id, None)
                .expect("send");
        // Make the refs/ name differ from the asset id, so only the title
        // can find it.
        std::fs::rename(
            project_dir.join("refs/clay_figure.png"),
            project_dir.join("refs/figure_from_lab.png"),
        )
        .expect("rename");
        concepts::modify_direction(&project_dir, &direction.id, |d| {
            d.images[1].ref_file = Some("refs/figure_from_lab.png".into());
            Ok(())
        })
        .expect("ref");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let jobs = ProviderJobs::new(Arc::new(MemoryStore::default()), Options::default(), |_| {});
        jobs.open_workspace(&root, &config);
        let ids: Vec<String> = sent.assets.iter().map(|a| a.asset.id.clone()).collect();
        let mut asked = ids.clone();
        asked.push(ids[0].clone());
        let options = batch_options(&session, &jobs, &manifest.id, &asked).expect("options");
        assert_eq!(options.items.len(), 2, "each asset once");
        assert_eq!(options.items[0].suggested, vec!["refs/desk_lamp.png"]);
        assert_eq!(options.items[1].suggested, vec!["refs/figure_from_lab.png"]);
        assert!(
            options
                .items
                .iter()
                .all(|i| i.variant == PresetVariant::Prop)
        );
        assert!(batch_options(&session, &jobs, &manifest.id, &["nope".into()]).is_err());
    }

    #[test]
    fn only_images_on_the_card_can_be_revealed() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("front.png"), b"png").expect("png");
        std::fs::write(dir.path().join("settings.md"), b"md").expect("md");
        let card = maquette_core::providers::HandoffCard {
            service: ProviderId::Tripo,
            site: handoff::site(ProviderId::Tripo).into(),
            mode: "Image to 3D".into(),
            rows: Vec::new(),
            images: vec!["front.png".into()],
            return_name: "lamp-1.glb".into(),
        };
        let json = serde_json::json!({
            "id": "lamp-1", "project": "p", "asset": "lamp", "assetName": "Lamp",
            "provider": "by-hand", "service": "tripo", "variant": "prop",
            "images": ["refs/lamp.png"], "tripo": null, "meshy": null, "state": "waiting",
            "phase": null, "progress": 0.0, "remoteId": null, "credits": null,
            "createdAtMs": 0.0, "startedAtMs": null, "finishedAtMs": null, "message": null,
            "inboxFile": null, "handoff": dir.path().display().to_string(), "card": card,
            "version": null, "viaAgent": false, "cancel": "stop-waiting"
        });
        let job: ProviderJob = serde_json::from_value(json).expect("job");
        assert_eq!(
            handoff_image(&job, "front.png").expect("front"),
            dir.path().join("front.png")
        );
        for bad in ["settings.md", "../front.png", "left.png"] {
            assert!(handoff_image(&job, bad).is_err(), "{bad}");
        }
    }
}
