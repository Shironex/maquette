//! Tools for the Lab: starting concepts, reading them and making object refs.

use std::path::Path;

use maquette_core::concepts::{Aspect, ConceptJob, ConceptJobState, NewConcepts, NewObjectRefs};
use maquette_core::{AssetKind, concepts, project};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::lab;

use super::sending::VariantArg;
use super::{McpTools, Out, reason, respond, to_value, workspace};

/// The aspect ratio of the concept images.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
pub enum AspectArg {
    #[serde(rename = "16:9")]
    Wide,
    #[serde(rename = "3:2")]
    Landscape,
    #[serde(rename = "1:1")]
    Square,
    #[serde(rename = "3:4")]
    Portrait,
    #[serde(rename = "9:16")]
    Tall,
}

impl From<AspectArg> for Aspect {
    fn from(value: AspectArg) -> Self {
        match value {
            AspectArg::Wide => Aspect::Wide,
            AspectArg::Landscape => Aspect::Landscape,
            AspectArg::Square => Aspect::Square,
            AspectArg::Portrait => Aspect::Portrait,
            AspectArg::Tall => Aspect::Tall,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConceptsArgs {
    /// The project id.
    pub project: String,
    /// The direction name, for example "Moon island porch". Reusing a name adds to it.
    pub direction: String,
    /// What the images should show.
    pub prompt: String,
    /// Reference images: paths relative to the project folder (for example
    /// "refs/island.png") or absolute paths inside it. At most 8.
    #[serde(default)]
    pub refs: Vec<String>,
    /// How many images, 1 to 12. 4 by default.
    pub count: Option<u32>,
    /// The aspect ratio, 16:9 by default.
    pub aspect: Option<AspectArg>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRefsArgs {
    /// The project id.
    pub project: String,
    /// The concept to take the objects from, relative to the project folder, for example
    /// "concepts/workbench-hero/workbench-hero_01.png" or "refs/workbench.png".
    pub source: String,
    /// One short name per object (60 characters at most), for example ["desk lamp", "clay figure"]. At most 12. The name becomes the asset name, with a capital first letter ("desk lamp" makes "Desk lamp"); put long descriptions in notes.
    pub objects: Vec<String>,
    /// Optional extra instructions per object, keyed by the object's name, for example
    /// {"island": "painted stone and moss, no wireframe lines"}. They go into the image prompt
    /// only; the asset keeps the short object name. Up to 400 characters each.
    pub notes: Option<std::collections::HashMap<String, String>>,
    /// "prop" (default) or "character": what the objects become on the Board.
    pub kind: Option<VariantArg>,
    /// The aspect ratio, 1:1 by default.
    pub aspect: Option<AspectArg>,
    /// The new direction is called "<name> objects". By default the source's direction name,
    /// or its file name.
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetConceptsArgs {
    /// The project id.
    pub project: String,
    /// One direction, by id or name. Without it, every direction.
    pub direction: Option<String>,
}

/// One concept job as the tools report it.
pub(super) fn job_value(job: &ConceptJob) -> Value {
    json!({
        "id": job.id,
        "direction": job.direction_name,
        "file": job.file,
        "title": job.title,
        "state": to_value(job.state),
        "message": job.message,
        "retryAtMs": job.retry_at_ms,
        "retries": job.retries,
        "usageLimit": job.usage_limit,
    })
}

/// The name an object refs direction starts with when the agent gives
/// none: the source's direction, else its file name.
fn source_name(project_dir: &Path, source: &str) -> String {
    let clean = source.replace('\\', "/");
    let mut parts = clean.split('/');
    if parts.next() == Some(concepts::CONCEPTS_DIR)
        && let Some(id) = parts.next()
        && let Ok(Some(direction)) = concepts::load_direction(project_dir, id)
    {
        return direction.name;
    }
    Path::new(&clean)
        .file_stem()
        .map(|s| s.to_string_lossy().replace(['_', '-'], " "))
        .unwrap_or_default()
}

#[tool_router(router = concepts_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Start concept image jobs in a project's Concept lab. Each image runs through the Codex CLI and takes about a minute; the images land in concepts/<direction>/ inside the project and show up in the app's Lab."
    )]
    async fn start_concepts(&self, Parameters(args): Parameters<ConceptsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let request = NewConcepts {
                    project: args.project,
                    direction: args.direction,
                    prompt: args.prompt,
                    refs: args.refs,
                    aspect: args.aspect.map(Aspect::from).unwrap_or_default(),
                    count: args.count.unwrap_or(4),
                };
                let tools = host.toolbox();
                let (direction, jobs) =
                    lab::generate(host.session(), host.concepts(), &tools, &request)
                        .map_err(|e| e.message)?;
                let jobs: Vec<Value> = jobs
                    .iter()
                    .map(|j| json!({ "id": j.id, "file": j.file, "state": to_value(j.state) }))
                    .collect();
                Ok(json!({
                    "status": "started",
                    "direction": { "id": direction.id, "name": direction.name },
                    "jobs": jobs,
                    "note": "Each file appears at its path, relative to the project folder, once Codex saves it.",
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read a project's Concept lab: each direction's prompt, refs and images (starred, picked, and the ref file a picked image became), plus the concept jobs still queued, running or waiting out a Codex rate limit or usage limit (running, state rate-limited with retryAtMs for a waiting one, and usageLimit true while it waits for the usage limit to lift) and the ones that failed or were cancelled (failed, with why), which retry_concept_jobs runs again.",
        annotations(read_only_hint = true)
    )]
    async fn get_concepts(&self, Parameters(args): Parameters<GetConceptsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let dir = project::dir(&root, &config.layout, &args.project).map_err(reason)?;
                project::load(&root, &config.layout, &args.project).map_err(reason)?;
                let wanted = args.direction.as_deref().map(str::trim);
                let directions: Vec<Value> = concepts::list_directions(&dir)
                    .into_iter()
                    .filter(|d| {
                        wanted.is_none_or(|w| d.id == w || d.name.trim().eq_ignore_ascii_case(w))
                    })
                    .map(|d| {
                        let images: Vec<Value> = d
                            .images
                            .iter()
                            .map(|i| {
                                json!({
                                    "file": i.file,
                                    "index": i.index,
                                    "width": i.width,
                                    "height": i.height,
                                    "starred": i.starred,
                                    "picked": i.picked,
                                    "refFile": i.ref_file,
                                    "title": i.title,
                                })
                            })
                            .collect();
                        json!({
                            "id": d.id,
                            "name": d.name,
                            "prompt": d.prompt,
                            "aspect": d.aspect.ratio(),
                            "refs": d.refs,
                            "source": d.source,
                            "kind": to_value(d.kind),
                            "images": images,
                        })
                    })
                    .collect();
                if let (Some(w), true) = (wanted, directions.is_empty()) {
                    return Err(format!("There is no direction {w:?} in {}.", args.project));
                }
                let jobs = host.concepts().jobs(Some(&args.project));
                let (running, failed): (Vec<_>, Vec<_>) = jobs
                    .iter()
                    .filter(|j| j.state != ConceptJobState::Done)
                    .partition(|j| !j.state.is_finished());
                Ok(json!({
                    "project": args.project,
                    "directions": directions,
                    "running": running.into_iter().map(job_value).collect::<Vec<_>>(),
                    "failed": failed.into_iter().map(job_value).collect::<Vec<_>>(),
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Make object refs from one concept: for each object named, Codex makes one image of only that object (centered, whole, front three-quarter view, plain light grey background, soft light, no text), with the concept attached as the reference. The images land in a new Lab direction \"<name> objects\", each titled by its object; read them with get_concepts. Each image spends Codex usage."
    )]
    async fn make_object_refs(&self, Parameters(args): Parameters<ObjectRefsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let dir = project::dir(&root, &config.layout, &args.project).map_err(reason)?;
                let name = args
                    .name
                    .clone()
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| source_name(&dir, &args.source));
                let request = NewObjectRefs {
                    project: args.project,
                    source: args.source,
                    source_name: name,
                    objects: args.objects,
                    notes: args.notes.unwrap_or_default(),
                    aspect: args.aspect.map_or(Aspect::Square, Aspect::from),
                    kind: match args.kind {
                        Some(VariantArg::Character) => AssetKind::Character,
                        _ => AssetKind::Prop,
                    },
                };
                let tools = host.toolbox();
                let (direction, jobs) =
                    lab::make_objects(host.session(), host.concepts(), &tools, &request)
                        .map_err(|e| e.message)?;
                let jobs: Vec<Value> = jobs
                    .iter()
                    .map(|j| {
                        json!({ "id": j.id, "object": j.title, "file": j.file, "state": to_value(j.state) })
                    })
                    .collect();
                Ok(json!({
                    "status": "started",
                    "direction": { "id": direction.id, "name": direction.name },
                    "jobs": jobs,
                    "note": "Each file appears at its path, relative to the project folder, once Codex saves it.",
                }))
            })
            .await;
        respond(found)
    }
}
