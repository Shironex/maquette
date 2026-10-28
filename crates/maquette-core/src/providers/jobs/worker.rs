//! The lane: starting queued jobs while there is room, one thread per job,
//! and settling a job once its model is in.

use std::path::PathBuf;
use std::sync::Arc;

use super::records::write_inbox;
use super::remote::{run_meshy, run_tripo};
use super::split_remote::{run_meshy_split, run_tripo_split};
use super::{
    Chain, Flow, FlowResult, JobPhase, JobState, Outcome, ProviderJob, Shared, Stored, Workspace,
    api_key, touch_asset, update,
};
use crate::assets::{AssetKind, AssetStatus};
use crate::inbox;
use crate::pipeline::history::now_ms;
use crate::providers::http::{Api, ApiError, StopFlag};
use crate::providers::meshy::{self, Meshy};
use crate::providers::presets::{MeshyPreset, PresetVariant, ProviderId, TripoPreset};
use crate::providers::tripo::{self, Tripo};

/// Starts queued jobs while the lane has room.
pub(super) fn pump(shared: &Arc<Shared>) {
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

pub(super) struct Run<'a> {
    shared: &'a Shared,
    id: &'a str,
}

impl Run<'_> {
    pub(super) fn chain(&self) -> Chain {
        self.shared
            .jobs()
            .iter()
            .find(|e| e.stored.job.id == self.id)
            .map(|e| e.stored.chain.clone())
            .unwrap_or_default()
    }

    pub(super) fn save(&self, change: impl FnOnce(&mut Stored)) {
        let _ = update(self.shared, self.id, true, |s| {
            change(s);
            Ok(())
        });
    }

    pub(super) fn progress(&self, phase: JobPhase, progress: f64) {
        let _ = update(self.shared, self.id, false, |s| {
            s.job.phase = Some(phase);
            s.job.progress = s.job.progress.max(progress.clamp(0.0, 1.0));
            Ok(())
        });
    }

    pub(super) fn phase(&self, phase: JobPhase, progress: f64) {
        self.save(|s| {
            s.job.phase = Some(phase);
            s.job.progress = progress;
        });
    }

    pub(super) fn credits(&self, task: &str, credits: Option<f64>) {
        let Some(credits) = credits else {
            return;
        };
        self.save(|s| {
            s.chain.credits.insert(task.to_string(), credits);
            s.job.credits = Some(s.chain.credits.values().sum());
        });
    }
}

pub(super) fn lerp((from, to): (f64, f64), t: f64) -> f64 {
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
            let textures = job
                .split
                .as_ref()
                .map(|_| super::split_bake::textures_in(workspace, &name));
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
                if let Some(textures) = textures {
                    super::split_bake::arrived(s, textures);
                }
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
            let client = Tripo { api: &api, stop };
            if let Some(split) = &job.split {
                let source = super::split::source_of(workspace, job).ok_or_else(|| {
                    Flow::Fail("The model to split is gone from the project.".to_string())
                })?;
                return run_tripo_split(run, &client, split, &source);
            }
            let preset = job.tripo.clone().unwrap_or_else(TripoPreset::prop);
            run_tripo(run, &client, &preset, job.variant, &images, &job.views)
        }
        ProviderId::Meshy => {
            let api = Api::new(
                meshy::SERVICE,
                &options.endpoints.meshy,
                &key,
                options.meshy,
            )?;
            let client = Meshy { api: &api, stop };
            if let Some(split) = &job.split {
                return run_meshy_split(run, &client, split);
            }
            let preset = job.meshy.clone().unwrap_or_else(MeshyPreset::prop);
            run_meshy(
                run,
                &client,
                &preset,
                job.variant,
                &images,
                job.prompt.as_deref(),
            )
        }
    }
}
