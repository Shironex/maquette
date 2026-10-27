//! Provider jobs: one per Send to 3D. Tripo and Meshy jobs run on their
//! own lane (four at a time) and poll until the model is ready, then the
//! GLB goes into the inbox as `<job id>.glb`, linked to its asset. By hand
//! jobs write a handoff folder and wait for the model to come back.
//!
//! Jobs are saved per project in `projects/<p>/providers.json` with every
//! remote task id, so a job sent before the app closed resumes polling the
//! next time the workspace opens instead of paying again.

mod cancel;
mod credits;
mod ids;
mod model;
mod records;
mod remote;
mod request;
mod send;
mod settle;
mod spend;
mod store;
#[cfg(test)]
mod tests;
mod worker;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use super::http::{ApiError, StopFlag, Timing};
use super::meshy;
use super::presets::{MeshyTexture, ProviderId, TripoTextureQuality};
use super::secrets::SecretStore;
use super::tripo;
use crate::error::{CoreError, CoreResult};
use crate::inbox::{SendNotes, SentMaps};
use crate::project;
use crate::workspace::WorkspaceConfig;

pub use model::{CancelMode, InboxLink, JobPhase, JobState, Provider, ProviderEvent, ProviderJob};
pub use request::{BatchFailure, BatchItem, BatchRequest, BatchSent, SendRequest};

use model::{Chain, JobsFile, Stored};
use store::{load, persist, resume_fixup, touch_asset, update};
use worker::pump;

pub const JOBS_FILE: &str = "providers.json";
const FILE_VERSION: u32 = 1;
/// Provider jobs running at once, across projects.
pub const LANE_LIMIT: usize = 4;
/// Finished jobs kept per project for the queue.
const KEEP_FINISHED: usize = 60;
const MAX_IMAGES: usize = 4;
/// The most assets one batch sends.
pub const MAX_BATCH: usize = 24;

/// What a send's settings say about the model it asked for.
pub(super) fn send_notes(job: &ProviderJob) -> SendNotes {
    let tripo = job
        .tripo
        .as_ref()
        .filter(|_| job.service == ProviderId::Tripo);
    let meshy = job
        .meshy
        .as_ref()
        .filter(|_| job.service == ProviderId::Meshy);
    let (texture, pbr, large_textures) = match (tripo, meshy) {
        (Some(t), _) => (
            t.texture,
            t.pbr,
            t.texture_quality != TripoTextureQuality::Standard,
        ),
        (None, Some(m)) => (
            m.should_texture,
            m.enable_pbr,
            m.texture_resolution != MeshyTexture::K2,
        ),
        (None, None) => return SendNotes::default(),
    };
    let maps = if !texture {
        SentMaps::NoTexture
    } else if !pbr {
        SentMaps::NoPbr
    } else {
        SentMaps::Any
    };
    SendNotes {
        maps,
        large_textures: large_textures && texture,
        by_hand_tripo: job.provider == Provider::ByHand && job.service == ProviderId::Tripo,
    }
}

/// The production API addresses, or a fake server's in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    pub tripo: String,
    pub meshy: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            tripo: tripo::BASE_URL.to_string(),
            meshy: meshy::BASE_URL.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub endpoints: Endpoints,
    pub tripo: Timing,
    pub meshy: Timing,
    pub limit: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            endpoints: Endpoints::default(),
            tripo: Timing::TRIPO,
            meshy: Timing::MESHY,
            limit: LANE_LIMIT,
        }
    }
}

#[derive(Clone)]
struct Workspace {
    root: PathBuf,
    config: WorkspaceConfig,
}

impl Workspace {
    fn project_dir(&self, project: &str) -> CoreResult<PathBuf> {
        project::dir(&self.root, &self.config.layout, project)
    }
}

struct Entry {
    stored: Stored,
    stop: Arc<StopFlag>,
    thread: bool,
    /// Set by Cancel, so a stopped thread knows it was not the app closing.
    cancelled: bool,
}

type Callback = Box<dyn Fn(ProviderEvent) + Send + Sync>;

struct Shared {
    jobs: Mutex<Vec<Entry>>,
    /// Held from a send's checks until its job is in the queue, so two sends
    /// at once cannot both pass the busy check or take the same id.
    sending: Mutex<()>,
    workspace: Mutex<Option<Workspace>>,
    on_event: Callback,
    secrets: Arc<dyn SecretStore>,
    options: Options,
}

impl Shared {
    fn jobs(&self) -> MutexGuard<'_, Vec<Entry>> {
        self.jobs.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn sending(&self) -> MutexGuard<'_, ()> {
        self.sending.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn workspace(&self) -> Option<Workspace> {
        self.workspace
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    fn emit(&self, event: ProviderEvent) {
        (self.on_event)(event);
    }
}

fn cancel_mode(stored: &Stored) -> CancelMode {
    let job = &stored.job;
    match job.state {
        JobState::Approval | JobState::Queued => CancelMode::Free,
        JobState::Waiting => CancelMode::StopWaiting,
        JobState::Running if stored.chain.model_task.is_none() && !stored.chain.sending => {
            CancelMode::Free
        }
        JobState::Running if job.provider == Provider::Meshy => {
            if matches!(job.phase, Some(JobPhase::Download)) {
                CancelMode::None
            } else {
                CancelMode::Provider
            }
        }
        _ => CancelMode::None,
    }
}

fn snapshot(stored: &Stored) -> ProviderJob {
    let mut job = stored.job.clone();
    job.cancel = cancel_mode(stored);
    job
}

pub(crate) fn is_glb(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[..4] == b"glTF"
}

enum Flow {
    Api(ApiError),
    Fail(String),
}

impl From<ApiError> for Flow {
    fn from(error: ApiError) -> Self {
        Self::Api(error)
    }
}

type FlowResult<T> = Result<T, Flow>;

struct Outcome {
    glb: Vec<u8>,
    note: Option<String>,
}

const CLOSED_WHILE_SENDING: &str = "Maquette closed while this job was being sent, so I cannot tell \
     whether it reached the provider. Check its dashboard before sending again, so you do not \
     pay twice.";

/// The service: holds every job of the open workspace.
#[derive(Clone)]
pub struct ProviderJobs {
    shared: Arc<Shared>,
}

impl ProviderJobs {
    pub fn new(
        secrets: Arc<dyn SecretStore>,
        options: Options,
        on_event: impl Fn(ProviderEvent) + Send + Sync + 'static,
    ) -> Self {
        Self {
            shared: Arc::new(Shared {
                jobs: Mutex::new(Vec::new()),
                sending: Mutex::new(()),
                workspace: Mutex::new(None),
                on_event: Box::new(on_event),
                secrets,
                options,
            }),
        }
    }

    pub fn secrets(&self) -> &Arc<dyn SecretStore> {
        &self.shared.secrets
    }

    pub fn options(&self) -> &Options {
        &self.shared.options
    }

    /// Loads every project's jobs and resumes the ones that were running.
    /// Jobs of a previously open workspace stop polling; their state stays
    /// saved for the next time it opens.
    pub fn open_workspace(&self, root: &Path, config: &WorkspaceConfig) {
        self.close();
        let workspace = Workspace {
            root: root.to_path_buf(),
            config: config.clone(),
        };
        *self
            .shared
            .workspace
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(workspace.clone());
        let projects = project::list(root, &config.layout).unwrap_or_default();
        let mut touched = Vec::new();
        let mut loaded = Vec::new();
        for manifest in &projects {
            let Ok(dir) = workspace.project_dir(&manifest.id) else {
                continue;
            };
            let (saved, older) = load(root, &dir);
            if older {
                touched.push(manifest.id.clone());
            }
            for mut stored in saved {
                // A job belongs to the project whose folder holds it, also
                // in a project folder copied by hand.
                let moved = stored.job.project != manifest.id;
                stored.job.project.clone_from(&manifest.id);
                if resume_fixup(&mut stored) || moved {
                    touched.push(manifest.id.clone());
                }
                loaded.push(stored);
            }
        }
        touched.extend(ids::make_unique(&mut loaded, |project, id| {
            workspace
                .project_dir(project)
                .is_ok_and(|dir| dir.join(super::handoff::HANDOFF_DIR).join(id).exists())
        }));
        self.shared
            .jobs()
            .extend(loaded.into_iter().map(|stored| Entry {
                stored,
                stop: Arc::new(StopFlag::default()),
                thread: false,
                cancelled: false,
            }));
        touched.sort();
        touched.dedup();
        for project in touched {
            persist(&self.shared, &project);
        }
        for manifest in &projects {
            settle::heal_stuck(&self.shared, &manifest.id);
        }
        pump(&self.shared);
    }

    /// A saved config change (presets, folder names) for later sends.
    pub fn set_config(&self, config: &WorkspaceConfig) {
        if let Some(workspace) = self
            .shared
            .workspace
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_mut()
        {
            workspace.config = config.clone();
        }
    }

    /// The open workspace's root, if any.
    pub fn root(&self) -> Option<PathBuf> {
        self.shared.workspace().map(|w| w.root)
    }

    /// Stops every polling thread, keeping the saved state. For app exit
    /// and workspace switches.
    pub fn close(&self) {
        let mut jobs = self.shared.jobs();
        for entry in jobs.iter() {
            entry.stop.stop();
        }
        jobs.clear();
    }

    pub fn list(&self, project: Option<&str>) -> Vec<ProviderJob> {
        self.shared
            .jobs()
            .iter()
            .filter(|e| project.is_none_or(|p| e.stored.job.project == p))
            .map(|e| snapshot(&e.stored))
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<ProviderJob> {
        self.shared
            .jobs()
            .iter()
            .find(|e| e.stored.job.id == id)
            .map(|e| snapshot(&e.stored))
    }

    /// Blocks until no job is queued or running, or `timeout` passes. For
    /// tests.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let end = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < end {
            let busy = self
                .shared
                .jobs()
                .iter()
                .any(|e| matches!(e.stored.job.state, JobState::Queued | JobState::Running));
            if !busy {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }
}

fn api_key(shared: &Shared, service: ProviderId) -> CoreResult<String> {
    shared.secrets.get(service)?.ok_or_else(|| {
        CoreError::Invalid(format!(
            "Add a {} API key in Settings > 3D providers first.",
            service.label()
        ))
    })
}
