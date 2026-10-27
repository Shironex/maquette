//! What a run is asked to do, and the setup the runner gets for it.

use std::collections::BTreeMap;

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
    } = ask;
    let ctx = context(session, project_id)?;
    let ship = maquette_core::ship::load(&ctx.project_dir)?;
    let mut vars = BTreeMap::new();
    vars.insert("workspace".to_string(), ctx.root.display().to_string());
    vars.insert(
        "siteRepo".to_string(),
        ctx.manifest.site_repo.clone().unwrap_or_default(),
    );
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
