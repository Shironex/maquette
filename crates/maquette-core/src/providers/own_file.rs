//! A model file someone made or fixed themselves, added as an asset's next
//! version: it goes into the inbox for that asset as a download attached
//! by hand does ([`ProviderJobs::attach`]), so the usual checks run and
//! Accept makes the version. A `.glb` is copied as it is; a `.gltf`,
//! `.fbx` or `.obj` is turned into one by a headless Blender run first.
//! The file itself is never moved or changed.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use super::jobs::{ModelAttached, ProviderJobs};
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::pipeline::tools::Toolbox;
use crate::project;
use crate::workspace::WorkspaceConfig;

/// The model files that can become a version.
pub const MODEL_TYPES: [&str; 4] = ["glb", "gltf", "fbx", "obj"];
/// Turns one model into a GLB.
const SCRIPT: &str = include_str!("own_file/to_glb.py");
const SCRIPT_FILE: &str = "to_glb.py";
/// The workspace cache folder an own file is turned into a GLB in.
pub const WORK_DIR: &str = "own-files";
const OUT_FILE: &str = "model.glb";

fn ext_of(file: &Path) -> Option<String> {
    file.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|e| MODEL_TYPES.contains(&e.as_str()))
}

/// Checks `file` can become a version: an absolute path to a `.glb`,
/// `.gltf`, `.fbx` or `.obj` that exists, outside the workspace's inbox and
/// every project's `models` folder (a version already there is no new
/// file). Returns its real path and its type.
///
/// # Errors
///
/// Says which rule the path breaks.
pub fn check(root: &Path, config: &WorkspaceConfig, file: &Path) -> CoreResult<(PathBuf, String)> {
    if !file.is_absolute() {
        return Err(CoreError::Invalid(format!(
            "{} is not an absolute path. Give the full path to the model file.",
            file.display()
        )));
    }
    let Some(ext) = ext_of(file) else {
        return Err(CoreError::Invalid(format!(
            "{} is not a model file. Add a .glb, .gltf, .fbx or .obj.",
            file.display()
        )));
    };
    let real = fs::canonicalize(file).map_err(|e| CoreError::io(file, e))?;
    if !real.is_file() {
        return Err(CoreError::Invalid(format!(
            "{} is not a file.",
            file.display()
        )));
    }
    let real_root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let inbox = real_root.join(&config.layout.inbox);
    if real.starts_with(&inbox) {
        return Err(CoreError::Invalid(
            "That file is in the inbox already. Accept it there.".to_string(),
        ));
    }
    let projects = real_root.join(&config.layout.projects);
    if let Ok(rest) = real.strip_prefix(&projects) {
        let mut parts = rest.components();
        let _project = parts.next();
        if parts.next().is_some_and(|c| c.as_os_str() == "models") {
            return Err(CoreError::Invalid(
                "That file is a version already. Add a file from outside the project's models folder."
                    .to_string(),
            ));
        }
    }
    Ok((real, ext))
}

/// A fresh scratch folder under the workspace cache.
fn scratch(root: &Path, config: &WorkspaceConfig) -> CoreResult<PathBuf> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let work = root
        .join(&config.layout.cache)
        .join(WORK_DIR)
        .join(format!("{stamp}-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    fs::create_dir_all(&work).map_err(|e| CoreError::io(&work, e))?;
    Ok(work)
}

/// Turns `file` into a GLB in `work` with Blender and returns its path.
fn to_glb(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    work: &Path,
    file: &Path,
) -> CoreResult<PathBuf> {
    let out = work.join(OUT_FILE);
    let extra = [file.display().to_string(), out.display().to_string()];
    runner.run_script(tools, work, (SCRIPT_FILE, SCRIPT), &extra)?;
    if !out.is_file() {
        return Err(CoreError::Invalid(format!(
            "Blender could not turn {} into a GLB.",
            file.display()
        )));
    }
    Ok(out)
}

/// Where a model file is added from. `asset` is an id or a name.
pub struct OwnFile<'a> {
    pub root: &'a Path,
    pub config: &'a WorkspaceConfig,
    pub project: &'a str,
    pub asset: &'a str,
    pub file: &'a Path,
}

/// Adds `own.file` to the inbox for `own.asset`, for its newest waiting By
/// hand send when it has one (so accepting closes it) or aimed at the
/// asset. Accept it afterwards to make the version.
///
/// # Errors
///
/// Fails for a path [`check`] refuses, an unknown project or asset, or a
/// file Blender cannot read.
pub fn add(
    studio: &ProviderJobs,
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    own: &OwnFile<'_>,
) -> CoreResult<ModelAttached> {
    let (real, ext) = check(own.root, own.config, own.file)?;
    let manifest = project::load(own.root, &own.config.layout, own.project)?;
    let asset = match crate::assets::find_by_name(&manifest, own.asset) {
        Some(found) if found.id != own.asset => found.id.clone(),
        _ => crate::assets::find(&manifest, own.asset)?.id.clone(),
    };
    if ext == "glb" {
        return studio.attach(own.project, &asset, &real);
    }
    let work = scratch(own.root, own.config)?;
    let done = to_glb(runner, tools, &work, &real)
        .and_then(|glb| studio.attach(own.project, &asset, &glb));
    let _ = fs::remove_dir_all(&work);
    done
}
