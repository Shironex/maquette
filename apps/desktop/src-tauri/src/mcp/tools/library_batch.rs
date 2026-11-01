//! Tools for the library at scale: tags on many entries, renames,
//! collections, removing and restoring, look-alikes and merges. They only
//! change the library (and, for a merge or its undo, which entry project
//! assets point at); none of it spends credits.

use std::collections::HashMap;

use maquette_core::library::{
    self, CollectionChange, LibraryCollection, LookAlikes, MergeEntries, TagEntries,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::library_batch as commands;

use super::{McpTools, Out, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagEntriesArgs {
    /// Library entry ids, as list_library returns them.
    pub entries: Vec<String>,
    /// Tags to put on every entry.
    #[serde(default)]
    pub add: Vec<String>,
    /// Tags to take off every entry.
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameEntryArgs {
    /// The library entry id.
    pub entry: String,
    /// The new name.
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectionArgs {
    /// create, add, remove, rename or delete.
    pub op: String,
    /// The collection id, for every op but create.
    pub collection: Option<String>,
    /// The name, for create and rename.
    pub name: Option<String>,
    /// Library entry ids, for create, add and remove.
    #[serde(default)]
    pub entries: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntriesArgs {
    /// Library entry ids.
    pub entries: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MergeArgs {
    /// The entry that stays.
    pub keep: String,
    /// The entry that goes into it.
    pub merge: String,
    /// Whether the merged entry's tags join the kept entry's. Defaults to the saved setting.
    pub copy_tags: Option<bool>,
}

/// The change `args` asks for, or what is missing.
fn change_of(args: CollectionArgs) -> Result<CollectionChange, String> {
    let collection = || {
        args.collection
            .clone()
            .ok_or_else(|| format!("Give the collection id for {}.", args.op))
    };
    let name = || {
        args.name
            .clone()
            .ok_or_else(|| format!("Give a name for {}.", args.op))
    };
    Ok(match args.op.as_str() {
        "create" => CollectionChange::Create {
            name: name()?,
            entries: args.entries.clone(),
        },
        "add" => CollectionChange::Add {
            collection: collection()?,
            entries: args.entries.clone(),
        },
        "remove" => CollectionChange::Remove {
            collection: collection()?,
            entries: args.entries.clone(),
        },
        "rename" => CollectionChange::Rename {
            collection: collection()?,
            name: name()?,
        },
        "delete" => CollectionChange::Delete {
            collection: collection()?,
        },
        other => {
            return Err(format!(
                "{other} is not an op. Use create, add, remove, rename or delete."
            ));
        }
    })
}

fn listed(collections: &[LibraryCollection]) -> Value {
    json!({
        "collections": collections.iter().map(|c| json!({
            "id": c.id,
            "name": c.name,
            "entries": c.entries,
        })).collect::<Vec<_>>(),
    })
}

/// The look-alikes as agents read them, each entry with its name.
fn found(alikes: &LookAlikes, names: &HashMap<String, String>) -> Value {
    let name = |id: &String| names.get(id).cloned();
    json!({
        "pairs": alikes.pairs.iter().map(|p| json!({
            "kind": to_value(p.kind),
            "a": p.a,
            "aName": name(&p.a),
            "b": p.b,
            "bName": name(&p.b),
            "score": p.score,
            "shape": p.shape,
            "colour": p.colour,
            "ratio": p.ratio,
        })).collect::<Vec<_>>(),
        "unseen": alikes.unseen,
        "merges": to_value(&alikes.merges),
        "settings": to_value(&alikes.settings),
        "tookMs": alikes.took_ms,
    })
}

#[tool_router(router = library_batch_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Put tags on and take tags off several library entries at once (entries are entry ids; add and remove are tags, lowercased). Every entry is checked first: an unknown entry, or one that would pass 16 tags, changes nothing. Answers each entry with its tags. Spends nothing."
    )]
    async fn tag_library_entries(&self, Parameters(args): Parameters<TagEntriesArgs>) -> Out {
        let done = self
            .blocking(move |host, _| {
                let request = TagEntries {
                    entries: args.entries,
                    add: args.add,
                    remove: args.remove,
                };
                let entries = commands::tag(host.session(), &request)?;
                host.library_changed();
                Ok(json!({
                    "entries": entries.iter().map(|e| json!({ "id": e.id, "name": e.name, "tags": e.tags })).collect::<Vec<_>>(),
                }))
            })
            .await;
        respond(done)
    }

    #[tool(
        description = "Rename a library entry. Its id (its folder) stays, so the projects that came from it still find it. Spends nothing."
    )]
    async fn rename_library_entry(&self, Parameters(args): Parameters<RenameEntryArgs>) -> Out {
        let done = self
            .blocking(move |host, _| {
                let entry = commands::rename(host.session(), &args.entry, &args.name)?;
                host.library_changed();
                Ok(json!({ "id": entry.id, "name": entry.name }))
            })
            .await;
        respond(done)
    }

    #[tool(
        description = "Change the library's collections: named groups of entries (an entry can sit in several). op create makes one from name (and entries), add and remove put entries in and take them out of collection, rename gives collection a new name, delete removes the collection only and never a model. Two collections cannot share a name. Answers every collection with its entry ids. Spends nothing."
    )]
    async fn set_library_collection(&self, Parameters(args): Parameters<CollectionArgs>) -> Out {
        let done = self
            .blocking(move |host, _| {
                let change = change_of(args)?;
                let all = commands::collection(host.session(), &change)?;
                host.library_changed();
                Ok(listed(&all))
            })
            .await;
        respond(done)
    }

    #[tool(
        description = "Remove entries from the library. Their folders move to library/.removed, so restore_to_library brings them back; the copies in projects stay as they are, and collections keep the ids for a restore. An unknown entry removes nothing. Spends nothing."
    )]
    async fn remove_from_library(&self, Parameters(args): Parameters<EntriesArgs>) -> Out {
        let done = self
            .blocking(move |host, _| {
                let removed = commands::remove(host.session(), &args.entries)?;
                host.library_changed();
                Ok(json!({ "removed": removed }))
            })
            .await;
        respond(done)
    }

    #[tool(
        description = "Put removed library entries back. An entry that merge_library_entries merged is put back by undoing the merge: the versions it added to the kept entry go, the kept entry's tags and refs come back and the project assets it moved point back at it (refused, with the reason, once the kept entry has newer versions or a project uses one the merge added). Answers the entries restored and the projects that changed. Spends nothing."
    )]
    async fn restore_to_library(&self, Parameters(args): Parameters<EntriesArgs>) -> Out {
        let done = self
            .blocking(move |host, _| {
                let back = commands::restore(host.session(), &args.entries)?;
                for project in &back.projects {
                    host.project_changed(project);
                }
                host.library_changed();
                Ok(json!({
                    "restored": back.entries,
                    "projects": back.projects.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
                }))
            })
            .await;
        respond(done)
    }

    #[tool(
        description = "Find library entries that may be one model twice: same-look pairs, whose pictures score at or over the threshold on outline, colours and proportions, and same-name pairs (names equal once a trailing number is dropped). Each pair names the older entry as a, with the scores when both pictures read. Pairs a person decided in the app are left out. Also answers the entries with no readable picture (unseen), the recent merges (each undone with restore_to_library on its merged entry) and the settings. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn find_look_alikes(&self) -> Out {
        let done = self
            .blocking(move |host, _| {
                let alikes = commands::look_alikes(host.session())?;
                let (root, _) = workspace(host)?;
                let names = library::list(&root)?
                    .into_iter()
                    .map(|e| (e.id, e.name))
                    .collect();
                Ok(found(&alikes, &names))
            })
            .await;
        respond(done)
    }

    #[tool(
        description = "Merge one library entry into another: every version of merge the kept entry lacks (by file) becomes keep's next version, its refs come along and, with copyTags (the saved setting by default), its tags. Project assets that came from merge point at keep's matching version, so every project keeps the model it uses. The merged entry then waits in library/.removed; restore_to_library on it undoes the merge. Spends nothing."
    )]
    async fn merge_library_entries(&self, Parameters(args): Parameters<MergeArgs>) -> Out {
        let done = self
            .blocking(move |host, _| {
                let copy_tags = match args.copy_tags {
                    Some(copy) => copy,
                    None => commands::look_settings(host.session())?.copy_tags,
                };
                let request = MergeEntries {
                    keep: args.keep,
                    merge: args.merge,
                    copy_tags,
                };
                let merged = commands::merge(host.session(), &request)?;
                for project in &merged.projects {
                    host.project_changed(project);
                }
                host.library_changed();
                Ok(json!({
                    "entry": merged.entry.id,
                    "name": merged.entry.name,
                    "versions": merged.entry.versions.len(),
                    "tags": merged.entry.tags,
                    "merge": to_value(&merged.merge),
                    "projects": merged.projects.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
                }))
            })
            .await;
        respond(done)
    }
}
