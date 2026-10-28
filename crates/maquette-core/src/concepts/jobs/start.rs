//! Starting a batch: a brief, a concept's objects or an asset's refs work
//! become a direction and one planned Codex job each, with the project's
//! style board when it is on, then go to the queue.

use super::super::backend::ApiPlan;
use super::super::objects::{self, NewObjectRefs};
use super::super::refs_jobs::{self, CleanGround, NewCleanRef, NewOtherViews, RefsJobKind};
use super::super::style;
use super::super::{Direction, prepare_with, prompt};
use super::{ConceptJob, Concepts, Lane, Planned, Start, codex_tool};
use crate::assets::Asset;
use crate::error::{CoreError, CoreResult};
use crate::refs::{AssetRef, RefView};

impl Concepts {
    /// Codex must be there for a Codex batch; an Image API batch must stay
    /// under the monthly cap with its `images` images of `attached` inputs.
    fn ready(&self, lane: &Lane<'_>, images: usize, attached: usize) -> CoreResult<()> {
        match &lane.api {
            None => codex_tool(lane.tools).map(|_| ()),
            Some(plan) => self.check_api_cap(plan, images, attached),
        }
    }
}

/// The style board's images a job attaches.
fn board_images(board: Option<&style::StyleBoard>) -> usize {
    board.map_or(0, |b| b.files().len())
}

fn lane_of<'a>(start: &Start<'a>, api: Option<ApiPlan>) -> Lane<'a> {
    Lane {
        project_dir: start.project_dir,
        cache_dir: start.cache_dir,
        tools: start.tools,
        timeout: start.timeout,
        api,
    }
}

impl Concepts {
    /// Queues one Codex job per image and returns the direction and jobs.
    ///
    /// # Errors
    ///
    /// Fails when Codex CLI is missing or the direction cannot be written.
    pub fn start(&self, start: Start<'_>) -> CoreResult<(Direction, Vec<ConceptJob>)> {
        let request = &start.request;
        let board = style::for_job(start.project_dir, request.use_style);
        let lane = lane_of(&start, start.api.clone());
        self.ready(
            &lane,
            request.count as usize,
            request.refs.len() + board_images(board.as_ref()),
        )?;
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
                    size: request.aspect.api_size(),
                    transparent: false,
                }
            })
            .collect();
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
        let board = style::for_job(lane.project_dir, request.use_style);
        self.ready(
            lane,
            request.objects.len(),
            1 + board_images(board.as_ref()),
        )?;
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
                size: request.aspect.api_size(),
                transparent: false,
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

impl Concepts {
    /// Queues the candidates for an asset's other views: `count` per view
    /// the request draws (see [`refs_jobs::views_to_draw`]), each with the
    /// front attached and the project's style board when it is on.
    /// `refs` are the asset's refs with their views.
    ///
    /// # Errors
    ///
    /// Fails when Codex CLI is missing, the asset has no front, every view
    /// is filled and none was named, or the direction cannot be written.
    pub fn start_views(
        &self,
        lane: &Lane<'_>,
        asset: &Asset,
        refs: &[AssetRef],
        request: &NewOtherViews,
    ) -> CoreResult<(Direction, Vec<ConceptJob>)> {
        if lane.api.is_none() {
            codex_tool(lane.tools)?;
        }
        if !(1..=refs_jobs::MAX_PER_VIEW).contains(&request.count) {
            return Err(CoreError::Invalid(format!(
                "Ask for 1 to {} images per view.",
                refs_jobs::MAX_PER_VIEW
            )));
        }
        let front = refs_jobs::front_of(refs)?;
        let views = refs_jobs::views_to_draw(refs, &request.views)?;
        let board = style::for_job(lane.project_dir, request.use_style);
        self.ready(
            lane,
            views.len() * request.count as usize,
            1 + board_images(board.as_ref()),
        )?;
        let titles: Vec<String> = views
            .iter()
            .flat_map(|v| (0..request.count).map(|_| v.name().to_string()))
            .collect();
        let (direction, files) = refs_jobs::prepare(
            lane.project_dir,
            asset,
            RefsJobKind::Views,
            &front,
            &titles,
            board.as_ref().map(|b| b.revision),
        )?;
        let planned = files
            .into_iter()
            .map(|p| {
                let view = RefView::SLOTS
                    .into_iter()
                    .find(|v| v.name() == p.title)
                    .unwrap_or(RefView::Left);
                Planned {
                    index: p.index,
                    prompt: refs_jobs::view_prompt(
                        asset,
                        view,
                        &lane.project_dir.join(&p.file),
                        board.as_ref(),
                    ),
                    file: p.file,
                    title: Some(p.title),
                    size: "auto",
                    transparent: false,
                }
            })
            .collect();
        let jobs = self.queue(
            lane,
            &request.project,
            &direction,
            &[front],
            board.as_ref(),
            planned,
        )?;
        Ok((direction, jobs))
    }

    /// Queues one clean copy of a ref, with the ref attached and no style
    /// board: the copy keeps the ref's own look.
    ///
    /// # Errors
    ///
    /// Fails when Codex CLI is missing, the file is not one of the
    /// asset's refs, or the direction cannot be written.
    pub fn start_clean(
        &self,
        lane: &Lane<'_>,
        asset: &Asset,
        refs: &[AssetRef],
        request: &NewCleanRef,
    ) -> CoreResult<(Direction, Vec<ConceptJob>)> {
        let file = request.file.trim().replace('\\', "/");
        if !refs.iter().any(|r| r.file == file) || !lane.project_dir.join(&file).is_file() {
            return Err(CoreError::NotFound(format!(
                "{file} among the refs of {}",
                asset.name
            )));
        }
        self.ready(lane, 1, 1)?;
        let (direction, files) = refs_jobs::prepare(
            lane.project_dir,
            asset,
            RefsJobKind::Clean,
            &file,
            std::slice::from_ref(&file),
            None,
        )?;
        let planned = files
            .into_iter()
            .map(|p| Planned {
                index: p.index,
                prompt: refs_jobs::clean_prompt(request, asset, &lane.project_dir.join(&p.file)),
                file: p.file,
                title: Some(p.title),
                size: if request.pad { "1024x1024" } else { "auto" },
                transparent: request.ground == CleanGround::Transparent,
            })
            .collect();
        let jobs = self.queue(lane, &request.project, &direction, &[file], None, planned)?;
        Ok((direction, jobs))
    }
}
