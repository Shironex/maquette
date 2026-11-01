//! Tools that read and change how a website project ships: the style
//! (scene or a hand-built site), the texture format, each asset's choice
//! and file name, the site's folders, and the size report.

use std::collections::BTreeMap;
use std::path::Path;

use maquette_core::app_state::RepoSetBy;
use maquette_core::project;
use maquette_core::ship::{AssetShip, ShipChoice, ShipSettingsChange, ShipStyle, TextureFormat};
use maquette_core::site::{self, SiteTarget};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::project::{checked_repo, set_repo};
use crate::commands::ship::{self as commands, ShipSave, ShipView};
use crate::mcp::host::Host;

use super::materials::find_asset;
use super::{McpTools, Out, ProjectArgs, ToolError, explicit, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssetShipEdit {
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// "auto" (ship unless the site's file is not Maquette's, or is smaller for a
    /// hand-built site), "never" (leave the site's file alone) or "always" (replace
    /// whatever the site has).
    pub choice: Option<String>,
    /// The file name the model ships as, without .glb: lowercase letters, digits, _ and -.
    /// null goes back to the asset id (hand-built) or the versioned name (scene).
    #[serde(default, deserialize_with = "explicit")]
    pub file: Option<Option<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetShipArgs {
    /// The project id of a website project.
    pub project: String,
    /// The site repo: a git repo or worktree on this computer, absolute or starting with ~,
    /// outside the workspace, saved per computer. Only null clears it. Refused while the project
    /// has a run going. A game's repo goes through set_game_settings.
    #[serde(default, deserialize_with = "explicit")]
    pub site_repo: Option<Option<String>>,
    /// "scene" (Maquette builds the scene: placed exports, versioned names) or "models"
    /// (a hand-built site: each model at its own origin, one stable file name per asset,
    /// a smaller site file is never replaced unless the asset is set to always).
    pub style: Option<String>,
    /// "ktx2" (needs a KTX2 decoder on the site), "webp" or "avif".
    pub textures: Option<String>,
    /// The WebP and AVIF quality, 1 to 100 (80 by default).
    pub quality: Option<u8>,
    /// Per asset choice and file name. Only the given fields change.
    #[serde(default)]
    pub assets: Vec<AssetShipEdit>,
    /// The site's models folder, relative to the site repo, for example
    /// "apps/landing/public/room/models". Written to the site's maquette.site.json.
    pub models: Option<String>,
    /// The site's posters folder, or null for none. Written with models.
    #[serde(default, deserialize_with = "explicit")]
    pub posters: Option<Option<String>>,
    /// The file Ship writes the scene data to, or null for none. Written with models.
    #[serde(default, deserialize_with = "explicit")]
    pub data: Option<Option<String>>,
}

fn word<T: serde::de::DeserializeOwned>(what: &str, value: &str, valid: &str) -> Result<T, String> {
    serde_json::from_value(json!(value.trim().to_lowercase()))
        .map_err(|_| format!("{what} must be one of {valid}, not {value:?}."))
}

const NOTE: &str = "Ship settings live in the project's ship.json and the site's folders in the site's maquette.site.json. textures and the style apply on the next run_pipeline (Compress and Export GLB run again); choices and file names apply on the next Ship. plan is a dry run of the next Ship from what Compress made last.";

fn view_value(view: &ShipView) -> Value {
    json!({
        "project": view.project,
        "settings": to_value(&view.settings),
        "target": to_value(&view.plan.target),
        "scriptReady": view.plan.script_ready,
        "originReady": view.plan.origin_ready,
        "plan": to_value(&view.plan),
        "note": NOTE,
    })
}

impl SetShipArgs {
    /// `repo` is the repo this call moves to, whose folders the ones left
    /// out then come from.
    fn save(
        &self,
        view: &ShipView,
        manifest: &maquette_core::ProjectManifest,
        repo: Option<&Option<String>>,
    ) -> Result<ShipSave, ToolError> {
        let mut assets = BTreeMap::new();
        for edit in &self.assets {
            let asset = find_asset(manifest, &edit.asset)?;
            let mut entry: AssetShip = view
                .settings
                .assets
                .get(&asset.id)
                .cloned()
                .unwrap_or_default();
            if let Some(choice) = &edit.choice {
                entry.choice = word::<ShipChoice>("choice", choice, "auto, never, always")?;
            }
            if let Some(file) = &edit.file {
                entry.file.clone_from(file);
            }
            assets.insert(asset.id.clone(), entry);
        }
        let target_changes = self.models.is_some() || self.posters.is_some() || self.data.is_some();
        let target = if target_changes {
            let now = &view.plan.target;
            let (models, posters, data) = match repo {
                Some(Some(repo)) => {
                    let found = site::load_config(Path::new(repo))?;
                    (found.models, found.posters, found.data)
                }
                _ => (now.models.clone(), now.posters.clone(), now.data.clone()),
            };
            Some(SiteTarget {
                models: self.models.clone().unwrap_or(models),
                posters: self.posters.clone().unwrap_or(posters),
                data: self.data.clone().unwrap_or(data),
            })
        } else {
            None
        };
        Ok(ShipSave {
            settings: ShipSettingsChange {
                style: self
                    .style
                    .as_deref()
                    .map(|s| word::<ShipStyle>("style", s, "scene, models"))
                    .transpose()?,
                textures: self
                    .textures
                    .as_deref()
                    .map(|t| word::<TextureFormat>("textures", t, "ktx2, webp, avif"))
                    .transpose()?,
                quality: self.quality,
                assets,
            },
            target,
        })
    }
}

/// The repo a call moves to: `None` when it leaves the repo alone, and an
/// inner `None` when it clears it. Only null clears: "" is refused.
fn new_repo(host: &dyn Host, args: &SetShipArgs) -> Result<Option<Option<String>>, ToolError> {
    let Some(wanted) = &args.site_repo else {
        return Ok(None);
    };
    let Some(wanted) = wanted else {
        return Ok(Some(None));
    };
    let path = checked_repo(host.session(), wanted)?;
    Ok(Some(Some(path.display().to_string())))
}

#[tool_router(router = ship_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get how a website project ships: the style (scene, or models for a hand-built site that loads one file per model), the texture format (ktx2, webp or avif) and quality, each asset's choice (auto, never, always) and file name, the site's folders from its maquette.site.json, whether the project's starters support the choices and the own origin export (scriptReady, originReady, else update_starters), and a dry run plan of the next Ship: each file new, replaced, unchanged or hand made, whether it ships or why it is held back, with bytes and triangles before and after, and the site files it leaves alone or removes.",
        annotations(read_only_hint = true)
    )]
    async fn get_ship_settings(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = commands::view(host.session(), &args.project)?;
                Ok(view_value(&view))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Change how a website project ships, as the Ship view does: style (scene or models), textures (ktx2, webp or avif), quality (1 to 100), per asset choice (auto, never, always) and file name (without .glb, null for the default), the site repo (siteRepo, null clears it), and the site's folders (models, posters, data; written to the site's maquette.site.json, keeping its other keys). Only the given fields change; everything is checked before anything is written. Spends nothing. Returns what get_ship_settings returns.",
        annotations(idempotent_hint = true)
    )]
    async fn set_ship_settings(&self, Parameters(args): Parameters<SetShipArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let manifest = project::load(&root, &config.layout, &args.project)?;
                let view = commands::view(host.session(), &args.project)?;
                let repo = new_repo(host, &args)?;
                let at = repo
                    .as_ref()
                    .map_or(manifest.site_repo.as_deref(), Option::as_deref);
                let save = args.save(&view, &manifest, repo.as_ref())?;
                // Everything is checked against where it goes before anything moves.
                commands::check_save(host.session(), &args.project, &save, at)?;
                if let Some(repo) = repo {
                    let moved = set_repo(
                        host.session(),
                        host.runner(),
                        &args.project,
                        manifest.name.clone(),
                        repo,
                        RepoSetBy::Agent,
                    )?;
                    host.project_changed(&moved);
                }
                let saved = commands::save(host.session(), &args.project, save)?;
                host.pipeline_changed(&args.project);
                Ok(view_value(&saved))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Get a website project's size report: for each asset with a model, the source file it came in as (bytes and triangles) against the file Compress made to ship (bytes and triangles), the totals over the models measured on both sides, and the same table as csv and markdown, ready to paste. Shipped numbers are empty before the first run.",
        annotations(read_only_hint = true)
    )]
    async fn get_size_report(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = commands::view(host.session(), &args.project)?;
                Ok(to_value(&view.report))
            })
            .await;
        respond(found)
    }
}
