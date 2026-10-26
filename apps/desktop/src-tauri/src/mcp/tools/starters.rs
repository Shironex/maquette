//! Tools that compare a project's starter scripts with the ones this build
//! ships and bring them up to date, as the Pipeline view's starter banner
//! does, for the project named in the call (never the one open in the app).

use maquette_core::pipeline::starter::{StarterState, StarterStatus};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::pipeline;

use super::{McpTools, Out, respond, to_value};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStartersArgs {
    /// The project id.
    pub project: String,
    /// The scripts to update, as `tools/bake_light.py`. Left out: every
    /// outdated or missing one (edited ones only with `replace`).
    #[serde(default)]
    pub scripts: Option<Vec<String>>,
    /// Also replace scripts edited by hand; each old file is kept as
    /// `<name>.bak`.
    #[serde(default)]
    pub replace: bool,
}

fn starters_value(project: &str, starters: &[StarterStatus]) -> Value {
    json!({
        "project": project,
        "starters": starters
            .iter()
            .map(|s| json!({ "path": s.path, "state": to_value(s.state), "backup": s.backup }))
            .collect::<Vec<_>>(),
    })
}

/// The scripts an update without a list covers.
fn offered(starters: &[StarterStatus], replace: bool) -> Vec<String> {
    starters
        .iter()
        .filter(|s| match s.state {
            StarterState::Outdated | StarterState::Missing => true,
            StarterState::Edited => replace,
            StarterState::Current => false,
        })
        .map(|s| s.path.clone())
        .collect()
}

#[tool_router(router = starters_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get how a project's starter scripts (tools/build_scene.py, bake_light.py, export_web.py, fixups.py, render_stops.py, ship.mjs) compare with the ones this Maquette ships: current, outdated (an untouched older copy, safe to update), edited (changed by hand) or missing.",
        annotations(read_only_hint = true)
    )]
    async fn get_starters(&self, Parameters(args): Parameters<super::ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = pipeline::view(host.session(), host.runner(), &args.project)
                    .map_err(|e| e.message)?;
                Ok(starters_value(&args.project, &view.starters))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Update a project's starter scripts, as the Update button on the Pipeline view's starter banner does, for the project given here whatever project the app has open. Without scripts it updates every outdated or missing one; an edited script is only replaced with replace true, keeping the old file as <name>.bak. Refused while the project's pipeline runs. The next run uses the new scripts. Returns each starter's state after the update; a script asked for that is still not current is reported as an error.",
        annotations(idempotent_hint = true)
    )]
    async fn update_starters(&self, Parameters(args): Parameters<UpdateStartersArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let scripts = match args.scripts {
                    Some(list) => list,
                    None => {
                        let view = pipeline::view(host.session(), host.runner(), &args.project)
                            .map_err(|e| e.message)?;
                        offered(&view.starters, args.replace)
                    }
                };
                if scripts.is_empty() {
                    return Err(format!(
                        "{}'s starter scripts are all current or edited by hand; pass replace true to replace edited ones.",
                        args.project
                    ));
                }
                let view = pipeline::update_starters(
                    host.session(),
                    host.runner(),
                    &args.project,
                    &scripts,
                    args.replace,
                )
                .map_err(|e| e.message)?;
                host.pipeline_changed(&args.project);
                Ok(starters_value(&args.project, &view.starters))
            })
            .await;
        respond(found)
    }
}
