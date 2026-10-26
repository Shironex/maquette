//! Scanning a folder of existing work, and inspecting a site repo, for the
//! import flow.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoInspection {
    pub path: String,
    pub exists: bool,
    pub is_git: bool,
    pub has_public_dir: bool,
    /// Where Ship writes in this site, from its `maquette.site.json`.
    pub site: SiteFolders,
    /// A `CMakeLists.txt` at the root: more likely a game than a site.
    pub looks_like_game: bool,
}

/// The folders Ship writes in a site, as New project says them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SiteFolders {
    /// The site has a `maquette.site.json`.
    pub config: bool,
    pub models: String,
    /// None copies no posters.
    pub posters: Option<String>,
    /// None writes no scene data.
    pub data: Option<String>,
    /// Why the config could not be read, when it could not.
    pub error: Option<String>,
}

fn site_folders(path: &Path) -> SiteFolders {
    let config = path.join(crate::site::CONFIG_FILE).is_file();
    let (found, error) = match crate::site::load_config(path) {
        Ok(found) => (found, None),
        Err(err) => (crate::site::SiteConfig::legacy(), Some(err.to_string())),
    };
    SiteFolders {
        config,
        models: found.models,
        posters: found.posters,
        data: found.data,
        error,
    }
}

pub fn inspect_site_repo(path: &Path) -> RepoInspection {
    RepoInspection {
        path: path.display().to_string(),
        exists: path.is_dir(),
        // `.git` is a file in worktrees and submodules.
        is_git: path.join(".git").exists(),
        has_public_dir: path.join("public").is_dir(),
        site: site_folders(path),
        looks_like_game: path.join("CMakeLists.txt").is_file(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BlendFile {
    /// Path relative to the scanned folder, with `/` separators.
    pub file: String,
    pub stem: String,
    pub version: Option<u32>,
}

impl BlendFile {
    /// `moon_island_v7.blend` gives stem `moon_island` and version 7. The
    /// separator before the `v` may be `_`, `-` or a space.
    pub fn parse(file: &str) -> Self {
        let name = file.rsplit('/').next().unwrap_or(file);
        let base = match name.rsplit_once('.') {
            Some((base, _)) => base,
            None => name,
        };
        let (stem, version) = split_version(base);
        Self {
            file: file.to_string(),
            stem: stem.to_string(),
            version,
        }
    }
}

fn split_version(base: &str) -> (&str, Option<u32>) {
    let digits = base.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return (base, None);
    }
    let head = &base[..base.len() - digits];
    let Some(before_v) = head.strip_suffix(['v', 'V']) else {
        return (base, None);
    };
    let Some(stem) = before_v.strip_suffix(['_', '-', ' ']) else {
        return (base, None);
    };
    let stem = stem.trim_end();
    match base[base.len() - digits..].parse() {
        Ok(version) if !stem.is_empty() => (stem, Some(version)),
        _ => (base, None),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportScan {
    pub path: String,
    pub blend_files: Vec<BlendFile>,
    /// File names of `.py`, `.mjs`, `.js` and `.sh` scripts directly inside
    /// `<folder>/tools/`.
    pub tool_scripts: Vec<String>,
}

pub(crate) fn has_ext(path: &Path, exts: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

pub(crate) fn sorted_children(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Looks for `.blend` files (top level and one folder down, `.blend1`
/// backups ignored) and tool scripts in an existing project folder.
pub fn scan_import(folder: &Path) -> CoreResult<ImportScan> {
    if !folder.is_dir() {
        return Err(CoreError::NotFound(format!(
            "the folder {}",
            folder.display()
        )));
    }
    let mut blend_files = Vec::new();
    for (name, path) in sorted_children(folder) {
        if path.is_file() && has_ext(&path, &["blend"]) {
            blend_files.push(BlendFile::parse(&name));
        } else if path.is_dir() && !name.starts_with('.') {
            for (child, child_path) in sorted_children(&path) {
                if child_path.is_file() && has_ext(&child_path, &["blend"]) {
                    blend_files.push(BlendFile::parse(&format!("{name}/{child}")));
                }
            }
        }
    }
    let tool_scripts = sorted_children(&folder.join("tools"))
        .into_iter()
        .filter(|(_, path)| path.is_file() && has_ext(path, &["py", "mjs", "js", "sh"]))
        .map(|(name, _)| name)
        .collect();
    Ok(ImportScan {
        path: folder.display().to_string(),
        blend_files,
        tool_scripts,
    })
}
