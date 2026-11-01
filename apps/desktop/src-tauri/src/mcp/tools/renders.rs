//! Render styles for agents: render_styles starts pictures of a project's
//! models or library entries, get_render and list_renders read how they
//! stand and where the files are, decide_render stops and goes on. The files
//! go to the exports folder, never anywhere an agent names.

use crate::mcp::tools::ToolError;
use maquette_core::packs::PackSource;
use maquette_core::renders::{
    RenderFile, RenderRecord, RenderRequest, RenderRow, RenderSettings, RenderStyle, Renders,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::renders as commands;

use super::{Host, McpTools, Out, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderStylesArgs {
    /// A project id: its models, or just the assets named in assets.
    pub project: Option<String>,
    /// Asset ids of the project; every model of it when left out.
    pub assets: Option<Vec<String>>,
    /// Library entry ids, as list_library returns them. Give project or entries.
    pub entries: Option<Vec<String>>,
    /// Any of blueprint, toon, clay, shaded, turntable, contact-sheet. Defaults to clay.
    pub styles: Option<Vec<String>>,
    /// Settings to change, by field: camera (front, three-quarter or side), size (512, 1024, 2048
    /// or 4096), transparent, lineColor, paperColor, lineWeight, crease, hiddenDashed, marks,
    /// titleBlock, clayColor, format (webp, gif or mp4, webp by default; gif and mp4 need
    /// ffmpeg), length (seconds), width (a loop's px), look (blueprint, toon, clay or shaded, for
    /// a turntable or contact sheet), tilt, stillBackground, tightCrop, columns and names.
    pub settings: Option<Value>,
    /// Also one contact sheet of all the models, in the settings' look. Defaults to false.
    #[serde(default)]
    pub sheet: bool,
    /// Writes over earlier renders instead of numbering the new files. Defaults to false.
    #[serde(default)]
    pub replace: bool,
    /// Only list the models, where their files would go and which cannot be read. Defaults to false.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderIdArgs {
    /// The render id, as render_styles or list_renders returns it.
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RenderChoice {
    /// Stop after the file running now.
    Stop,
    /// Go on with a stopped render from the next file.
    Resume,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecideRenderArgs {
    /// The render id.
    pub id: String,
    /// stop or resume.
    pub choice: RenderChoice,
}

fn service(host: &dyn Host) -> Result<&Renders, String> {
    host.renders()
        .ok_or_else(|| "Render styles are not available in this session.".to_string())
}

fn source_of(args: &RenderStylesArgs) -> Result<PackSource, String> {
    match (&args.project, &args.entries) {
        (Some(project), None) => Ok(PackSource::Project {
            project: project.clone(),
        }),
        (None, Some(entries)) => Ok(PackSource::Entries {
            entries: entries.clone(),
        }),
        _ => Err("Give project or entries.".into()),
    }
}

fn settings_of(over: Option<&Value>) -> Result<RenderSettings, String> {
    let Some(over) = over else {
        return Ok(RenderSettings::default());
    };
    if !over.is_object() {
        return Err("settings is an object of the fields to change.".into());
    }
    serde_json::from_value(over.clone()).map_err(|e| format!("The settings do not read: {e}"))
}

fn styles_of(names: Option<&Vec<String>>) -> Result<Vec<RenderStyle>, String> {
    let Some(names) = names else {
        return Ok(vec![RenderStyle::Clay]);
    };
    names
        .iter()
        .map(|name| {
            serde_json::from_value(Value::String(name.clone())).map_err(|_| {
                format!(
                    "There is no style {name}: use blueprint, toon, clay, shaded, turntable or contact-sheet."
                )
            })
        })
        .collect()
}

fn file_json(file: &RenderFile) -> Value {
    json!({
        "style": to_value(file.style),
        "file": file.file,
        "state": to_value(file.state),
        "bytes": file.bytes,
        "seconds": file.seconds,
        "moving": file.moving.map(to_value),
        "problem": file.problem.as_ref().map(|p| json!({
            "kind": to_value(p.kind),
            "message": p.message,
        })),
    })
}

/// What an agent needs from a record: how it stands and each file.
fn summary(record: &RenderRecord) -> Value {
    let items: Vec<Value> = record
        .items
        .iter()
        .map(|i| {
            json!({
                "key": i.target.key,
                "name": i.target.name,
                "version": i.target.version,
                "files": i.files.iter().map(file_json).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "id": record.id,
        "state": to_value(record.state),
        "label": record.label,
        "dir": record.dir,
        "done": record.done(),
        "of": record.total(),
        "problems": record.problems(),
        "message": record.message,
        "items": items,
        "sheet": record.sheet.as_ref().map(file_json),
    })
}

#[tool_router(router = renders_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Render pictures of a project's models or library entries: blueprint (lines on blue paper with height and width marks and a title block), toon (flat colour, ink lines), clay (one colour, cavity), shaded (as built, studio light), turntable (one full turn, a WebP loop by default; GIF and MP4 need ffmpeg) or contact-sheet (eight angles on one PNG). Headless Blender, one file at a time, no credits. Files go to the project's exports/renders/<asset>/ (a library entry's to the workspace's exports/renders/<entry>/); a GIF follows fixed rules: one palette, 20 fps, Bayer dither, cropped to what moves, and a problem says when it passes the 2 MB target. A model with no texture leaves toon out and says so. dryRun only lists the models. Answers the render's id and files; follow it with get_render, then open the files to look. Spends no credits."
    )]
    async fn render_styles(&self, Parameters(args): Parameters<RenderStylesArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let source = source_of(&args)?;
                let planned = commands::plan(host.session(), &source)?;
                if args.dry_run {
                    return Ok(json!({ "dryRun": true, "plan": to_value(&planned) }));
                }
                let settings = settings_of(args.settings.as_ref())?;
                let styles = styles_of(args.styles.as_ref())?;
                let keys: Vec<String> = match &args.assets {
                    Some(assets) if args.project.is_some() => assets.clone(),
                    Some(_) => return Err("assets goes with project.".into()),
                    None => planned.targets.iter().map(|t| t.key.clone()).collect(),
                };
                let request = RenderRequest {
                    source,
                    rows: keys
                        .into_iter()
                        .map(|key| RenderRow {
                            key,
                            styles: styles.clone(),
                        })
                        .collect(),
                    settings,
                    replace: args.replace,
                    sheet: args.sheet,
                };
                let record =
                    commands::start(host.session(), service(host)?, host.toolbox(), &request)?;
                Ok(summary(&record))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read one render: its state (running, stopped or done), how many files are done, and each model's files with their path, state (queued, running, done, warn, failed or skipped), bytes, seconds, a turntable's moving area, and a problem with its kind (no-texture, over-target, out-of-memory, missing-tool, unreadable or failed) and a message that names the next step. Reads only."
    )]
    async fn get_render(&self, Parameters(args): Parameters<RenderIdArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, _) = workspace(host)?;
                let record = service(host)?
                    .get(&root, &args.id)
                    .map_err(|e| e.to_string())?;
                Ok(summary(&record))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "List the newest renders (id, state, what they were of, how far each got, the folder; a render Maquette closed on reads as stopped and decide_render resume goes on with it). Reads only."
    )]
    async fn list_renders(&self) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, _) = workspace(host)?;
                let recent: Vec<Value> = service(host)?
                    .recent(&root)
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.id,
                            "state": to_value(r.state),
                            "label": r.label,
                            "dir": r.dir,
                            "done": r.done(),
                            "of": r.total(),
                            "problems": r.problems(),
                            "createdAt": r.created_at,
                        })
                    })
                    .collect();
                Ok(json!({ "renders": recent }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Answer a render: stop stops it after the file running now, resume goes on with a stopped render from the next file. Answers the render as get_render does."
    )]
    async fn decide_render(&self, Parameters(args): Parameters<DecideRenderArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, _) = workspace(host)?;
                let renders = service(host)?;
                let record = match args.choice {
                    RenderChoice::Stop => renders.stop(&args.id),
                    RenderChoice::Resume => renders.resume(&root, host.toolbox(), &args.id),
                };
                record
                    .map(|r| summary(&r))
                    .map_err(|e| e.to_string())
                    .map_err(ToolError::from)
            })
            .await;
        respond(found)
    }
}
