//! The one dev server the app runs, for one site at a time, with the state
//! the Page view and the agents read: starting, running, failed or idle,
//! its address, who asked for it and the last lines it printed.
//!
//! A start returns at once and a watcher thread follows the server: it is
//! running once its address accepts connections, failed when it exits or
//! does not answer within [`START_TIMEOUT`]. A server that already answers
//! at the site's address is reused and never stopped, since Maquette did
//! not start it.
//!
//! **No orphans.** The server runs in its own process group (a job object
//! on Windows), so a stop ends the dev tool and everything it started. On
//! macOS and Linux it also runs under a small `sh` watchdog that holds the
//! read end of a pipe from the app: when the app goes away for any reason,
//! even a crash, the pipe closes and the watchdog ends the group.

use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

mod process;
use process::{Proc, exit_message, read_output, spawn};

use super::{PreviewPlan, answers};
use crate::error::{CoreError, CoreResult};

/// How long a server gets to answer after it starts.
pub const START_TIMEOUT: Duration = Duration::from_secs(90);
const POLL: Duration = Duration::from_millis(250);
/// Lines of output kept for the log.
pub const LOG_LINES: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ServerState {
    #[default]
    Idle,
    Starting,
    Running,
    Failed,
}

/// Who asked for the server. The Page view lets go of its own when it
/// closes; one Preview on site or an agent asked for stays until stopped.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum ServerOwner {
    Page,
    Agent,
    Pipeline,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub state: ServerState,
    /// The project whose site it serves.
    pub project: Option<String>,
    pub site: Option<String>,
    /// Where it answers, once it does.
    pub url: Option<String>,
    /// The command as it ran, port filled in.
    pub command: Option<String>,
    pub owner: Option<ServerOwner>,
    /// Something already answered at the site's address, so Maquette did
    /// not start it and does not stop it.
    pub external: bool,
    /// RFC 3339 UTC.
    pub started_at: Option<String>,
    /// Why it failed.
    pub message: Option<String>,
    /// The last [`LOG_LINES`] lines it printed, oldest first.
    pub log: Vec<String>,
}

#[derive(Default)]
struct Slot {
    status: ServerStatus,
    proc_: Option<Proc>,
    /// Bumped on every start and stop, so old threads know to quit.
    generation: u64,
    /// The address the output printed.
    found: Option<String>,
    /// New lines since the last notice.
    dirty: bool,
}

type Notify = Box<dyn Fn(&ServerStatus) + Send + Sync>;

struct Shared {
    slot: Mutex<Slot>,
    changed: Condvar,
    notify: Notify,
}

fn lock(shared: &Shared) -> MutexGuard<'_, Slot> {
    shared.slot.lock().unwrap_or_else(|p| p.into_inner())
}

impl Shared {
    /// Wakes waiters and tells the app, outside the lock.
    fn publish(&self, status: &ServerStatus) {
        self.changed.notify_all();
        (self.notify)(status);
    }
}

/// The one dev server the app runs.
pub struct SitePreview {
    shared: Arc<Shared>,
}

impl Default for SitePreview {
    fn default() -> Self {
        Self::new(|_| {})
    }
}

impl SitePreview {
    /// `notify` hears every change of state, and new log lines a few times
    /// a second at most.
    pub fn new(notify: impl Fn(&ServerStatus) + Send + Sync + 'static) -> Self {
        Self {
            shared: Arc::new(Shared {
                slot: Mutex::new(Slot::default()),
                changed: Condvar::new(),
                notify: Box::new(notify),
            }),
        }
    }

    pub fn status(&self) -> ServerStatus {
        lock(&self.shared).status.clone()
    }

    /// Starts the plan's server for `project`, reuses the one already
    /// serving its site, or reuses whatever answers at its address. Returns
    /// at once; the status says how it goes.
    pub fn begin(&self, plan: &PreviewPlan, project: &str, owner: ServerOwner) -> ServerStatus {
        let site = plan.site.to_string_lossy().into_owned();
        let mut slot = lock(&self.shared);
        let same = slot.status.site.as_deref() == Some(site.as_str())
            && matches!(
                slot.status.state,
                ServerState::Starting | ServerState::Running
            );
        if same {
            // Preview on site and agents outrank the view, so the view's
            // close never stops a server someone else asked for.
            slot.status.owner = slot.status.owner.max(Some(owner));
            slot.status.project = Some(project.to_string());
            let status = slot.status.clone();
            drop(slot);
            self.shared.publish(&status);
            return status;
        }
        let old = slot.proc_.take();
        slot.generation += 1;
        let generation = slot.generation;
        slot.found = None;
        slot.status = ServerStatus {
            state: ServerState::Starting,
            project: Some(project.to_string()),
            site: Some(site),
            command: Some(plan.line.clone()),
            owner: Some(owner),
            started_at: Some(crate::project::now_rfc3339()),
            ..ServerStatus::default()
        };
        drop(slot);
        drop(old);
        if let Some(url) = plan.url.as_deref().filter(|u| answers(u)) {
            return self.settle(generation, |s| {
                s.state = ServerState::Running;
                s.url = Some(url.to_string());
                s.external = true;
                s.command = None;
                s.log = vec![format!("{url} already answers; Maquette did not start it.")];
            });
        }
        match spawn(plan) {
            Ok(mut proc_) => {
                let streams = (proc_.child.stdout().take(), proc_.child.stderr().take());
                let mut slot = lock(&self.shared);
                if slot.generation == generation {
                    slot.status.log.push(format!("$ {}", plan.line));
                    slot.proc_ = Some(proc_);
                }
                let status = slot.status.clone();
                drop(slot);
                if let Some(out) = streams.0 {
                    read_output(out, Arc::clone(&self.shared), generation);
                }
                if let Some(err) = streams.1 {
                    read_output(err, Arc::clone(&self.shared), generation);
                }
                let shared = Arc::clone(&self.shared);
                let plan = plan.clone();
                thread::spawn(move || watch(&shared, generation, &plan));
                self.shared.publish(&status);
                status
            }
            Err(err) => self.settle(generation, |s| {
                s.state = ServerState::Failed;
                s.message = Some(err.to_string());
            }),
        }
    }

    /// Applies `change` when `generation` is still the current start.
    fn settle(&self, generation: u64, change: impl FnOnce(&mut ServerStatus)) -> ServerStatus {
        let mut slot = lock(&self.shared);
        if slot.generation == generation {
            change(&mut slot.status);
        }
        let status = slot.status.clone();
        drop(slot);
        self.shared.publish(&status);
        status
    }

    /// Waits up to `timeout` for the server to stop starting.
    pub fn wait(&self, timeout: Duration) -> ServerStatus {
        let deadline = Instant::now() + timeout;
        let mut slot = lock(&self.shared);
        while slot.status.state == ServerState::Starting {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            slot = self
                .shared
                .changed
                .wait_timeout(slot, left)
                .map_or_else(|p| p.into_inner().0, |(s, _)| s);
        }
        slot.status.clone()
    }

    /// [`Self::begin`], then waits until the server answers, as Preview on
    /// site does. Returns its address.
    ///
    /// # Errors
    ///
    /// Fails with the reason and the last lines of output when the server
    /// stops early or does not answer in time.
    pub fn start(
        &self,
        plan: &PreviewPlan,
        project: &str,
        owner: ServerOwner,
    ) -> CoreResult<String> {
        self.begin(plan, project, owner);
        let status = self.wait(START_TIMEOUT + POLL * 4);
        match (status.state, status.url) {
            (ServerState::Running, Some(url)) => Ok(url),
            _ => Err(CoreError::Unavailable(status.message.unwrap_or_else(
                || "The site's dev server did not start.".to_string(),
            ))),
        }
    }

    /// Stops the server the app started, if any. One that was already
    /// running elsewhere is only forgotten.
    pub fn stop(&self) {
        let mut slot = lock(&self.shared);
        let proc_ = slot.proc_.take();
        slot.generation += 1;
        let was = slot.status.state;
        slot.status.state = ServerState::Idle;
        slot.status.url = None;
        slot.status.owner = None;
        slot.status.external = false;
        slot.status.message = None;
        let status = slot.status.clone();
        drop(slot);
        if let Some(mut proc_) = proc_ {
            let _ = proc_.child.start_kill();
            let _ = proc_.child.wait();
        }
        if was != ServerState::Idle {
            self.shared.publish(&status);
        }
    }

    /// Stops the server when it serves `project` for `owner` only: the Page
    /// view closing stops its own server, not one Preview on site started.
    pub fn release(&self, project: &str, owner: ServerOwner) -> bool {
        let status = self.status();
        let mine = status.project.as_deref() == Some(project) && status.owner == Some(owner);
        if mine {
            self.stop();
        }
        mine
    }
}

impl Drop for SitePreview {
    fn drop(&mut self) {
        let proc_ = lock(&self.shared).proc_.take();
        drop(proc_);
    }
}

/// Follows one start: running once it answers, failed when it exits or
/// does not answer in time, and new log lines passed on as they come.
fn watch(shared: &Shared, generation: u64, plan: &PreviewPlan) {
    let started = Instant::now();
    loop {
        thread::sleep(POLL);
        let mut slot = lock(shared);
        if slot.generation != generation {
            return;
        }
        let exited = slot
            .proc_
            .as_mut()
            .is_none_or(|p| !matches!(p.child.try_wait(), Ok(None)));
        if exited {
            slot.proc_ = None;
            let early = slot.status.state == ServerState::Starting;
            slot.status.state = ServerState::Failed;
            slot.status.url = None;
            slot.status.message = Some(exit_message(early, &slot.status.log, plan));
            let status = slot.status.clone();
            drop(slot);
            shared.publish(&status);
            return;
        }
        let dirty = std::mem::take(&mut slot.dirty);
        if slot.status.state == ServerState::Starting {
            let candidate = plan
                .url
                .clone()
                .or_else(|| slot.found.clone())
                .or_else(|| plan.expected_url());
            drop(slot);
            let up = candidate.as_deref().is_some_and(answers);
            let mut slot = lock(shared);
            if slot.generation != generation {
                return;
            }
            if up {
                slot.status.state = ServerState::Running;
                slot.status.url = candidate;
            } else if started.elapsed() > START_TIMEOUT {
                slot.proc_ = None;
                slot.status.state = ServerState::Failed;
                slot.status.message = Some(format!(
                    "The site's dev server did not answer within {} seconds, so Maquette stopped it.",
                    START_TIMEOUT.as_secs()
                ));
            } else if !dirty {
                continue;
            }
            let status = slot.status.clone();
            drop(slot);
            shared.publish(&status);
        } else if dirty {
            let status = slot.status.clone();
            drop(slot);
            shared.publish(&status);
        }
    }
}

#[cfg(test)]
mod tests;
