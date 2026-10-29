//! Cancel and Deny: what stopping a job costs, and asking Meshy to call
//! off a task that has not started.

use std::sync::Arc;

use super::{
    CancelMode, JobState, Provider, ProviderJob, ProviderJobs, Stored, api_key, cancel_mode, pump,
    settle, update,
};
use crate::error::{CoreError, CoreResult};
use crate::http::{Api, ApiError, StopFlag};
use crate::meshy::{self, Meshy};
use crate::presets::ProviderId;
use maquette_base::history::now_ms;

/// What stopping a job in `mode` says, and whether the provider gives the
/// credits back. It does only for a model task it called off before it
/// started: a job that already has its model task on the provider (a
/// resumed one, or a Meshy chain at its rig step) has spent them.
pub(super) fn stopped(mode: CancelMode, deny: bool, stored: &Stored) -> (String, bool) {
    let has_model = stored.chain.model_task.is_some();
    let message = match mode {
        CancelMode::Free if deny => "Denied in Maquette. Nothing was spent.",
        CancelMode::Free if has_model => {
            "Cancelled. The provider already has this model's task, so its credits are spent."
        }
        CancelMode::Free => "Cancelled. Nothing was spent.",
        CancelMode::StopWaiting => "Stopped waiting for this model.",
        CancelMode::Provider if stored.chain.rig_task.is_some() => {
            "Cancelled the rig before Meshy started it. The model's credits are spent."
        }
        CancelMode::Provider => "Cancelled before Meshy started it. Meshy refunds the credits.",
        CancelMode::None => "Cancelled.",
    };
    let refunded = mode == CancelMode::Provider && stored.chain.rig_task.is_none();
    (message.to_string(), refunded)
}

impl ProviderJobs {
    /// Cancels a job where that is still possible, and says plainly when it
    /// is not. Meshy is asked over the network, so call it off the UI thread.
    pub fn cancel(&self, id: &str) -> CoreResult<ProviderJob> {
        self.stop_job(id, false)
    }

    /// Turns down a send an agent asked for. Nothing was sent, so nothing
    /// is spent.
    pub fn deny(&self, id: &str) -> CoreResult<ProviderJob> {
        self.stop_job(id, true)
    }

    fn stop_job(&self, id: &str, deny: bool) -> CoreResult<ProviderJob> {
        let (stored, stop) = {
            let jobs = self.shared.jobs();
            let entry = jobs
                .iter()
                .find(|e| e.stored.job.id == id)
                .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
            (entry.stored.clone(), Arc::clone(&entry.stop))
        };
        if deny && stored.job.state != JobState::Approval {
            return Err(CoreError::Invalid(
                "This send is not waiting for approval.".to_string(),
            ));
        }
        let mode = cancel_mode(&stored);
        match mode {
            CancelMode::Provider => {
                // Marked first, so a poll that sees CANCELED does not fail it.
                self.mark_cancelled(id, true);
                if let Err(err) = self.cancel_meshy(&stored) {
                    self.mark_cancelled(id, false);
                    return Err(err);
                }
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
            CancelMode::Free | CancelMode::StopWaiting => {}
        }
        let (message, refunded) = stopped(mode, deny, &stored);
        self.mark_cancelled(id, true);
        stop.stop();
        update(&self.shared, id, true, |stored| {
            stored.job.state = JobState::Cancelled;
            stored.chain.refunded = refunded;
            stored.job.finished_at_ms = Some(now_ms());
            stored.job.message = Some(message.clone());
            Ok(())
        })?;
        let job = self
            .get(id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
        settle::settle_asset(&self.shared, &job.project, &job.asset);
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
}
