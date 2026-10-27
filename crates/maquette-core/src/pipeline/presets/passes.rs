//! The gltf-transform calls one model's preset turns into.

use std::path::Path;

use super::ResolvedPreset;
use crate::workspace::{MeshoptLevel, TextureCodec};

/// The slots of the color group. They go in an extglob, because
/// gltf-transform's argument parser splits a brace glob at its comma and
/// then refuses the extra argument.
const COLOR_SLOTS: [&str; 2] = ["baseColorTexture", "emissiveTexture"];
const NORMAL_SLOT: &str = "normalTexture";

/// One gltf-transform invocation of the compress chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressPass {
    /// What the log says before the pass runs.
    pub label: String,
    pub args: Vec<String>,
}

/// Which map groups a model has, so a group it lacks costs no extra pass.
/// Unknown (an unreadable file) counts as every group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapGroups {
    pub color: bool,
    pub normal: bool,
}

impl MapGroups {
    pub const ALL: Self = Self {
        color: true,
        normal: true,
    };
}

pub(super) fn arg(path: &Path) -> String {
    path.display().to_string()
}

fn codec_word(codec: TextureCodec) -> &'static str {
    match codec {
        TextureCodec::Etc1s => "etc1s",
        TextureCodec::Uastc => "uastc",
    }
}

/// The `--slots` value: one slot by name, several as an extglob.
fn slots_glob(slots: &[&str]) -> String {
    match slots {
        [one] => (*one).to_string(),
        many => format!("@({})", many.join("|")),
    }
}

/// One texture pass: the maps it takes, their codec and size.
struct TexturePass {
    codec: TextureCodec,
    size: u32,
    /// Empty for the last pass, which takes every map still left.
    slots: Vec<&'static str>,
    what: &'static str,
}

/// The texture passes, in order: base color and emissive, then normal maps,
/// each only when its codec or size differs from the rest; then everything
/// left. A texture already encoded is skipped by the later passes, so the
/// last one takes the data maps and anything else at the data size.
fn texture_plan(preset: &ResolvedPreset, groups: MapGroups) -> Vec<TexturePass> {
    let main = preset.texture_size;
    let data = preset.data_texture_size.unwrap_or(main);
    let codecs = preset.codecs;
    let rest = (codecs.other, data);
    let mut plan: Vec<TexturePass> = Vec::new();
    if groups.color && (codecs.color, main) != rest {
        plan.push(TexturePass {
            codec: codecs.color,
            size: main,
            slots: COLOR_SLOTS.to_vec(),
            what: "color ",
        });
    }
    if groups.normal && (codecs.normal, data) != rest {
        match plan.first_mut() {
            Some(color) if (color.codec, color.size) == (codecs.normal, data) => {
                color.slots.push(NORMAL_SLOT);
                color.what = "color and normal ";
            }
            _ => plan.push(TexturePass {
                codec: codecs.normal,
                size: data,
                slots: vec![NORMAL_SLOT],
                what: "normal map ",
            }),
        }
    }
    let what = match plan.first() {
        None => "",
        Some(first) if first.slots.contains(&COLOR_SLOTS[0]) => "data map ",
        Some(_) => "remaining ",
    };
    plan.push(TexturePass {
        codec: codecs.other,
        size: data,
        slots: Vec::new(),
        what,
    });
    plan
}

/// The gltf-transform passes for one model, in order:
///
/// 1. `etc1s` or `uastc` with `--resize <size>` (skipped without toktx, so
///    the textures stay PNG or JPEG). One pass when every map shares a codec
///    and size; otherwise base color and emissive first (`--slots`), then
///    normal maps, then the rest, each group only when it differs,
/// 2. `meshopt --level medium|high`, or `quantize` for the low level
///    (gltf-transform's meshopt has no low level, so low keeps the geometry
///    uncompressed and only quantises it).
///
/// The texture passes write `tmp` and files beside it, the last pass writes
/// `output`.
pub fn compress_passes(
    input: &Path,
    output: &Path,
    tmp: &Path,
    preset: &ResolvedPreset,
    toktx: bool,
    groups: MapGroups,
) -> Vec<CompressPass> {
    let mut passes = Vec::new();
    let mut source = input.to_path_buf();
    if toktx {
        let plan = texture_plan(preset, groups);
        let last = plan.len() - 1;
        for (index, pass) in plan.iter().enumerate() {
            let to = match index {
                0 => tmp.to_path_buf(),
                i if i == last => tmp.with_extension("data.glb"),
                _ => tmp.with_extension("normal.glb"),
            };
            let codec = codec_word(pass.codec);
            let mut args = vec![
                codec.to_string(),
                arg(&source),
                arg(&to),
                "--resize".to_string(),
                pass.size.to_string(),
            ];
            if !pass.slots.is_empty() {
                args.extend(["--slots".to_string(), slots_glob(&pass.slots)]);
            }
            passes.push(CompressPass {
                label: format!(
                    "{} {}textures to {} at {} px",
                    preset.model,
                    pass.what,
                    codec.to_uppercase(),
                    pass.size
                ),
                args,
            });
            source = to;
        }
    }
    let geometry = geometry_pass(preset, &source, output);
    passes.push(geometry);
    passes
}

/// The last pass: `meshopt --level medium|high`, or `quantize` for low.
pub(super) fn geometry_pass(preset: &ResolvedPreset, source: &Path, output: &Path) -> CompressPass {
    match preset.meshopt {
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
    }
}
