//! Tools for the animation library: clips kept once outside any project,
//! skeleton only. Importing reads FBX files and pack zips headless; saving
//! copies a rigged model's clips in. Blender only ever runs headless and
//! none of it spends credits.

use std::path::Path;

use maquette_core::animations::import::ClipStatus;
use maquette_core::animations::{AnimationItem, ClipsFrom, ImportRequest, SaveClips};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::animations as commands;

use super::{McpTools, Out, respond, to_short_value, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAnimationsArgs {
    /// Words to look for in the clip names, packs and tags.
    pub query: Option<String>,
    /// Only clips in this pack.
    pub pack: Option<String>,
    /// Only clips with this tag.
    pub tag: Option<String>,
    /// Only clips made on this rig: mixamo, tripo or custom.
    pub rig: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportAnimationsArgs {
    /// FBX files or pack zips of them, inside the workspace (the library or a
    /// project included). Absolute, or relative to the workspace root.
    pub paths: Vec<String>,
    /// The pack single FBX files go to. Defaults to Singles; a zip's clips go
    /// to a pack named after the zip.
    pub singles: Option<String>,
    /// Also add clips whose keys the library has already, as clips of their
    /// own. Defaults to false: they are skipped.
    #[serde(default)]
    pub keep_duplicates: bool,
    /// Keep each FBX next to its clip. Defaults to false: only the skeleton
    /// and keys are kept.
    #[serde(default)]
    pub keep_files: bool,
    /// Only say what would be added.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveModelClipsArgs {
    /// The project id, with asset: its newest model's clips.
    pub project: Option<String>,
    /// The asset id, as list_assets returns it.
    pub asset: Option<String>,
    /// Or an Inbox file name, for a Tripo job's animations.
    pub inbox: Option<String>,
    /// The clips to save, by name. Defaults to every clip that did not come
    /// from the library.
    pub clips: Option<Vec<String>>,
    /// The pack they go to. Defaults to "<model> moves".
    pub pack: Option<String>,
    /// Tags for every saved clip. Defaults to tags read from the clip names.
    pub tags: Option<Vec<String>>,
    /// Only say what would be saved.
    #[serde(default)]
    pub dry_run: bool,
}

fn matches(item: &AnimationItem, args: &ListAnimationsArgs) -> bool {
    let clip = &item.clip;
    let lower = |s: &str| s.trim().to_lowercase();
    let pack = args
        .pack
        .as_deref()
        .is_none_or(|p| lower(&clip.pack) == lower(p));
    let tag = args
        .tag
        .as_deref()
        .is_none_or(|t| clip.tags.contains(&lower(t)));
    let rig = args
        .rig
        .as_deref()
        .is_none_or(|r| to_value(clip.rig.kind).as_str() == Some(lower(r).as_str()));
    let words = args
        .query
        .as_deref()
        .map(str::to_lowercase)
        .unwrap_or_default();
    let text = format!("{} {} {}", clip.name, clip.pack, clip.tags.join(" ")).to_lowercase();
    pack && tag && rig && words.split_whitespace().all(|w| text.contains(w))
}

fn listed(item: &AnimationItem) -> Value {
    let clip = &item.clip;
    json!({
        "id": clip.id,
        "name": clip.name,
        "pack": clip.pack,
        "tags": clip.tags,
        "rig": to_value(clip.rig.kind),
        "bones": clip.rig.bones.len(),
        "frames": clip.frames,
        "fps": to_short_value(clip.fps),
        "seconds": to_short_value(clip.length),
        "rootMotion": clip.root_motion.as_ref().map(to_short_value),
        "loops": clip.loops,
        "bytes": clip.bytes,
        "revision": clip.revision,
        "from": clip.origin.file,
    })
}

/// The paths as absolute ones, refusing any outside the workspace.
fn inside(root: &Path, paths: &[String]) -> Result<Vec<String>, String> {
    let base = root.canonicalize().map_err(|e| e.to_string())?;
    paths
        .iter()
        .map(|p| {
            let path = Path::new(p.trim());
            let full = if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            };
            let real = full
                .canonicalize()
                .map_err(|_| format!("{} is not there.", full.display()))?;
            if real.starts_with(&base) {
                Ok(real.display().to_string())
            } else {
                Err(format!(
                    "{} is outside the workspace. Copy it into the workspace first.",
                    full.display()
                ))
            }
        })
        .collect()
}

#[tool_router(router = animations_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List the animation library: clips kept once outside any project (library/animations at the workspace root), skeleton only, that rigged models can take. Each clip has its pack, tags, source rig (mixamo, tripo or custom) and bone count, frames, rate, seconds, root motion, whether it loops, its size and the file it came from. Filter with query, pack, tag and rig. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn list_animations(&self, Parameters(args): Parameters<ListAnimationsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = commands::get(host.session())?;
                let clips: Vec<Value> = view
                    .items
                    .iter()
                    .filter(|i| matches(i, &args))
                    .map(listed)
                    .collect();
                Ok(json!({ "dir": view.dir, "total": view.items.len(), "clips": clips }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Import animation clips into the library from Mixamo FBX files or pack zips inside the workspace. Blender reads them headless: each FBX is one clip, the skin is dropped and only the skeleton and keys are kept; the rig is read from the bone names; a file with no keys is left out; a clip whose keys the library has is skipped (keepDuplicates adds it anyway); new keys for a clip of the same name and pack replace it as its next revision, and models that baked it are offered an update. A zip's clips go to a pack named after it, single files to Singles. Mixamo clips may be used in your projects but not redistributed as raw files: the library stays on this machine. dryRun only says what would be added. Spends nothing."
    )]
    async fn import_animations(&self, Parameters(args): Parameters<ImportAnimationsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, _) = workspace(host)?;
                let paths = inside(&root, &args.paths)?;
                let tools = host.toolbox();
                let plan = commands::read_import(host.session(), &tools, &paths)?;
                let keep_both: Vec<String> = plan
                    .files
                    .iter()
                    .flat_map(|f| &f.clips)
                    .filter(|c| args.keep_duplicates && c.status == ClipStatus::Duplicate)
                    .map(|c| c.key.clone())
                    .collect();
                let request = ImportRequest {
                    token: plan.token.clone(),
                    singles: args.singles.clone(),
                    keep_files: args.keep_files,
                    keep_both,
                    ..ImportRequest::default()
                };
                if args.dry_run {
                    return Ok(json!({ "dryRun": true, "plan": to_short_value(&plan) }));
                }
                let done = commands::import(host.session(), &request)?;
                host.library_changed();
                let names = |list: &[maquette_core::animations::AnimationClip]| {
                    list.iter()
                        .map(|c| json!({ "id": c.id, "name": c.name, "pack": c.pack }))
                        .collect::<Vec<_>>()
                };
                let left: Vec<Value> = plan
                    .files
                    .iter()
                    .flat_map(|f| &f.left_out)
                    .map(|l| json!({ "file": l.file, "reason": l.reason }))
                    .collect();
                Ok(json!({
                    "added": names(&done.added),
                    "revised": names(&done.revised),
                    "skippedDuplicates": done.skipped,
                    "leftOut": left,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Save a rigged model's clips to the animation library as a pack: the newest version of a project asset (project and asset), or a Tripo job's animations waiting in the Inbox (inbox, its file name). Only the skeleton and the clips' keys go in; the mesh stays where it is. Clips that came from the library are never saved back, and keys the library has already are skipped. The rig is read from the bone names (a Tripo job's rig is tripo). dryRun lists the clips the model offers. Runs Blender headless; spends nothing."
    )]
    async fn save_model_clips(&self, Parameters(args): Parameters<SaveModelClipsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let from = ClipsFrom {
                    project: args.project.clone(),
                    asset: args.asset.clone(),
                    inbox: args.inbox.clone(),
                };
                let plan = commands::save_plan(host.session(), host.studio(), &from)?;
                if args.dry_run {
                    return Ok(json!({ "dryRun": true, "plan": to_short_value(&plan) }));
                }
                let clips = args.clips.clone().unwrap_or_else(|| {
                    plan.clips
                        .iter()
                        .filter(|c| c.from_library.is_none())
                        .map(|c| c.name.clone())
                        .collect()
                });
                let request = SaveClips {
                    from,
                    clips,
                    pack: args.pack.clone().unwrap_or_else(|| plan.pack.clone()),
                    tags: args.tags.clone().unwrap_or_default(),
                };
                let tools = host.toolbox();
                let saved = commands::save(host.session(), host.studio(), &tools, &request)?;
                host.library_changed();
                Ok(json!({
                    "pack": request.pack,
                    "added": saved.added.iter().map(|c| json!({ "id": c.id, "name": c.name })).collect::<Vec<_>>(),
                    "skipped": saved.skipped,
                }))
            })
            .await;
        respond(found)
    }
}
