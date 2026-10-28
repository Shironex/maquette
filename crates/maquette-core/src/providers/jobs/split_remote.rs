//! The paid split chains: Tripo's segmentation (and part completion after
//! it, when asked) and Meshy's Auto Split, resuming from the saved task ids
//! like a generation.

use std::path::Path;

use super::remote::{create_failed, download, poll_meshy, poll_tripo};
use super::worker::Run;
use super::{CLOSED_WHILE_SENDING, Flow, FlowResult, JobPhase, Outcome, SplitJob};
use crate::providers::http::ApiError;
use crate::providers::meshy::{self, Meshy};
use crate::providers::tripo::{self, Tripo};

pub(super) fn run_tripo_split(
    run: &Run<'_>,
    client: &Tripo<'_>,
    split: &SplitJob,
    source: &Path,
) -> FlowResult<Outcome> {
    let settings = split.tripo.clone().unwrap_or_default();
    let segment = if settings.complete.is_some() {
        (0.05, 0.6)
    } else {
        (0.05, 0.95)
    };
    let chain = run.chain();
    let segment_task = match chain.model_task {
        Some(task) => task,
        None if chain.sending => return Err(Flow::Fail(CLOSED_WHILE_SENDING.to_string())),
        None => {
            let input = match &split.input_task {
                Some(task) => task.clone(),
                None => {
                    run.phase(JobPhase::Upload, 0.0);
                    client.upload(source)?
                }
            };
            if client.stop.is_stopped() {
                return Err(ApiError::Stopped.into());
            }
            run.save(|s| s.chain.sending = true);
            let task = client
                .segment(&settings, &input)
                .map_err(|e| create_failed(run, tripo::SERVICE, e))?;
            run.save(|s| {
                s.chain.model_task = Some(task.clone());
                s.job.remote_id = Some(task.clone());
                s.job.phase = Some(JobPhase::Split);
                s.job.progress = segment.0;
            });
            task
        }
    };
    let done = poll_tripo(run, client, &segment_task, JobPhase::Split, segment)?;
    run.credits(&segment_task, done.credits);
    let (final_task, output) = match settings.complete {
        None => (segment_task, done.output),
        Some(mode) => {
            let task = match run.chain().complete_task {
                Some(task) => task,
                None => {
                    let task = client.complete(mode, &segment_task)?;
                    run.save(|s| {
                        s.chain.complete_task = Some(task.clone());
                        s.job.remote_id = Some(task.clone());
                    });
                    task
                }
            };
            let completed = poll_tripo(run, client, &task, JobPhase::Complete, (0.6, 0.95))?;
            run.credits(&task, completed.credits);
            (task, completed.output)
        }
    };
    run.phase(JobPhase::Download, 0.96);
    let url = tripo::model_url(&output).ok_or_else(|| {
        Flow::Fail("Tripo finished the split but sent no model link.".to_string())
    })?;
    let glb = download(client.api, client.stop, &url, || {
        Ok(tripo::model_url(&client.task(&final_task)?.output))
    })?;
    Ok(Outcome { glb, note: None })
}

pub(super) fn run_meshy_split(
    run: &Run<'_>,
    client: &Meshy<'_>,
    split: &SplitJob,
) -> FlowResult<Outcome> {
    let settings = split.meshy.clone().unwrap_or_default();
    let range = (0.02, 0.95);
    let chain = run.chain();
    let task = match chain.model_task {
        Some(task) => task,
        None if chain.sending => return Err(Flow::Fail(CLOSED_WHILE_SENDING.to_string())),
        None => {
            let input = split
                .input_task
                .clone()
                .ok_or_else(|| Flow::Fail("Meshy has no task for this model.".to_string()))?;
            if client.stop.is_stopped() {
                return Err(ApiError::Stopped.into());
            }
            run.save(|s| s.chain.sending = true);
            let task = client
                .split(&settings, &input)
                .map_err(|e| create_failed(run, meshy::SERVICE, e))?;
            run.save(|s| {
                s.chain.model_task = Some(task.clone());
                s.chain.meshy_kind = Some(meshy::Kind::Split.path().to_string());
                s.job.remote_id = Some(task.clone());
                s.job.phase = Some(JobPhase::Split);
                s.job.progress = range.0;
            });
            task
        }
    };
    let done = poll_meshy(
        run,
        client,
        meshy::Kind::Split,
        &task,
        JobPhase::Split,
        range,
    )?;
    run.credits(&task, done.credits);
    run.phase(JobPhase::Download, 0.96);
    let url = meshy::glb_url(meshy::Kind::Split, &done.body)
        .ok_or_else(|| Flow::Fail("Meshy finished the split but sent no GLB link.".to_string()))?;
    let glb = download(client.api, client.stop, &url, || {
        Ok(meshy::glb_url(
            meshy::Kind::Split,
            &client.task(meshy::Kind::Split, &task)?.body,
        ))
    })?;
    Ok(Outcome { glb, note: None })
}
