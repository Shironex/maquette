//! Starting a batch: a brief or a concept's objects become a direction
//! and one planned Codex job each, with the project's style board when it
//! is on, then go to the queue.

use super::super::objects::{self, NewObjectRefs};
use super::super::style;
use super::super::{Direction, prepare_with, prompt};
use super::{ConceptJob, Concepts, Lane, Planned, Start, codex_tool};
use crate::error::CoreResult;

impl Concepts {
    /// Queues one Codex job per image and returns the direction and jobs.
    ///
    /// # Errors
    ///
    /// Fails when Codex CLI is missing or the direction cannot be written.
    pub fn start(&self, start: Start<'_>) -> CoreResult<(Direction, Vec<ConceptJob>)> {
        codex_tool(start.tools)?;
        let request = &start.request;
        let board = style::for_job(start.project_dir, request.use_style);
        let (direction, files) = prepare_with(
            start.project_dir,
            request,
            board.as_ref().map(|b| b.revision),
        )?;
        let planned = files
            .into_iter()
            .enumerate()
            .map(|(n, (index, file))| {
                #[expect(clippy::cast_possible_truncation, reason = "a batch is at most 12")]
                let text = prompt(
                    request,
                    n as u32 + 1,
                    &start.project_dir.join(&file),
                    board.as_ref(),
                );
                Planned {
                    index,
                    file,
                    prompt: text,
                    title: None,
                }
            })
            .collect();
        let lane = Lane {
            project_dir: start.project_dir,
            cache_dir: start.cache_dir,
            tools: start.tools,
            timeout: start.timeout,
        };
        let jobs = self.queue(
            &lane,
            &request.project,
            &direction,
            &request.refs,
            board.as_ref(),
            planned,
        )?;
        Ok((direction, jobs))
    }

    /// Queues one Codex job per object, each with its own prompt and the
    /// source concept attached. `request` is already checked with
    /// [`objects::clean`].
    ///
    /// # Errors
    ///
    /// Fails when Codex CLI is missing or the direction cannot be written.
    pub fn start_objects(
        &self,
        lane: &Lane<'_>,
        request: &NewObjectRefs,
    ) -> CoreResult<(Direction, Vec<ConceptJob>)> {
        codex_tool(lane.tools)?;
        let board = style::for_job(lane.project_dir, request.use_style);
        let (direction, files) = objects::prepare_with(
            lane.project_dir,
            request,
            board.as_ref().map(|b| b.revision),
        )?;
        let planned = files
            .into_iter()
            .map(|p| Planned {
                index: p.index,
                prompt: objects::object_prompt(
                    &p.object,
                    &p.notes,
                    request.kind,
                    &request.source_name,
                    request.aspect,
                    &lane.project_dir.join(&p.file),
                    board.as_ref(),
                ),
                file: p.file,
                title: Some(p.object),
            })
            .collect();
        let refs = [request.source.clone()];
        let jobs = self.queue(
            lane,
            &request.project,
            &direction,
            &refs,
            board.as_ref(),
            planned,
        )?;
        Ok((direction, jobs))
    }
}
