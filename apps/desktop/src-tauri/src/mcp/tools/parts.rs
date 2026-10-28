//! Tools that read and name an asset's parts, the meshes object states and
//! explode tracks move by name, and compare two versions part by part. A
//! split itself is a fix-up (add_fixups, op split-parts).

use std::collections::BTreeMap;

use maquette_core::prepare::{self, PartsPatch, PrepPatch};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::parts::{self as commands, AssetParts, PartsSide};
use crate::commands::prepare as prep_commands;
use crate::mcp::host::Host;

use super::prepare::PrepArgs;
use super::{McpTools, Out, reason, respond, to_short_value};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetPartsArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// Part (by its id or the name it ships as) to its new name. Its id as the new name
    /// forgets the rename.
    #[serde(default)]
    pub rename: BTreeMap<String, String>,
    /// Groups of two or more parts to join into one mesh, named after the first.
    #[serde(default)]
    pub merge: Vec<Vec<String>>,
    /// Parts whose join is undone.
    #[serde(default)]
    pub unmerge: Vec<String>,
    /// Parts to draw as a ghost in the Inspector. They still ship.
    #[serde(default)]
    pub hide: Vec<String>,
    /// Hidden parts to draw again.
    #[serde(default)]
    pub show: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SideArgs {
    /// The version number, as list_assets returns them.
    pub version: u32,
    /// After the asset's fix-ups (a split in Blender among them), run headless on a copy.
    #[serde(default)]
    pub fixed: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompareArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    pub left: SideArgs,
    pub right: SideArgs,
}

/// The asset's parts, after running its split headless when it has not run.
fn parts(host: &dyn Host, project: &str, wanted: &str) -> Result<AssetParts, String> {
    let id = super::prepare::asset_id(host, project, wanted)?;
    commands::run_listing(host.session(), &host.toolbox(), project, &id).map_err(|e| e.message)
}

fn summary(found: &AssetParts) -> Value {
    json!({
        "project": found.project,
        "asset": found.asset,
        "name": found.name,
        "version": found.version,
        "split": found.split,
        "parts": to_short_value(&found.rows),
        "note": "id is the name in the model, name the one it ships as: the export node names object states and explode tracks pick parts by. A joined part lists the ids joined into it. Hidden parts still ship; hiding is only for looking in the Inspector. A model that is one mesh lists one part: split it with add_fixups (op split-parts), or on Tripo or Meshy with split_with_provider.",
    })
}

/// The id of a part named by id or by the name it ships as.
fn part_id(found: &AssetParts, wanted: &str) -> Result<String, String> {
    let wanted = wanted.trim();
    for row in &found.rows {
        if row.id == wanted || row.name == wanted {
            return Ok(row.id.clone());
        }
        if let Some(joined) = row.joined.iter().find(|j| j.as_str() == wanted) {
            return Ok(joined.clone());
        }
    }
    let names: Vec<&str> = found.rows.iter().map(|r| r.name.as_str()).collect();
    Err(format!(
        "{} has no part {wanted:?}. Its parts: {}.",
        found.name,
        names.join(", ")
    ))
}

impl SetPartsArgs {
    fn patch(&self, found: &AssetParts) -> Result<PartsPatch, String> {
        let ids = |list: &[String]| -> Result<Vec<String>, String> {
            list.iter().map(|p| part_id(found, p)).collect()
        };
        Ok(PartsPatch {
            rename: self
                .rename
                .iter()
                .map(|(from, to)| Ok((part_id(found, from)?, to.clone())))
                .collect::<Result<_, String>>()?,
            merge: self
                .merge
                .iter()
                .map(|group| ids(group))
                .collect::<Result<_, _>>()?,
            unmerge: ids(&self.unmerge)?,
            hide: ids(&self.hide)?,
            show: ids(&self.show)?,
        })
    }
}

#[tool_router(router = parts_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List an asset's parts as its newest model ships them: each part's id (the name in the model), the name it ships as (the export node name object states and explode tracks pick it by), triangles, materials, whether it is hidden in the Inspector and the parts joined into it. When the asset's fix-ups split it (split-parts), the split runs first, headless on a copy (blender -b, never a window); the model file never changes."
    )]
    async fn get_parts(&self, Parameters(args): Parameters<PrepArgs>) -> Out {
        let found = self
            .blocking(move |host, _| parts(host, &args.project, &args.asset).map(|f| summary(&f)))
            .await;
        respond(found)
    }

    #[tool(
        description = "Rename, join, unjoin, hide or show an asset's parts, by id or by the name they ship as (see get_parts). Renames and joins ship in the next build (the export node names follow them, so name the parts before object states or explode tracks use them); hiding only draws the part as a ghost in the Inspector. Names must be unique and at most 64 characters. Nothing is written when any part of the change is refused. Returns the parts like get_parts.",
        annotations(idempotent_hint = true)
    )]
    async fn set_parts(&self, Parameters(args): Parameters<SetPartsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let found = parts(host, &args.project, &args.asset)?;
                let patch = PrepPatch {
                    parts: Some(args.patch(&found)?),
                    ..PrepPatch::default()
                };
                let current =
                    prep_commands::get(host.session(), &args.project).map_err(|e| e.message)?;
                prepare::apply_patch(&current, &found.asset, &patch).map_err(reason)?;
                prep_commands::update(host.session(), &args.project, |now| {
                    prepare::apply_patch(now, &found.asset, &patch)
                })
                .map_err(|e| e.message)?;
                host.prepare_changed(&args.project);
                let after = commands::get(host.session(), &args.project, &found.asset)
                    .map_err(|e| e.message)?;
                Ok(summary(&after))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Compare two versions of an asset part by part, the way the Inspector's Compare does before and after a split: for each side the parts Motion can move (none for a model that is one mesh), draw calls, triangles, textures and the largest one, transfer size in bytes and its Inbox checks (pass, warning, fail). A side with fixed: true is that version after the asset's fix-ups, run headless on a copy (so v2 fixed against v2 shows what a split in Blender does).",
        annotations(read_only_hint = true)
    )]
    async fn compare_versions(&self, Parameters(args): Parameters<CompareArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let id = super::prepare::asset_id(host, &args.project, &args.asset)?;
                let side = |s: &SideArgs| PartsSide {
                    version: s.version,
                    fixed: s.fixed,
                };
                let (left, right) = (side(&args.left), side(&args.right));
                let report = commands::compare(
                    host.session(),
                    &host.toolbox(),
                    &args.project,
                    &id,
                    [&left, &right],
                )
                .map_err(|e| e.message)?;
                Ok(json!({
                    "project": args.project,
                    "asset": id,
                    "left": { "version": left.version, "fixed": left.fixed, "numbers": to_short_value(&report.left) },
                    "right": { "version": right.version, "fixed": right.fixed, "numbers": to_short_value(&report.right) },
                }))
            })
            .await;
        respond(found)
    }
}

#[cfg(test)]
mod tests;
