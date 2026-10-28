//! Committing an import: the chosen clips are copied from the scratch
//! folder into the library, a revision keeps its clip's id and tags.

use std::fs;
use std::path::Path;

use super::{
    ClipStatus, ImportFile, ImportRequest, Imported, PlanClip, StagedEntry, classify, stage,
};
use crate::animations::blender::{self, FileReport};
use crate::animations::{AnimationClip, ClipOrigin, ClipRig, ClipSource};
use crate::error::{CoreError, CoreResult};
use crate::workspace::WorkspaceConfig;

/// The scratch folder of an import that was read.
fn work_of(root: &Path, config: &WorkspaceConfig, token: &str) -> CoreResult<std::path::PathBuf> {
    let work = root
        .join(&config.layout.cache)
        .join(crate::animations::ANIMATIONS_DIR)
        .join(token);
    if !token.starts_with("import-")
        || !crate::workspace::is_single_segment(token)
        || !work.join(stage::STAGED_FILE).is_file()
    {
        return Err(CoreError::NotFound(
            "that import. Add the files again".to_string(),
        ));
    }
    Ok(work)
}

/// Adds the clips of an import that was read, as `request` chose, then
/// removes its scratch folder.
///
/// # Errors
///
/// Fails for an import that is gone, or when a clip cannot be written.
pub fn commit(
    root: &Path,
    config: &WorkspaceConfig,
    request: &ImportRequest,
) -> CoreResult<Imported> {
    let work = work_of(root, config, &request.token)?;
    let staged = stage::load(&work)?;
    let report = blender::read_report(&work)?;
    let _turn = crate::animations::turn();
    let existing = crate::animations::list(root)?;
    let plan = classify::classify(&request.token, &staged, &report, &existing, Some(request));
    let mut done = Imported {
        added: Vec::new(),
        revised: Vec::new(),
        skipped: 0,
        left_out: plan.left_out,
    };
    let mut taken: Vec<String> = existing.iter().map(|c| c.id.clone()).collect();
    for file in &plan.files {
        for clip in &file.clips {
            if !classify::goes_in(clip, Some(request)) {
                done.skipped += 1;
                continue;
            }
            let entry = stage::entry_of(&staged, &clip.key)
                .ok_or_else(|| CoreError::NotFound(format!("the file of {}", clip.name)))?;
            let read = report.files.iter().find(|f| f.id == entry.id);
            let out = read.and_then(|f| {
                let n: usize = clip.key.rsplit('/').next()?.parse().ok()?;
                f.clips.get(n)
            });
            let (Some(read), Some(out)) = (read, out) else {
                continue;
            };
            let parts = Parts {
                work: &work,
                file,
                entry,
                read,
                out,
                keep: request.keep_files,
            };
            let revise = clip.status == ClipStatus::Revision
                && !request.keep_both.iter().any(|k| k == &clip.key);
            let previous = revise
                .then(|| clip.matched.as_ref().and_then(|m| m.id.as_deref()))
                .flatten()
                .and_then(|id| existing.iter().find(|c| c.id == id));
            let made = write_clip(root, clip, &parts, previous, &mut taken)?;
            if previous.is_some() {
                done.revised.push(made);
            } else {
                done.added.push(made);
            }
        }
    }
    if let Err(err) = fs::remove_dir_all(&work) {
        tracing::warn!(error = %err, "could not remove the import's scratch folder");
    }
    Ok(done)
}

/// What one clip is written from.
struct Parts<'a> {
    work: &'a Path,
    file: &'a ImportFile,
    entry: &'a StagedEntry,
    read: &'a FileReport,
    out: &'a blender::ClipReport,
    keep: bool,
}

fn write_clip(
    root: &Path,
    clip: &PlanClip,
    parts: &Parts<'_>,
    previous: Option<&AnimationClip>,
    taken: &mut Vec<String>,
) -> CoreResult<AnimationClip> {
    let rel = parts
        .out
        .out
        .as_deref()
        .ok_or_else(|| CoreError::NotFound(format!("the skeleton of {}", clip.name)))?;
    let id = previous.map_or_else(
        || crate::animations::free_id(root, taken, &clip.name),
        |p| p.id.clone(),
    );
    taken.push(id.clone());
    let folder = crate::animations::clip_dir(root, &id)?;
    let from = parts.work.join(blender::OUT_DIR).join(rel);
    crate::assets::copy::copy_file(&from, &folder.join(crate::animations::CLIP_GLB))?;
    let kept = if parts.keep {
        let ext = Path::new(&parts.entry.file)
            .extension()
            .map_or_else(|| "fbx".to_string(), |e| e.to_string_lossy().to_lowercase());
        let name = format!("source.{ext}");
        crate::assets::copy::copy_file(&parts.work.join(&parts.entry.path), &folder.join(&name))?;
        Some(name)
    } else {
        None
    };
    let now = crate::project::now_rfc3339();
    let made = AnimationClip {
        version: crate::animations::CLIP_VERSION,
        id,
        name: clip.name.clone(),
        pack: clip.pack.clone(),
        tags: previous.map_or_else(|| clip.tags.clone(), |p| p.tags.clone()),
        rig: ClipRig {
            kind: clip.rig,
            bones: parts.read.bones.clone(),
            hips_height: parts.read.hips_height,
        },
        frames: clip.frames,
        fps: clip.fps,
        length: clip.length,
        root_motion: clip.root_motion.clone(),
        loops: clip.loops,
        bytes: clip.bytes,
        file: crate::animations::CLIP_GLB.to_string(),
        origin: ClipOrigin {
            source: ClipSource::Import,
            file: parts.entry.file.clone(),
            archive: parts.file.archive.then(|| parts.file.name.clone()),
            kept,
            project: None,
            asset: None,
            version: None,
        },
        hash: parts.out.hash.clone(),
        revision: previous.map_or(1, |p| p.revision + 1),
        created_at: previous.map_or_else(|| now.clone(), |p| p.created_at.clone()),
        updated_at: now,
    };
    crate::animations::store(root, &made)?;
    Ok(made)
}
