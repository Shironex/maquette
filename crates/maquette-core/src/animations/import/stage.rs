//! Staging an import: every dropped file is copied into the scratch folder,
//! and a pack zip's FBX files are unzipped there, so Blender reads copies
//! and the originals are never touched. `staged.json` remembers them for
//! the commit.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::LeftOut;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;

pub(super) const STAGED_FILE: &str = "staged.json";
const IN_DIR: &str = "in";
/// One FBX inside a zip is never unpacked past this.
const MAX_ENTRY_BYTES: u64 = 512 * 1024 * 1024;

/// One FBX to read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StagedEntry {
    /// `<file>-<entry>`, the reader's job id.
    pub id: String,
    /// Its name: the file's, or its name inside the zip.
    pub file: String,
    /// Its copy, relative to the scratch folder.
    pub path: String,
}

/// One dropped file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StagedFile {
    pub name: String,
    pub archive: bool,
    pub bytes: u64,
    pub entries: Vec<StagedEntry>,
    /// What in it is not read, and why.
    pub left_out: Vec<LeftOut>,
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

fn is_fbx(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("fbx"))
}

fn is_zip(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

/// The FBX files of a zip, by their name inside it; macOS's resource forks
/// left out.
fn zip_entries(archive: &mut zip::ZipArchive<File>) -> Vec<String> {
    let mut names: Vec<String> = archive
        .file_names()
        .filter_map(Result::ok)
        .filter(|n| is_fbx(n) && !n.starts_with("__MACOSX/") && !n.contains("/._"))
        .map(|n| n.into_owned())
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

fn unzip(path: &Path, work: &Path, index: usize, out: &mut StagedFile) -> CoreResult<()> {
    let name = file_name(path);
    let reader = File::open(path).map_err(|e| CoreError::io(path, e))?;
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| CoreError::Invalid(format!("{name} is not a zip I can read. {e}")))?;
    let inner = zip_entries(&mut archive);
    if inner.is_empty() {
        out.left_out.push(LeftOut {
            file: name,
            reason: "it holds no FBX files".to_string(),
        });
        return Ok(());
    }
    for (n, entry_name) in inner.iter().enumerate() {
        let mut entry = archive.by_name(entry_name).map_err(|e| {
            CoreError::Invalid(format!("Could not read {entry_name} in {name}. {e}"))
        })?;
        let short = entry_name
            .rsplit('/')
            .next()
            .unwrap_or(entry_name)
            .to_string();
        if entry.size() > MAX_ENTRY_BYTES {
            out.left_out.push(LeftOut {
                file: short,
                reason: "it is too big to be a clip".to_string(),
            });
            continue;
        }
        let rel = format!("{IN_DIR}/{index}/{n}.fbx");
        let target = work
            .join(IN_DIR)
            .join(index.to_string())
            .join(format!("{n}.fbx"));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        let mut file = File::create(&target).map_err(|e| CoreError::io(&target, e))?;
        // The size in the header can lie, so the copy is capped as well.
        io::copy(&mut (&mut entry).take(MAX_ENTRY_BYTES), &mut file)
            .map_err(|e| CoreError::io(&target, e))?;
        out.entries.push(StagedEntry {
            id: format!("{index}-{n}"),
            file: short,
            path: rel,
        });
    }
    Ok(())
}

/// Copies each of `paths` into `work` (unzipping packs) and records them.
///
/// # Errors
///
/// Fails when none of them can be read, or the scratch folder cannot be
/// written.
pub(super) fn stage(work: &Path, paths: &[String]) -> CoreResult<Vec<StagedFile>> {
    if paths.is_empty() {
        return Err(CoreError::Invalid(
            "Pick FBX files or a pack zip to import.".to_string(),
        ));
    }
    let mut staged = Vec::new();
    for (index, raw) in paths.iter().enumerate() {
        let path = Path::new(raw);
        let name = file_name(path);
        let bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let mut out = StagedFile {
            name: name.clone(),
            archive: is_zip(&name),
            bytes,
            entries: Vec::new(),
            left_out: Vec::new(),
        };
        if !path.is_file() {
            out.left_out.push(LeftOut {
                file: name,
                reason: "the file is not there".to_string(),
            });
        } else if out.archive {
            unzip(path, work, index, &mut out)?;
        } else if is_fbx(&name) {
            let rel = format!("{IN_DIR}/{index}/0.fbx");
            crate::assets::copy::copy_file(path, &work.join(&rel))?;
            out.entries.push(StagedEntry {
                id: format!("{index}-0"),
                file: name,
                path: rel,
            });
        } else {
            out.left_out.push(LeftOut {
                file: name,
                reason: "it is not an FBX file or a zip of them".to_string(),
            });
        }
        staged.push(out);
    }
    write_json(&work.join(STAGED_FILE), &staged)?;
    Ok(staged)
}

pub(super) fn load(work: &Path) -> CoreResult<Vec<StagedFile>> {
    let file = work.join(STAGED_FILE);
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))
}

/// The staged FBX a plan clip key (`<entry id>/<clip>`) names.
pub(super) fn entry_of<'a>(staged: &'a [StagedFile], key: &str) -> Option<&'a StagedEntry> {
    let (id, _) = key.rsplit_once('/')?;
    staged.iter().flat_map(|f| &f.entries).find(|e| e.id == id)
}
