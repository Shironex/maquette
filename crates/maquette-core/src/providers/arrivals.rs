//! The Downloads watcher for By hand sends. While a send waits for its
//! model, a new `.glb`, `.fbx` or `.zip` in the watched folder (Downloads by
//! default, set in Settings, with an off switch) is measured until it is
//! whole, hashed so it is taken once, matched to its send (the export name
//! first, then the time since the handoff and the sends waiting) and copied
//! into the inbox, where the usual checks run. A file it cannot place waits
//! for "Which asset is this?".
//!
//! Only reads: it never moves, renames or deletes anything in the folder,
//! never opens a provider's site and never spends anything.

mod folder;
mod matching;
mod seen;
mod settle;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use serde::{Deserialize, Serialize};

pub use folder::{WATCHED, WatchTiming, is_watched};

use super::handback::{SLACK_MS, modified_ms, zip_models};
use super::jobs::{JobState, Provider, ProviderJob, ProviderJobs};
use super::presets::ProviderId;
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::now_ms;
use folder::FolderWatch;
use matching::Verdict;

/// Arrivals kept for the app and the agents, newest first.
const KEEP: usize = 30;
/// Hex characters of the content hash that name an arrival.
const ID_LEN: usize = 16;

/// Where a file the watcher saw ended up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ArrivalState {
    /// Copied into the inbox for its send.
    Imported,
    /// Waiting for you to say which asset it is.
    Unmatched,
    /// An FBX: the inbox checks read GLB, so it was left alone.
    NeedsGlb,
    /// Matched, but the file could not be read as a model.
    Failed,
    /// You said it is not a model for Maquette.
    Dismissed,
}

/// One file the watcher took a look at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Arrival {
    /// The start of its content hash.
    pub id: String,
    /// Absolute.
    pub path: String,
    pub name: String,
    /// The GLB inside, for a zip.
    pub inner: Option<String>,
    pub bytes: f64,
    pub at_ms: f64,
    pub state: ArrivalState,
    pub job: Option<String>,
    pub project: Option<String>,
    pub asset_name: Option<String>,
    /// Why it matched, or why it was not taken, in a few words.
    pub reason: String,
    /// The inbox file it became.
    pub inbox_file: Option<String>,
}

/// A By hand send waiting for its model, as the watcher matches it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PendingHandoff {
    pub job: String,
    pub project: String,
    pub asset: String,
    pub asset_name: String,
    pub service: ProviderId,
    /// The export name it waits for, without an extension.
    pub expected_name: String,
    /// The handoff folder (absolute), with `send.json` in it.
    pub handoff: Option<String>,
    /// About what the site charges, from a subscription.
    pub credits: Option<f64>,
    pub created_at_ms: f64,
}

/// The watcher as Settings, the app and the agents see it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub enabled: bool,
    /// The folder watched (absolute), when one is set.
    pub folder: Option<String>,
    /// True when Settings picked the folder; false for the system Downloads.
    pub custom: bool,
    /// True while the folder is watched.
    pub watching: bool,
    /// Why it is not watched, when it should be.
    pub error: Option<String>,
    pub pending: Vec<PendingHandoff>,
    /// Newest first.
    pub arrivals: Vec<Arrival>,
}

type Callback = Box<dyn Fn(&Arrival) + Send + Sync>;

#[derive(Default)]
struct State {
    enabled: bool,
    folder: Option<PathBuf>,
    watch: Option<FolderWatch>,
    error: Option<String>,
    arrivals: Vec<Arrival>,
    /// The full hash of each arrival, by id.
    hashes: Vec<(String, String)>,
    /// Arrivals being copied in by "Which asset is this?" right now.
    assigning: Vec<String>,
}

struct Inner {
    studio: ProviderJobs,
    timing: WatchTiming,
    /// Where `downloads-seen.json` lives: the app's data folder.
    seen_dir: PathBuf,
    state: Mutex<State>,
    on_arrival: Callback,
}

impl Inner {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// The service: one per app, following whichever workspace is open.
#[derive(Clone)]
pub struct Arrivals {
    inner: Arc<Inner>,
}

fn waiting(studio: &ProviderJobs) -> Vec<ProviderJob> {
    studio
        .list(None)
        .into_iter()
        .filter(|j| j.provider == Provider::ByHand && j.state == JobState::Waiting)
        .collect()
}

fn pending(job: &ProviderJob) -> PendingHandoff {
    let card = job.card.as_ref();
    PendingHandoff {
        job: job.id.clone(),
        project: job.project.clone(),
        asset: job.asset.clone(),
        asset_name: job.asset_name.clone(),
        service: job.service,
        expected_name: card
            .map(|c| c.export_name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| super::naming::export_stem(&job.id, &job.asset, job.service)),
        handoff: job.handoff.clone(),
        credits: card.and_then(|c| c.credits),
        created_at_ms: job.created_at_ms,
    }
}

/// Every By hand send of the open workspace waiting for its model.
pub fn pending_handoffs(studio: &ProviderJobs) -> Vec<PendingHandoff> {
    waiting(studio).iter().map(pending).collect()
}

fn is_fbx(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("fbx"))
}

impl Arrivals {
    /// `seen_dir` keeps the downloads already taken, per computer.
    pub fn new(
        studio: ProviderJobs,
        timing: WatchTiming,
        seen_dir: PathBuf,
        on_arrival: impl Fn(&Arrival) + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                studio,
                timing,
                seen_dir,
                state: Mutex::new(State::default()),
                on_arrival: Box::new(on_arrival),
            }),
        }
    }

    /// Turns the watch on or off, or points it at another folder.
    pub fn configure(&self, enabled: bool, folder: Option<PathBuf>) {
        // The old watch is dropped outside the lock: its thread may be
        // waiting for the lock to report a file.
        let old = {
            let mut state = self.inner.state();
            state.enabled = enabled;
            state.folder.clone_from(&folder);
            state.error = None;
            state.watch.take()
        };
        drop(old);
        let (Some(dir), true) = (folder, enabled) else {
            return;
        };
        let weak: Weak<Inner> = Arc::downgrade(&self.inner);
        let started = folder::watch_folder(&dir, self.inner.timing, move |path| {
            if let Some(inner) = weak.upgrade() {
                let _ = (Arrivals { inner }).take(&path);
            }
        });
        let mut state = self.inner.state();
        let replaced = match started {
            Ok(watch) => state.watch.replace(watch),
            Err(err) => {
                tracing::warn!(error = %err, "downloads are not watched");
                state.error = Some(err.to_string());
                None
            }
        };
        // A watch an overlapping call started is dropped outside the lock too.
        drop(state);
        drop(replaced);
    }

    /// Stops watching, for app exit.
    pub fn stop(&self) {
        let old = self.inner.state().watch.take();
        drop(old);
    }

    pub fn status(&self) -> WatchStatus {
        let pending = pending_handoffs(&self.inner.studio);
        let state = self.inner.state();
        WatchStatus {
            enabled: state.enabled,
            folder: state.folder.as_ref().map(|f| f.display().to_string()),
            custom: false,
            watching: state.watch.is_some(),
            error: state.error.clone(),
            pending,
            arrivals: state.arrivals.clone(),
        }
    }

    fn remember(&self, arrival: Arrival, hash: String) -> Arrival {
        {
            let mut state = self.inner.state();
            state.arrivals.retain(|a| a.id != arrival.id);
            state.hashes.retain(|(id, _)| *id != arrival.id);
            state.arrivals.insert(0, arrival.clone());
            state.hashes.push((arrival.id.clone(), hash));
            state.arrivals.truncate(KEEP);
            let ids: Vec<String> = state.arrivals.iter().map(|a| a.id.clone()).collect();
            state.hashes.retain(|(id, _)| ids.contains(id));
        }
        (self.inner.on_arrival)(&arrival);
        arrival
    }

    /// The folder of the seen list, after taking over an open workspace's
    /// older one.
    fn seen_dir(&self) -> &Path {
        if let Some(root) = self.inner.studio.root()
            && let Err(err) = seen::adopt(&root, &self.inner.seen_dir)
        {
            tracing::warn!(error = %err, "could not move the older downloads list");
        }
        &self.inner.seen_dir
    }

    fn mark_seen(&self, hash: &str, job: Option<&str>) {
        let entry = seen::Entry {
            hash: hash.to_string(),
            job: job.map(str::to_string),
            at_ms: now_ms(),
        };
        if let Err(err) = seen::add(self.seen_dir(), entry) {
            tracing::warn!(error = %err, "could not remember a download");
        }
    }

    /// Copies `path` into the inbox for `job` and says how it went.
    fn import(&self, mut arrival: Arrival, hash: &str, job: &ProviderJob) -> Arrival {
        arrival.job = Some(job.id.clone());
        arrival.project = Some(job.project.clone());
        arrival.asset_name = Some(job.asset_name.clone());
        if is_fbx(Path::new(&arrival.path)) {
            arrival.state = ArrivalState::NeedsGlb;
            arrival.reason = format!(
                "An FBX for {}. Export it as GLB: the inbox checks read GLB.",
                job.asset_name
            );
            self.mark_seen(hash, None);
            return arrival;
        }
        match self
            .inner
            .studio
            .hand_back(&job.id, Path::new(&arrival.path))
        {
            Ok(done) => {
                arrival.state = ArrivalState::Imported;
                arrival.inbox_file = done.inbox_file;
                self.mark_seen(hash, Some(&job.id));
            }
            Err(err) => {
                arrival.state = ArrivalState::Failed;
                arrival.reason = err.to_string();
            }
        }
        arrival
    }

    /// Looks at one settled file. `None` when it is not for Maquette: no
    /// send waits, it is older than every handoff, or it was taken before.
    pub fn take(&self, path: &Path) -> Option<Arrival> {
        let meta = std::fs::metadata(path).ok().filter(|m| m.is_file())?;
        if meta.len() == 0 || !is_watched(path) {
            return None;
        }
        let waiting = waiting(&self.inner.studio);
        let oldest = waiting
            .iter()
            .map(|j| j.created_at_ms)
            .min_by(f64::total_cmp)?;
        let modified = modified_ms(&meta)?;
        if modified + SLACK_MS < oldest {
            return None;
        }
        self.inner.studio.root()?;
        let hash = seen::hash_file(path).ok()?;
        let id = hash[..ID_LEN].to_string();
        // Asked about already, or failed once: the same bytes fare no better.
        let asked = self.inner.state().arrivals.iter().any(|a| a.id == id);
        if asked || seen::contains(self.seen_dir(), &hash) {
            return None;
        }
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let inner_name = if name.to_ascii_lowercase().ends_with(".zip") {
            zip_models(path).ok().and_then(|m| match m.as_slice() {
                [one] => Some(one.clone()),
                _ => None,
            })
        } else {
            None
        };
        #[expect(clippy::cast_precision_loss, reason = "file sizes fit")]
        let arrival = Arrival {
            id,
            path: path.display().to_string(),
            name: name.clone(),
            inner: inner_name.clone(),
            bytes: meta.len() as f64,
            at_ms: now_ms(),
            state: ArrivalState::Unmatched,
            job: None,
            project: None,
            asset_name: None,
            reason: String::new(),
            inbox_file: None,
        };
        let arrival = match matching::decide(&name, inner_name.as_deref(), modified, &waiting) {
            Verdict::Job { id, reason } => {
                let job = waiting.iter().find(|j| j.id == id)?;
                self.import(Arrival { reason, ..arrival }, &hash, job)
            }
            Verdict::Ask { reason } => Arrival { reason, ..arrival },
        };
        Some(self.remember(arrival, hash))
    }

    fn find(&self, id: &str) -> CoreResult<(Arrival, String)> {
        let state = self.inner.state();
        let arrival = state
            .arrivals
            .iter()
            .find(|a| a.id == id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("the download {id}")))?;
        let hash = state
            .hashes
            .iter()
            .find(|(a, _)| a == id)
            .map(|(_, h)| h.clone())
            .unwrap_or_default();
        Ok((arrival, hash))
    }

    /// "Which asset is this?": takes an unmatched download for `job`.
    pub fn assign(&self, id: &str, job: &str) -> CoreResult<Arrival> {
        {
            let mut state = self.inner.state();
            if state.assigning.iter().any(|a| a == id) {
                return Err(CoreError::Invalid(
                    "That download is being added already.".to_string(),
                ));
            }
            state.assigning.push(id.to_string());
        }
        let done = self.assign_to(id, job);
        self.inner.state().assigning.retain(|a| a != id);
        done
    }

    fn assign_to(&self, id: &str, job: &str) -> CoreResult<Arrival> {
        let (arrival, hash) = self.find(id)?;
        if !matches!(
            arrival.state,
            ArrivalState::Unmatched | ArrivalState::Failed
        ) {
            return Err(CoreError::Invalid(format!(
                "{} was already taken care of.",
                arrival.name
            )));
        }
        let target = waiting(&self.inner.studio)
            .into_iter()
            .find(|j| j.id == job)
            .ok_or_else(|| {
                CoreError::Invalid(format!("The send {job} is not waiting for a model."))
            })?;
        let reason = format!("You chose {}", target.asset_name);
        let done = self.import(Arrival { reason, ..arrival }, &hash, &target);
        Ok(self.remember(done, hash))
    }

    /// Leaves a download alone for good.
    pub fn dismiss(&self, id: &str) -> CoreResult<Arrival> {
        let (arrival, hash) = self.find(id)?;
        self.mark_seen(&hash, None);
        let done = Arrival {
            state: ArrivalState::Dismissed,
            reason: "Not a model for Maquette".to_string(),
            ..arrival
        };
        let mut state = self.inner.state();
        if let Some(slot) = state.arrivals.iter_mut().find(|a| a.id == id) {
            *slot = done.clone();
        }
        Ok(done)
    }
}
