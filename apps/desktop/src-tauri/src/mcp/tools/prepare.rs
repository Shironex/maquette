//! Tools that read and change an asset's real size and fix-ups, the
//! preparation the Build scene step gives its model, and preview what the
//! fix-ups do on its newest model (headless).

use maquette_core::prepare::{self, Fixup, FixupKind, PrepPatch, RealSize, SizeAxis, SizeUnit};
use maquette_core::project;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::commands::prepare::{
    self as commands, AssetPrep, FixupDefaults, PreviewRequest, preview_job,
};
use crate::mcp::host::Host;

use super::materials::find_asset;
use super::{McpTools, Out, reason, respond, to_short_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PrepArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SizeEdit {
    /// The side the size measures: "height" (y, up), "width" (x), "depth" (z) or "longest".
    pub axis: String,
    /// The size in `unit`, from 0.1 cm to 1000 m.
    pub value: f32,
    /// "cm" or "m".
    pub unit: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetSizeArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// The real object's size along one side, or null to clear it.
    pub size: Option<SizeEdit>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FixupEdit {
    /// One of "apply-transforms", "remove-loose", "merge", "decimate", "shrink-textures",
    /// "origin-bottom".
    pub op: String,
    /// decimate: the triangle target, 100 to 10,000,000. Defaults to the workspace budget.
    pub triangles: Option<u32>,
    /// shrink-textures: the longest image edge in pixels, 64 to 16384. Defaults to the
    /// workspace texture size.
    pub max_size: Option<u32>,
    /// remove-loose: parts this share of the model's diagonal or smaller go, 0 to 0.5.
    /// Defaults to 0.02.
    pub min_share: Option<f32>,
    /// merge: one mesh per material instead of one mesh. Defaults to false.
    pub by_material: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddFixupsArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// The fix-ups to add; one of a kind already there is replaced. Each kind at most once.
    pub fixups: Vec<FixupEdit>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoveFixupsArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// The kinds to remove, as add_fixups names them. Leave out with `all`.
    #[serde(default)]
    pub ops: Vec<String>,
    /// Remove every fix-up of the asset.
    #[serde(default)]
    pub all: bool,
}

/// One of the words a field takes, or a message naming them.
fn word<T: DeserializeOwned>(field: &str, text: &str, words: &str) -> Result<T, String> {
    serde_json::from_value(json!(text.trim().to_lowercase()))
        .map_err(|_| format!("{field} must be {words}, not {text:?}."))
}

const OPS: &str =
    "apply-transforms, remove-loose, merge, decimate, shrink-textures or origin-bottom";

fn parse_kind(text: &str) -> Result<FixupKind, String> {
    word("op", text, OPS)
}

impl SizeEdit {
    fn size(&self) -> Result<RealSize, String> {
        Ok(RealSize {
            axis: word::<SizeAxis>("size.axis", &self.axis, "height, width, depth or longest")?,
            value: self.value,
            unit: word::<SizeUnit>("size.unit", &self.unit, "cm or m")?,
        })
    }
}

impl FixupEdit {
    fn fixup(&self, defaults: &FixupDefaults) -> Result<Fixup, String> {
        Ok(match parse_kind(&self.op)? {
            FixupKind::ApplyTransforms => Fixup::ApplyTransforms,
            FixupKind::RemoveLoose => Fixup::RemoveLoose {
                min_share: self.min_share.unwrap_or(defaults.loose_share),
            },
            FixupKind::Merge => Fixup::Merge {
                by_material: self.by_material.unwrap_or(false),
            },
            FixupKind::Decimate => Fixup::Decimate {
                triangles: self.triangles.unwrap_or(defaults.triangles),
            },
            FixupKind::ShrinkTextures => Fixup::ShrinkTextures {
                max_size: self.max_size.unwrap_or(defaults.max_texture),
            },
            FixupKind::OriginBottom => Fixup::OriginBottom,
        })
    }
}

impl RemoveFixupsArgs {
    fn patch(&self) -> Result<PrepPatch, String> {
        if !self.all && self.ops.is_empty() {
            return Err("Name the ops to remove, or set all.".to_string());
        }
        Ok(PrepPatch {
            clear_fixups: self.all,
            remove: self
                .ops
                .iter()
                .map(|op| parse_kind(op))
                .collect::<Result<_, _>>()?,
            ..PrepPatch::default()
        })
    }
}

/// The asset's id, found by id or name.
fn asset_id(host: &dyn Host, project_id: &str, wanted: &str) -> Result<String, String> {
    let (root, config) = workspace(host)?;
    let manifest = project::load(&root, &config.layout, project_id).map_err(reason)?;
    Ok(find_asset(&manifest, wanted)?.id.clone())
}

fn view(host: &dyn Host, project_id: &str, asset: &str) -> Result<AssetPrep, String> {
    commands::asset_view(host.session(), project_id, asset).map_err(|e| e.message)
}

/// The tools' answer: the size, the fix-ups and what the model measures.
fn summary(found: &AssetPrep) -> Value {
    let entry = &found.entry;
    json!({
        "project": found.project,
        "asset": found.asset,
        "name": found.name,
        "kind": found.kind,
        "size": to_short_value(entry.size),
        "sizeText": entry.size.map(|s| s.describe()),
        "fixups": to_short_value(&entry.fixups),
        "fixupText": entry.fixups.iter().map(Fixup::describe).collect::<Vec<_>>(),
        "version": found.version,
        "measured": to_short_value(found.measured),
        "afterFixups": to_short_value(found.fixed),
        "scale": to_short_value(found.scale),
        "suggestion": to_short_value(&found.suggestion),
        "defaults": to_short_value(&found.defaults),
        "lastPreview": found.report.as_ref().map(to_short_value),
        "note": "Sizes are metres, width (x), height (y, up), depth (z). The Build scene step applies the fix-ups in a fixed order, then scales the model to its real size, before the layout places it; the inbox file and the asset's versions never change. A suggestion is never applied on its own.",
    })
}

/// Checks `patch` whole against the saved file, then applies it to the
/// file as it is on disk (the app may have saved since), so nothing is
/// written when any part is refused.
fn change(host: &dyn Host, project_id: &str, wanted: &str, patch: &PrepPatch) -> Out {
    let found = (|| {
        let id = asset_id(host, project_id, wanted)?;
        let current = commands::get(host.session(), project_id).map_err(|e| e.message)?;
        prepare::apply_patch(&current, &id, patch).map_err(reason)?;
        commands::update(host.session(), project_id, |now| {
            prepare::apply_patch(now, &id, patch)
        })
        .map_err(|e| e.message)?;
        host.prepare_changed(project_id);
        view(host, project_id, &id).map(|v| summary(&v))
    })();
    respond(found)
}

#[tool_router(router = prepare_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get an asset's real size and fix-ups: the size its model is scaled to (axis height, width, depth or longest; value; unit cm or m), the fix-ups applied to it before the scene is built, what its newest model measures in metres before and after them, the real size scale, a size suggested from its name and kind (never applied on its own), the defaults new fix-ups start from and the last preview's numbers.",
        annotations(read_only_hint = true)
    )]
    async fn get_real_size(&self, Parameters(args): Parameters<PrepArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let id = asset_id(host, &args.project, &args.asset)?;
                view(host, &args.project, &id).map(|v| summary(&v))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Set or clear (size: null) an asset's real size: { axis: height | width | depth | longest, value, unit: cm | m }, 0.1 cm to 1000 m. The Build scene step scales the model uniformly so that side measures the size, after its fix-ups and before the layout places it (the layout's own scale stays a tweak on top). Nothing is written when the size is refused. Returns the result like get_real_size.",
        annotations(idempotent_hint = true)
    )]
    async fn set_real_size(&self, Parameters(args): Parameters<SetSizeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let size = args.size.as_ref().map(SizeEdit::size).transpose()?;
                let patch = PrepPatch {
                    size: Some(size),
                    ..PrepPatch::default()
                };
                Ok(change(host, &args.project, &args.asset, &patch))
            })
            .await;
        found.unwrap_or_else(|err| respond(Err(err)))
    }

    #[tool(
        description = "List an asset's fix-ups in the order they apply, with the numbers of the last preview when one ran. Returns the result like get_real_size.",
        annotations(read_only_hint = true)
    )]
    async fn list_fixups(&self, Parameters(args): Parameters<PrepArgs>) -> Out {
        self.get_real_size(Parameters(args)).await
    }

    #[tool(
        description = "Add fix-ups to an asset; one of a kind already there is replaced. Each is { op, ...numbers }: apply-transforms (bakes object transforms, rigged meshes keep theirs), remove-loose { minShare 0 to 0.5, default 0.02 of the model's diagonal }, merge { byMaterial, default false }, decimate { triangles, default the workspace budget }, shrink-textures { maxSize px, default the workspace texture size }, origin-bottom (bottom centre to the origin, on the floor). They always apply in that order, headless, on a copy: the inbox file and the asset's versions never change, and every new version gets them on the next run. Nothing is written when any one is refused. Returns the result like get_real_size; preview_fixups shows what they do.",
        annotations(idempotent_hint = true)
    )]
    async fn add_fixups(&self, Parameters(args): Parameters<AddFixupsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                if args.fixups.is_empty() {
                    return Err("Give at least one fix-up to add.".to_string());
                }
                let (_, config) = workspace(host)?;
                let defaults = commands::defaults(&config);
                let add = args
                    .fixups
                    .iter()
                    .map(|f| f.fixup(&defaults))
                    .collect::<Result<_, _>>()?;
                let patch = PrepPatch {
                    add,
                    ..PrepPatch::default()
                };
                Ok(change(host, &args.project, &args.asset, &patch))
            })
            .await;
        found.unwrap_or_else(|err| respond(Err(err)))
    }

    #[tool(
        description = "Remove fix-ups from an asset by op (as add_fixups names them), or all of them with all: true. Nothing is written when an op is not one of the kinds. Returns the result like get_real_size.",
        annotations(idempotent_hint = true)
    )]
    async fn remove_fixups(&self, Parameters(args): Parameters<RemoveFixupsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let patch = args.patch()?;
                Ok(change(host, &args.project, &args.asset, &patch))
            })
            .await;
        found.unwrap_or_else(|err| respond(Err(err)))
    }

    #[tool(
        description = "Run an asset's saved fix-ups and real size on its newest model, headless (blender -b, never a window), and return the numbers before and after each fix-up: triangles, objects, largest texture, bounds in metres and file size. The model is only read; the fixed copy lives in the workspace cache. A preview that already ran answers at once. Nothing to preview when the asset has no size and no fix-ups."
    )]
    async fn preview_fixups(&self, Parameters(args): Parameters<PrepArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let id = asset_id(host, &args.project, &args.asset)?;
                let found = view(host, &args.project, &id)?;
                if found.entry.is_empty() {
                    return Err(format!(
                        "{} has no real size or fix-ups to preview. Add some first.",
                        found.name
                    ));
                }
                let request = PreviewRequest {
                    project: Some(args.project.clone()),
                    asset: Some(id.clone()),
                    inbox: None,
                    plan: found.entry.clone(),
                };
                let job = preview_job(host.session(), &request).map_err(|e| e.message)?;
                let report = job.run(&host.toolbox()).map_err(|e| e.message)?;
                Ok(json!({
                    "project": args.project,
                    "asset": id,
                    "report": to_short_value(&report),
                }))
            })
            .await;
        respond(found)
    }
}

#[cfg(test)]
mod tests;
