//! Compression presets: per model texture size, texture codec and meshopt
//! level, and how they become gltf-transform args.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::CompressDefaults;
use crate::error::{CoreError, CoreResult};
use crate::workspace::{MeshoptLevel, TextureCodec, is_single_segment};

pub const MIN_TEXTURE: u32 = 64;
pub const MAX_TEXTURE: u32 = 8192;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompressPreset {
    /// The GLB file stem the row applies to, for example `character`.
    pub model: String,
    /// What the table shows, for example `Character`.
    pub label: String,
    /// The longest texture edge after compression, in pixels.
    pub texture_size: u32,
    pub codec: TextureCodec,
    pub meshopt: MeshoptLevel,
    /// Why this row cannot change, for example "High spins her upper body".
    /// Locked rows refuse edits until the lock is removed from the file.
    pub lock: Option<String>,
}

impl CompressPreset {
    pub(crate) fn check(&self) -> Result<(), String> {
        if !is_single_segment(&self.model) {
            return Err(format!(
                "{:?} is not a model name. Use the GLB file name without .glb.",
                self.model
            ));
        }
        if !(MIN_TEXTURE..=MAX_TEXTURE).contains(&self.texture_size) {
            return Err(format!(
                "{}: the texture size must be between {MIN_TEXTURE} and {MAX_TEXTURE} px.",
                self.label
            ));
        }
        Ok(())
    }
}

/// The preset a model gets: its own row, or the workspace defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPreset {
    pub model: String,
    pub texture_size: u32,
    pub codec: TextureCodec,
    pub meshopt: MeshoptLevel,
    pub locked: bool,
}

pub fn resolve(
    presets: &[CompressPreset],
    model: &str,
    defaults: CompressDefaults,
) -> ResolvedPreset {
    match presets.iter().find(|p| p.model == model) {
        Some(p) => ResolvedPreset {
            model: model.to_string(),
            texture_size: p.texture_size,
            codec: p.codec,
            meshopt: p.meshopt,
            locked: p.lock.is_some(),
        },
        None => ResolvedPreset {
            model: model.to_string(),
            texture_size: defaults.texture_size,
            codec: defaults.codec,
            meshopt: defaults.meshopt,
            locked: false,
        },
    }
}

/// One gltf-transform invocation of the compress chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressPass {
    /// What the log says before the pass runs.
    pub label: String,
    pub args: Vec<String>,
}

fn arg(path: &Path) -> String {
    path.display().to_string()
}

/// The gltf-transform passes for one model, in order:
///
/// 1. `etc1s` or `uastc` with `--resize <size>` (skipped without toktx, so
///    the textures stay PNG or JPEG),
/// 2. `meshopt --level medium|high`, or `quantize` for the low level
///    (gltf-transform's meshopt has no low level, so low keeps the geometry
///    uncompressed and only quantises it).
///
/// The texture pass writes `tmp`, the last pass writes `output`.
pub fn compress_passes(
    input: &Path,
    output: &Path,
    tmp: &Path,
    preset: &ResolvedPreset,
    toktx: bool,
) -> Vec<CompressPass> {
    let mut passes = Vec::new();
    let mut source = input;
    if toktx {
        let codec = match preset.codec {
            TextureCodec::Etc1s => "etc1s",
            TextureCodec::Uastc => "uastc",
        };
        passes.push(CompressPass {
            label: format!(
                "{} textures to {} at {} px",
                preset.model,
                codec.to_uppercase(),
                preset.texture_size
            ),
            args: vec![
                codec.to_string(),
                arg(input),
                arg(tmp),
                "--resize".to_string(),
                preset.texture_size.to_string(),
            ],
        });
        source = tmp;
    }
    let geometry = match preset.meshopt {
        MeshoptLevel::Low => CompressPass {
            label: format!("{} geometry quantized (meshopt low)", preset.model),
            args: vec!["quantize".to_string(), arg(source), arg(output)],
        },
        level => {
            let level = if level == MeshoptLevel::High {
                "high"
            } else {
                "medium"
            };
            CompressPass {
                label: format!("{} geometry with meshopt {level}", preset.model),
                args: vec![
                    "meshopt".to_string(),
                    arg(source),
                    arg(output),
                    "--level".to_string(),
                    level.to_string(),
                ],
            }
        }
    };
    passes.push(geometry);
    passes
}

/// Merges an edited preset table over the stored one. Locked rows must come
/// back unchanged (the lock reason is the error), and removing a locked row
/// is refused too.
pub fn apply(
    stored: &[CompressPreset],
    incoming: Vec<CompressPreset>,
) -> CoreResult<Vec<CompressPreset>> {
    for old in stored.iter().filter(|p| p.lock.is_some()) {
        let reason = old.lock.as_deref().unwrap_or_default();
        match incoming.iter().find(|p| p.model == old.model) {
            Some(new) if new == old => {}
            _ => {
                return Err(CoreError::Invalid(format!(
                    "{} is locked: {reason}",
                    old.label
                )));
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for preset in &incoming {
        preset.check().map_err(CoreError::Invalid)?;
        if !seen.insert(preset.model.as_str()) {
            return Err(CoreError::Invalid(format!(
                "{} has two preset rows.",
                preset.label
            )));
        }
        if preset.lock.is_some() && !stored.iter().any(|p| p == preset) {
            return Err(CoreError::Invalid(format!(
                "{}: locks are set in pipeline.json, not from the table.",
                preset.label
            )));
        }
    }
    Ok(incoming)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const DEFAULTS: CompressDefaults = CompressDefaults {
        texture_size: 2048,
        codec: TextureCodec::Etc1s,
        meshopt: MeshoptLevel::High,
    };

    fn row(model: &str, size: u32, meshopt: MeshoptLevel, lock: Option<&str>) -> CompressPreset {
        CompressPreset {
            model: model.into(),
            label: model.into(),
            texture_size: size,
            codec: TextureCodec::Etc1s,
            meshopt,
            lock: lock.map(str::to_string),
        }
    }

    fn character() -> CompressPreset {
        row(
            "character",
            2048,
            MeshoptLevel::Medium,
            Some("Character stays on medium. High spins her upper body."),
        )
    }

    fn args_of(passes: &[CompressPass]) -> Vec<Vec<&str>> {
        passes
            .iter()
            .map(|p| p.args.iter().map(String::as_str).collect())
            .collect()
    }

    #[test]
    fn rows_win_over_defaults() {
        let presets = vec![character(), row("sakura", 1024, MeshoptLevel::High, None)];
        let c = resolve(&presets, "character", DEFAULTS);
        assert_eq!(c.meshopt, MeshoptLevel::Medium);
        assert!(c.locked);
        let s = resolve(&presets, "sakura", DEFAULTS);
        assert_eq!((s.texture_size, s.locked), (1024, false));
        let d = resolve(&presets, "bench", DEFAULTS);
        assert_eq!(
            (d.texture_size, d.codec, d.meshopt),
            (2048, TextureCodec::Etc1s, MeshoptLevel::High)
        );
    }

    #[test]
    fn presets_map_to_gltf_transform_args() {
        let input = PathBuf::from("web/raw/character.glb");
        let output = PathBuf::from("web/dist/character.glb");
        let tmp = PathBuf::from("cache/tmp/character.glb");
        let preset = resolve(&[character()], "character", DEFAULTS);
        let passes = compress_passes(&input, &output, &tmp, &preset, true);
        assert_eq!(
            args_of(&passes),
            vec![
                vec![
                    "etc1s",
                    "web/raw/character.glb",
                    "cache/tmp/character.glb",
                    "--resize",
                    "2048"
                ],
                vec![
                    "meshopt",
                    "cache/tmp/character.glb",
                    "web/dist/character.glb",
                    "--level",
                    "medium"
                ],
            ]
        );

        let mut uastc = row("island", 1536, MeshoptLevel::High, None);
        uastc.codec = TextureCodec::Uastc;
        let preset = resolve(&[uastc], "island", DEFAULTS);
        let passes = compress_passes(&input, &output, &tmp, &preset, true);
        assert_eq!(passes[0].args[0], "uastc");
        assert_eq!(passes[0].args[4], "1536");
        assert_eq!(passes[1].args[4], "high");

        let low = resolve(&[row("far", 256, MeshoptLevel::Low, None)], "far", DEFAULTS);
        let passes = compress_passes(&input, &output, &tmp, &low, false);
        assert_eq!(
            args_of(&passes),
            vec![vec![
                "quantize",
                "web/raw/character.glb",
                "web/dist/character.glb"
            ]]
        );
    }

    #[test]
    fn without_toktx_only_geometry_runs() {
        let preset = resolve(&[], "bench", DEFAULTS);
        let passes = compress_passes(
            Path::new("in.glb"),
            Path::new("out.glb"),
            Path::new("tmp.glb"),
            &preset,
            false,
        );
        assert_eq!(
            args_of(&passes),
            vec![vec!["meshopt", "in.glb", "out.glb", "--level", "high"]]
        );
    }

    #[test]
    fn locked_rows_cannot_change() {
        let stored = vec![character(), row("sakura", 1024, MeshoptLevel::High, None)];
        let mut edited = stored.clone();
        edited[1].texture_size = 512;
        assert_eq!(apply(&stored, edited).expect("edit")[1].texture_size, 512);

        let mut spun = stored.clone();
        spun[0].meshopt = MeshoptLevel::High;
        let err = apply(&stored, spun).expect_err("locked");
        assert!(err.to_string().contains("High spins her upper body"));

        let dropped = vec![stored[1].clone()];
        assert!(apply(&stored, dropped).is_err());

        let mut sneaky = stored.clone();
        sneaky[1].lock = Some("mine".into());
        assert!(apply(&stored, sneaky).is_err());

        let mut tiny = stored.clone();
        tiny[1].texture_size = 8;
        assert!(apply(&stored, tiny).is_err());

        let mut twice = stored.clone();
        twice.push(stored[1].clone());
        assert!(apply(&stored, twice).is_err());
    }
}
