//! What a run is asked to do, the setup the runner gets for it, and the gate
//! a run an agent starts passes before Ship.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use maquette_core::fs::real_folder;
use maquette_core::materials;
use maquette_core::pipeline::runner::{RunSetup, StepGate};
use maquette_core::pipeline::tools::Toolbox;

use super::{Context, context, defaults};
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

/// Where a website's Ship writes, for its card: the repo and the folders in
/// it, as the site's `maquette.site.json` says now. No project can change
/// them while a run into that repo is queued or going (`commands::ship`).
/// Only the repo when that file cannot be read.
fn site_destination(repo: String) -> String {
    let Ok(site) = maquette_core::site::load_config(Path::new(&repo)) else {
        return repo;
    };
    let posters = site
        .posters
        .map_or_else(|| "no posters".to_string(), |p| format!("posters in {p}"));
    let data = site
        .data
        .map_or_else(|| "no data file".to_string(), |d| format!("data in {d}"));
    format!("{repo}: models in {}, {posters}, {data}", site.models)
}

pub(crate) fn setup(
    session: &Session,
    project_id: &str,
    ask: RunAsk,
    tools: Toolbox,
) -> CommandResult<RunSetup> {
    setup_in(
        session,
        context(session, project_id)?,
        project_id,
        ask,
        tools,
    )
}

/// [`setup`] on the workspace `ctx` was read from.
fn setup_in(
    session: &Session,
    ctx: Context,
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

/// [`setup`] for a run an agent starts, with the gate it passes before Ship,
/// handed to `submit`, which queues the run. A game's Ship always waits. A
/// website's waits when an agent chose the repo it ships into (the run names
/// one, or an agent set the project's and no person has set it since) or the
/// site folders in it (an agent set that repo's folders, from any project, and
/// no person has saved them since), whatever "Ask before shipping" says, and
/// otherwise while that rule is on.
///
/// The repo turn is held from reading the repo and its marks until `submit`
/// returns, so no repo or folder save lands between the gate's answer and the
/// queued run, where the save's own during-run check would not see it.
pub(crate) fn agent_run<R>(
    session: &Session,
    hub: &Arc<Hub>,
    project_id: &str,
    ask: RunAsk,
    tools: Toolbox,
    submit: impl FnOnce(RunSetup, Option<Arc<dyn StepGate>>) -> CommandResult<R>,
) -> CommandResult<R> {
    let named_here = ask.repo.is_some();
    let _turn = session.repo_turn();
    // The workspace is read once: a switch during the call cannot have the
    // marks looked up in another workspace than the run was set up in.
    let ctx = context(session, project_id)?;
    let root = ctx.root.clone();
    let ask_before_shipping = ctx.config.agents.rules.ask_before_shipping;
    let setup = setup_in(session, ctx, project_id, ask, tools)?;
    let to = destination(&setup);
    if let Some(gate) = crate::commands::game::ship_gate(session, project_id, hub, to.clone()) {
        return submit(setup, Some(gate));
    }
    let by_agent = crate::commands::site::gate::repo_set_by_agent(session, &root, project_id);
    let folders = to.as_deref().is_some_and(|repo| {
        crate::commands::site::gate::folders_set_by_agent(session, Path::new(repo))
    });
    let reason = if named_here || by_agent {
        ShipReason::AgentRepo
    } else if folders {
        ShipReason::AgentFolders
    } else if ask_before_shipping {
        ShipReason::Agent
    } else {
        return submit(setup, None);
    };
    let gate = ShipGate {
        hub: Arc::clone(hub),
        reason,
        destination: to.map(site_destination),
    };
    submit(setup, Some(Arc::new(gate)))
}
