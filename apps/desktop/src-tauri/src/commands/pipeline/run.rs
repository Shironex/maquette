//! What a run is asked to do, the setup the runner gets for it, and the gate
//! a run an agent starts passes before Ship.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use maquette_core::fs::real_folder;
use maquette_core::materials;
use maquette_core::pipeline::runner::{RunSetup, StepGate};
use maquette_core::pipeline::tools::Toolbox;

use super::{context, defaults};
use crate::commands::CommandResult;
use crate::mcp::gate::{ShipGate, ShipReason};
use crate::mcp::hub::Hub;
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
        // moved during the wait changes nothing. An agent's repo is checked
        // again for that link; a repo a person chose stays their call.
        None => match ctx.manifest.site_repo.as_deref() {
            Some(saved) => {
                let found = real_folder(Path::new(saved));
                if crate::commands::site::gate::repo_set_by_agent(session, &ctx.root, project_id) {
                    crate::commands::project::refuse_ship_folder(session, &found)?;
                }
                Some(found.display().to_string())
            }
            None => None,
        },
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

/// [`setup`] for a run an agent starts, with the gate it passes before Ship.
/// A game's Ship always waits. A website's waits when an agent chose the
/// repo it ships into (the run names one, or an agent set the project's and
/// no person has set it since), whatever "Ask before shipping" says, and
/// otherwise while that rule is on.
pub(crate) fn agent_run(
    session: &Session,
    hub: &Arc<Hub>,
    project_id: &str,
    ask: RunAsk,
    tools: Toolbox,
) -> CommandResult<(RunSetup, Option<Arc<dyn StepGate>>)> {
    let named_here = ask.repo.is_some();
    // One turn for the repo `setup` reads and the mark read below: a save
    // landing between the two would have the gate answer for another repo.
    let _turn = session.repo_turn();
    let setup = setup(session, project_id, ask, tools)?;
    let to = destination(&setup);
    if let Some(gate) = crate::commands::game::ship_gate(session, project_id, hub, to.clone()) {
        return Ok((setup, Some(gate)));
    }
    let (root, config) = session.workspace()?;
    let by_agent = crate::commands::site::gate::repo_set_by_agent(session, &root, project_id);
    let reason = if named_here || by_agent {
        ShipReason::AgentRepo
    } else if config.agents.rules.ask_before_shipping {
        ShipReason::Agent
    } else {
        return Ok((setup, None));
    };
    let gate = ShipGate {
        hub: Arc::clone(hub),
        reason,
        destination: to,
    };
    Ok((setup, Some(Arc::new(gate))))
}
