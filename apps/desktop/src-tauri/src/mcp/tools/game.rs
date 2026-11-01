//! Tools for game projects: read the game settings and budgets, scan and
//! import the game's own models, change the settings and see what Ship
//! would change. Ship itself still waits for a person in the app.

use maquette_core::app_state::RepoSetBy;
use maquette_core::game::{
    self, FolderBudget, GameExport, GameFormat, GameSettingsChange, GameTemplateId,
};
use maquette_core::project;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::game as commands;
use crate::commands::project::{checked_repo, set_repo};

use super::{McpTools, Out, ProjectArgs, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportGameArgs {
    /// The project id of a game project.
    pub project: String,
    /// Model files to import, relative to the model folder, as scan_game_models lists them
    /// (for example "props/brazier.glb"). Files already imported are skipped.
    pub files: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BudgetEdit {
    /// The folder inside the model folder, for example "props". "" is the model folder itself.
    pub folder: String,
    /// The most triangles one model in that folder may have, at least 100.
    pub triangles: u32,
    /// The largest texture edge in px, at least 64.
    pub texture: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetGameArgs {
    /// The project id of a game project.
    pub project: String,
    /// Which templates to preview against this game: template ids, or "all". Left out, each
    /// template is listed with its id, name and numbers only.
    pub previews: Option<Vec<String>>,
}

/// The ids of the built-in templates as `set_game_settings` takes them.
fn template_ids() -> Vec<String> {
    game::game_templates()
        .iter()
        .map(|t| to_value(t.id).as_str().unwrap_or_default().to_string())
        .collect()
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShipPlanArgs {
    /// The project id.
    pub project: String,
    /// A game project only: plan against this repo instead of the project's (an absolute path
    /// to a git repo or worktree, for example an agent's worktree). Nothing is saved.
    pub repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetGameSettingsArgs {
    /// The project id of a game project.
    pub project: String,
    /// The game repo on this computer: an absolute path to a folder that is a git repo or a
    /// git worktree. Saved for this computer only, as picking it in the app does.
    pub repo: Option<String>,
    /// "glb" or "obj": the format of models made in Maquette. Imported models keep theirs.
    pub format: Option<String>,
    /// The model folder inside the game repo, for example "assets/models".
    pub models_dir: Option<String>,
    /// "png", "jpg" or "keep": the texture format Game export writes.
    pub textures: Option<String>,
    /// The longest texture edge Game export writes for models in a folder with no budget:
    /// 256, 512, 1024, 2048 or 4096 px. A folder's budget texture size wins for its models.
    pub texture_cap: Option<u32>,
    /// true leaves normal and metal-roughness maps out of exported models, for a game whose
    /// shader reads neither; false (the default) keeps them.
    pub drop_maps: Option<bool>,
    /// Whether the pipeline runs Bake light (off by default for games).
    pub bake: Option<bool>,
    /// The whole budget table, one row per folder. Leave it out to keep it.
    pub budgets: Option<Vec<BudgetEdit>>,
    /// Per asset (id as list_assets returns it), the folder its model goes to inside the
    /// model folder, for example "kit" or "enemies/bosses": relative, no "..". null or "" puts
    /// it back where its kind or imported file puts it. The next Ship writes it there and
    /// removes its old file when Maquette wrote or imported that file.
    pub asset_folders: Option<std::collections::BTreeMap<String, Option<String>>>,
    /// A template id from get_game (prop-set, character, level-kit, level-kit-obj, web-game,
    /// godot, unity, unreal, mobile, vr-standalone or game-jam): fills the format, the texture
    /// format, size cap and map setting it names, and its folder budgets, keeping other rows
    /// and every existing file. get_game with previews says what each would change. Refused
    /// together with any other setting except repo.
    pub template: Option<String>,
}

fn word<T: serde::de::DeserializeOwned>(what: &str, value: &str) -> Result<T, String> {
    serde_json::from_value(json!(value.trim().to_ascii_lowercase()))
        .map_err(|_| format!("{value:?} is not a {what}."))
}

impl SetGameSettingsArgs {
    /// The template to apply, refused alongside the settings it would race.
    fn template(&self) -> Result<Option<GameTemplateId>, String> {
        let Some(name) = self.template.as_deref() else {
            return Ok(None);
        };
        let others = self.format.is_some()
            || self.models_dir.is_some()
            || self.textures.is_some()
            || self.texture_cap.is_some()
            || self.drop_maps.is_some()
            || self.bake.is_some()
            || self.budgets.is_some()
            || self.asset_folders.is_some();
        if others {
            return Err("Apply a template on its own, then change other settings.".to_string());
        }
        word(
            &format!("game template ({})", template_ids().join(", ")),
            name,
        )
        .map(Some)
    }

    fn change(&self, current: &GameExport) -> Result<GameSettingsChange, String> {
        let format = self
            .format
            .as_deref()
            .map(|f| word::<GameFormat>("model format (glb or obj)", f))
            .transpose()?;
        let textures = self
            .textures
            .as_deref()
            .map(|t| word("texture format (png, jpg or keep)", t))
            .transpose()?;
        let touched = textures.is_some() || self.texture_cap.is_some() || self.drop_maps.is_some();
        let export = touched.then(|| GameExport {
            textures: textures.unwrap_or(current.textures),
            texture_cap: self.texture_cap.unwrap_or(current.texture_cap),
            drop_maps: self.drop_maps.unwrap_or(current.drop_maps),
        });
        Ok(GameSettingsChange {
            format,
            models_dir: self.models_dir.clone(),
            export,
            bake: self.bake,
            asset_folders: self.asset_folders.clone(),
            budgets: self.budgets.as_ref().map(|rows| {
                rows.iter()
                    .map(|b| FolderBudget {
                        folder: b.folder.trim().trim_matches('/').to_string(),
                        triangles: b.triangles,
                        texture: b.texture,
                    })
                    .collect()
            }),
        })
    }
}

#[tool_router(router = game_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get a game project's settings: the game repo, the model format, the model folder, the Game export settings (texture format, size cap and whether normal and metal-roughness maps are dropped), whether Bake light runs, the budget per folder, per asset its file in the model folder (with the folder set for it, if any) and its numbers against its folder's budget, and templates: every built-in game template as {template: {id, name, detail, why, group, format, textures, textureCap, dropMaps, budgets}}. Without previews that is all. Pass previews (template ids, or \"all\") to add to those entries what the template would change in this game: formatBefore and formatAfter, exportBefore and exportAfter, the budget rows that differ, kept files, new paths, models over budget before and after, notes and changes. Before the template list grew, every call returned every preview; the entries still hold the same fields, now only for the templates you ask for. A website project answers that it is not a game.",
        annotations(read_only_hint = true)
    )]
    async fn get_game(&self, Parameters(args): Parameters<GetGameArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let manifest = project::load(&root, &config.layout, &args.project)?;
                let settings = game::settings_of(&manifest)?;
                let dir = project::dir(&root, &config.layout, &args.project)?;
                let wanted = args.previews.unwrap_or_default();
                let ids = template_ids();
                if let Some(bad) = wanted.iter().find(|w| *w != "all" && !ids.contains(w)) {
                    return Err(format!(
                        "{bad:?} is not a game template ({}). Use one of those or \"all\".",
                        ids.join(", ")
                    )
                    .into());
                }
                let all = wanted.iter().any(|w| w == "all");
                let templates = game::game_templates()
                    .into_iter()
                    .zip(ids)
                    .map(|(t, id)| {
                        if all || wanted.contains(&id) {
                            game::preview_template(&dir, &manifest, t.id).map(|p| to_value(&p))
                        } else {
                            Ok(json!({ "template": to_value(&t) }))
                        }
                    })
                    .collect::<Result<Vec<Value>, _>>()?;
                let assets: Vec<Value> = manifest
                    .assets
                    .iter()
                    .map(|a| {
                        json!({
                            "id": a.id,
                            "name": a.name,
                            "file": game::file_of(settings, a),
                            "folder": settings.folders.get(&a.id),
                            "budget": game::check_asset(settings, a).map(|c| to_value(&c)),
                        })
                    })
                    .collect();
                Ok(json!({
                    "project": manifest.id,
                    "target": to_value(manifest.target),
                    "gameRepo": manifest.site_repo,
                    "settings": {
                        "format": to_value(settings.format),
                        "modelsDir": settings.models_dir,
                        "export": to_value(settings.export),
                        "bake": settings.bake,
                        "budgets": to_value(&settings.budgets),
                    },
                    "assets": assets,
                    "templates": templates,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Scan a game project's model folder in its game repo: every .glb and .obj with its folder, format, triangles, textures, size on disk, its folder's budget check and the asset it was imported as, if any. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn scan_game_models(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let scan = commands::scan(host.session(), &args.project)?;
                Ok(to_value(&scan))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Import models from a game project's model folder: each file becomes an asset whose version 1 is the repo file, and joins the ship record so a later Ship may replace it. Over budget models import as they are, and each new model gets a preview rendered headless in the background. Files left out stay hand made and Ship never touches them. Nothing in the game repo changes. Returns the imported asset ids and the skipped files with why."
    )]
    async fn import_game_models(&self, Parameters(args): Parameters<ImportGameArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let tools = host.toolbox();
                let result = commands::import(
                    host.session(),
                    &args.project,
                    &args.files,
                    host.thumbs(),
                    &tools,
                )?;
                host.project_changed(&result.project);
                host.pipeline_changed(&args.project);
                Ok(json!({
                    "project": result.project.id,
                    "imported": result.imported,
                    "skipped": to_value(&result.skipped),
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Change a game project's settings: repo (the game repo on this computer, an absolute path to a git repo or worktree, saved per computer), format, modelsDir, textures, textureCap, dropMaps (true leaves normal and metal-roughness maps out of exported models; false, the default, keeps them), bake, assetFolders (per asset id, its own folder inside the model folder such as kit, or null to put it back; the next Ship moves the file and removes the old one it owns) the budgets table (each field left out keeps what is saved) and template (an id from get_game: prop-set, character, level-kit, level-kit-obj, web-game, godot, unity, unreal, mobile, vr-standalone or game-jam, alone or with repo: fills the format, the texture format, size cap and map dropping the template names (the four shape templates leave those alone) and its folder budgets, keeps other rows, adds no assets, and models already shipped keep their path). The pipeline follows: Bake light goes in or out and Export uses the new texture settings on the next run. Refused, changing nothing, when a value is out of range. Returns the settings like get_game."
    )]
    async fn set_game_settings(&self, Parameters(args): Parameters<SetGameSettingsArgs>) -> Out {
        let project_id = args.project.clone();
        let changed = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let manifest = project::load(&root, &config.layout, &args.project)?;
                let current = game::settings_of(&manifest)?;
                let change = args.change(&current.export)?;
                let template = args.template()?;
                if let Some(repo) = args.repo.as_deref() {
                    let path = checked_repo(host.session(), repo)?;
                    let path = Some(path.display().to_string());
                    set_repo(
                        host.session(),
                        host.runner(),
                        &args.project,
                        manifest.name,
                        path,
                        RepoSetBy::Agent,
                    )?;
                }
                let saved = match template {
                    Some(id) => commands::apply_template(host.session(), &args.project, id),
                    None => commands::save_settings(host.session(), &args.project, change),
                }?;
                host.project_changed(&saved);
                host.pipeline_changed(&args.project);
                Ok(())
            })
            .await;
        if let Err(message) = changed {
            return respond(Err(message));
        }
        self.get_game(Parameters(GetGameArgs {
            project: project_id,
            previews: None,
        }))
        .await
    }

    #[tool(
        description = "Show what Ship would write. For a website: the same plan get_ship_settings returns (each file new, replaced, unchanged or hand made, whether it ships or why it is held back, sizes and triangles before and after, the site files left alone or removed). For a game: what it would write into the model folder from the last Game export: each file as new, replaced (Maquette wrote or imported it before; imported is true when the game's file is still the imported original), unchanged or hand made (not in the ship record, so Ship leaves it alone), with sizes and triangles before and after, the branch, the ship record size after the run, the checks (budgets, no KTX2 or meshopt) and uncommitted, how many files git status lists in the game repo. Files of assets moved to another folder (set_game_settings assetFolders) show as removed. repo plans a game against another git repo or worktree for this look only. Reads only; Ship runs from run_pipeline (which takes the same repo for one run) and waits for a person in the app.",
        annotations(read_only_hint = true)
    )]
    async fn get_ship_plan(&self, Parameters(args): Parameters<ShipPlanArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let manifest = project::load(&root, &config.layout, &args.project)?;
                if game::settings_of(&manifest).is_err() {
                    let view = crate::commands::ship::view(host.session(), &args.project)?;
                    return Ok(to_value(&view.plan));
                }
                let plan =
                    commands::ship_plan(host.session(), &args.project, args.repo.as_deref())?;
                Ok(to_value(&plan))
            })
            .await;
        respond(found)
    }
}
