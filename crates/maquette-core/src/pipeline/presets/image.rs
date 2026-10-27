//! The gltf-transform calls of a model whose textures ship as WebP or AVIF.

use std::path::Path;

use super::ResolvedPreset;
use super::passes::{CompressPass, MapGroups, arg, geometry_pass};
use crate::ship::TextureFormat;

fn resize(
    model: &str,
    from: &Path,
    to: &Path,
    size: u32,
    what: &str,
    pattern: Option<String>,
) -> CompressPass {
    let mut args = vec![
        "resize".to_string(),
        arg(from),
        arg(to),
        "--width".to_string(),
        size.to_string(),
        "--height".to_string(),
        size.to_string(),
    ];
    if let Some(pattern) = pattern {
        args.extend(["--pattern".to_string(), pattern]);
    }
    CompressPass {
        label: format!("{model} {what}textures resized to at most {size} px"),
        args,
    }
}

fn encode(
    model: &str,
    (from, to): (&Path, &Path),
    (format, quality): (TextureFormat, u8),
) -> CompressPass {
    let word = format.word();
    CompressPass {
        label: format!(
            "{model} textures to {} at quality {quality}",
            word.to_uppercase()
        ),
        args: vec![
            word.to_string(),
            arg(from),
            arg(to),
            "--quality".to_string(),
            quality.to_string(),
        ],
    }
}

/// The marker the smaller group's images carry in their name while the
/// passes run (see [`needs_split`]); the runner takes it off afterwards.
pub const SMALL_MARK: &str = "-maquette-small";

/// True when the color maps and the data and normal maps get different
/// sizes, so the images of the smaller group are marked first.
pub fn needs_split(preset: &ResolvedPreset, groups: MapGroups) -> bool {
    groups.color
        && preset
            .data_texture_size
            .is_some_and(|d| d != preset.texture_size)
}

/// True when the color maps are the smaller group.
pub fn color_is_smaller(preset: &ResolvedPreset) -> bool {
    preset
        .data_texture_size
        .is_some_and(|d| d > preset.texture_size)
}

/// The passes for one model, in order. gltf-transform's `webp` and `avif`
/// cannot resize and its `resize` cannot pick slots, only texture names. So
/// with one size: resize every map, encode every map. With a split (color
/// maps at `texture_size`, the data and normal maps at
/// `data_texture_size`), `input` is a copy whose smaller group's images end
/// their names with [`SMALL_MARK`]: resize every map to the larger size,
/// then the marked ones to the smaller, then encode every map. Then meshopt
/// or quantize, as for KTX2. Codecs do not apply: every map is `format`.
pub fn image_passes(
    input: &Path,
    output: &Path,
    tmp: &Path,
    preset: &ResolvedPreset,
    (format, quality): (TextureFormat, u8),
    groups: MapGroups,
) -> Vec<CompressPass> {
    let model = preset.model.as_str();
    let main = preset.texture_size;
    let data = preset.data_texture_size.unwrap_or(main);
    let mut passes = Vec::new();
    let mut source = input.to_path_buf();
    let mut next = |make: &dyn Fn(&Path, &Path) -> CompressPass| {
        let to = tmp.with_extension(format!("img{}.glb", passes.len() + 1));
        passes.push(make(&source, &to));
        source = to;
    };
    if needs_split(preset, groups) {
        let small_what = if color_is_smaller(preset) {
            "color "
        } else {
            "data and normal map "
        };
        next(&|f, t| resize(model, f, t, main.max(data), "", None));
        next(&|f, t| {
            let pattern = format!("*{SMALL_MARK}");
            resize(model, f, t, main.min(data), small_what, Some(pattern))
        });
    } else {
        let size = if groups.color { main } else { data };
        next(&|f, t| resize(model, f, t, size, "", None));
    }
    next(&|f, t| encode(model, (f, t), (format, quality)));
    passes.push(geometry_pass(preset, &source, output));
    passes
}
