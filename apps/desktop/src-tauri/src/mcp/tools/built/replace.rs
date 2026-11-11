//! What replace_with_built_asset does: a built asset in the place of another
//! model, as the Board's Use in place of a model does, through the same
//! helpers in `commands/built/replace.rs`. It needs no approval: it changes
//! nothing outside the project, one call undoes it, and Ship is gated on its
//! own (the Ship views, and the Ship approval card when a run waits on
//! one, show the swap).

use maquette_core::built::replace::UndoReport;
use maquette_core::project;
use rmcp::schemars;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::built::replace as commands;
use crate::mcp::host::Host;
use crate::mcp::tools::hints::{step, with_next};
use crate::mcp::tools::materials::find_asset;
use crate::mcp::tools::{ToolError, to_short_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReplaceArgs {
    /// The project id.
    pub project: String,
    /// The asset whose model is replaced, by id or name. It keeps its id, its name and
    /// everything keyed by it: the layout placement, camera stops, path ends, object states,
    /// explode track, clip choices, ship choice and file name, game file.
    pub asset: String,
    /// The built asset whose script and newest model take the place, by id or name, as
    /// list_built_assets lists it. It has to be current (rebuild_built_asset first when it is
    /// stale). Its own card goes to the asset trash. Leave out with undo.
    pub built: Option<String>,
    /// Report what would change and change nothing. Defaults to false.
    #[serde(default)]
    pub dry_run: bool,
    /// Bring back the model the asset had before the replace, with its real size, fix-ups,
    /// material tweaks and library origin, and restore the built asset's card from the trash.
    /// The versions added since move to the workspace trash. Refused when one of them did not
    /// come from the script (notBuilt, an Inbox accept): a person undoes that on the Board.
    /// After a Ship wrote the built model, get_ship_plan and a Ship approval name the swap
    /// back. Defaults to false.
    #[serde(default)]
    pub undo: bool,
}

const UNDO: &str = "Call replace_with_built_asset with the same project and asset and undo: true to bring the old model back as it was.";

/// The asset id an agent named by id or name.
fn id_of(host: &dyn Host, project_id: &str, wanted: &str) -> Result<String, ToolError> {
    let (root, config) = workspace(host)?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    Ok(find_asset(&manifest, wanted)?.id.clone())
}

/// Why an agent may not undo: a version among those that leave came from
/// elsewhere than the script (an Inbox accept, maybe a paid one).
fn refusal(report: &UndoReport) -> Option<String> {
    if report.not_built.is_empty() {
        return None;
    }
    let list: Vec<String> = report.not_built.iter().map(|n| format!("v{n}")).collect();
    Some(format!(
        "{} of {} did not come from its script (an Inbox accept, say), and an undo would take it off with the built versions. Agents cannot undo this one: a person can bring the old model back from the Board, which shows what leaves.",
        list.join(", "),
        report.name
    ))
}

fn undo(host: &dyn Host, args: &ReplaceArgs, asset: &str) -> Result<Value, ToolError> {
    let plan = commands::undo_plan(host, &args.project, asset)?;
    let why = refusal(&plan);
    let (status, report) = match (&why, args.dry_run) {
        (Some(_), true) => ("refused", plan),
        (Some(why), false) => return Err(why.clone().into()),
        (None, true) => ("would undo", plan),
        (None, false) => ("undone", commands::undo(host, &args.project, asset)?.report),
    };
    let mut out = to_short_value(&report);
    out["status"] = json!(status);
    if let Some(why) = why {
        out["why"] = json!(why);
    }
    Ok(out)
}

fn replace(host: &dyn Host, args: &ReplaceArgs, asset: &str) -> Result<Value, ToolError> {
    let Some(built) = args.built.as_deref() else {
        return Err(
            "Name the built asset that takes the place (built), or set undo."
                .to_string()
                .into(),
        );
    };
    let built = id_of(host, &args.project, built)?;
    let (status, report) = if args.dry_run {
        (
            "would replace",
            commands::plan(host, &args.project, asset, &built)?,
        )
    } else {
        (
            "replaced",
            commands::run(host, &args.project, asset, &built)?.report,
        )
    };
    let mut out = to_short_value(&report);
    out["status"] = json!(status);
    if args.dry_run {
        return Ok(out);
    }
    out["undo"] = json!(UNDO);
    Ok(with_next(
        out,
        vec![step(
            "run_pipeline",
            "Builds, bakes and ships the built model in the asset's place. buildOnly stops before Ship.",
        )],
    ))
}

/// Runs the tool: the replace, its dry run or its undo.
pub(super) fn run(host: &dyn Host, args: &ReplaceArgs) -> Result<Value, ToolError> {
    let asset = id_of(host, &args.project, &args.asset)?;
    if args.undo {
        undo(host, args, &asset)
    } else {
        replace(host, args, &asset)
    }
}
