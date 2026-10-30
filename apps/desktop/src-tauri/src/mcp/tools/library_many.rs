//! add_many_to_library: several project assets into the library in one
//! call, or with dryRun the plan of what each would become. Copies files,
//! never spends credits.

use maquette_core::library::{AddToLibrary, CollectionChange, PlannedAdd};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::{library as commands, library_batch as batch};

use super::library::asset_id;
use super::{McpTools, Out, respond, to_value};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LibraryAssetArgs {
    /// The asset id or name, as list_assets returns it.
    pub asset: String,
    /// The entry's name. Defaults to the asset's.
    pub name: Option<String>,
    /// This asset's tags. They replace the shared tags for this asset.
    pub tags: Option<Vec<String>>,
    /// A library entry id: add the model as that entry's next version instead of the one the
    /// asset belongs to.
    pub into: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddManyToLibraryArgs {
    /// The project id.
    pub project: String,
    /// 1 to 200 assets. With dryRun it may be left out, meaning every asset in the plan.
    #[serde(default)]
    pub assets: Vec<LibraryAssetArgs>,
    /// Tags for every asset that names none of its own.
    pub tags: Option<Vec<String>>,
    /// A collection id every added entry joins.
    pub collection: Option<String>,
    /// Only say what each asset would become, adding nothing. Defaults to false.
    #[serde(default)]
    pub dry_run: bool,
}

/// The most assets one add_many_to_library call takes.
const MAX_BATCH: usize = 200;

/// Adds one asset as add_to_library does and answers its result row, or the
/// reason it failed.
fn add_one(
    host: &dyn super::Host,
    project: &str,
    item: &LibraryAssetArgs,
    shared: Option<&Vec<String>>,
) -> Result<Value, String> {
    let asset = asset_id(host, project, &item.asset)?;
    let request = AddToLibrary {
        project: project.to_string(),
        asset,
        name: item.name.clone(),
        tags: item.tags.clone().or_else(|| shared.cloned()),
        into: item.into.clone(),
    };
    let added = commands::add(host.session(), host.studio(), &request).map_err(|e| e.message)?;
    host.project_changed(&added.project);
    Ok(json!({
        "asset": item.asset,
        "ok": true,
        "entry": added.entry.id,
        "name": added.entry.name,
        "version": added.version,
        "added": added.added,
        "tags": added.entry.tags,
    }))
}

/// The plan for the project, cut to the assets asked about when any are.
fn dry_run(host: &dyn super::Host, args: &AddManyToLibraryArgs) -> Result<Value, String> {
    let mut plan = batch::plan_many(host.session(), &args.project).map_err(|e| e.message)?;
    if !args.assets.is_empty() {
        let wanted = args
            .assets
            .iter()
            .map(|a| asset_id(host, &args.project, &a.asset))
            .collect::<Result<Vec<_>, _>>()?;
        plan.assets
            .retain(|a: &PlannedAdd| wanted.contains(&a.asset));
    }
    Ok(json!({ "dryRun": true, "plan": to_value(&plan) }))
}

#[tool_router(router = library_many_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Add several project assets to the library in one call, each as add_to_library does (1 to 200 assets). tags are shared by every asset that names none of its own; an asset's into (an entry id) adds its model as that entry's next version instead of the one it belongs to; collection (a collection id) is joined by every added entry. Each asset answers on its own, with ok true and its entry, version and added, or ok false and why, and one failure does not stop the rest; the answer also counts added and failed. dryRun adds nothing and answers the plan: for every asset with a model (or only the ones named) whether it is new, an entry's next version, the same file as an entry, a name clash, a look-alike of an entry's picture or unreadable (a missing or cut short model), with the entry it meets, and the suggested project tag. Copies files, spends nothing and needs no approval."
    )]
    async fn add_many_to_library(&self, Parameters(args): Parameters<AddManyToLibraryArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                if args.dry_run {
                    return dry_run(host, &args);
                }
                if args.assets.is_empty() || args.assets.len() > MAX_BATCH {
                    return Err(format!("Give 1 to {MAX_BATCH} assets."));
                }
                if let Some(id) = &args.collection {
                    let known = batch::collections(host.session()).map_err(|e| e.message)?;
                    if !known.iter().any(|c| &c.id == id) {
                        return Err(format!(
                            "There is no collection {id}. list_library lists the collections."
                        ));
                    }
                }
                let results: Vec<Value> = args
                    .assets
                    .iter()
                    .map(|item| {
                        add_one(host, &args.project, item, args.tags.as_ref()).unwrap_or_else(
                            |why| json!({ "asset": item.asset, "ok": false, "error": why }),
                        )
                    })
                    .collect();
                let ok: Vec<String> = results
                    .iter()
                    .filter(|r| r["ok"] == json!(true))
                    .filter_map(|r| r["entry"].as_str().map(str::to_string))
                    .collect();
                if let Some(id) = &args.collection
                    && !ok.is_empty()
                {
                    let change = CollectionChange::Add {
                        collection: id.clone(),
                        entries: ok.clone(),
                    };
                    batch::collection(host.session(), &change).map_err(|e| e.message)?;
                }
                if !ok.is_empty() {
                    host.library_changed();
                }
                Ok(json!({
                    "added": ok.len(),
                    "failed": results.len() - ok.len(),
                    "results": results,
                    "collection": args.collection,
                }))
            })
            .await;
        respond(found)
    }
}
