//! Codex usage for the status bar, the Lab and agents: the last read, a
//! fresh read on demand, a read soon after a Codex job settles, and Limit
//! lifted, retry now. A read that finds the usage limit active moves the
//! jobs it holds to the reset time it names.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use maquette_core::concepts::backend::ImageBackend;
use maquette_core::concepts::codex_usage::{self, CodexUsage};
use maquette_core::concepts::{ConceptEvent, ConceptJob, ConceptJobState, Concepts};
use maquette_core::pipeline::tools::Toolbox;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager as _, State};
use tauri_specta::Event as _;

use super::CommandResult;
use super::lab::Lab;
use crate::events::CodexUsageUpdate;
use crate::session::Session;

/// Reads started by a job settling are at least this far apart; one more
/// waits for the gap instead of being dropped.
const GAP: Duration = Duration::from_secs(10);

/// The last read, kept for the status bar and the next window that opens.
#[derive(Default)]
pub struct CodexUsageWatch {
    last: Mutex<Option<CodexUsage>>,
    /// A read after a job settled is waiting or running.
    pending: AtomicBool,
    started: Mutex<Option<Instant>>,
}

impl CodexUsageWatch {
    fn last(&self) -> MutexGuard<'_, Option<CodexUsage>> {
        self.last.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn started(&self) -> MutexGuard<'_, Option<Instant>> {
        self.started.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn latest(&self) -> Option<CodexUsage> {
        self.last().clone()
    }

    fn store(&self, usage: &CodexUsage) {
        *self.last() = Some(usage.clone());
    }
}

/// What Limit lifted, retry now did.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageLift {
    pub usage: CodexUsage,
    /// True when the read says the limit lifted and the held jobs ran again.
    pub lifted: bool,
    /// The jobs that ran again, or the ones still waiting.
    pub jobs: Vec<ConceptJob>,
    pub message: String,
}

/// Moves the jobs the usage limit holds to the reset time a read names.
/// Its time beats the one read from Codex's message and the backoff.
pub(crate) fn apply(lab: &Concepts, usage: &CodexUsage) {
    if let Some(at) = usage.resets_at_ms() {
        let moved = lab.retime_usage_limit(at);
        if !moved.is_empty() {
            tracing::info!(jobs = moved.len(), "held jobs wait for Codex's usage reset");
        }
    }
}

/// Reads the usage now and applies it to the Lab. Blocks up to the read's
/// timeout.
pub(crate) fn read_now(tools: &Toolbox, lab: &Concepts) -> CodexUsage {
    let usage = codex_usage::read(tools);
    apply(lab, &usage);
    usage
}

/// Limit lifted, retry now: reads the usage again and, when ordinary usage
/// is allowed and the five hour window is not full, runs every held job
/// now. Otherwise the jobs keep waiting and the message says until when.
pub(crate) fn lift(tools: &Toolbox, lab: &Concepts) -> UsageLift {
    let usage = codex_usage::read(tools);
    if usage.can_run_now() {
        let jobs = lab.lift_usage_limit();
        let message = match jobs.len() {
            0 => "The usage limit has lifted. No job was waiting for it.".to_string(),
            1 => "The usage limit has lifted. 1 job runs again now.".to_string(),
            n => format!("The usage limit has lifted. {n} jobs run again now."),
        };
        return UsageLift {
            usage,
            lifted: true,
            jobs,
            message,
        };
    }
    apply(lab, &usage);
    let message = match (usage.known, usage.resets_at_ms()) {
        (false, _) => format!(
            "Could not read the Codex usage, so the jobs keep waiting. {}",
            usage.reason.clone().unwrap_or_default()
        ),
        (true, Some(at)) => format!(
            "The usage limit still holds until about {}. The jobs run again on their own then.",
            codex_usage::local_time(at)
        ),
        (true, None) => {
            "Codex does not allow ordinary usage yet. The jobs keep waiting.".to_string()
        }
    };
    UsageLift {
        usage,
        lifted: false,
        jobs: lab.usage_held(),
        message,
    }
}

/// Keeps a read and tells the windows.
pub(crate) fn publish(app: &AppHandle, usage: &CodexUsage) {
    if let Some(watch) = app.try_state::<CodexUsageWatch>() {
        watch.store(usage);
    }
    if let Err(err) = CodexUsageUpdate(usage.clone()).emit(app) {
        tracing::warn!(error = %err, "could not emit a Codex usage event");
    }
}

/// True for a job event that should be followed by a usage read: a job
/// finished well or failed, or met the usage limit. Jobs on the Image API
/// spend no Codex usage.
pub(crate) fn settles(event: &ConceptEvent) -> bool {
    match event {
        ConceptEvent::Job { job } if job.backend == ImageBackend::Codex => {
            matches!(job.state, ConceptJobState::Done | ConceptJobState::Failed)
                || (job.state == ConceptJobState::RateLimited && job.usage_limit)
        }
        _ => false,
    }
}

/// Reads the usage on a thread of its own soon after a job settled: at
/// most one waits at a time, at least [`GAP`] after the last one started.
pub(crate) fn read_soon(app: &AppHandle) {
    let Some(watch) = app.try_state::<CodexUsageWatch>() else {
        return;
    };
    if watch.pending.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("codex-usage".into())
        .spawn(move || {
            let watch = app.state::<CodexUsageWatch>();
            let since = watch.started().map(|t| t.elapsed());
            if let Some(since) = since.filter(|s| *s < GAP) {
                std::thread::sleep(GAP - since);
            }
            *watch.started() = Some(Instant::now());
            watch.pending.store(false, Ordering::SeqCst);
            let tools = super::toolchain::toolbox_blocking(app.state::<Session>().inner());
            let usage = read_now(&tools, &app.state::<Lab>().inner().0);
            publish(&app, &usage);
        });
    if let Err(err) = spawned {
        watch.pending.store(false, Ordering::SeqCst);
        tracing::warn!(error = %err, "the Codex usage read did not start");
    }
}

/// The last read, without reading again. `None` before the first.
#[tauri::command(async)]
#[specta::specta]
pub fn codex_usage_get(watch: State<'_, CodexUsageWatch>) -> Option<CodexUsage> {
    watch.latest()
}

/// Reads the usage now (the Lab asks every minute while it is open).
#[tauri::command]
#[specta::specta]
pub async fn codex_usage_read(
    app: AppHandle,
    session: State<'_, Session>,
) -> CommandResult<CodexUsage> {
    let tools = super::toolchain::toolbox(&session).await?;
    let handle = app.clone();
    let usage = tauri::async_runtime::spawn_blocking(move || {
        read_now(&tools, &handle.state::<Lab>().inner().0)
    })
    .await
    .map_err(super::join_failed)?;
    if let Some(watch) = app.try_state::<CodexUsageWatch>() {
        *watch.started() = Some(Instant::now());
    }
    publish(&app, &usage);
    Ok(usage)
}

/// Limit lifted, retry now (see [`lift`]).
#[tauri::command]
#[specta::specta]
pub async fn lab_lift_usage_limit(
    app: AppHandle,
    session: State<'_, Session>,
) -> CommandResult<UsageLift> {
    let tools = super::toolchain::toolbox(&session).await?;
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        lift(&tools, &handle.state::<Lab>().inner().0)
    })
    .await
    .map_err(super::join_failed)?;
    publish(&app, &result.usage);
    Ok(result)
}
