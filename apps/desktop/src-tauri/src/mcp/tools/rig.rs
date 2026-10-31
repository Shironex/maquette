//! The rig doctor for agents: start_rig_doctor reads a model, run_rig_doctor
//! starts its pose test or fix, get_rig_doctor and list_rig_doctors read how
//! cases stand. No tool approves a joint move or adds a draft as a version:
//! a person does that in the app. Nothing here spends credits.

use maquette_core::rig::{RigDoctors, RigRecord, RigRun, RigStarter, recent};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::rig::{self as commands, RigTarget};

use super::{Host, McpTools, Out, reason, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartRigArgs {
    /// An Inbox item id, as list_inbox returns it. Give this, or project and asset.
    pub inbox: Option<String>,
    /// A project id.
    pub project: Option<String>,
    /// An asset id in that project.
    pub asset: Option<String>,
    /// The version to work from. Defaults to the newest.
    pub version: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RigStep {
    /// The pose test before: sheets, defects by place, and the chains of a creature.
    PoseTest,
    /// The fix, the verify step and the pose test after, into a draft.
    Fix,
    /// A second, last fix with a wider seam. A third is refused.
    Round2,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunRigArgs {
    /// The case id, as start_rig_doctor returns it.
    pub id: String,
    /// pose-test, fix or round-2.
    pub step: RigStep,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RigIdArgs {
    /// The case id, as start_rig_doctor or list_rig_doctors returns it.
    pub id: String,
}

fn doctors(host: &dyn Host) -> Result<&RigDoctors, String> {
    host.rig()
        .ok_or_else(|| "The rig doctor is not available in this session.".to_string())
}

/// What a list needs from a case.
fn line(case: &RigRecord) -> Value {
    json!({
        "id": case.id,
        "name": case.name,
        "state": to_value(case.state),
        "route": to_value(case.route),
        "round": case.round,
        "startedBy": to_value(case.started_by),
        "landed": case.landed,
    })
}

#[tool_router(router = rig_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Open the rig doctor's case of a rigged model: give inbox (an Inbox item) or project and asset (with version, or the newest). It reads the file at once with no Blender: the route (humanoid for Mixamo names, creature otherwise), what the doctor refuses and why (no skin, several skinned meshes, interleaved or compressed data), hand and arm weight far from the limb, islands and whether it is one continuous mesh. An Inbox model must be accepted (accept_inbox_model) before it can run. Answers the case, whose id the other rig tools take. Starts nothing and spends no credits."
    )]
    async fn start_rig_doctor(&self, Parameters(args): Parameters<StartRigArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let target = RigTarget {
                    inbox: args.inbox,
                    project: args.project,
                    asset: args.asset,
                    version: args.version,
                };
                let case =
                    commands::open(host.session(), doctors(host)?, &target, RigStarter::Agent)
                        .map_err(|e| e.message)?;
                Ok(to_value(&case))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Start a rig doctor case's pose test, fix or second round in headless Blender, on a copy: minutes, and it answers at once; follow it with get_rig_doctor. step pose-test renders the sheets and names the defects; fix changes joints and weights only, verifies that and tests again into a draft; round-2 is one last try with a wider seam. The draft never replaces the model: when it is ready a person gets an approval card in the app, looks at the sheets and adds it as the next version. A joint move is offered only there, naming the clips it would break. No tool approves either. Spends no credits."
    )]
    async fn run_rig_doctor(&self, Parameters(args): Parameters<RunRigArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let what = match args.step {
                    RigStep::PoseTest => RigRun::PoseTest,
                    RigStep::Fix => RigRun::Fix,
                    RigStep::Round2 => RigRun::Round2,
                };
                let case = doctors(host)?
                    .run(&root, &config, host.toolbox(), &args.id, what)
                    .map_err(reason)?;
                Ok(line(&case))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read a rig doctor case: its state (refused, ready, testing, tested, fixing, fixed, stopped, landed, discarded, cancelled or failed), the model, what it refuses, defects by place, the chains of a creature, the before and after sheets with each pose's stretch and seam gap in mm, the fix steps, the verify checks, the joint move if one is asked (with the clips it would break), progress, the log and why it stopped. Look at the sheet files: the numbers say where to look."
    )]
    async fn get_rig_doctor(&self, Parameters(args): Parameters<RigIdArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let case = doctors(host)?
                    .get(&root, &config, &args.id)
                    .map_err(reason)?;
                Ok(to_value(&case))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "List the rig doctor's cases, newest first: id, name, state, route, round, who started it and the version it landed as."
    )]
    async fn list_rig_doctors(&self) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let lines: Vec<Value> = recent(&root, &config).iter().map(line).collect();
                Ok(json!({ "cases": lines }))
            })
            .await;
        respond(found)
    }
}
