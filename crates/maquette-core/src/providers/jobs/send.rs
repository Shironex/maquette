//! Sending: checking a request and resolving its settings, starting one
//! send or a batch, and releasing a send that waits for approval.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{
    BatchFailure, BatchRequest, BatchSent, CancelMode, Chain, Entry, JobState, MAX_BATCH,
    MAX_IMAGES, Provider, ProviderEvent, ProviderJob, ProviderJobs, SendRequest, Shared, Stored,
    Workspace, api_key, persist, pump, snapshot, touch_asset, update,
};
use crate::assets::{self, AssetStatus, Stage};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::now_ms;
use crate::project;
use crate::providers::handoff::Handoff;
use crate::providers::http::StopFlag;
use crate::providers::meshy;
use crate::providers::presets::{ProviderId, meshy_problem, tripo_problem};
use crate::workspace::is_single_segment;

impl ProviderJobs {
    /// Checks a send and resolves its settings, without starting anything.
    pub fn prepare(&self, request: &SendRequest) -> CoreResult<ProviderJob> {
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        prepare(&self.shared, &workspace, request)
    }

    /// Starts a send. With `approval`, an API job waits in Approval until
    /// [`ProviderJobs::approve`]; By hand never waits.
    pub fn send(
        &self,
        request: &SendRequest,
        approval: bool,
        via_agent: bool,
    ) -> CoreResult<ProviderJob> {
        self.send_in(request, approval, via_agent, None)
    }

    /// Sends several assets to one provider, one model each, on the batch
    /// settings or an item's own. Every item is checked before any goes out, so a bad one stops
    /// the batch while nothing is spent. With `approval`, the API jobs all
    /// wait in Approval for one decision.
    pub fn send_batch(
        &self,
        request: &BatchRequest,
        approval: bool,
        via_agent: bool,
    ) -> CoreResult<BatchSent> {
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        if request.items.is_empty() {
            return Err(CoreError::Invalid(
                "Choose at least one asset to send.".to_string(),
            ));
        }
        if request.items.len() > MAX_BATCH {
            return Err(CoreError::Invalid(format!(
                "Send at most {MAX_BATCH} assets in one batch."
            )));
        }
        for (n, item) in request.items.iter().enumerate() {
            if request.items[..n].iter().any(|i| i.asset == item.asset) {
                return Err(CoreError::Invalid(format!(
                    "{} is in the batch twice.",
                    item.asset
                )));
            }
        }
        for item in &request.items {
            prepare(&self.shared, &workspace, &request.send_request(item)).map_err(|err| {
                let name =
                    project::load(&workspace.root, &workspace.config.layout, &request.project)
                        .ok()
                        .and_then(|m| assets::find(&m, &item.asset).ok().map(|a| a.name.clone()))
                        .unwrap_or_else(|| item.asset.clone());
                CoreError::Invalid(format!("{name}: {err}"))
            })?;
        }
        let batch = {
            let jobs = self.shared.jobs();
            let stamp = now_ms() as u64;
            (0u32..)
                .map(|n| {
                    if n == 0 {
                        format!("batch-{stamp:x}")
                    } else {
                        format!("batch-{stamp:x}-{n}")
                    }
                })
                .find(|id| {
                    !jobs
                        .iter()
                        .any(|e| e.stored.job.batch.as_deref() == Some(id))
                })
                .unwrap_or_else(|| format!("batch-{stamp:x}"))
        };
        let mut sent = BatchSent {
            batch: batch.clone(),
            jobs: Vec::new(),
            failed: Vec::new(),
        };
        for item in &request.items {
            match self.send_in(
                &request.send_request(item),
                approval,
                via_agent,
                Some(&batch),
            ) {
                Ok(job) => sent.jobs.push(job),
                Err(err) => sent.failed.push(BatchFailure {
                    asset: item.asset.clone(),
                    message: err.to_string(),
                }),
            }
        }
        Ok(sent)
    }

    /// Every job of a batch, in the order it was sent.
    pub fn batch(&self, batch: &str) -> Vec<ProviderJob> {
        self.shared
            .jobs()
            .iter()
            .filter(|e| e.stored.job.batch.as_deref() == Some(batch))
            .map(|e| snapshot(&e.stored))
            .collect()
    }

    fn send_in(
        &self,
        request: &SendRequest,
        approval: bool,
        via_agent: bool,
        batch: Option<&str>,
    ) -> CoreResult<ProviderJob> {
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        let mut job = prepare(&self.shared, &workspace, request)?;
        job.via_agent = via_agent;
        job.batch = batch.map(str::to_string);
        let project_dir = workspace.project_dir(&job.project)?;
        let status = if job.provider == Provider::ByHand {
            let images: Vec<PathBuf> = job.images.iter().map(|i| project_dir.join(i)).collect();
            let (dir, card) = Handoff {
                id: &job.id,
                asset_name: &job.asset_name,
                service: job.service,
                variant: job.variant,
                images: &images,
                tripo: job.tripo.as_ref(),
                meshy: job.meshy.as_ref(),
            }
            .write(&project_dir)?;
            job.handoff = Some(dir.display().to_string());
            job.card = Some(card);
            job.state = JobState::Waiting;
            job.message = Some("Waiting for your model".to_string());
            AssetStatus::Queued
        } else if approval {
            job.state = JobState::Approval;
            job.message = Some("Waiting for your approval before spending credits".to_string());
            AssetStatus::Queued
        } else {
            job.state = JobState::Queued;
            AssetStatus::Queued
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
            Some(Stage::Tripo),
            status,
        );
        self.shared.emit(ProviderEvent::Job {
            job: Box::new(snapshot(&stored)),
        });
        pump(&self.shared);
        Ok(self.get(&job.id).unwrap_or(job))
    }

    /// Releases a job waiting for approval.
    pub fn approve(&self, id: &str) -> CoreResult<ProviderJob> {
        update(&self.shared, id, true, |stored| {
            if stored.job.state != JobState::Approval {
                return Err(CoreError::Invalid(
                    "This send is not waiting for approval.".to_string(),
                ));
            }
            stored.job.state = JobState::Queued;
            stored.job.message = None;
            Ok(())
        })?;
        pump(&self.shared);
        self.get(id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))
    }
}

fn check_image(project_dir: &Path, rel: &str) -> CoreResult<PathBuf> {
    let clean = rel.replace('\\', "/");
    let ok = !clean.starts_with('/')
        && !clean.contains(':')
        && clean
            .split('/')
            .all(|part| is_single_segment(part) || part.is_empty());
    if !ok || clean.split('/').any(|p| p.is_empty()) {
        return Err(CoreError::Invalid(format!(
            "{rel} is not an image in this project."
        )));
    }
    let path = project_dir.join(&clean);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) {
        return Err(CoreError::Invalid(format!(
            "{rel} is not a PNG, JPEG or WebP image."
        )));
    }
    let meta = fs::metadata(&path).map_err(|e| CoreError::io(&path, e))?;
    if meta.len() > meshy::MAX_IMAGE_BYTES {
        return Err(CoreError::Invalid(format!(
            "{rel} is over 20 MB, the most Tripo and Meshy take."
        )));
    }
    Ok(path)
}

pub(super) fn next_id(jobs: &[Entry], project_dir: &Path, project: &str, asset: &str) -> String {
    let taken = |id: &str| {
        jobs.iter()
            .any(|e| e.stored.job.project == project && e.stored.job.id == id)
            || project_dir
                .join(crate::providers::handoff::HANDOFF_DIR)
                .join(id)
                .exists()
    };
    (1u32..)
        .map(|n| format!("{asset}-{n}"))
        .find(|id| !taken(id))
        .unwrap_or_else(|| format!("{asset}-x"))
}

fn prepare(
    shared: &Shared,
    workspace: &Workspace,
    request: &SendRequest,
) -> CoreResult<ProviderJob> {
    let manifest = project::load(&workspace.root, &workspace.config.layout, &request.project)?;
    let asset = assets::find(&manifest, &request.asset)?.clone();
    let project_dir = workspace.project_dir(&request.project)?;
    if request.images.is_empty() {
        return Err(CoreError::Invalid(
            "Pick at least one image to send.".to_string(),
        ));
    }
    if request.images.len() > MAX_IMAGES {
        return Err(CoreError::Invalid(format!(
            "Send at most {MAX_IMAGES} images: front, left, back and right."
        )));
    }
    for image in &request.images {
        check_image(&project_dir, image)?;
    }
    let service = request.provider.service().unwrap_or(request.service);
    let providers = &workspace.config.providers;
    let (tripo, meshy) = match service {
        ProviderId::Tripo => {
            let preset = request
                .tripo
                .clone()
                .unwrap_or_else(|| providers.tripo(request.variant).clone());
            if let Some(problem) = tripo_problem(&preset) {
                return Err(CoreError::Invalid(problem));
            }
            (Some(preset), None)
        }
        ProviderId::Meshy => {
            let preset = request
                .meshy
                .clone()
                .unwrap_or_else(|| providers.meshy(request.variant).clone());
            if let Some(problem) = meshy_problem(&preset) {
                return Err(CoreError::Invalid(problem));
            }
            (None, Some(preset))
        }
    };
    if let Some(service) = request.provider.service() {
        api_key(shared, service)?;
    }
    let jobs = shared.jobs();
    if let Some(busy) = jobs.iter().find(|e| {
        e.stored.job.project == request.project
            && e.stored.job.asset == request.asset
            && e.stored.job.provider == request.provider
            && matches!(
                e.stored.job.state,
                JobState::Approval | JobState::Queued | JobState::Running
            )
    }) {
        return Err(CoreError::Invalid(format!(
            "{} is already on its way to {} ({}).",
            asset.name,
            request.provider.label(),
            busy.stored.job.id
        )));
    }
    let id = next_id(&jobs, &project_dir, &request.project, &asset.id);
    Ok(ProviderJob {
        id,
        project: request.project.clone(),
        asset: asset.id.clone(),
        asset_name: asset.name.clone(),
        provider: request.provider,
        service,
        variant: request.variant,
        images: request.images.clone(),
        tripo,
        meshy,
        state: JobState::Queued,
        phase: None,
        progress: 0.0,
        remote_id: None,
        credits: None,
        created_at_ms: now_ms(),
        started_at_ms: None,
        finished_at_ms: None,
        message: None,
        inbox_file: None,
        handoff: None,
        card: None,
        version: None,
        via_agent: false,
        cancel: CancelMode::Free,
        batch: None,
    })
}
