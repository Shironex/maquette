//! Removing assets from a project and restoring them: the Board's Remove
//! from project and its Recently removed list, through the same helpers in
//! `commands/trash.rs`.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::json;

use crate::commands::trash;

use super::{McpTools, Out, ProjectArgs, respond, to_value};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAssetsArgs {
    /// The project id.
    pub project: String,
    /// The asset ids to remove, as list_assets returns them: 1 to 50.
    pub assets: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RestoreAssetArgs {
    /// The project id.
    pub project: String,
    /// The trash id remove_assets returned as `trash`, or list_removed_assets lists as `id`.
    pub trash: String,
}

#[tool_router(router = trash_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Remove assets from a project's Board, as the Board's Remove from project does. Each asset leaves project.json and its own files (refs, model versions, thumbnails, By hand handoff folders, built scripts) move to the workspace trash at trash/<project>/<asset>-<time>/, never deleted, with removed.json holding the asset entry, the original paths and what the layout, real size and fix-ups, material tweaks and built source said about it. Its placement, camera stop models, path ends, object states, clips and explode track are cleaned from the layout; a library entry it came from stays. Sends waiting for its model (By hand, or a paid send still waiting for approval or its turn) are stopped first, like Stop waiting and Cancel in the 3D queue. Each asset answers on its own: removed (with its trash id and path), not found, or refused with why: a Tripo or Meshy send making its model now (wait for the Inbox or cancel it first, so the credits are not lost) or a build running. A pipeline run going in the project refuses the whole call. restore_asset puts one back. The open app updates live.",
        annotations(destructive_hint = true, idempotent_hint = true)
    )]
    async fn remove_assets(&self, Parameters(args): Parameters<RemoveAssetsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let done = trash::remove(host, &args.project, &args.assets)?;
                Ok(json!({
                    "project": done.project.id,
                    "assets": done.outcomes.iter().map(to_value).collect::<Vec<_>>(),
                    "left": done.project.assets.len(),
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Put an asset removed by remove_assets back on the project's Board: its files move back from the trash to where they were and its layout, real size and fix-ups, material tweaks and built source come back where nothing took their place. The sends the remove stopped stay stopped. Refused while a pipeline run is going in the project, when an asset with its id or name is on the Board again, or when one of its files is back in the project (nothing is written over). Returns the asset."
    )]
    async fn restore_asset(&self, Parameters(args): Parameters<RestoreAssetArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let done = trash::restore(host, &args.project, &args.trash)?;
                Ok(json!({
                    "project": done.project.id,
                    "asset": {
                        "id": done.asset.id,
                        "name": done.asset.name,
                        "stage": to_value(done.asset.stage),
                        "versions": done.asset.versions.len(),
                    },
                    "files": done.files,
                    "stopped": done.stopped,
                    "warnings": done.warnings,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "List the assets removed from a project that are still in the workspace trash, newest first: each trash id (for restore_asset), the asset id, name, kind and versions, when it was removed, how many files it took and its folder relative to the workspace.",
        annotations(read_only_hint = true)
    )]
    async fn list_removed_assets(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let removed = trash::list(host.session(), &args.project)?;
                Ok(json!({ "project": args.project, "removed": removed }))
            })
            .await;
        respond(found)
    }
}
