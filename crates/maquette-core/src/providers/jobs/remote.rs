//! The paid chains on Tripo and Meshy: create, poll, rig check, rig,
//! animate and download, resuming from the saved task ids.

use std::path::PathBuf;

use super::worker::{Run, lerp};
use super::{CLOSED_WHILE_SENDING, Flow, FlowResult, JobPhase, Outcome, is_glb};
use crate::providers::http::{Api, ApiError, StopFlag};
use crate::providers::meshy::{self, Meshy};
use crate::providers::presets::{MeshyPreset, PresetVariant, TripoPreset};
use crate::providers::tripo::{self, Tripo};

/// Network blips while polling a paid task should not fail it.
fn transient(error: &ApiError) -> bool {
    matches!(error, ApiError::Network(_) | ApiError::Unreachable(_))
        || matches!(error, ApiError::Status { status, .. } if *status >= 500)
}

/// True when a failed create leaves open that the provider made the task:
/// the request went out but no clear answer came back.
fn may_have_created(error: &ApiError) -> bool {
    matches!(error, ApiError::Network(_) | ApiError::Malformed(_))
}

/// What a failed model create means for the job. Unless the task may
/// exist, the chain stops saying a send is in flight, so a resume starts
/// again instead of refusing with [`CLOSED_WHILE_SENDING`]; when it may,
/// the message says to check before sending again.
fn create_failed(run: &Run<'_>, service: &str, error: ApiError) -> Flow {
    if may_have_created(&error) {
        return Flow::Fail(format!(
            "{} It may have reached {service} anyway: check its dashboard before sending again, \
             so you do not pay twice.",
            error.message(service)
        ));
    }
    run.save(|s| s.chain.sending = false);
    error.into()
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

pub(super) fn run_tripo(
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
            let task = client
                .generate(preset, &tokens)
                .map_err(|e| create_failed(run, tripo::SERVICE, e))?;
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

pub(super) fn run_meshy(
    run: &Run<'_>,
    client: &Meshy<'_>,
    preset: &MeshyPreset,
    variant: PresetVariant,
    images: &[PathBuf],
    prompt: Option<&str>,
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
            let (kind, task) = client
                .generate(preset, &uris, prompt)
                .map_err(|e| create_failed(run, meshy::SERVICE, e))?;
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
