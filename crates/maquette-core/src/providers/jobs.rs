//! Provider jobs: one per Send to 3D. Tripo and Meshy jobs run on their
//! own lane (four at a time) and poll until the model is ready, then the
//! GLB goes into the inbox as `<job id>.glb`, linked to its asset. By hand
//! jobs write a handoff folder and wait for the model to come back.
//!
//! Jobs are saved per project in `projects/<p>/providers.json` with every
//! remote task id, so a job sent before the app closed resumes polling the
//! next time the workspace opens instead of paying again.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::handoff::{Handoff, HandoffCard};
use super::http::{Api, ApiError, StopFlag, Timing};
use super::meshy::{self, Meshy};
use super::presets::{
    MeshyPreset, PresetVariant, ProviderId, TripoPreset, meshy_problem, tripo_problem,
};
use super::secrets::SecretStore;
use super::tripo::{self, Tripo};
use crate::assets::{self, AssetKind, AssetStatus, Stage};
use crate::error::{CoreError, CoreResult};
use crate::fs::write_atomic;
use crate::inbox::{self, InboxItem};
use crate::json::write_json;
use crate::pipeline::history::now_ms;
use crate::project::{self, ProjectManifest};
use crate::workspace::{WorkspaceConfig, is_single_segment};

pub const JOBS_FILE: &str = "providers.json";
const FILE_VERSION: u32 = 1;
/// Provider jobs running at once, across projects.
pub const LANE_LIMIT: usize = 4;
/// Finished jobs kept per project for the queue.
const KEEP_FINISHED: usize = 60;
const MAX_IMAGES: usize = 4;

/// How a model gets made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Tripo,
    Meshy,
    ByHand,
}

impl Provider {
    pub fn service(self) -> Option<ProviderId> {
        match self {
            Self::Tripo => Some(ProviderId::Tripo),
            Self::Meshy => Some(ProviderId::Meshy),
            Self::ByHand => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Tripo => "Tripo",
            Self::Meshy => "Meshy",
            Self::ByHand => "By hand",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum JobState {
    /// An agent asked; it waits for you before spending credits.
    Approval,
    Queued,
    Running,
    /// By hand: waiting for your model.
    Waiting,
    /// The GLB is in the inbox, linked to the asset.
    Inbox,
    Accepted,
    Failed,
    Cancelled,
}

impl JobState {
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Approval | Self::Queued | Self::Running | Self::Waiting
        )
    }
}

/// What the job is doing now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum JobPhase {
    Upload,
    Generate,
    RigCheck,
    Rig,
    Animate,
    Download,
}

/// What Cancel would do, so the UI can say it before you press it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum CancelMode {
    /// Nothing was sent, so nothing is spent.
    Free,
    /// Meshy refunds a task that has not started; a started one refuses.
    Provider,
    /// By hand: stop waiting.
    StopWaiting,
    /// Too late: Tripo has no cancel, or the job is over.
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProviderJob {
    /// `<asset id>-<n>`, also the handoff folder and the inbox file name.
    pub id: String,
    pub project: String,
    pub asset: String,
    pub asset_name: String,
    pub provider: Provider,
    /// The service whose settings it uses (for By hand, whose web app).
    pub service: ProviderId,
    pub variant: PresetVariant,
    /// Input images, relative to the project folder, front first.
    pub images: Vec<String>,
    pub tripo: Option<TripoPreset>,
    pub meshy: Option<MeshyPreset>,
    pub state: JobState,
    pub phase: Option<JobPhase>,
    /// 0 to 1.
    pub progress: f64,
    /// The provider's id of the current task.
    pub remote_id: Option<String>,
    /// Credits the provider reported, summed over the chain.
    pub credits: Option<f64>,
    pub created_at_ms: f64,
    pub started_at_ms: Option<f64>,
    pub finished_at_ms: Option<f64>,
    /// Why it failed or stopped, or a note on how it went.
    pub message: Option<String>,
    /// The file name in the inbox once the model is there.
    pub inbox_file: Option<String>,
    /// By hand: the handoff folder (absolute).
    pub handoff: Option<String>,
    pub card: Option<HandoffCard>,
    /// The asset version it became.
    pub version: Option<u32>,
    pub via_agent: bool,
    pub cancel: CancelMode,
}

/// What a Send to 3D asks for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    pub project: String,
    pub asset: String,
    pub provider: Provider,
    /// For By hand: whose web app and settings. Ignored otherwise.
    pub service: ProviderId,
    pub variant: PresetVariant,
    /// Relative to the project folder, front first, at most 4.
    pub images: Vec<String>,
    /// This send's settings; `None` takes the saved preset.
    pub tripo: Option<TripoPreset>,
    pub meshy: Option<MeshyPreset>,
}

/// An inbox model that came from a send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InboxLink {
    pub job: String,
    pub project: String,
    pub asset: String,
    pub asset_name: String,
    pub provider: Provider,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProviderEvent {
    Job {
        job: Box<ProviderJob>,
    },
    /// An asset moved or changed status.
    Project {
        project: ProjectManifest,
    },
}

/// The remote side of a job, saved so a restart picks it up.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Chain {
    /// Set just before the create call. With no task id after a restart,
    /// the create may or may not have reached the provider.
    sending: bool,
    model_task: Option<String>,
    /// Meshy: which endpoint the model task lives under.
    meshy_kind: Option<String>,
    rig_check_task: Option<String>,
    rig_type: Option<String>,
    riggable: Option<bool>,
    rig_task: Option<String>,
    retarget_task: Option<String>,
    /// Credits per task id, so a resumed poll does not count twice.
    credits: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    job: ProviderJob,
    #[serde(default)]
    chain: Chain,
}

#[derive(Debug, Serialize, Deserialize)]
struct JobsFile {
    version: u32,
    jobs: Vec<Stored>,
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
    workspace: Mutex<Option<Workspace>>,
    on_event: Callback,
    secrets: Arc<dyn SecretStore>,
    options: Options,
}

impl Shared {
    fn jobs(&self) -> MutexGuard<'_, Vec<Entry>> {
        self.jobs.lock().unwrap_or_else(|p| p.into_inner())
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

/// Lower case with `_` and spaces read as `-`, for matching file names to
/// job ids.
fn normalize(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c == '_' || c == ' ' { '-' } else { c })
        .collect()
}

/// The job id an inbox file name points at: the whole stem, or the
/// longest job id it starts with, followed by `-`.
fn match_job<'a>(stem: &str, ids: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let stem = normalize(stem);
    ids.filter(|id| {
        let id = normalize(id);
        stem == id
            || stem
                .strip_prefix(&id)
                .is_some_and(|rest| rest.starts_with('-'))
    })
    .max_by_key(|id| id.len())
}

fn file_stem(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().into_owned())
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
        {
            let mut jobs = self.shared.jobs();
            for manifest in &projects {
                let Ok(dir) = workspace.project_dir(&manifest.id) else {
                    continue;
                };
                for mut stored in load(&dir) {
                    if resume_fixup(&mut stored) {
                        touched.push(manifest.id.clone());
                    }
                    jobs.push(Entry {
                        stored,
                        stop: Arc::new(StopFlag::default()),
                        thread: false,
                        cancelled: false,
                    });
                }
            }
        }
        touched.dedup();
        for project in touched {
            persist(&self.shared, &project);
        }
        pump(&self.shared);
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
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        let mut job = prepare(&self.shared, &workspace, request)?;
        job.via_agent = via_agent;
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

    /// Cancels a job where that is still possible, and says plainly when it
    /// is not. Meshy is asked over the network, so call it off the UI thread.
    pub fn cancel(&self, id: &str) -> CoreResult<ProviderJob> {
        let (stored, stop) = {
            let jobs = self.shared.jobs();
            let entry = jobs
                .iter()
                .find(|e| e.stored.job.id == id)
                .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
            (entry.stored.clone(), Arc::clone(&entry.stop))
        };
        let mode = cancel_mode(&stored);
        let message = match mode {
            CancelMode::Free => "Cancelled. Nothing was spent.".to_string(),
            CancelMode::StopWaiting => "Stopped waiting for this model.".to_string(),
            CancelMode::Provider => {
                // Marked first, so a poll that sees CANCELED does not fail it.
                self.mark_cancelled(id, true);
                if let Err(err) = self.cancel_meshy(&stored) {
                    self.mark_cancelled(id, false);
                    return Err(err);
                }
                "Cancelled before Meshy started it. Meshy refunds the credits.".to_string()
            }
            CancelMode::None => {
                return Err(CoreError::Invalid(match (stored.job.provider, stored.job.state) {
                    (Provider::Tripo, JobState::Running) => {
                        "Tripo cannot cancel a task once it is sent, so this one will finish and \
                         arrive in the Inbox. The credits are spent."
                            .to_string()
                    }
                    (_, JobState::Running) => {
                        "The model is downloading and will be in the Inbox in a moment.".to_string()
                    }
                    _ => "This job is already over.".to_string(),
                }));
            }
        };
        self.mark_cancelled(id, true);
        stop.stop();
        update(&self.shared, id, true, |stored| {
            stored.job.state = JobState::Cancelled;
            stored.job.finished_at_ms = Some(now_ms());
            stored.job.message = Some(message.clone());
            Ok(())
        })?;
        let job = self
            .get(id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
        touch_asset(
            &self.shared,
            &job.project,
            &job.asset,
            None,
            AssetStatus::Queued,
        );
        pump(&self.shared);
        Ok(job)
    }

    fn mark_cancelled(&self, id: &str, cancelled: bool) {
        let mut jobs = self.shared.jobs();
        if let Some(entry) = jobs.iter_mut().find(|e| e.stored.job.id == id) {
            entry.cancelled = cancelled;
        }
    }

    fn cancel_meshy(&self, stored: &Stored) -> CoreResult<()> {
        let key = api_key(&self.shared, ProviderId::Meshy)?;
        let options = &self.shared.options;
        let api = Api::new(
            meshy::SERVICE,
            &options.endpoints.meshy,
            &key,
            options.meshy,
        )
        .map_err(|e| CoreError::Unavailable(e.message(meshy::SERVICE)))?;
        let stop = StopFlag::default();
        let client = Meshy {
            api: &api,
            stop: &stop,
        };
        let (kind, task) = match (&stored.chain.rig_task, &stored.chain.model_task) {
            (Some(rig), _) => (meshy::Kind::Rigging, rig.clone()),
            (None, Some(model)) => (
                stored
                    .chain
                    .meshy_kind
                    .as_deref()
                    .and_then(meshy::Kind::from_path)
                    .unwrap_or(meshy::Kind::Image),
                model.clone(),
            ),
            (None, None) => {
                return Err(CoreError::Invalid(
                    "The request is on its way to Meshy. Try again in a moment.".to_string(),
                ));
            }
        };
        client.cancel(kind, &task).map_err(|e| match e {
            ApiError::Conflict(message) => CoreError::Invalid(message),
            other => CoreError::Unavailable(other.message(meshy::SERVICE)),
        })
    }

    /// Drops finished jobs of a project from the queue.
    pub fn clear_finished(&self, project: &str) {
        self.shared.jobs().retain(|e| {
            e.stored.job.project != project
                || e.stored.job.state.is_active()
                || e.stored.job.state == JobState::Inbox
        });
        persist(&self.shared, project);
    }

    /// Fills `link` on inbox items that came from a send, and moves a By
    /// hand job whose model showed up in the inbox to the Inbox state.
    pub fn link_inbox(&self, items: &mut [InboxItem]) {
        let mut arrived = Vec::new();
        {
            let jobs = self.shared.jobs();
            for item in items.iter_mut() {
                let stem = file_stem(&item.id);
                let Some(id) = match_job(&stem, jobs.iter().map(|e| e.stored.job.id.as_str()))
                else {
                    continue;
                };
                let Some(entry) = jobs.iter().find(|e| e.stored.job.id == id) else {
                    continue;
                };
                let job = &entry.stored.job;
                item.link = Some(InboxLink {
                    job: job.id.clone(),
                    project: job.project.clone(),
                    asset: job.asset.clone(),
                    asset_name: job.asset_name.clone(),
                    provider: job.provider,
                });
                if job.variant == PresetVariant::Character && item.kind_guessed {
                    item.kind = AssetKind::Character;
                }
                if job.state == JobState::Waiting {
                    arrived.push((job.id.clone(), item.id.clone()));
                }
            }
        }
        for (job, file) in arrived {
            let _ = self.arrived(&job, &file);
        }
    }

    fn arrived(&self, id: &str, file: &str) -> CoreResult<()> {
        if let Some(job) = self.get(id) {
            touch_asset(
                &self.shared,
                &job.project,
                &job.asset,
                None,
                AssetStatus::Review,
            );
        }
        update(&self.shared, id, true, |stored| {
            stored.job.state = JobState::Inbox;
            stored.job.progress = 1.0;
            stored.job.finished_at_ms = Some(now_ms());
            stored.job.inbox_file = Some(file.to_string());
            stored.job.message = Some("In the Inbox, ready for its checks".to_string());
            Ok(())
        })
    }

    /// By hand: copies a GLB someone dropped on the asset's card into the
    /// inbox under the job's name, so it goes through the usual checks.
    pub fn hand_back(&self, id: &str, file: &Path) -> CoreResult<ProviderJob> {
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        let job = self
            .get(id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
        let bytes = fs::read(file).map_err(|e| CoreError::io(file, e))?;
        if !is_glb(&bytes) {
            return Err(CoreError::Invalid(
                "That file is not a GLB. Download the model as .glb and drop it again.".to_string(),
            ));
        }
        let name = write_inbox(&workspace, &job.id, &bytes)?;
        if job.variant == PresetVariant::Character {
            let _ = inbox::set_kind(
                &workspace.root,
                &workspace.config,
                &name,
                AssetKind::Character,
            );
        }
        self.arrived(&job.id, &name)?;
        self.get(id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))
    }

    /// Marks the job behind an accepted inbox model as done.
    pub fn accepted(&self, item: &str, version: u32) {
        let id = {
            let jobs = self.shared.jobs();
            let by_file = jobs
                .iter()
                .find(|e| e.stored.job.inbox_file.as_deref() == Some(item))
                .map(|e| e.stored.job.id.clone());
            by_file.or_else(|| {
                match_job(
                    &file_stem(item),
                    jobs.iter().map(|e| e.stored.job.id.as_str()),
                )
                .map(str::to_string)
            })
        };
        if let Some(id) = id {
            let _ = update(&self.shared, &id, true, |stored| {
                stored.job.state = JobState::Accepted;
                stored.job.version = Some(version);
                stored.job.inbox_file = Some(item.to_string());
                stored.job.message = Some(format!("Accepted as v{version}"));
                stored.job.finished_at_ms.get_or_insert_with(now_ms);
                Ok(())
            });
        }
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

/// Fixes a saved job up for a fresh start. True when it changed.
fn resume_fixup(stored: &mut Stored) -> bool {
    let job = &mut stored.job;
    match job.state {
        JobState::Approval => {
            job.state = JobState::Failed;
            job.message = Some(
                "Maquette closed before you approved this send. Nothing was spent.".to_string(),
            );
            job.finished_at_ms = Some(now_ms());
            true
        }
        JobState::Running if stored.chain.sending && stored.chain.model_task.is_none() => {
            job.state = JobState::Failed;
            job.message = Some(CLOSED_WHILE_SENDING.to_string());
            job.finished_at_ms = Some(now_ms());
            true
        }
        JobState::Running => {
            // Queued again; the chain says where to pick up.
            job.state = JobState::Queued;
            job.message = Some(if stored.chain.model_task.is_some() {
                "Picking up where it left off".to_string()
            } else {
                "Starting again".to_string()
            });
            true
        }
        _ => false,
    }
}

fn load(project_dir: &Path) -> Vec<Stored> {
    let path = project_dir.join(JOBS_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            tracing::warn!(error = %err, path = %path.display(), "could not read the provider jobs");
            return Vec::new();
        }
    };
    match serde_json::from_str::<JobsFile>(&text) {
        Ok(file) if file.version <= FILE_VERSION => file.jobs,
        Ok(_) => {
            tracing::warn!(path = %path.display(), "provider jobs from a newer Maquette, ignored");
            Vec::new()
        }
        Err(err) => {
            tracing::warn!(error = %err, path = %path.display(), "unreadable provider jobs");
            Vec::new()
        }
    }
}

/// Writes a project's jobs. Keeps the newest finished ones only.
fn persist(shared: &Shared, project: &str) {
    let Some(workspace) = shared.workspace() else {
        return;
    };
    let Ok(dir) = workspace.project_dir(project) else {
        return;
    };
    let jobs = shared.jobs();
    write_project(&dir, &jobs, project);
}

fn write_project(dir: &Path, jobs: &[Entry], project: &str) {
    let mut mine: Vec<Stored> = jobs
        .iter()
        .filter(|e| e.stored.job.project == project)
        .map(|e| e.stored.clone())
        .collect();
    let finished = mine.iter().filter(|s| !s.job.state.is_active()).count();
    if finished > KEEP_FINISHED {
        let mut drop = finished - KEEP_FINISHED;
        mine.retain(|s| {
            if drop > 0 && !s.job.state.is_active() && s.job.state != JobState::Inbox {
                drop -= 1;
                false
            } else {
                true
            }
        });
    }
    let file = JobsFile {
        version: FILE_VERSION,
        jobs: mine,
    };
    if let Err(err) = write_json(&dir.join(JOBS_FILE), &file) {
        tracing::warn!(error = %err, "could not save the provider jobs");
    }
}

/// Changes one job, saves it when `save` and tells the UI.
fn update(
    shared: &Shared,
    id: &str,
    save: bool,
    change: impl FnOnce(&mut Stored) -> CoreResult<()>,
) -> CoreResult<()> {
    let (job, project) = {
        let mut jobs = shared.jobs();
        let entry = jobs
            .iter_mut()
            .find(|e| e.stored.job.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
        change(&mut entry.stored)?;
        (snapshot(&entry.stored), entry.stored.job.project.clone())
    };
    if save {
        persist(shared, &project);
    }
    shared.emit(ProviderEvent::Job { job: Box::new(job) });
    Ok(())
}

fn touch_asset(
    shared: &Shared,
    project: &str,
    asset: &str,
    stage: Option<Stage>,
    status: AssetStatus,
) {
    let Some(workspace) = shared.workspace() else {
        return;
    };
    let changed = project::modify(&workspace.root, &workspace.config.layout, project, |m| {
        if let Some(stage) = stage
            && assets::find(m, asset)?.stage < stage
        {
            assets::move_to(m, asset, stage)?;
        }
        assets::set_status(m, asset, status)
    });
    match changed {
        Ok(manifest) => shared.emit(ProviderEvent::Project { project: manifest }),
        Err(err) => tracing::warn!(error = %err, "could not update the asset"),
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

fn next_id(jobs: &[Entry], project_dir: &Path, project: &str, asset: &str) -> String {
    let taken = |id: &str| {
        jobs.iter()
            .any(|e| e.stored.job.project == project && e.stored.job.id == id)
            || project_dir
                .join(super::handoff::HANDOFF_DIR)
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
    })
}

/// Starts queued jobs while the lane has room.
fn pump(shared: &Arc<Shared>) {
    let start: Vec<(String, Arc<StopFlag>)> = {
        let mut jobs = shared.jobs();
        let running = jobs.iter().filter(|e| e.thread).count();
        let free = shared.options.limit.saturating_sub(running);
        let mut start = Vec::new();
        for entry in jobs
            .iter_mut()
            .filter(|e| !e.thread && e.stored.job.state == JobState::Queued)
            .take(free)
        {
            entry.thread = true;
            start.push((entry.stored.job.id.clone(), Arc::clone(&entry.stop)));
        }
        start
    };
    for (id, stop) in start {
        let shared = Arc::clone(shared);
        let spawned = std::thread::Builder::new()
            .name(format!("provider-{id}"))
            .spawn(move || worker(&shared, &id, &stop));
        if let Err(err) = spawned {
            tracing::warn!(error = %err, "could not start a provider job");
        }
    }
}

struct Run<'a> {
    shared: &'a Shared,
    id: &'a str,
}

impl Run<'_> {
    fn chain(&self) -> Chain {
        self.shared
            .jobs()
            .iter()
            .find(|e| e.stored.job.id == self.id)
            .map(|e| e.stored.chain.clone())
            .unwrap_or_default()
    }

    fn save(&self, change: impl FnOnce(&mut Stored)) {
        let _ = update(self.shared, self.id, true, |s| {
            change(s);
            Ok(())
        });
    }

    fn progress(&self, phase: JobPhase, progress: f64) {
        let _ = update(self.shared, self.id, false, |s| {
            s.job.phase = Some(phase);
            s.job.progress = s.job.progress.max(progress.clamp(0.0, 1.0));
            Ok(())
        });
    }

    fn phase(&self, phase: JobPhase, progress: f64) {
        self.save(|s| {
            s.job.phase = Some(phase);
            s.job.progress = progress;
        });
    }

    fn credits(&self, task: &str, credits: Option<f64>) {
        let Some(credits) = credits else {
            return;
        };
        self.save(|s| {
            s.chain.credits.insert(task.to_string(), credits);
            s.job.credits = Some(s.chain.credits.values().sum());
        });
    }
}

fn lerp((from, to): (f64, f64), t: f64) -> f64 {
    from + (to - from) * t.clamp(0.0, 1.0)
}

fn worker(shared: &Arc<Shared>, id: &str, stop: &StopFlag) {
    let job = {
        let jobs = shared.jobs();
        jobs.iter()
            .find(|e| e.stored.job.id == id)
            .map(|e| e.stored.job.clone())
    };
    let Some(job) = job else {
        return;
    };
    let workspace = shared.workspace();
    let run = Run { shared, id };
    run.save(|s| {
        s.job.state = JobState::Running;
        s.job.started_at_ms.get_or_insert_with(now_ms);
        s.job.message = None;
    });
    touch_asset(shared, &job.project, &job.asset, None, AssetStatus::Running);
    let result = match (&workspace, job.provider.service()) {
        (Some(workspace), Some(service)) => {
            run_remote(shared, &run, workspace, &job, service, stop)
        }
        _ => Err(Flow::Fail("Open the workspace first.".to_string())),
    };
    let cancelled = {
        let jobs = shared.jobs();
        jobs.iter()
            .find(|e| e.stored.job.id == id)
            .is_none_or(|e| e.cancelled)
    };
    match result {
        _ if cancelled => {}
        Err(Flow::Api(ApiError::Stopped)) => {
            // The app is closing or the workspace changed: the saved chain
            // resumes next time.
        }
        Ok(outcome) => finish(shared, &run, workspace.as_ref(), &job, outcome),
        Err(flow) => {
            let message = match flow {
                Flow::Api(error) => error.message(job.provider.label()),
                Flow::Fail(message) => message,
            };
            touch_asset(shared, &job.project, &job.asset, None, AssetStatus::Failed);
            run.save(|s| {
                s.job.state = JobState::Failed;
                s.job.finished_at_ms = Some(now_ms());
                s.job.message = Some(message);
            });
        }
    }
    {
        let mut jobs = shared.jobs();
        if let Some(entry) = jobs.iter_mut().find(|e| e.stored.job.id == id) {
            entry.thread = false;
        }
    }
    pump(shared);
}

fn write_inbox(workspace: &Workspace, id: &str, bytes: &[u8]) -> CoreResult<String> {
    let dir = inbox::inbox_dir(&workspace.root, &workspace.config);
    fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    let name = (1u32..)
        .map(|n| {
            if n == 1 {
                format!("{id}.glb")
            } else {
                format!("{id}-{n}.glb")
            }
        })
        .find(|name| !dir.join(name).exists())
        .unwrap_or_else(|| format!("{id}.glb"));
    let path = dir.join(&name);
    write_atomic(&path, bytes).map_err(|e| CoreError::io(&path, e))?;
    Ok(name)
}

fn finish(
    shared: &Shared,
    run: &Run<'_>,
    workspace: Option<&Workspace>,
    job: &ProviderJob,
    outcome: Outcome,
) {
    let Some(workspace) = workspace else {
        return;
    };
    match write_inbox(workspace, &job.id, &outcome.glb) {
        Ok(name) => {
            if job.variant == PresetVariant::Character {
                let _ = inbox::set_kind(
                    &workspace.root,
                    &workspace.config,
                    &name,
                    AssetKind::Character,
                );
            }
            // The asset first, so whoever sees the job arrive sees the Board too.
            touch_asset(shared, &job.project, &job.asset, None, AssetStatus::Review);
            run.save(|s| {
                s.job.state = JobState::Inbox;
                s.job.phase = None;
                s.job.progress = 1.0;
                s.job.finished_at_ms = Some(now_ms());
                s.job.inbox_file = Some(name.clone());
                s.job.message = Some(
                    outcome
                        .note
                        .clone()
                        .unwrap_or_else(|| "In the Inbox, ready for its checks".to_string()),
                );
            });
        }
        Err(err) => {
            touch_asset(shared, &job.project, &job.asset, None, AssetStatus::Failed);
            run.save(|s| {
                s.job.state = JobState::Failed;
                s.job.finished_at_ms = Some(now_ms());
                s.job.message = Some(format!("The model arrived but could not be saved: {err}"));
            });
        }
    }
}

fn run_remote(
    shared: &Shared,
    run: &Run<'_>,
    workspace: &Workspace,
    job: &ProviderJob,
    service: ProviderId,
    stop: &StopFlag,
) -> FlowResult<Outcome> {
    let key = api_key(shared, service).map_err(|e| Flow::Fail(e.to_string()))?;
    let project_dir = workspace
        .project_dir(&job.project)
        .map_err(|e| Flow::Fail(e.to_string()))?;
    let images: Vec<PathBuf> = job.images.iter().map(|i| project_dir.join(i)).collect();
    let options = &shared.options;
    match service {
        ProviderId::Tripo => {
            let api = Api::new(
                tripo::SERVICE,
                &options.endpoints.tripo,
                &key,
                options.tripo,
            )?;
            let preset = job.tripo.clone().unwrap_or_else(TripoPreset::prop);
            run_tripo(
                run,
                &Tripo { api: &api, stop },
                &preset,
                job.variant,
                &images,
            )
        }
        ProviderId::Meshy => {
            let api = Api::new(
                meshy::SERVICE,
                &options.endpoints.meshy,
                &key,
                options.meshy,
            )?;
            let preset = job.meshy.clone().unwrap_or_else(MeshyPreset::prop);
            run_meshy(
                run,
                &Meshy { api: &api, stop },
                &preset,
                job.variant,
                &images,
            )
        }
    }
}

/// Network blips while polling a paid task should not fail it.
fn transient(error: &ApiError) -> bool {
    matches!(error, ApiError::Network(_))
        || matches!(error, ApiError::Status { status, .. } if *status >= 500)
}

fn poll_tripo(
    run: &Run<'_>,
    client: &Tripo<'_>,
    task: &str,
    phase: JobPhase,
    range: (f64, f64),
) -> FlowResult<tripo::Task> {
    let timing = client.api.timing;
    let mut blips = 0;
    loop {
        match client.task(task) {
            Ok(found) => {
                blips = 0;
                match found.status {
                    tripo::Status::Running { progress } => {
                        run.progress(phase, lerp(range, f64::from(progress) / 100.0));
                    }
                    tripo::Status::Success => return Ok(found),
                    tripo::Status::Over(message) => return Err(Flow::Fail(message)),
                }
            }
            Err(error) if transient(&error) && blips < timing.max_retries * 3 => blips += 1,
            Err(error) => return Err(error.into()),
        }
        client.stop.sleep(timing.poll.max(timing.min_poll))?;
    }
}

fn poll_meshy(
    run: &Run<'_>,
    client: &Meshy<'_>,
    kind: meshy::Kind,
    task: &str,
    phase: JobPhase,
    range: (f64, f64),
) -> FlowResult<meshy::Task> {
    let timing = client.api.timing;
    let mut blips = 0;
    loop {
        let mut wait = timing.poll;
        match client.task(kind, task) {
            Ok(found) => {
                blips = 0;
                if let Some(asked) = found.retry_after {
                    wait = timing.clamp(asked);
                }
                match found.status {
                    meshy::Status::Pending => run.progress(phase, range.0),
                    meshy::Status::Running { progress } => {
                        run.progress(phase, lerp(range, f64::from(progress) / 100.0));
                    }
                    meshy::Status::Succeeded => return Ok(found),
                    meshy::Status::Over(message) => return Err(Flow::Fail(message)),
                }
            }
            Err(error) if transient(&error) && blips < timing.max_retries * 3 => blips += 1,
            Err(error) => return Err(error.into()),
        }
        client.stop.sleep(wait.max(timing.min_poll))?;
    }
}

/// Downloads at once; an expired link gets one fresh one from the task.
fn download(
    api: &Api,
    stop: &StopFlag,
    url: &str,
    fresh: impl Fn() -> FlowResult<Option<String>>,
) -> FlowResult<Vec<u8>> {
    let bytes = match api.download(stop, url) {
        Ok(bytes) => bytes,
        Err(ApiError::Expired) => {
            let url = fresh()?.ok_or(Flow::Api(ApiError::Expired))?;
            api.download(stop, &url)?
        }
        Err(error) => return Err(error.into()),
    };
    if !is_glb(&bytes) {
        return Err(Flow::Fail(format!(
            "{} sent a file that is not a GLB.",
            api.service
        )));
    }
    Ok(bytes)
}

fn run_tripo(
    run: &Run<'_>,
    client: &Tripo<'_>,
    preset: &TripoPreset,
    variant: PresetVariant,
    images: &[PathBuf],
) -> FlowResult<Outcome> {
    let rig = variant == PresetVariant::Character && preset.rig;
    let animate = rig && !preset.animations.is_empty();
    let generate = if rig { (0.05, 0.7) } else { (0.05, 0.95) };
    let chain = run.chain();
    let model_task = match chain.model_task {
        Some(task) => task,
        None if chain.sending => return Err(Flow::Fail(CLOSED_WHILE_SENDING.to_string())),
        None => {
            run.phase(JobPhase::Upload, 0.0);
            let mut tokens = Vec::new();
            for (i, image) in images.iter().enumerate() {
                tokens.push(client.upload(image)?);
                run.progress(
                    JobPhase::Upload,
                    0.05 * (i + 1) as f64 / images.len() as f64,
                );
            }
            if client.stop.is_stopped() {
                return Err(ApiError::Stopped.into());
            }
            run.save(|s| s.chain.sending = true);
            let task = client.generate(preset, &tokens)?;
            run.save(|s| {
                s.chain.model_task = Some(task.clone());
                s.job.remote_id = Some(task.clone());
                s.job.phase = Some(JobPhase::Generate);
                s.job.progress = generate.0;
            });
            task
        }
    };
    let done = poll_tripo(run, client, &model_task, JobPhase::Generate, generate)?;
    run.credits(&model_task, done.credits);
    let mut final_task = model_task.clone();
    let mut output = done.output;
    let mut note = None;

    if rig {
        let chain = run.chain();
        let riggable = match (chain.riggable, chain.rig_type.clone()) {
            (Some(riggable), rig_type) => (riggable, rig_type),
            (None, _) => {
                let check = match chain.rig_check_task {
                    Some(task) => task,
                    None => {
                        let task = client.rig_check(&model_task)?;
                        run.save(|s| {
                            s.chain.rig_check_task = Some(task.clone());
                            s.job.remote_id = Some(task.clone());
                        });
                        task
                    }
                };
                let found = poll_tripo(run, client, &check, JobPhase::RigCheck, (0.7, 0.75))?;
                run.credits(&check, found.credits);
                let riggable = found
                    .output
                    .get("riggable")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let rig_type = found
                    .output
                    .get("rig_type")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                run.save(|s| {
                    s.chain.riggable = Some(riggable);
                    s.chain.rig_type.clone_from(&rig_type);
                });
                (riggable, rig_type)
            }
        };
        if riggable.0 {
            let rig_type = riggable.1.unwrap_or_else(|| "biped".to_string());
            let rig_task = match run.chain().rig_task {
                Some(task) => task,
                None => {
                    let task = client.rig(preset, &model_task, &rig_type)?;
                    run.save(|s| {
                        s.chain.rig_task = Some(task.clone());
                        s.job.remote_id = Some(task.clone());
                    });
                    task
                }
            };
            let rigged = poll_tripo(run, client, &rig_task, JobPhase::Rig, (0.75, 0.85))?;
            run.credits(&rig_task, rigged.credits);
            final_task.clone_from(&rig_task);
            output = rigged.output;
            if animate {
                let retarget = match run.chain().retarget_task {
                    Some(task) => task,
                    None => {
                        let task = client.retarget(&rig_task, &preset.animations)?;
                        run.save(|s| {
                            s.chain.retarget_task = Some(task.clone());
                            s.job.remote_id = Some(task.clone());
                        });
                        task
                    }
                };
                let animated = poll_tripo(run, client, &retarget, JobPhase::Animate, (0.85, 0.95))?;
                run.credits(&retarget, animated.credits);
                final_task = retarget;
                output = animated.output;
            }
        } else {
            note = Some(
                "Tripo says this model cannot be rigged, so it arrives without a rig.".to_string(),
            );
        }
    }

    run.phase(JobPhase::Download, 0.96);
    let url = tripo::model_url(&output)
        .ok_or_else(|| Flow::Fail("Tripo finished but sent no model link.".to_string()))?;
    let glb = download(client.api, client.stop, &url, || {
        Ok(tripo::model_url(&client.task(&final_task)?.output))
    })?;
    Ok(Outcome { glb, note })
}

fn run_meshy(
    run: &Run<'_>,
    client: &Meshy<'_>,
    preset: &MeshyPreset,
    variant: PresetVariant,
    images: &[PathBuf],
) -> FlowResult<Outcome> {
    let rig = variant == PresetVariant::Character && preset.rig;
    let generate = if rig { (0.02, 0.8) } else { (0.02, 0.95) };
    let chain = run.chain();
    let (kind, model_task) = match (chain.model_task, chain.meshy_kind) {
        (Some(task), kind) => (
            kind.as_deref()
                .and_then(meshy::Kind::from_path)
                .unwrap_or(meshy::Kind::Image),
            task,
        ),
        (None, _) if chain.sending => return Err(Flow::Fail(CLOSED_WHILE_SENDING.to_string())),
        (None, _) => {
            run.phase(JobPhase::Upload, 0.0);
            let uris = images
                .iter()
                .map(|path| meshy::data_uri(path))
                .collect::<Result<Vec<_>, _>>()
                .map_err(Flow::Fail)?;
            if client.stop.is_stopped() {
                return Err(ApiError::Stopped.into());
            }
            run.save(|s| s.chain.sending = true);
            let (kind, task) = client.generate(preset, &uris)?;
            run.save(|s| {
                s.chain.model_task = Some(task.clone());
                s.chain.meshy_kind = Some(kind.path().to_string());
                s.job.remote_id = Some(task.clone());
                s.job.phase = Some(JobPhase::Generate);
                s.job.progress = generate.0;
            });
            (kind, task)
        }
    };
    let done = poll_meshy(run, client, kind, &model_task, JobPhase::Generate, generate)?;
    run.credits(&model_task, done.credits);
    let (mut final_kind, mut final_task, mut body) = (kind, model_task.clone(), done.body);

    if rig {
        let rig_task = match run.chain().rig_task {
            Some(task) => task,
            None => {
                let task = client.rig(preset, &model_task)?;
                run.save(|s| {
                    s.chain.rig_task = Some(task.clone());
                    s.job.remote_id = Some(task.clone());
                });
                task
            }
        };
        let rigged = poll_meshy(
            run,
            client,
            meshy::Kind::Rigging,
            &rig_task,
            JobPhase::Rig,
            (0.8, 0.95),
        )?;
        run.credits(&rig_task, rigged.credits);
        final_kind = meshy::Kind::Rigging;
        final_task = rig_task;
        body = rigged.body;
    }

    run.phase(JobPhase::Download, 0.96);
    let url = meshy::glb_url(final_kind, &body)
        .ok_or_else(|| Flow::Fail("Meshy finished but sent no GLB link.".to_string()))?;
    let glb = download(client.api, client.stop, &url, || {
        Ok(meshy::glb_url(
            final_kind,
            &client.task(final_kind, &final_task)?.body,
        ))
    })?;
    Ok(Outcome { glb, note: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbox_names_match_the_longest_job_id() {
        let ids = ["lantern-1", "lantern-1-2", "fox-3"];
        let find = |stem: &str| match_job(stem, ids.iter().copied());
        assert_eq!(find("lantern-1"), Some("lantern-1"));
        assert_eq!(find("Lantern_1"), Some("lantern-1"));
        assert_eq!(find("lantern-1-2"), Some("lantern-1-2"));
        assert_eq!(find("lantern-1-final"), Some("lantern-1"));
        assert_eq!(find("lantern-10"), None);
        assert_eq!(find("fox 3 v2"), Some("fox-3"));
        assert_eq!(find("wolf-1"), None);
    }
}
