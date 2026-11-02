//! Which refs belong to an asset when nobody tagged them: the images sent
//! from the Lab direction of its name (or object refs titled like it),
//! else the files in `refs/` named after it.

use std::fs;
use std::path::Path;

use crate::assets::Asset;
use crate::concepts::{self, REFS_DIR};
use crate::project;

const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// Every image in `refs/` (not its subfolders), relative to the project
/// folder, by name.
pub fn ref_files(project_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(project_dir.join(REFS_DIR)) else {
        return Vec::new();
    };
    let mut out: Vec<String> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && !crate::workspace::sync::is_sync_path(p))
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .filter_map(|p| Some(format!("{REFS_DIR}/{}", p.file_name()?.to_string_lossy())))
        .collect();
    out.sort();
    out
}

/// The refs an asset came with, in the order they always went: images
/// sent from the Lab direction of the same name (or the name it was first
/// made with, which its id keeps) or object refs titled like it, else the
/// files in `refs/` named after the asset. `files` is [`ref_files`].
pub fn suggested(project_dir: &Path, asset: &Asset, files: &[String]) -> Vec<String> {
    let named = |text: &str| {
        text.trim().eq_ignore_ascii_case(asset.name.trim())
            || project::slugify(text.trim()) == asset.id
            || project::folder_slug(text.trim()) == asset.id
    };
    let mut from_lab: Vec<String> = Vec::new();
    for d in concepts::list_directions(project_dir) {
        // Views and clean copies arrive through the asset's own refs.
        if d.refs_job.is_some() {
            continue;
        }
        let whole = named(&d.name);
        for image in d.images {
            let mine = whole || image.title.as_deref().is_some_and(named);
            if let Some(file) = image.ref_file.filter(|_| mine)
                && files.contains(&file)
                && !from_lab.contains(&file)
            {
                from_lab.push(file);
            }
        }
    }
    if !from_lab.is_empty() {
        return from_lab;
    }
    let slug = asset.id.replace('-', "_");
    files
        .iter()
        .filter(|f| {
            let name = f.rsplit('/').next().unwrap_or(f);
            name.to_lowercase().replace('-', "_").starts_with(&slug)
        })
        .cloned()
        .collect()
}
