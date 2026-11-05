//! The recipe library for agents: listing and reading library recipes with
//! their revisions, using one in a project, proposing a save back (which
//! only raises an approval: the person saves in the app), and moving recipe
//! files in and out of the workspace. Agents import only files inside the
//! workspace and export only new files under `exports/recipes`, never
//! anywhere else. Nothing here bakes, runs Blender or spends anything.

use maquette_core::animations::recipes::library::{
    self, AGENT_EXPORTS, ExportSource, LibraryRecipeDoc, RecipeUsed, UseRecipe,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::hints::{step, with_next};
use super::recipes::{change_view, recipe_view};
use super::{McpTools, Out, ToolError, WAITING, respond, to_value, workspace};
use crate::commands::{recipe_library as commands, recipes};

/// Where a person saves a recipe an agent proposed.
pub(super) const SAVE_IN_APP: &str = "The approval prompt in the Maquette app: Review opens the save to library sheet in the Motion view's Recipes tab, where the person reads what changes and saves. Also listed under Settings > Agents. No tool can approve.";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecipeArgs {
    /// The library recipe's id, as list_library_recipes returns it.
    pub entry: String,
    /// A revision number; the newest when left out.
    pub revision: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UseRecipeArgs {
    /// The project it is copied into.
    pub project: String,
    /// The library recipe's id.
    pub entry: String,
    /// A revision number; the newest when left out.
    pub revision: Option<u32>,
    /// The project recipe's name; the library recipe's when left out.
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProposeSaveArgs {
    /// The project id.
    pub project: String,
    /// The project's recipe id.
    pub recipe: String,
    /// The library recipe to add a revision to. Left out, the one the
    /// recipe came from (when it is still in the library), else a new one.
    pub entry: Option<String>,
    /// True proposes a new library recipe even when the recipe came from one.
    #[serde(default)]
    pub new_entry: bool,
}

/// An agent asked to import a recipe file into the library, which only a
/// person does.
pub(super) const LIBRARY_IMPORT: &str = "library-import";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportRecipeArgs {
    /// The .recipe.json to import: a path relative to the workspace root, or
    /// an absolute path inside the workspace. Files elsewhere are refused.
    pub path: String,
    /// The project to import it into as a new recipe. Agents never import
    /// into the recipe library: import into a project, then
    /// propose_recipe_save, and the person saves it there.
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExportRecipeArgs {
    /// A project's recipe: the project id, with recipe.
    pub project: Option<String>,
    pub recipe: Option<String>,
    /// Or a library recipe, by id, with revision (the newest when left out).
    pub entry: Option<String>,
    pub revision: Option<u32>,
    /// The file's name, without a folder; the recipe's name when left out.
    /// It lands in exports/recipes in the workspace, never over a file
    /// already there (a -2 is added).
    pub file: Option<String>,
}

/// A library recipe's revision as agents read it.
fn library_view(doc: &LibraryRecipeDoc) -> Value {
    let recipe = &doc.recipe;
    let mut base = json!({
        "name": recipe.base.name, "frames": recipe.base.frames, "hash": recipe.base.hash,
        "library": to_value(doc.base.status),
    });
    if let Some(clip) = &doc.base.clip {
        base["clip"] = json!(clip);
    }
    if let Some(warning) = &doc.base.warning {
        base["warning"] = json!(warning);
    }
    json!({
        "entry": doc.entry.id, "name": doc.entry.name, "version": doc.version,
        "revision": doc.revision, "revisions": to_value(&doc.entry.revisions),
        "recipe": {
            "name": recipe.name, "mixamo": doc.mixamo, "base": base,
            "blanks": to_value(&recipe.blanks),
            "changes": recipe.layers.iter().map(change_view).collect::<Vec<_>>(),
        },
    })
}

/// A recipe brought into a project, with its notes.
fn used_view(used: &RecipeUsed) -> Value {
    let mut out = recipe_view(&used.doc);
    if !used.notes.is_empty() {
        out["notes"] = json!(used.notes);
    }
    let mut next = vec![step("get_recipe", "Read it back.")];
    if used.doc.recipe.blanks.iter().any(|b| !b.filled()) {
        next.insert(
            0,
            step(
                "update_recipe",
                "Fill the open blanks with this project's props or points.",
            ),
        );
    }
    with_next(out, next)
}

#[tool_router(router = recipe_library_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List the recipe library: recipes kept outside any project (library/recipes at the workspace root), each with its id, name, newest revision, every revision with where it came from (the project and recipe it was saved from, or the file it was imported from, and when), the newest revision's base clip, frames, Mixamo flag and change and blank counts, and its version. An entry that cannot be read lists with its problem. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn list_library_recipes(&self) -> Out {
        let found = self
            .blocking(|host, _| {
                let listed = commands::list(host.session())?;
                let out = json!({ "recipes": to_value(&listed) });
                Ok(if listed.is_empty() {
                    with_next(
                        out,
                        vec![step(
                            "propose_recipe_save",
                            "Propose saving a project's recipe to the library; a person saves it.",
                        )],
                    )
                } else {
                    with_next(out, vec![step("use_recipe", "Copy one into a project.")])
                })
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read a library recipe: its name, version, every revision with where it came from, and one revision (the newest unless revision is given) in full: name, base clip (name, frames, hash, and library: found, hash-differs or missing in this computer's clip library), mixamo, blanks and changes. A revision is never changed once saved. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn get_library_recipe(&self, Parameters(args): Parameters<LibraryRecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let doc = commands::get(host.session(), &args.entry, args.revision)?;
                Ok(with_next(
                    library_view(&doc),
                    vec![step("use_recipe", "Copy this revision into a project.")],
                ))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Copy a library recipe's revision (the newest by default) into a project as a new recipe, as the Animations library's Use in project does. It remembers the library recipe and revision, so get_recipe can say when the library has a newer one. Point blanks keep their points; a prop blank whose asset the project does not have opens, and notes says which. The base clip is matched in the clip library by name and frame count: missing, the recipe is still made and says so; another hash warns. Writes one file, spends nothing. Website and game projects both take recipes."
    )]
    async fn use_recipe(&self, Parameters(args): Parameters<UseRecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let request = UseRecipe {
                    project: args.project,
                    entry: args.entry,
                    revision: args.revision,
                    name: args.name,
                };
                let used = commands::use_in(host.session(), &request)?;
                host.recipe_changed(&request.project);
                Ok(used_view(&used))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Propose saving a project's recipe to the recipe library: as the next revision of the library recipe it came from (or of entry), or as a new library recipe (newEntry, or when it came from none). Nothing is saved by this tool: it answers status waiting for approval with the approval id and what would change against the newest revision, in plain lines, and the person reads that in the app's save to library sheet and saves there. A save always adds a revision, never overwrites one. A recipe that holds what the newest revision holds is refused. Asking twice returns the same approval. Poll get_recipe: once saved, its library field names the revision. Spends nothing."
    )]
    async fn propose_recipe_save(&self, Parameters(args): Parameters<ProposeSaveArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                let doc = recipes::get(host.session(), &args.project, &args.recipe)?;
                let came_from = match &doc.recipe.from {
                    Some(from) => commands::linked(host.session(), from)?.map(|entry| entry.id),
                    None => None,
                };
                let entry = if args.new_entry { None } else { args.entry.or(came_from) };
                let plan =
                    commands::plan_save(host.session(), &args.project, &args.recipe, entry.as_deref())?;
                if plan.same {
                    return Err(ToolError::from(format!(
                        "{} holds what r{} of {} holds already, so there is nothing to save.",
                        plan.recipe_name,
                        plan.latest.unwrap_or_default(),
                        plan.entry_name
                    )));
                }
                let approval = hub.request_recipe_save(&plan);
                let target = match &plan.entry {
                    Some(id) => format!("r{} of the library recipe {id}", plan.revision),
                    None => format!("a new library recipe, {}", plan.entry_name),
                };
                let answer = json!({
                    "status": WAITING,
                    "approval": approval,
                    "whereToApprove": SAVE_IN_APP,
                    "message": format!("Saving {} as {target} waits for a person to read what changes and save it in Maquette. Nothing is saved until then.", plan.recipe_name),
                    "plan": to_value(&plan),
                });
                Ok(with_next(
                    answer,
                    vec![step("get_recipe", "Follow the save: its library field names the revision once the person saves.")],
                ))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Import a .recipe.json into a project as a new recipe. Agents never import into the recipe library: import into a project, then propose_recipe_save, and the person saves it. path is relative to the workspace root or absolute inside it; files outside the workspace are refused (the person imports those from the app's file picker). The file is checked whole before anything is written: at most 256 KB (checked before it is read), version 1, at most 32 changes and 16 blanks, and every change and blank check. A taken id gets a fresh one; nothing in the file becomes a path. The base clip is matched by name and frame count; another hash warns, a missing one is noted. Prop blanks whose asset the project lacks open. Answers with notes. Spends nothing."
    )]
    async fn import_recipe(&self, Parameters(args): Parameters<ImportRecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let Some(project) = args.project else {
                    return Err(ToolError {
                        message: "Agents import a recipe file into a project only. To add it to the recipe library, import it into a project, then propose_recipe_save: the person saves it there.".into(),
                        code: LIBRARY_IMPORT.into(),
                    });
                };
                let (root, _) = workspace(host)?;
                let file = library::agent_import_path(&root, &args.path)?;
                let used = commands::import_to_project(host.session(), &project, &file)?;
                host.recipe_changed(&project);
                Ok(used_view(&used))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Export a recipe to one .recipe.json: a project's recipe (project and recipe) or a library recipe's revision (entry, revision the newest by default). The file holds only the base clip's name, frames, length, hash and Mixamo flag, the blanks and the changes, never keys, a mesh or the library link, so it can be shared. Agents write only new files under exports/recipes in the workspace (file names the file; a taken name gets -2); the person can export anywhere from the app. Answers with the path relative to the workspace. Spends nothing."
    )]
    async fn export_recipe(&self, Parameters(args): Parameters<ExportRecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, _) = workspace(host)?;
                let source = ExportSource {
                    project: args.project,
                    recipe: args.recipe,
                    entry: args.entry,
                    revision: args.revision,
                };
                let (_, config) = workspace(host)?;
                let recipe = library::source_recipe(&root, &config.layout, &source)?;
                let name = args.file.unwrap_or_else(|| recipe.name.clone());
                let done = library::agent_export(&root, &config.layout, &source, &name)?;
                Ok(json!({
                    "path": format!("{}/{}", AGENT_EXPORTS.join("/"), done.file),
                    "file": done.file,
                }))
            })
            .await;
        respond(found)
    }
}
