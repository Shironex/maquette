//! Baking the texture of a model back onto the parts a provider split it
//! into, free and headless (`bake_back.py`). Meshy's Auto Split always
//! drops the texture and Tripo's may: each part gets its own UV map and
//! image, and the base colour of the model it came from bakes onto it,
//! selected to active, by ray distance. The parts keep their names,
//! places and shapes.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::blender::{self, Script};
use crate::base::ports::TextureBake;
use crate::error::{CoreError, CoreResult};
use crate::pipeline::tools::Toolbox;

pub const SCRIPT: &str = include_str!("bake_back.py");
const SCRIPT_FILE: &str = "bake_back.py";
const OUTPUT_FILE: &str = "baked.glb";
const STATS_FILE: &str = "stats.json";
/// The edge of each part's baked image, in pixels.
pub const IMAGE_SIZE: u32 = 1024;

/// What the bake made, per part.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BakedPart {
    pub name: String,
    /// The mean colour of the texels the bake filled, 0 to 1.
    pub mean: [f32; 3],
}

#[derive(Debug, Deserialize)]
struct Stats {
    parts: Vec<BakedPart>,
}

/// The script's arguments for one bake.
pub(crate) fn script(source: &Path, target: &Path, size: u32) -> Script<'static> {
    let args = [
        "--source".to_string(),
        source.display().to_string(),
        "--target".to_string(),
        target.display().to_string(),
        "--output".to_string(),
        OUTPUT_FILE.to_string(),
        "--stats".to_string(),
        STATS_FILE.to_string(),
        "--size".to_string(),
        size.to_string(),
    ];
    Script {
        file: SCRIPT_FILE,
        text: SCRIPT,
        what: "the texture bake back",
        args: args.to_vec(),
        outputs: vec![OUTPUT_FILE.to_string(), STATS_FILE.to_string()],
    }
}

/// Bakes `source`'s texture onto the parts of `target` inside `dir` and
/// returns the baked model and its parts. Blocks until Blender is done.
///
/// # Errors
///
/// Fails without Blender or when the bake fails.
pub fn run(
    dir: &Path,
    source: &Path,
    target: &Path,
    size: u32,
    tools: &Toolbox,
) -> CoreResult<(PathBuf, Vec<BakedPart>)> {
    blender::run(dir, &script(source, target, size), tools)?;
    let file = dir.join(STATS_FILE);
    let text = std::fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    let stats: Stats = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    Ok((dir.join(OUTPUT_FILE), stats.parts))
}

/// The bake back at [`IMAGE_SIZE`], as the provider jobs ask for it when
/// a split's texture goes back onto its parts.
pub struct Bake;

impl TextureBake for Bake {
    fn bake(
        &self,
        dir: &Path,
        source: &Path,
        target: &Path,
        tools: &Toolbox,
    ) -> CoreResult<PathBuf> {
        run(dir, source, target, IMAGE_SIZE, tools).map(|(baked, _)| baked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_blender::{blender, glb_json, parts_model, toolbox};

    /// The textured one mesh lantern bakes onto the five untextured parts
    /// Tripo would send: names kept, one image each, the right colours.
    #[test]
    fn the_lantern_texture_bakes_back_onto_its_tripo_parts() {
        let Some(blender) = blender() else { return };
        let dir = tempfile::tempdir().expect("tempdir");
        let source = parts_model(&blender, dir.path(), "lantern");
        let target = parts_model(&blender, dir.path(), "lantern-tripo");
        let work = dir.path().join("bake");
        let (baked, parts) = run(&work, &source, &target, 32, &toolbox(&blender)).expect("bake");
        let doc = glb_json(&baked);
        let mut names: Vec<&str> = doc["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .filter(|n| n.get("mesh").is_some())
            .filter_map(|n| n["name"].as_str())
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            (0..5)
                .map(|i| format!("tripo_part_{i}"))
                .collect::<Vec<_>>()
        );
        assert_eq!(doc["images"].as_array().map(Vec::len), Some(5));
        let mean = |name: &str| parts.iter().find(|p| p.name == name).expect(name).mean;
        // tripo_part_2 is the glowing crystal, tripo_part_1 the dark iron cage.
        let crystal = mean("tripo_part_2");
        assert!(
            crystal[2] > crystal[0] + 0.1,
            "the crystal stays blue: {crystal:?}"
        );
        assert!(mean("tripo_part_1")[0] < crystal[0], "the iron stays dark");
    }
}
