//! Export packs for agents: export_pack plans or starts a pack of library
//! entries, a collection or a project's models; get_pack and list_packs
//! read how packs stand; decide_pack answers what a pack asks for. A pack
//! an agent asks for is written inside the workspace only.

use maquette_core::packs::{
    PackDecision, PackItemState, PackPreset, PackRecord, PackRequest, PackSettings, PackSource,
    Packs, ZipChoice, presets,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::packs as commands;

use super::{Host, McpTools, Out, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExportPackArgs {
    /// Library entry ids, as list_library returns them. Give one of entries, collection or project.
    pub entries: Option<Vec<String>>,
    /// A library collection id: every entry in it.
    pub collection: Option<String>,
    /// A project id: every accepted model of the project.
    pub project: Option<String>,
    /// A preset id from list_packs. Defaults to plain-obj, the built in one.
    pub preset: Option<String>,
    /// Settings that change the preset's, by field: name, format (obj, glb or both), realSize,
    /// pivotBottom, oneMaterial, texture (512, 1024, 2048, 4096 or 0 to keep), riggedGlb,
    /// compression (none or web), budgets ([{kind, triangles}], kind prop, character or scene,
    /// triangles null for none), readme, manifest, contactSheet, refs and zip.
    pub settings: Option<Value>,
    /// Where to write the pack, inside the workspace (relative to it or absolute). Defaults to
    /// exports/ at the workspace root.
    pub out: Option<String>,
    /// Only plan the pack: the models, the triangles per kind against the budgets, the size and
    /// which models lose Mixamo clips. Starts nothing. Defaults to false.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackIdArgs {
    /// The pack id, as export_pack or list_packs returns it.
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PackChoice {
    /// Raise one model's triangle budget and export it again.
    Raise,
    /// Keep one model's warning and let it into the pack.
    Keep,
    /// Export one failed or warned model again as it is.
    Again,
    /// Zip the models that pass now and leave the rest out.
    ZipPassing,
    /// Zip every model that has files, warnings and all.
    ZipAll,
    /// Stop after the model running now.
    Stop,
    /// Go on with a stopped pack from the model it reached.
    Resume,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecidePackArgs {
    /// The pack id.
    pub id: String,
    /// raise, keep, again, zip-passing, zip-all, stop or resume.
    pub choice: PackChoice,
    /// The model's key in the pack (its items' model.key), for raise, keep and again.
    pub key: Option<String>,
    /// The new triangle budget, for raise.
    pub triangles: Option<u32>,
}

fn packs(host: &dyn Host) -> Result<&Packs, String> {
    host.packs()
        .ok_or_else(|| "Export packs are not available in this session.".to_string())
}

fn source(args: &ExportPackArgs) -> Result<PackSource, String> {
    match (&args.entries, &args.collection, &args.project) {
        (Some(entries), None, None) => Ok(PackSource::Entries {
            entries: entries.clone(),
        }),
        (None, Some(collection), None) => Ok(PackSource::Collection {
            collection: collection.clone(),
        }),
        (None, None, Some(project)) => Ok(PackSource::Project {
            project: project.clone(),
        }),
        _ => Err("Give one of entries, collection or project.".into()),
    }
}

/// The preset's settings with the agent's fields laid over them.
fn settings(
    all: &[PackPreset],
    preset: &str,
    over: Option<&Value>,
) -> Result<PackSettings, String> {
    let base = all
        .iter()
        .find(|p| p.id == preset)
        .ok_or_else(|| format!("There is no preset {preset}. list_packs lists the presets."))?;
    let Some(over) = over else {
        return Ok(base.settings.clone());
    };
    let Value::Object(fields) = over else {
        return Err("settings is an object of the fields to change.".into());
    };
    let mut merged = to_value(&base.settings);
    if let Value::Object(target) = &mut merged {
        for (key, value) in fields {
            target.insert(key.clone(), value.clone());
        }
    }
    serde_json::from_value(merged).map_err(|e| format!("The settings do not read: {e}"))
}

/// What an agent needs from a record: how it stands, each model's check
/// and what waits for it.
fn summary(record: &PackRecord) -> Value {
    let items: Vec<Value> = record
        .items
        .iter()
        .map(|i| {
            json!({
                "key": i.model.key,
                "name": i.model.name,
                "state": to_value(i.state),
                "triangles": i.triangles,
                "trianglesBefore": i.model.triangles(),
                "budget": i.budget,
                "heightM": i.height_m,
                "note": i.note,
                "suggest": i.suggest,
                "clipsLeftOut": i.clips_left_out,
                "files": i.files,
            })
        })
        .collect();
    let needs = record
        .items
        .iter()
        .filter(|i| matches!(i.state, PackItemState::Warn | PackItemState::Fail))
        .count();
    json!({
        "id": record.id,
        "name": record.name,
        "state": to_value(record.state),
        "label": record.label,
        "dir": record.dir,
        "zip": record.zip,
        "done": record.done(),
        "of": record.items.len(),
        "needYou": needs,
        "message": record.message,
        "items": items,
    })
}

fn decision(args: &DecidePackArgs) -> Result<PackDecision, String> {
    let key = || {
        args.key
            .clone()
            .ok_or_else(|| "Give the model's key.".to_string())
    };
    Ok(match args.choice {
        PackChoice::Raise => PackDecision::Raise {
            key: key()?,
            triangles: args
                .triangles
                .ok_or_else(|| "Give the new triangles budget.".to_string())?,
        },
        PackChoice::Keep => PackDecision::Keep { key: key()? },
        PackChoice::Again => PackDecision::Again { key: key()? },
        _ => return Err("Not a decision about one model.".into()),
    })
}

#[tool_router(router = packs_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Export library models as one pack for another engine or a person: give entries (library entry ids), a collection id or a project id, and a preset (list_packs; plain-obj by default) with settings to change. Blender runs headless on each model: real size, the pivot at the bottom centre, a triangle budget per kind (props, characters, scenes; rigged models are never decimated), textures as PNG, materials named body, OBJ and MTL or GLB or both, then opens each written file again and checks it. Mixamo clips are never shared: they are taken out and the bones kept. The pack (with README.txt, manifest.json and a contact sheet) is written inside the workspace only, to exports/<name> by default, and zipped once every model passes; a warning or failure waits for decide_pack. dryRun only answers the plan. Answers the pack's id and state; follow it with get_pack. Spends no credits."
    )]
    async fn export_pack(&self, Parameters(args): Parameters<ExportPackArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, _) = workspace(host)?;
                let source = source(&args)?;
                let all = presets(&root).map_err(|e| e.to_string())?;
                let preset = args.preset.clone().unwrap_or_else(|| "plain-obj".into());
                let settings = settings(&all, &preset, args.settings.as_ref())?;
                if args.dry_run {
                    let plan = commands::plan(host.session(), host.studio(), &source, &settings)
                        .map_err(|e| e.message)?;
                    return Ok(json!({ "dryRun": true, "settings": to_value(&settings), "plan": to_value(&plan) }));
                }
                let request = PackRequest {
                    source,
                    settings,
                    out: args.out.clone(),
                    preset: Some(preset),
                };
                let record = commands::start(
                    host.session(),
                    host.studio(),
                    packs(host)?,
                    host.toolbox(),
                    &request,
                    false,
                )
                .map_err(|e| e.message)?;
                Ok(summary(&record))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read one export pack: its state (running, waiting, zipping, done, stopped or failed), each model's check (queued, running, pass, warn, fail or kept) with triangles before and after, height, note and the budget to raise to when thin parts were lost, how many need you, and the folder and zip. Reads only."
    )]
    async fn get_pack(&self, Parameters(args): Parameters<PackIdArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, _) = workspace(host)?;
                let record = packs(host)?
                    .get(&root, &args.id)
                    .map_err(|e| e.to_string())?;
                Ok(summary(&record))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "List the newest export packs (id, name, state, how far each got, folder and zip; a pack Maquette closed on reads as stopped and decide_pack resume goes on with it) and the pack presets with their settings. Reads only."
    )]
    async fn list_packs(&self) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, _) = workspace(host)?;
                let recent: Vec<Value> = packs(host)?
                    .recent(&root)
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.id,
                            "name": r.name,
                            "state": to_value(r.state),
                            "done": r.done(),
                            "of": r.items.len(),
                            "dir": r.dir,
                            "zip": r.zip,
                            "message": r.message,
                        })
                    })
                    .collect();
                let all = presets(&root).map_err(|e| e.to_string())?;
                Ok(json!({ "packs": recent, "presets": to_value(&all) }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Answer what an export pack asks for. raise (key, triangles) exports a model again with a higher triangle budget, keep (key) lets a warned model into the pack, again (key) exports a model again as it is; zip-passing zips the models that pass and leaves the rest out, zip-all zips every model with files; stop stops after the model running now and resume goes on with a stopped pack. On a stopped pack keep only records the answer, and raise or again runs just that one model and the pack is stopped again until resume; the queued models do not start. Answers the pack as get_pack does."
    )]
    async fn decide_pack(&self, Parameters(args): Parameters<DecidePackArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, _) = workspace(host)?;
                let service = packs(host)?;
                let record = match args.choice {
                    PackChoice::ZipPassing => service.zip(&root, &args.id, ZipChoice::Passing),
                    PackChoice::ZipAll => service.zip(&root, &args.id, ZipChoice::All),
                    PackChoice::Stop => service.stop(&args.id),
                    PackChoice::Resume => {
                        return commands::resume(host.session(), service, host.toolbox(), &args.id)
                            .map(|r| summary(&r))
                            .map_err(|e| e.message);
                    }
                    _ => {
                        let decision = decision(&args)?;
                        return commands::decide(
                            host.session(),
                            service,
                            host.toolbox(),
                            &args.id,
                            &decision,
                        )
                        .map(|r| summary(&r))
                        .map_err(|e| e.message);
                    }
                };
                record.map(|r| summary(&r)).map_err(|e| e.to_string())
            })
            .await;
        respond(found)
    }
}
