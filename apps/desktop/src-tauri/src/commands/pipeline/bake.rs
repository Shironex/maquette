//! The Bake settings: the lightmap size, samples and denoiser of the Bake
//! light step, saved into its args in `pipeline.json`.

use maquette_core::pipeline::{self, bake::BakePatch, runner::PipelineRunner};
use tauri::State;

use super::{PipelineView, Pipelines, context, view};
use crate::commands::CommandResult;
use crate::session::Session;

pub(crate) fn save_bake(
    session: &Session,
    runner: &PipelineRunner,
    project_id: &str,
    patch: &BakePatch,
) -> CommandResult<PipelineView> {
    let ctx = context(session, project_id)?;
    pipeline::update(&ctx.project_dir, &ctx.manifest, |def| {
        pipeline::bake::apply(&ctx.project_dir, ctx.manifest.template, def, patch)?;
        Ok(())
    })?;
    view(session, runner, project_id)
}

/// Changes the given bake settings; the next run bakes with them.
#[tauri::command(async)]
#[specta::specta]
pub(crate) fn pipeline_save_bake(
    session: State<'_, Session>,
    runner: State<'_, Pipelines>,
    project: String,
    patch: BakePatch,
) -> CommandResult<PipelineView> {
    save_bake(&session, &runner.0, &project, &patch)
}
