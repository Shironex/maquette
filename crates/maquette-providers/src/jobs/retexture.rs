//! Painting a version of an asset again on Meshy: a paid send like any other.
//! The version goes up as a file, copied into the job's folder when it goes
//! out, and the result lands in the Inbox as the next version. Meshy's own
//! answer decides whether the parts stay parts, so the Inbox checks compare
//! what came back with the version it painted.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use serde::{Deserialize, Serialize};

use super::ids::next_id;
use super::remote::{create_failed, download, poll_meshy};
use super::split::busy;
use super::split_files::{SOURCE_FILE, split_dir, version_file};
use super::worker::Run;
use super::{
    CLOSED_WHILE_SENDING, CancelMode, Chain, Entry, Flow, FlowResult, JobPhase, JobState, Outcome,
    Provider, ProviderEvent, ProviderJob, ProviderJobs, RetextureJob, Stored, Workspace, api_key,
    persist, pump, snapshot, touch_asset,
};
use crate::error::{CoreError, CoreResult};
use crate::http::{ApiError, StopFlag};
use crate::meshy::{self, MAX_RETEXTURE_MODEL_BYTES, Meshy};
use crate::presets::{PresetVariant, ProviderId};
use crate::split::{MAX_RETEXTURE_PROMPT, MESHY_RETEXTURE_CREDITS};
use maquette_base::history::now_ms;
use maquette_model::assets::{self, AssetKind, AssetStatus};
use maquette_model::project;
use maquette_model::sends::RetextureSource;

/// What a re-texture asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RetextureRequest {
    pub project: String,
    pub asset: String,
    /// The version to paint; it stays as it is.
    pub version: u32,
    /// What the new texture looks like.
    pub prompt: String,
}

/// A version's numbers once read: its file never changes, and the Inbox asks
/// on every listing.
static SOURCES: LazyLock<Mutex<HashMap<PathBuf, (u32, u32)>>> = LazyLock::new(Mutex::default);

/// What the Inbox compares a re-texture with: the version it painted, while
/// that version's file is there to read.
pub(super) fn retexture_source(
    job: &ProviderJob,
    workspace: &Workspace,
) -> Option<RetextureSource> {
    let version = job.retexture.as_ref()?.from_version;
    let file = version_file(workspace, &job.project, &job.asset, version)?;
    let known = SOURCES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&file)
        .copied();
    let (objects, triangles) = match known {
        Some(numbers) => numbers,
        None => {
            let report = maquette_glb::analyse::analyse(&file).ok()?;
            let numbers = (report.objects, report.triangles);
            SOURCES
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(file, numbers);
            numbers
        }
    };
    Some(RetextureSource {
        version,
        objects,
        triangles,
    })
}

impl ProviderJobs {
    /// Starts a re-texture. With `approval` it waits in Approval until
    /// [`ProviderJobs::approve`], as a paid send does.
    ///
    /// # Errors
    ///
    /// Fails without a key, a prompt or the version, for a model too large
    /// for Meshy, or while a split or re-texture of the asset is on its way.
    pub fn send_retexture(
        &self,
        request: &RetextureRequest,
        approval: bool,
        via_agent: bool,
    ) -> CoreResult<ProviderJob> {
        let _one = self.shared.sending();
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        let prompt = request.prompt.trim();
        if prompt.is_empty() {
            return Err(CoreError::Invalid(
                "Say what the new texture should look like.".to_string(),
            ));
        }
        if prompt.chars().count() > MAX_RETEXTURE_PROMPT {
            return Err(CoreError::Invalid(format!(
                "Keep the texture prompt under {MAX_RETEXTURE_PROMPT} characters; Meshy refuses longer ones."
            )));
        }
        let manifest = project::load(&workspace.root, &workspace.config.layout, &request.project)?;
        let asset = assets::find(&manifest, &request.asset)?.clone();
        let file = version_file(&workspace, &request.project, &asset.id, request.version)
            .ok_or_else(|| {
                CoreError::Invalid(format!("{} has no v{}.", asset.name, request.version))
            })?;
        let size = fs::metadata(&file)
            .map_err(|e| CoreError::io(&file, e))?
            .len();
        if size > MAX_RETEXTURE_MODEL_BYTES {
            return Err(CoreError::Invalid(format!(
                "v{} is over 75 MB, too large to send to Meshy.",
                request.version
            )));
        }
        api_key(&self.shared, ProviderId::Meshy)?;
        if let Some(id) = busy(&self.shared, &request.project, &asset.id) {
            return Err(CoreError::Invalid(format!(
                "{} has a split or re-texture on its way ({id}).",
                asset.name
            )));
        }
        let project_dir = workspace.project_dir(&request.project)?;
        let id = next_id(&self.shared.jobs(), &project_dir, &asset.id);
        let dir = split_dir(&workspace, &id);
        fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
        let copy = dir.join(SOURCE_FILE);
        fs::copy(&file, &copy).map_err(|e| CoreError::io(&copy, e))?;
        let job = ProviderJob {
            id,
            project: request.project.clone(),
            asset: asset.id.clone(),
            asset_name: asset.name.clone(),
            provider: Provider::Meshy,
            service: ProviderId::Meshy,
            variant: if asset.kind == AssetKind::Character {
                PresetVariant::Character
            } else {
                PresetVariant::Prop
            },
            images: Vec::new(),
            views: Vec::new(),
            tripo: None,
            meshy: None,
            state: if approval {
                JobState::Approval
            } else {
                JobState::Queued
            },
            phase: None,
            progress: 0.0,
            remote_id: None,
            credits: None,
            created_at_ms: now_ms(),
            started_at_ms: None,
            finished_at_ms: None,
            message: approval
                .then(|| "Waiting for your approval before spending credits".to_string()),
            inbox_file: None,
            handoff: None,
            card: None,
            version: None,
            via_agent,
            cancel: CancelMode::Free,
            batch: None,
            prompt: Some(prompt.to_string()),
            redo_of: None,
            estimate: Some(MESHY_RETEXTURE_CREDITS),
            split: None,
            studio: None,
            retexture: Some(RetextureJob {
                from_version: request.version,
            }),
        };
        let stored = Stored {
            job: job.clone(),
            chain: Chain::default(),
        };
        self.shared.jobs().push(Entry {
            stored: stored.clone(),
            stop: Arc::new(StopFlag::default()),
            thread: false,
            cancelled: false,
        });
        persist(&self.shared, &job.project);
        touch_asset(
            &self.shared,
            &job.project,
            &job.asset,
            None,
            AssetStatus::Queued,
        );
        self.shared.emit(ProviderEvent::Job {
            job: Box::new(snapshot(&stored)),
        });
        pump(&self.shared);
        Ok(self.get(&job.id).unwrap_or(job))
    }
}

/// The paid chain, resuming from the saved task id like a generation.
pub(super) fn run_meshy_retexture(
    run: &Run<'_>,
    client: &Meshy<'_>,
    job: &ProviderJob,
    source: &Path,
) -> FlowResult<Outcome> {
    let range = (0.02, 0.95);
    let chain = run.chain();
    let task = match chain.model_task {
        Some(task) => task,
        None if chain.sending => return Err(Flow::Fail(CLOSED_WHILE_SENDING.to_string())),
        None => {
            run.phase(JobPhase::Upload, 0.0);
            let model = meshy::model_data_uri(source).map_err(Flow::Fail)?;
            if client.stop.is_stopped() {
                return Err(ApiError::Stopped.into());
            }
            run.save(|s| s.chain.sending = true);
            let prompt = job.prompt.as_deref().unwrap_or_default();
            let task = client
                .retexture(&model, prompt)
                .map_err(|e| create_failed(run, meshy::SERVICE, e))?;
            run.save(|s| {
                s.chain.model_task = Some(task.clone());
                s.chain.meshy_kind = Some(meshy::Kind::Retexture.path().to_string());
                s.job.remote_id = Some(task.clone());
                s.job.phase = Some(JobPhase::Generate);
                s.job.progress = range.0;
            });
            task
        }
    };
    let kind = meshy::Kind::Retexture;
    let done = poll_meshy(run, client, kind, &task, JobPhase::Generate, range)?;
    run.credits(&task, done.credits);
    run.phase(JobPhase::Download, 0.96);
    let url = meshy::glb_url(kind, &done.body).ok_or_else(|| {
        Flow::Fail("Meshy finished the re-texture but sent no GLB link.".to_string())
    })?;
    let glb = download(client.api, client.stop, &url, || {
        Ok(meshy::glb_url(kind, &client.task(kind, &task)?.body))
    })?;
    Ok(Outcome { glb, note: None })
}
