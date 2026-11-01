//! What a run is asked to do, and the setup the runner gets for it.

use std::collections::BTreeMap;
use std::path::Path;

use maquette_core::fs::real_folder;
use maquette_core::materials;
use maquette_core::pipeline::runner::RunSetup;
use maquette_core::pipeline::tools::Toolbox;

use super::{context, defaults};
use crate::commands::CommandResult;
use crate::session::Session;

/// What a run is asked to do besides its steps.
#[derive(Debug, Clone, Default, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RunAsk {
    /// Start at this step.
    #[serde(default)]
    pub from: Option<String>,
    /// Ignore the cache.
    #[serde(default)]
    pub force: bool,
    /// Stop before Ship.
    #[serde(default)]
    pub build_only: bool,
    /// Ship into this repo for this run only (an agent's worktree, say),
    /// in place of the project's; nothing is saved.
    #[serde(default)]
    pub repo: Option<String>,
}

/// Where a run's Ship writes, for the approval to show. None with no repo:
/// it then only fills the project's shipped folder.
pub(crate) fn destination(setup: &RunSetup) -> Option<String> {
    setup
        .vars
        .get("siteRepo")
        .filter(|repo| !repo.is_empty())
        .cloned()
}

pub(crate) fn setup(
    session: &Session,
    project_id: &str,
    ask: RunAsk,
    tools: Toolbox,
) -> CommandResult<RunSetup> {
    let RunAsk {
        from,
        force,
        build_only,
        repo,
    } = ask;
    let ctx = context(session, project_id)?;
    let repo = match repo {
        Some(path) => Some(
            crate::commands::project::checked_repo(session, &path)?
                .display()
                .to_string(),
        ),
        // The folder itself, so the card names what Ship writes to and a link
        // moved during the wait changes nothing.
        None => ctx
            .manifest
            .site_repo
            .as_deref()
            .map(|r| real_folder(Path::new(r)).display().to_string()),
    };
    let ship = maquette_core::ship::load(&ctx.project_dir)?;
    let mut vars = BTreeMap::new();
    vars.insert("workspace".to_string(), ctx.root.display().to_string());
    vars.insert("siteRepo".to_string(), repo.unwrap_or_default());
    // The Ship step reads which file each asset shipped as from here.
    let index = maquette_core::site::write_ship_index(&ctx.cache_dir, &ctx.manifest, &ship)?;
    crate::commands::game::before_run(&ctx.project_dir, &ctx.manifest)?;
    vars.insert("shipIndex".to_string(), index.display().to_string());
    vars.insert("exportOrigin".to_string(), ship.origin_word().to_string());
    let tweaks = materials::by_model_stem(&ctx.manifest, &materials::load(&ctx.project_dir)?);
    Ok(RunSetup {
        project: project_id.to_string(),
        project_dir: ctx.project_dir,
        cache_dir: ctx.cache_dir,
        defaults: defaults(&ctx.config),
        def: ctx.def,
        tools,
        vars,
        shipped_names: maquette_core::assets::shipped_names(&ctx.manifest, &ship),
        materials: tweaks,
        from,
        force,
        textures: ship.textures,
        quality: ship.quality,
        build_only,
    })
}
