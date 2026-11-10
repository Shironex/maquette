//! The lane: starting queued jobs while there is room, one thread per job,
//! and settling a job once its model is in.

use std::path::PathBuf;
use std::sync::Arc;

use super::records::write_inbox;
use super::remote::{run_meshy, run_tripo};
use super::retexture::run_meshy_retexture;
use super::split_files::{SOURCE_FILE, files_dir, source_of};
use super::split_remote::{run_meshy_split, run_tripo_split};
use super::{
    Chain, Flow, FlowResult, JobKey, JobPhase, JobState, Outcome, ProviderEvent, ProviderJob,
    Shared, Stored, Workspace, api_key, touch_in, update_in,
};
use crate::http::{Api, ApiError, StopFlag};
use crate::meshy::{self, Meshy};
use crate::presets::{MeshyPreset, PresetVariant, ProviderId, TripoPreset};
use crate::tripo::{self, Tripo};
use maquette_base::history::now_ms;
use maquette_model::assets::{AssetKind, AssetStatus};

/// Starts queued jobs while the lane has room.
pub(super) fn pump(shared: &Arc<Shared>) {
    let (generation, start): (u64, Vec<(JobKey, Arc<StopFlag>)>) = {
        let mut jobs = shared.jobs();
        let Some(generation) = shared.generation() else {
            return;
        };
        let running = jobs.iter().filter(|e| e.thread).count();
        let free = shared.options.limit.saturating_sub(running);
        let mut start = Vec::new();
        for entry in jobs
            .iter_mut()
            .filter(|e| !e.thread && e.stored.job.state == JobState::Queued)
            .take(free)
        {
            entry.thread = true;
            start.push((JobKey::of(&entry.stored.job), Arc::clone(&entry.stop)));
        }
        (generation, start)
    };
    for (key, stop) in start {
        let shared = Arc::clone(shared);
        let spawned = std::thread::Builder::new()
            .name(format!("provider-{}-{}", key.project, key.id))
            .spawn(move || worker(&shared, &key, &stop, generation));
        if let Err(err) = spawned {
            tracing::warn!(error = %err, "could not start a provider job");
        }
    }
}

/// A job thread: the job it runs (by project and id, never the id alone)
/// and the workspace generation it started under. Once that workspace
/// closes, it changes nothing.
pub(super) struct Run<'a> {
    shared: &'a Shared,
    key: &'a JobKey,
    generation: u64,
}

impl<'a> Run<'a> {
    /// A job thread's view of the job `key` names, as one started now
    /// would have.
    #[cfg(test)]
    pub(super) fn current(shared: &'a Shared, key: &'a JobKey) -> Self {
        let generation = shared.generation().expect("an open workspace");
        Self {
            shared,
            key,
            generation,
        }
    }

    /// The job as it is saved, while the workspace this thread started
    /// under is open.
    fn stored(&self) -> Option<Stored> {
        let jobs = self.shared.jobs();
        if !self.current_workspace() {
            return None;
        }
        jobs.iter()
            .find(|e| e.is(&self.key.project, &self.key.id))
            .map(|e| e.stored.clone())
    }

    /// True while the workspace this thread started under is open.
    fn current_workspace(&self) -> bool {
        self.shared.generation() == Some(self.generation)
    }

    pub(super) fn chain(&self) -> Chain {
        self.stored().map(|s| s.chain).unwrap_or_default()
    }

    pub(super) fn save(&self, change: impl FnOnce(&mut Stored)) {
        let _ = update_in(self.shared, self.generation, self.key, true, |s| {
            change(s);
            Ok(())
        });
    }

    /// Moves the job's asset in `workspace`, the one this thread started
    /// under, and tells the UI while that is still the open one.
    fn touch(&self, workspace: &Workspace, job: &ProviderJob, status: AssetStatus) {
        if let Some(manifest) = touch_in(workspace, &job.project, &job.asset, None, status)
            && self.current_workspace()
        {
            self.shared
                .emit(ProviderEvent::Project { project: manifest });
        }
    }

    pub(super) fn progress(&self, phase: JobPhase, progress: f64) {
        let _ = update_in(self.shared, self.generation, self.key, false, |s| {
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

fn worker(shared: &Arc<Shared>, key: &JobKey, stop: &StopFlag, generation: u64) {
    let run = Run {
        shared,
        key,
        generation,
    };
    let Some(workspace) = shared.workspace().filter(|w| w.generation == generation) else {
        return;
    };
    let Some(stored) = run.stored() else {
        return;
    };
    let job = &stored.job;
    run.save(|s| {
        s.job.state = JobState::Running;
        s.job.started_at_ms.get_or_insert_with(now_ms);
        s.job.message = None;
    });
    run.touch(&workspace, job, AssetStatus::Running);
    let result = match job.provider.service() {
        Some(service) => run_remote(shared, &run, &workspace, &stored, service, stop),
        None => Err(Flow::Fail("Open the workspace first.".to_string())),
    };
    let cancelled = {
        let jobs = shared.jobs();
        !run.current_workspace()
            || jobs
                .iter()
                .find(|e| e.is(&key.project, &key.id))
                .is_none_or(|e| e.cancelled)
    };
    match result {
        _ if cancelled => {}
        Err(Flow::Api(ApiError::Stopped)) => {
            // The app is closing or the workspace changed: the saved chain
            // resumes next time.
        }
        Ok(outcome) => finish(shared, &run, &workspace, job, outcome),
        Err(flow) => {
            let message = match flow {
                Flow::Api(error) => error.message(job.provider.label()),
                Flow::Fail(message) => message,
            };
            run.touch(&workspace, job, AssetStatus::Failed);
            run.save(|s| {
                s.job.state = JobState::Failed;
                s.job.finished_at_ms = Some(now_ms());
                s.job.message = Some(message);
            });
        }
    }
    {
        let mut jobs = shared.jobs();
        if run.current_workspace()
            && let Some(entry) = jobs.iter_mut().find(|e| e.is(&key.project, &key.id))
        {
            entry.thread = false;
        }
    }
    pump(shared);
}

fn finish(
    shared: &Shared,
    run: &Run<'_>,
    workspace: &Workspace,
    job: &ProviderJob,
    outcome: Outcome,
) {
    match write_inbox(workspace, &job.id, &outcome.glb) {
        Ok(name) => {
            if job.variant == PresetVariant::Character {
                let _ = shared.ports.inbox.set_kind(
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
            run.touch(workspace, job, AssetStatus::Review);
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
            run.touch(workspace, job, AssetStatus::Failed);
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
    stored: &Stored,
    service: ProviderId,
    stop: &StopFlag,
) -> FlowResult<Outcome> {
    let job = &stored.job;
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
                let source = source_of(workspace, stored).ok_or_else(|| {
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
            if job.retexture.is_some() {
                let source = files_dir(workspace, stored).join(SOURCE_FILE);
                return run_meshy_retexture(run, &client, job, &source);
            }
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
