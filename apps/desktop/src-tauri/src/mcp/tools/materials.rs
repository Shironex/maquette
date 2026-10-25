//! Tools that read and change an asset's material tweaks and the codec of
//! each of its texture map groups.

use std::collections::BTreeMap;

use maquette_core::materials::{
    self, FactorMode, MaterialsPatch, RoughnessTweak, SceneMaterials, TweakPatch,
};
use maquette_core::pipeline::CompressDefaults;
use maquette_core::pipeline::presets::{self, CompressPreset};
use maquette_core::{Asset, ProjectManifest, TextureCodec, project};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::{materials as commands, pipeline};
use crate::mcp::host::Host;

use super::{McpTools, Out, explicit, reason, respond, to_short_value, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetMaterialsArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoughnessEdit {
    /// "override" replaces the roughness (and drops the metal and roughness map so it applies
    /// evenly); "multiply" scales the model's roughness and keeps the map.
    pub mode: String,
    /// 0 to 1 for override, 0 to 2 for multiply.
    pub value: f32,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TweakEdit {
    /// Multiplies the base color, "#rrggbb" in sRGB (the color map stays). null clears it.
    #[serde(default, deserialize_with = "explicit")]
    pub tint: Option<Option<String>>,
    /// The roughness change, or null to clear it.
    #[serde(default, deserialize_with = "explicit")]
    pub roughness: Option<Option<RoughnessEdit>>,
    /// Replaces the metalness, 0 to 1 (drops the metal and roughness map). null clears it.
    #[serde(default, deserialize_with = "explicit")]
    pub metalness: Option<Option<f32>>,
    /// The emissive strength, 0 to 50 (KHR_materials_emissive_strength). null clears it.
    #[serde(default, deserialize_with = "explicit")]
    pub emissive_strength: Option<Option<f32>>,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CodecsEdit {
    /// Base color and emissive maps: "etc1s" or "uastc". null follows the row's codec.
    #[serde(default, deserialize_with = "explicit")]
    pub color: Option<Option<String>>,
    /// Normal maps: "etc1s" or "uastc". null goes back to the default, UASTC.
    #[serde(default, deserialize_with = "explicit")]
    pub normal: Option<Option<String>>,
    /// Metal and roughness, occlusion and any other map. null follows the row's codec.
    #[serde(default, deserialize_with = "explicit")]
    pub other: Option<Option<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetMaterialsArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// Drop every tweak of the asset first, before the rest applies.
    #[serde(default)]
    pub reset: bool,
    /// The change for every material of the model. Only the fields you give change.
    pub all: Option<TweakEdit>,
    /// Changes per material name (as get_material_tweaks lists them); a field set here wins
    /// over `all`. null removes that material's entry.
    #[serde(default)]
    pub materials: BTreeMap<String, Option<TweakEdit>>,
    /// The texture codec per map group for the asset's model. Only the groups you give change.
    pub codecs: Option<CodecsEdit>,
}

fn parse_codec(group: &str, word: &str) -> Result<TextureCodec, String> {
    serde_json::from_value(json!(word.to_lowercase()))
        .map_err(|_| format!("codecs.{group} must be etc1s or uastc, not {word:?}."))
}

impl TweakEdit {
    fn patch(&self) -> Result<TweakPatch, String> {
        let roughness = match &self.roughness {
            None => None,
            Some(None) => Some(None),
            Some(Some(r)) => {
                let mode: FactorMode = serde_json::from_value(json!(r.mode.to_lowercase()))
                    .map_err(|_| {
                        format!(
                            "roughness.mode must be override or multiply, not {:?}.",
                            r.mode
                        )
                    })?;
                Some(Some(RoughnessTweak {
                    mode,
                    value: r.value,
                }))
            }
        };
        Ok(TweakPatch {
            tint: self.tint.clone(),
            roughness,
            metalness: self.metalness,
            emissive_strength: self.emissive_strength,
        })
    }
}

impl SetMaterialsArgs {
    fn patch(&self) -> Result<MaterialsPatch, String> {
        let mut materials = BTreeMap::new();
        for (name, edit) in &self.materials {
            let change = edit.as_ref().map(TweakEdit::patch).transpose()?;
            materials.insert(name.clone(), change);
        }
        Ok(MaterialsPatch {
            reset: self.reset,
            all: self.all.as_ref().map(TweakEdit::patch).transpose()?,
            materials,
        })
    }
}

/// The asset an agent names: by id, else by name.
fn find_asset<'a>(manifest: &'a ProjectManifest, wanted: &str) -> Result<&'a Asset, String> {
    let key = wanted.trim().to_lowercase();
    manifest
        .assets
        .iter()
        .find(|a| a.id == key)
        .or_else(|| {
            manifest
                .assets
                .iter()
                .find(|a| a.name.to_lowercase() == key)
        })
        .ok_or_else(|| {
            let ids: Vec<&str> = manifest.assets.iter().map(|a| a.id.as_str()).collect();
            format!("Unknown asset {wanted:?}. Assets: {}.", ids.join(", "))
        })
}

/// What the tools work on: the asset, its newest model file's stem (its
/// preset row) and its materials' names.
struct Target {
    asset: Asset,
    stem: Option<String>,
    names: Vec<String>,
}

fn target(host: &dyn Host, project_id: &str, wanted: &str) -> Result<Target, String> {
    let (root, config) = workspace(host)?;
    let manifest = project::load(&root, &config.layout, project_id).map_err(reason)?;
    let dir = project::dir(&root, &config.layout, project_id).map_err(reason)?;
    let asset = find_asset(&manifest, wanted)?.clone();
    let latest = asset.latest().map(|v| dir.join(&v.file));
    let stem = latest
        .as_ref()
        .and_then(|f| f.file_stem())
        .map(|s| s.to_string_lossy().into_owned());
    let names = latest
        .as_deref()
        .map(materials::material_names)
        .unwrap_or_default();
    Ok(Target { asset, stem, names })
}

/// The tools' answer: the tweaks, the material names and the codecs.
fn summary(
    host: &dyn Host,
    project_id: &str,
    target: &Target,
    saved: &SceneMaterials,
) -> Result<Value, String> {
    let view = pipeline::view(host.session(), host.runner(), project_id).map_err(|e| e.message)?;
    let defaults = CompressDefaults {
        texture_size: view.defaults.texture_size,
        codec: view.defaults.codec,
        meshopt: view.defaults.meshopt,
    };
    let codecs = target.stem.as_deref().map(|stem| {
        let row = view.def.presets.iter().find(|p| p.model == stem);
        let resolved = presets::resolve(&view.def.presets, stem, defaults);
        let built = view.models.iter().find(|m| m.model == stem);
        json!({
            "model": stem,
            "codecs": to_value(resolved.codecs),
            "rowCodec": to_value(row.map_or(defaults.codec, |r| r.codec)),
            "lock": row.and_then(|r| r.lock.clone()),
            "rawBytes": built.and_then(|m| m.raw_bytes),
            "shippedBytes": built.and_then(|m| m.shipped_bytes),
        })
    });
    let tweaks = saved
        .models
        .get(&target.asset.id)
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "project": project_id,
        "asset": target.asset.id,
        "name": target.asset.name,
        "materials": target.names,
        "tweaks": to_short_value(&tweaks),
        "compression": codecs,
        "note": "Tweaks and codecs apply to the shipped model the next time Compress runs; the source model never changes.",
    }))
}

/// The preset table with the asset's map codecs changed, checked against
/// the locks but not saved.
fn with_codecs(
    host: &dyn Host,
    project_id: &str,
    stem: &str,
    label: &str,
    edit: &CodecsEdit,
) -> Result<Vec<CompressPreset>, String> {
    let view = pipeline::view(host.session(), host.runner(), project_id).map_err(|e| e.message)?;
    let mut table = view.def.presets.clone();
    let index = table
        .iter()
        .position(|p| p.model == stem)
        .unwrap_or_else(|| {
            table.push(CompressPreset {
                model: stem.to_string(),
                label: label.to_string(),
                texture_size: view.defaults.texture_size,
                data_texture_size: None,
                codec: view.defaults.codec,
                map_codecs: None,
                meshopt: view.defaults.meshopt,
                lock: None,
            });
            table.len() - 1
        });
    let row = &mut table[index];
    let mut codecs = row.map_codecs.unwrap_or_default();
    for (group, change, slot) in [
        ("color", &edit.color, &mut codecs.color),
        ("normal", &edit.normal, &mut codecs.normal),
        ("other", &edit.other, &mut codecs.other),
    ] {
        if let Some(change) = change {
            *slot = change
                .as_deref()
                .map(|word| parse_codec(group, word))
                .transpose()?;
        }
    }
    row.map_codecs = (!codecs.is_empty()).then_some(codecs);
    presets::apply(&view.def.presets, table).map_err(reason)
}

#[tool_router(router = materials_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get an asset's material tweaks and texture codecs: the names of its model's materials, the tweaks stored for every material (`all`) and per material name, and the codec each map group (color, normal, other) is compressed with, with the model's raw and shipped size.",
        annotations(read_only_hint = true)
    )]
    async fn get_material_tweaks(&self, Parameters(args): Parameters<GetMaterialsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let target = target(host, &args.project, &args.asset)?;
                let saved = commands::get(host.session(), &args.project).map_err(|e| e.message)?;
                summary(host, &args.project, &target, &saved)
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Change an asset's material tweaks and texture codecs, as the Inspector's Material panel does. `all` changes every material; `materials` changes one by name (its fields win over all; null removes the entry). A tweak has tint (\"#rrggbb\", multiplies the base color), roughness ({ mode: override or multiply, value }), metalness (0 to 1, replaces it) and emissiveStrength (0 to 50); only the fields you give change and null clears one. An override of roughness or metalness drops the model's metal and roughness map, so the value applies evenly. `codecs` sets etc1s or uastc per map group (color, normal, other; normal maps default to uastc, null goes back to the default). Locked preset rows refuse codec changes, and nothing is written when any part is refused. The source model never changes: Compress applies the tweaks to the shipped GLB on the next run, and the open app updates live. Returns the result like get_material_tweaks.",
        annotations(idempotent_hint = true)
    )]
    async fn set_material_tweaks(&self, Parameters(args): Parameters<SetMaterialsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let target = target(host, &args.project, &args.asset)?;
                let change = args.patch()?;
                let table = match (&args.codecs, &target.stem) {
                    (Some(edit), Some(stem)) => Some(with_codecs(
                        host,
                        &args.project,
                        stem,
                        &target.asset.name,
                        edit,
                    )?),
                    (Some(_), None) => {
                        return Err(format!(
                            "{} has no model yet, so it has no textures to compress.",
                            target.asset.name
                        ));
                    }
                    (None, _) => None,
                };
                // Applied to the file on disk and checked whole before it is written.
                let saved = commands::update(host.session(), &args.project, |current| {
                    materials::apply_patch(current, &target.asset.id, &change, &target.names)
                })
                .map_err(|e| e.message)?;
                host.materials_changed(&args.project);
                if let Some(table) = table {
                    pipeline::save_presets(host.session(), host.runner(), &args.project, table)
                        .map_err(|e| e.message)?;
                    host.pipeline_changed(&args.project);
                }
                summary(host, &args.project, &target, &saved)
            })
            .await;
        respond(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_turn_into_core_patches_and_bad_words_are_refused() {
        let args: SetMaterialsArgs = serde_json::from_value(json!({
            "project": "bench",
            "asset": "laptop",
            "all": { "tint": "#3a3936", "roughness": { "mode": "Override", "value": 0.62 }, "emissiveStrength": null },
            "materials": { "Screen": null }
        }))
        .expect("args");
        let patch = args.patch().expect("patch");
        let all = patch.all.expect("all");
        assert_eq!(all.tint, Some(Some("#3a3936".to_string())));
        assert_eq!(
            all.roughness.flatten().map(|r| r.mode),
            Some(FactorMode::Override)
        );
        assert_eq!(all.metalness, None, "left out stays");
        assert_eq!(all.emissive_strength, Some(None), "null clears");
        assert_eq!(patch.materials.get("Screen"), Some(&None));

        let bad: SetMaterialsArgs = serde_json::from_value(json!({
            "project": "bench",
            "asset": "laptop",
            "all": { "roughness": { "mode": "lerp", "value": 0.5 } }
        }))
        .expect("args");
        assert!(
            bad.patch()
                .expect_err("mode")
                .contains("override or multiply")
        );
        assert!(
            parse_codec("normal", "bc7")
                .expect_err("codec")
                .contains("codecs.normal")
        );
        assert_eq!(parse_codec("color", "UASTC"), Ok(TextureCodec::Uastc));
    }
}
