//! Tools for the library: models kept outside any project. Adding copies a
//! Board asset in, using copies an entry out as a new asset, and updating
//! brings a newer library version into an asset that came from it. The
//! library copies, never links, and none of it spends credits.

use maquette_core::library::{AddToLibrary, LibraryItem, UpdatePlan};
use maquette_core::project;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::library as commands;

use super::materials::find_asset;
use super::{McpTools, Out, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListLibraryArgs {
    /// Words to look for in the names, tags and the projects that use an entry.
    pub query: Option<String>,
    /// Only entries with this tag.
    pub tag: Option<String>,
    /// A project id: also list its assets that have a newer library version waiting.
    pub project: Option<String>,
    /// How many entries one call returns: 25 by default, 100 at most.
    pub limit: Option<u32>,
    /// The nextCursor of the previous call, to read the next page. Opaque: pass it back as is.
    pub cursor: Option<String>,
    /// true lists every entry in full (versions, sizes, credits and where each is used). By
    /// default each entry is compact: id, name, kind, tags, newest version, version count and
    /// use count.
    #[serde(default)]
    pub full: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddToLibraryArgs {
    /// The project id.
    pub project: String,
    /// The asset id or name, as list_assets returns it.
    pub asset: String,
    /// The entry's name. Defaults to the asset's.
    pub name: Option<String>,
    /// The entry's tags, which replace the ones it has. Left out, they stay.
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UseFromLibraryArgs {
    /// The library entry id, as list_library returns it.
    pub entry: String,
    /// The project to copy it into.
    pub project: String,
    /// The library version. Defaults to the newest.
    pub version: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFromLibraryArgs {
    /// The project id.
    pub project: String,
    /// The asset id or name.
    pub asset: String,
    /// Only say what the update would keep and drop. Defaults to false.
    #[serde(default)]
    pub dry_run: bool,
}

fn matches(item: &LibraryItem, query: Option<&str>, tag: Option<&str>) -> bool {
    let entry = &item.entry;
    let tagged = tag.is_none_or(|t| entry.tags.iter().any(|e| e == &t.trim().to_lowercase()));
    let words = query.map(str::to_lowercase).unwrap_or_default();
    let found = words.split_whitespace().all(|w| {
        entry.name.to_lowercase().contains(w)
            || entry.tags.iter().any(|t| t.contains(w))
            || item
                .uses
                .iter()
                .any(|u| u.project_name.to_lowercase().contains(w))
    });
    tagged && found
}

/// The most entries one list_library call returns.
const MAX_LIMIT: u32 = 100;
/// How many it returns when limit is left out.
const DEFAULT_LIMIT: u32 = 25;

/// One entry in the short form: no sizes, credits or per project uses.
fn compact(item: &LibraryItem) -> Value {
    let entry = &item.entry;
    json!({
        "id": entry.id,
        "name": entry.name,
        "kind": to_value(entry.kind),
        "tags": entry.tags,
        "latest": entry.latest().map(|v| v.version),
        "versions": entry.versions.len(),
        "uses": item.uses.len(),
    })
}

/// The page of `matching` that follows `cursor` (an entry id), at most
/// `limit` long, and the cursor of the page after it.
fn page<'a>(
    matching: &[&'a LibraryItem],
    cursor: Option<&str>,
    limit: u32,
) -> Result<(Vec<&'a LibraryItem>, Option<String>), String> {
    let start = match cursor.map(str::trim).filter(|c| !c.is_empty()) {
        None => 0,
        Some(after) => matching
            .iter()
            .position(|i| i.entry.id == after)
            .map(|at| at + 1)
            .ok_or("That cursor does not match an entry any more; list again from the start.")?,
    };
    let limit = limit.clamp(1, MAX_LIMIT) as usize;
    let shown: Vec<&LibraryItem> = matching.iter().skip(start).take(limit).copied().collect();
    let next = (start + shown.len() < matching.len())
        .then(|| shown.last().map(|i| i.entry.id.clone()))
        .flatten();
    Ok((shown, next))
}

/// One entry as agents read it.
fn listed(item: &LibraryItem) -> Value {
    let entry = &item.entry;
    let latest = entry.latest();
    json!({
        "id": entry.id,
        "name": entry.name,
        "kind": to_value(entry.kind),
        "tags": entry.tags,
        "versions": entry.versions.len(),
        "latest": latest.map(|v| v.version),
        "source": latest.map(|v| to_value(v.source)),
        "format": item.format.map(to_value),
        "triangles": latest.and_then(|v| v.stats.as_ref()).map(|s| s.triangles),
        "bytes": latest.and_then(|v| v.stats.as_ref()).map(|s| s.bytes),
        "creditsSpent": item.spent,
        "creditsSaved": item.saved,
        "usedIn": item.uses.iter().map(|u| json!({
            "project": u.project,
            "asset": u.asset,
            "version": u.version,
            "updateAvailable": u.update,
        })).collect::<Vec<_>>(),
    })
}

fn planned(plan: &UpdatePlan) -> Value {
    json!({
        "asset": plan.asset,
        "entry": plan.entry,
        "from": plan.from,
        "to": plan.to,
        "becomesVersion": plan.next_version,
        "kept": plan.kept,
        "dropped": plan.dropped.iter().map(|d| json!({ "setting": d.what, "why": d.why })).collect::<Vec<_>>(),
        "note": plan.note,
    })
}

/// The asset id `wanted` names in `project_id`.
fn asset_id(host: &dyn super::Host, project_id: &str, wanted: &str) -> Result<String, String> {
    let (root, config) = workspace(host)?;
    let manifest = project::load(&root, &config.layout, project_id).map_err(super::reason)?;
    find_asset(&manifest, wanted).map(|a| a.id.clone())
}

#[tool_router(router = library_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List the library: models kept outside any project (in library/ at the workspace root) that any project can copy in. Each entry is compact by default: id, name, kind, tags, newest version, version count and use count. Pass full true for what made the newest (tripo, meshy, by-hand, built or imported), its format, triangles, the credits it cost and the credits reuse saved, and every project asset that came from it with the library version it is on and whether a newer one is waiting. Filter with query and tag; give project to list that project's assets with an update waiting. It returns 25 entries at a time (limit, 100 at most) with total, the number that match; when nextCursor is not null pass it as cursor for the next page. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn list_library(&self, Parameters(args): Parameters<ListLibraryArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = commands::get(host.session()).map_err(|e| e.message)?;
                let matching: Vec<&LibraryItem> = view
                    .items
                    .iter()
                    .filter(|i| matches(i, args.query.as_deref(), args.tag.as_deref()))
                    .collect();
                let (shown, next) = page(
                    &matching,
                    args.cursor.as_deref(),
                    args.limit.unwrap_or(DEFAULT_LIMIT),
                )?;
                let entries: Vec<Value> = shown
                    .into_iter()
                    .map(|i| if args.full { listed(i) } else { compact(i) })
                    .collect();
                let updates: Option<Vec<Value>> = args.project.as_ref().map(|p| {
                    view.items
                        .iter()
                        .flat_map(|i| i.uses.iter().map(move |u| (i, u)))
                        .filter(|(_, u)| &u.project == p && u.update)
                        .map(|(i, u)| {
                            json!({
                                "asset": u.asset,
                                "entry": i.entry.id,
                                "on": u.version,
                                "newest": i.entry.latest().map(|v| v.version),
                            })
                        })
                        .collect()
                });
                Ok(json!({
                    "dir": view.dir,
                    "total": matching.len(),
                    "entries": entries,
                    "nextCursor": next,
                    "updates": updates,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Add a project asset's newest model to the library with its picture, refs, real size, fix-ups and material tweaks. An asset already in the library (it came from an entry, or was added before) gets its newer model as that entry's next version, and other projects that use the entry then show an update waiting; the same model twice adds nothing. The asset remembers the entry. Copies files, spends nothing and needs no approval."
    )]
    async fn add_to_library(&self, Parameters(args): Parameters<AddToLibraryArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let asset = asset_id(host, &args.project, &args.asset)?;
                let request = AddToLibrary {
                    project: args.project.clone(),
                    asset,
                    name: args.name.clone(),
                    tags: args.tags.clone(),
                };
                let added = commands::add(host.session(), host.studio(), &request)
                    .map_err(|e| e.message)?;
                host.project_changed(&added.project);
                host.library_changed();
                Ok(json!({
                    "entry": added.entry.id,
                    "name": added.entry.name,
                    "version": added.version,
                    "added": added.added,
                    "tags": added.entry.tags,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Copy a library entry into a project as a new asset at Blender: the model (newest version unless version is given), its picture and refs, and the real size, fix-ups and material tweaks it had. The asset records which entry and version it came from; the project owns the copy. A website project takes .glb models only. A game project takes .glb or .obj and its next run exports the model in the game's format; the note says so. Spends nothing and needs no approval."
    )]
    async fn use_from_library(&self, Parameters(args): Parameters<UseFromLibraryArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let used = commands::use_entry(host.session(), host.studio(), &args.entry, args.version, &args.project)
                    .map_err(|e| e.message)?;
                host.project_changed(&used.project);
                if used.settings {
                    host.prepare_changed(&args.project);
                    host.materials_changed(&args.project);
                }
                host.library_changed();
                Ok(json!({
                    "project": used.project.id,
                    "asset": { "id": used.asset.id, "name": used.asset.name, "stage": to_value(used.asset.stage) },
                    "origin": used.asset.origin.as_ref().map(to_value),
                    "settings": used.settings,
                    "note": used.note,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Update a project asset that came from the library to the entry's newest version: the new model comes in as the asset's next version, and the older versions stay. The asset's own real size, fix-ups and material tweaks stay, except ones that no longer apply (a decimate to more triangles than the new model has, a texture shrink it does not need, a tweak for a material it lacks), which are dropped with the reason. dryRun only says what would happen. Nothing updates on its own. Spends nothing and needs no approval."
    )]
    async fn update_from_library(
        &self,
        Parameters(args): Parameters<UpdateFromLibraryArgs>,
    ) -> Out {
        let found = self
            .blocking(move |host, _| {
                let asset = asset_id(host, &args.project, &args.asset)?;
                if args.dry_run {
                    let plan = commands::plan(host.session(), &args.project, &asset)
                        .map_err(|e| e.message)?;
                    return Ok(json!({ "dryRun": true, "plan": planned(&plan) }));
                }
                let done = commands::update(host.session(), host.studio(), &args.project, &asset)
                    .map_err(|e| e.message)?;
                host.project_changed(&done.project);
                if !done.plan.dropped.is_empty() {
                    host.prepare_changed(&args.project);
                    host.materials_changed(&args.project);
                }
                host.library_changed();
                Ok(json!({ "updated": true, "plan": planned(&done.plan) }))
            })
            .await;
        respond(found)
    }
}
