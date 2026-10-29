//! Adding a built asset: its script and helpers are copied into the
//! project's `built/` folder (a file already there with the same content is
//! shared), the asset goes on the Board at Blender and `built.json` records
//! where its files are. Nothing is built here; the builder in core's `built` module does that.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{BUILT_DIR, BuiltSource, MAX_FILE_BYTES, MAX_HELPERS, ROOT_MAX_CHARS, update};
use crate::assets::{self, Asset, AssetKind, NewAsset, Stage};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest};
use crate::workspace::{FolderLayout, is_single_segment};

/// What Add built asset asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewBuilt {
    pub name: String,
    pub kind: AssetKind,
    /// The script, an absolute path or one relative to the project folder.
    pub script: String,
    /// Files the script imports or reads, the same way.
    pub helpers: Vec<String>,
    /// The object to export with its children, when `build()` does not
    /// return it.
    pub root: Option<String>,
    /// Overwrite files in `built/` that differ from these.
    pub replace: bool,
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

/// A file to bring in: where it is and the name it gets in `built/`.
struct Incoming {
    from: PathBuf,
    name: String,
}

fn incoming(project_dir: &Path, raw: &str, what: &str) -> CoreResult<Incoming> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(invalid(format!("Name the {what} file.")));
    }
    let given = Path::new(raw);
    let from = if given.is_absolute() {
        given.to_path_buf()
    } else {
        project_dir.join(given)
    };
    let meta = fs::metadata(&from)
        .map_err(|_| invalid(format!("There is no {what} file at {}.", from.display())))?;
    if !meta.is_file() {
        return Err(invalid(format!("{} is not a file.", from.display())));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(invalid(format!(
            "{} is over 50 MB, too large for a built asset.",
            from.display()
        )));
    }
    let name = from
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| is_single_segment(n) && !n.starts_with('.'))
        .ok_or_else(|| invalid(format!("{} has no usable file name.", from.display())))?;
    Ok(Incoming { from, name })
}

fn clean_root(root: Option<&str>) -> CoreResult<Option<String>> {
    let Some(root) = root.map(str::trim).filter(|r| !r.is_empty()) else {
        return Ok(None);
    };
    if root.len() > ROOT_MAX_CHARS {
        return Err(invalid(format!(
            "Keep the root object name to {ROOT_MAX_CHARS} characters, as Blender does."
        )));
    }
    Ok(Some(root.to_string()))
}

/// The files to bring in, checked whole: a script ending in `.py`, unique
/// names and no clash with a different file in `built/` unless `replace`.
fn plan(project_dir: &Path, new: &NewBuilt) -> CoreResult<Vec<Incoming>> {
    if new.helpers.len() > MAX_HELPERS {
        return Err(invalid(format!(
            "A built asset takes up to {MAX_HELPERS} helper files."
        )));
    }
    let script = incoming(project_dir, &new.script, "script")?;
    if !script.name.to_lowercase().ends_with(".py") {
        return Err(invalid(format!(
            "{} is not a Python script. Pick the .py file that builds the model.",
            script.name
        )));
    }
    let mut files = vec![script];
    for helper in &new.helpers {
        let file = incoming(project_dir, helper, "helper")?;
        if files.iter().any(|f| f.name == file.name) {
            if files.iter().any(|f| f.from == file.from) {
                continue;
            }
            return Err(invalid(format!(
                "Two files are called {}. Each file in built/ needs its own name.",
                file.name
            )));
        }
        files.push(file);
    }
    let dir = project_dir.join(BUILT_DIR);
    let clashes: Vec<String> = files
        .iter()
        .filter(|f| differs(&f.from, &dir.join(&f.name)))
        .map(|f| format!("{BUILT_DIR}/{}", f.name))
        .collect();
    if !clashes.is_empty() && !new.replace {
        return Err(invalid(format!(
            "{} already in the project with other content. Replace them, or rename the files.",
            match clashes.as_slice() {
                [one] => format!("{one} is"),
                many => format!("{} are", many.join(", ")),
            }
        )));
    }
    Ok(files)
}

/// True when `target` exists and is not the same file or the same bytes.
fn differs(from: &Path, target: &Path) -> bool {
    if !target.exists() {
        return false;
    }
    let same_file = match (fs::canonicalize(from), fs::canonicalize(target)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    !same_file && fs::read(from).ok() != fs::read(target).ok()
}

fn copy_in(dir: &Path, files: &[Incoming]) -> CoreResult<()> {
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    for file in files {
        let target = dir.join(&file.name);
        if !differs(&file.from, &target) && target.exists() {
            continue;
        }
        let bytes = fs::read(&file.from).map_err(|e| CoreError::io(&file.from, e))?;
        crate::fs::write_atomic(&target, &bytes).map_err(|e| CoreError::io(&target, e))?;
    }
    Ok(())
}

/// Adds a built asset to the project: copies its files into `built/`,
/// puts the asset on the Board at Blender (queued until it builds) and
/// records it in `built.json`. Returns the asset and the project.
///
/// # Errors
///
/// Refuses, writing nothing, an empty or taken name, a missing script, a
/// script that is not `.py`, two files with one name, or a file that would
/// overwrite a different one in `built/` without `replace`.
pub fn add(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    new: &NewBuilt,
) -> CoreResult<(Asset, ProjectManifest)> {
    let project_dir = project::dir(workspace_root, layout, project_id)?;
    let current = project::load(workspace_root, layout, project_id)?;
    let name = assets::clean_name(&new.name)?;
    if let Some(taken) = assets::find_by_name(&current, &name) {
        return Err(invalid(format!(
            "{} is already an asset here. Give the built asset another name.",
            taken.name
        )));
    }
    let root = clean_root(new.root.as_deref())?;
    let files = plan(&project_dir, new)?;
    super::load(&project_dir)?;
    copy_in(&project_dir.join(BUILT_DIR), &files)?;
    let mut added: Option<Asset> = None;
    let manifest = project::modify(workspace_root, layout, project_id, |manifest| {
        added = Some(assets::add(
            manifest,
            NewAsset {
                name,
                kind: new.kind,
                stage: Stage::Blender,
            },
        )?);
        Ok(())
    })?;
    let asset = added.ok_or_else(|| invalid("The built asset was not added."))?;
    let rel = |f: &Incoming| format!("{BUILT_DIR}/{}", f.name);
    let source = BuiltSource {
        script: rel(&files[0]),
        helpers: files[1..].iter().map(rel).collect(),
        root,
        last: None,
    };
    let recorded = update(&project_dir, |file| {
        file.assets.insert(asset.id.clone(), source);
        Ok(())
    });
    if let Err(err) = recorded {
        // Without its record the card would be a plain asset: take it back.
        let _ = project::modify(workspace_root, layout, project_id, |m| {
            m.assets.retain(|a| a.id != asset.id);
            Ok(())
        });
        return Err(err);
    }
    Ok((asset, manifest))
}
