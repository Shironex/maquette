//! Which file a Blender thumbnail render reads. Blender cannot import KTX2
//! textures or meshopt geometry, so a compressed model is rendered from its
//! uncompressed export when one exists. The order, first match wins:
//!
//! 1. The file itself, when Blender reads it: any `.blend`, and any glTF
//!    without `KHR_texture_basisu` textures or meshopt geometry (Draco is
//!    fine, Blender decodes it).
//! 2. Its uncompressed export, the first of these that exists and that
//!    Blender reads:
//!    - `<dir>/../raw/<name>` when the file sits in a `dist` folder: the
//!      Compress step's output next to its input (`web/dist` and `web/raw`);
//!    - `<dir>/raw/<name>`;
//!    - `<project>/web/raw/<stem in snake case>.glb`, the pipeline's raw
//!      export of the matching model (the build step names a model's
//!      collection after its file, and the export names the GLB after the
//!      collection), for a file inside a project folder.
//! 3. Otherwise no render: the web view snapshots its own preview, which
//!    reads every codec, and stores that.
//!
//! The thumbnail is still keyed by the content of the file that was asked
//! for, so a new compressed build gets a fresh render.

use std::fs;
use std::path::{Path, PathBuf};

/// Extensions Blender's glTF importer cannot read.
pub const BLENDER_UNREADABLE: [&str; 3] = [
    "KHR_texture_basisu",
    "EXT_meshopt_compression",
    "KHR_meshopt_compression",
];

/// Where a render reads the model from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Blender reads the file itself.
    Original,
    /// The file is compressed; this uncompressed export stands in.
    Uncompressed(PathBuf),
    /// Nothing Blender can read: the preview snapshot stands in, for this
    /// reason.
    Snapshot(String),
}

/// The `extensionsUsed` of a `.glb` or `.gltf`, or `None` when the file
/// cannot be read as glTF.
pub fn extensions_used(file: &Path) -> Option<Vec<String>> {
    let bytes = fs::read(file).ok()?;
    let json: &[u8] = if bytes.starts_with(b"glTF") {
        // 12 byte header, then the JSON chunk: length, type, data.
        let len = u32::from_le_bytes(bytes.get(12..16)?.try_into().ok()?) as usize;
        if bytes.get(16..20)? != b"JSON" {
            return None;
        }
        bytes.get(20..20 + len)?
    } else {
        &bytes
    };
    let value: serde_json::Value = serde_json::from_slice(json).ok()?;
    let used = value
        .get("extensionsUsed")
        .and_then(|v| v.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|e| e.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Some(used)
}

/// The extensions of `file` Blender cannot import, empty when it reads the
/// file (a `.blend` always).
pub fn unreadable_extensions(file: &Path) -> Vec<String> {
    if is_blend(file) {
        return Vec::new();
    }
    extensions_used(file)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| BLENDER_UNREADABLE.contains(&e.as_str()))
        .collect()
}

fn is_blend(file: &Path) -> bool {
    file.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("blend"))
}

fn snake(stem: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in stem.chars() {
        if c.is_ascii_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('_');
            }
            gap = false;
            out.push(c.to_ascii_lowercase());
        } else {
            gap = true;
        }
    }
    if out.is_empty() {
        "model".to_string()
    } else {
        out
    }
}

/// The nearest folder at or above `dir` (three levels at most) that holds
/// a `project.json`.
fn project_of(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .take(4)
        .find(|d| d.join(crate::project::PROJECT_FILE).is_file())
        .map(Path::to_path_buf)
}

/// Where an uncompressed copy of `file` may be, in the order they are tried.
pub fn uncompressed_candidates(file: &Path) -> Vec<PathBuf> {
    let (Some(dir), Some(name)) = (file.parent(), file.file_name()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if dir
        .file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case("dist"))
        && let Some(up) = dir.parent()
    {
        out.push(up.join("raw").join(name));
    }
    out.push(dir.join("raw").join(name));
    if let (Some(project), Some(stem)) = (project_of(dir), file.file_stem()) {
        let raw = project
            .join("web")
            .join("raw")
            .join(format!("{}.glb", snake(&stem.to_string_lossy())));
        out.push(raw);
    }
    out.retain(|p| p != file);
    out.dedup();
    out
}

/// What a Blender render of `file` should read (see the module docs).
pub fn pick(file: &Path) -> Source {
    let blockers = unreadable_extensions(file);
    if blockers.is_empty() {
        return Source::Original;
    }
    for candidate in uncompressed_candidates(file) {
        if candidate.is_file()
            && extensions_used(&candidate).is_some()
            && unreadable_extensions(&candidate).is_empty()
        {
            return Source::Uncompressed(candidate);
        }
    }
    Source::Snapshot(format!(
        "Blender cannot read {}, and there is no uncompressed export of this model, so the preview stands in.",
        blockers.join(" or ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gltf(path: &Path, extensions: &[&str]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        let json = serde_json::json!({
            "asset": { "version": "2.0" },
            "extensionsUsed": extensions,
        });
        fs::write(path, serde_json::to_vec(&json).expect("json")).expect("write");
    }

    #[test]
    fn reads_the_json_chunk_of_a_glb() {
        let dir = tempfile::tempdir().expect("tempdir");
        let glb = dir.path().join("crate.glb");
        fs::write(&glb, crate::inbox::fixture::clean_prop()).expect("glb");
        assert_eq!(extensions_used(&glb), Some(Vec::new()));
        assert_eq!(pick(&glb), Source::Original);
        fs::write(dir.path().join("junk.glb"), b"not a model").expect("junk");
        assert_eq!(extensions_used(&dir.path().join("junk.glb")), None);
    }

    #[test]
    fn plain_files_and_blends_render_as_they_are() {
        let dir = tempfile::tempdir().expect("tempdir");
        let draco = dir.path().join("rock.gltf");
        gltf(&draco, &["KHR_draco_mesh_compression"]);
        assert_eq!(pick(&draco), Source::Original);
        assert_eq!(pick(&dir.path().join("scene.blend")), Source::Original);
    }

    #[test]
    fn a_compressed_build_renders_from_its_raw_export_first() {
        let dir = tempfile::tempdir().expect("tempdir");
        let web = dir.path().join("web");
        let shipped = web.join("dist/bench.gltf");
        gltf(&shipped, &["KHR_texture_basisu", "EXT_meshopt_compression"]);
        // No uncompressed copy yet: the snapshot stands in, and says why.
        match pick(&shipped) {
            Source::Snapshot(why) => {
                assert!(why.contains("KHR_texture_basisu or EXT_meshopt_compression"));
            }
            other => panic!("expected a snapshot, got {other:?}"),
        }
        gltf(&web.join("dist/raw/bench.gltf"), &[]);
        assert_eq!(
            pick(&shipped),
            Source::Uncompressed(web.join("dist/raw/bench.gltf"))
        );
        // The Compress step's own input, beside the dist folder, comes first.
        gltf(&web.join("raw/bench.gltf"), &[]);
        assert_eq!(
            pick(&shipped),
            Source::Uncompressed(web.join("raw/bench.gltf"))
        );
        // A raw copy that is itself compressed does not count.
        gltf(&web.join("raw/bench.gltf"), &["EXT_meshopt_compression"]);
        assert_eq!(
            pick(&shipped),
            Source::Uncompressed(web.join("dist/raw/bench.gltf"))
        );
    }

    #[test]
    fn a_project_model_finds_the_pipelines_matching_export() {
        let dir = tempfile::tempdir().expect("tempdir");
        let project = dir.path().join("moon-island");
        fs::create_dir_all(&project).expect("mkdir");
        fs::write(project.join(crate::project::PROJECT_FILE), b"{}").expect("manifest");
        let model = project.join("models/Sakura-Tree_v7.glb");
        gltf(&model, &["KHR_texture_basisu"]);
        let raw = project.join("web/raw/sakura_tree_v7.glb");
        assert_eq!(
            uncompressed_candidates(&model),
            vec![project.join("models/raw/Sakura-Tree_v7.glb"), raw.clone()]
        );
        gltf(&raw, &[]);
        assert_eq!(pick(&model), Source::Uncompressed(raw));
    }
}
