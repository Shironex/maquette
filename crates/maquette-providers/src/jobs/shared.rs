//! What every job thread and command shares: the job list, the open
//! workspace and the locks that keep its saved file whole.

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, MutexGuard};

use super::model::Stored;
use super::{Options, ProviderEvent};
use crate::error::{CoreError, CoreResult};
use crate::http::StopFlag;
use crate::secrets::SecretStore;
use maquette_model::project;
use maquette_model::sends::SendPorts;
use maquette_model::workspace::WorkspaceConfig;

#[derive(Clone)]
pub(super) struct Workspace {
    pub(super) root: PathBuf,
    pub(super) config: WorkspaceConfig,
    /// Counts the opens, so a job thread or send of a workspace opened
    /// before is told apart from one of this one even when the job ids
    /// match (a copied or synced workspace).
    pub(super) generation: u64,
}

impl Workspace {
    pub(super) fn project_dir(&self, project: &str) -> CoreResult<PathBuf> {
        project::dir(&self.root, &self.config.layout, project)
    }
}

pub(super) struct Entry {
    pub(super) stored: Stored,
    pub(super) stop: Arc<StopFlag>,
    pub(super) thread: bool,
    /// Set by Cancel, so a stopped thread knows it was not the app closing.
    pub(super) cancelled: bool,
}

pub(super) type Callback = Box<dyn Fn(ProviderEvent) + Send + Sync>;

pub(super) struct Shared {
    pub(super) jobs: Mutex<Vec<Entry>>,
    /// Held from a change to the job list through writing it, and by
    /// close, so close never clears the list between a change and its
    /// write (which would save the project's file empty). Taken before
    /// `jobs`, never while holding it.
    pub(super) saves: Mutex<()>,
    /// Held from a send's checks until its job is in the queue, so two sends
    /// at once cannot both pass the busy check or take the same id.
    pub(super) sending: Mutex<()>,
    pub(super) workspace: Mutex<Option<Workspace>>,
    /// The generation the last open gave out.
    pub(super) opened: AtomicU64,
    pub(super) on_event: Callback,
    pub(super) secrets: Arc<dyn SecretStore>,
    pub(super) options: Options,
    /// The inbox and the bake back, as the app wired them in.
    pub(super) ports: SendPorts,
    /// Runs once at the start of the next save, so a test can step in
    /// between a change and its write.
    #[cfg(test)]
    pub(super) before_save: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    /// Runs once as a send is about to queue its job, so a test can switch
    /// the workspace after the send read it.
    #[cfg(test)]
    pub(super) before_queue: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

impl Shared {
    pub(super) fn jobs(&self) -> MutexGuard<'_, Vec<Entry>> {
        self.jobs.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub(super) fn sending(&self) -> MutexGuard<'_, ()> {
        self.sending.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Runs the test hook set in `before_queue`, if any.
    #[cfg(test)]
    pub(super) fn run_before_queue(&self) {
        let hook = self
            .before_queue
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        if let Some(hook) = hook {
            hook();
        }
    }

    /// Take before changing the job list in a way that gets saved, and
    /// hold until [`persist`](super::store::persist) is done.
    pub(super) fn saving(&self) -> Saving<'_> {
        Saving {
            _held: self.saves.lock().unwrap_or_else(|p| p.into_inner()),
        }
    }

    /// The open workspace, checked under `saving` just before a change
    /// that gets saved, so nothing is written once the jobs have closed.
    /// With `read`, the generation of the workspace the caller read
    /// earlier: a send prepared there is refused when another has opened
    /// since, rather than queued in it.
    pub(super) fn still_open(
        &self,
        _saving: &Saving<'_>,
        read: Option<u64>,
    ) -> CoreResult<Workspace> {
        let workspace = self
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        if read.is_some_and(|g| g != workspace.generation) {
            return Err(CoreError::Unavailable(
                "The workspace changed while this was being sent, so nothing was queued. \
                 Send it again."
                    .to_string(),
            ));
        }
        Ok(workspace)
    }

    /// The open workspace's generation, `None` once the jobs have closed.
    pub(super) fn generation(&self) -> Option<u64> {
        self.workspace
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .map(|w| w.generation)
    }

    pub(super) fn workspace(&self) -> Option<Workspace> {
        self.workspace
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub(super) fn emit(&self, event: ProviderEvent) {
        (self.on_event)(event);
    }
}

/// Proof that the caller holds [`Shared::saving`], which [`persist`](super::store::persist) asks
/// for.
pub(super) struct Saving<'a> {
    pub(super) _held: MutexGuard<'a, ()>,
}
