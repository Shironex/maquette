//! Wavefront `.obj` files for the game import: their numbers, and the
//! `.mtl` and texture files that come with them. Only reads.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::assets::ModelStats;
use crate::error::{CoreError, CoreResult};
use crate::inbox::images;

/// The `.mtl` keywords that name a texture file.
const MAP_KEYWORDS: [&str; 9] = [
    "map_Kd", "map_Ka", "map_Ks", "map_Ke", "map_d", "map_Bump", "map_bump", "bump", "norm",
];

/// An `.obj` and the files it reads.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ObjModel {
    pub(crate) stats: ModelStats,
    /// The `.mtl` files its `mtllib` lines name, that exist.
    pub(crate) materials: Vec<PathBuf>,
    /// The texture files those name, that exist.
    pub(crate) textures: Vec<PathBuf>,
}

/// The file a map line names: its last word, after the options. Paths are
/// relative to the `.mtl`, with `/` or `\`.
fn map_file(line: &str) -> Option<&str> {
    let mut words = line.split_whitespace();
    let keyword = words.next()?;
    if !MAP_KEYWORDS.contains(&keyword) {
        return None;
    }
    words.last()
}

fn file_bytes(path: &Path) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "model sizes stay far below 2^53"
    )]
    fs::metadata(path).map_or(0.0, |m| m.len() as f64)
}

/// Reads an `.obj`: triangles (a face of n corners is n - 2), the
/// materials it names and their textures, and their sizes on disk.
pub(crate) fn read(path: &Path) -> CoreResult<ObjModel> {
    let text = fs::read_to_string(path).map_err(|e| CoreError::io(path, e))?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut triangles = 0u32;
    let mut materials = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        if let Some(face) = line.strip_prefix("f ") {
            let corners = u32::try_from(face.split_whitespace().count()).unwrap_or(0);
            triangles = triangles.saturating_add(corners.saturating_sub(2));
        } else if let Some(names) = line.strip_prefix("mtllib ") {
            let file = dir.join(names.trim().replace('\\', "/"));
            if file.is_file() && !materials.contains(&file) {
                materials.push(file);
            }
        }
    }
    let mut textures: BTreeSet<PathBuf> = BTreeSet::new();
    for mtl in &materials {
        let Ok(text) = fs::read_to_string(mtl) else {
            continue;
        };
        let base = mtl.parent().unwrap_or(Path::new("."));
        for line in text.lines() {
            if let Some(file) = map_file(line.trim()) {
                let tex = base.join(file.replace('\\', "/"));
                if tex.is_file() {
                    textures.insert(tex);
                }
            }
        }
    }
    let mut max_texture = 0;
    for tex in &textures {
        if let Some((w, h)) = fs::read(tex).ok().and_then(|b| images::dimensions(&b)) {
            max_texture = max_texture.max(w.max(h));
        }
    }
    let bytes = file_bytes(path)
        + materials.iter().map(|m| file_bytes(m)).sum::<f64>()
        + textures.iter().map(|t| file_bytes(t)).sum::<f64>();
    Ok(ObjModel {
        stats: ModelStats {
            triangles,
            bytes,
            textures: u32::try_from(textures.len()).unwrap_or(u32::MAX),
            max_texture,
            joints: 0,
            animations: 0,
        },
        materials,
        textures: textures.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_count_as_triangles_and_maps_resolve_from_the_mtl() {
        let dir = tempfile::tempdir().expect("dir");
        let models = dir.path().join("models");
        let textures = dir.path().join("textures");
        fs::create_dir_all(&models).expect("models");
        fs::create_dir_all(&textures).expect("textures");
        fs::write(
            models.join("stile.obj"),
            "mtllib stile.mtl\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 2 3 4\n",
        )
        .expect("obj");
        fs::write(
            models.join("stile.mtl"),
            "newmtl wood\nmap_Kd ../textures/wood.png\nmap_Bump -bm 1.0 ../textures/missing.png\n",
        )
        .expect("mtl");
        fs::write(textures.join("wood.png"), images::write_png(4, 2, &[0; 24])).expect("png");
        let model = read(&models.join("stile.obj")).expect("read");
        assert_eq!(model.stats.triangles, 3);
        assert_eq!(model.stats.textures, 1, "a missing map is left out");
        assert_eq!(model.stats.max_texture, 4);
        assert_eq!(model.materials, [models.join("stile.mtl")]);
        assert_eq!(model.textures, [models.join("../textures/wood.png")]);
    }
}
