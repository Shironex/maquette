use super::*;
use std::path::{Path, PathBuf};

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
        data_texture_size: None,
        codec: TextureCodec::Etc1s,
        map_codecs: None,
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

/// A model with color maps and no normal map, as the older tests assumed.
const NO_NORMAL: MapGroups = MapGroups {
    color: true,
    normal: false,
};

fn args_of(passes: &[CompressPass]) -> Vec<Vec<&str>> {
    passes
        .iter()
        .map(|p| p.args.iter().map(String::as_str).collect())
        .collect()
}

#[test]
fn labels_read_as_words() {
    assert_eq!(model_label("sakura_large"), "Sakura large");
    assert_eq!(model_label("desk-lamp__v2"), "Desk lamp v2");
    assert_eq!(model_label("_"), "_");
}

#[test]
fn rows_win_over_defaults() {
    let presets = vec![character(), row("sakura", 1024, MeshoptLevel::High, None)];
    let c = resolve(&presets, "character", DEFAULTS);
    assert_eq!(c.meshopt, MeshoptLevel::Medium);
    assert!(c.locked);
    let s = resolve(&presets, "sakura", DEFAULTS);
    assert_eq!((s.texture_size, s.locked), (1024, false));
    assert_eq!(
        s.codecs.normal,
        TextureCodec::Uastc,
        "normal maps default to UASTC"
    );
    let d = resolve(&presets, "bench", DEFAULTS);
    assert_eq!(
        (d.texture_size, d.codecs.color, d.meshopt),
        (2048, TextureCodec::Etc1s, MeshoptLevel::High)
    );
}

#[test]
fn presets_map_to_gltf_transform_args() {
    let input = PathBuf::from("web/raw/character.glb");
    let output = PathBuf::from("web/dist/character.glb");
    let tmp = PathBuf::from("cache/tmp/character.glb");
    let preset = resolve(&[character()], "character", DEFAULTS);
    let passes = compress_passes(&input, &output, &tmp, &preset, true, NO_NORMAL);
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
    let passes = compress_passes(&input, &output, &tmp, &preset, true, NO_NORMAL);
    assert_eq!(passes[0].args[0], "uastc");
    assert_eq!(passes[0].args[4], "1536");
    assert_eq!(passes[1].args[4], "high");

    let low = resolve(&[row("far", 256, MeshoptLevel::Low, None)], "far", DEFAULTS);
    let passes = compress_passes(&input, &output, &tmp, &low, false, MapGroups::ALL);
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
fn a_data_map_size_splits_the_texture_pass_in_two() {
    let input = PathBuf::from("raw/island.glb");
    let output = PathBuf::from("dist/island.glb");
    let tmp = PathBuf::from("tmp/island.tex.glb");
    let mut split = row("island", 2048, MeshoptLevel::High, None);
    split.data_texture_size = Some(1024);
    let preset = resolve(&[split.clone()], "island", DEFAULTS);
    assert_eq!(preset.data_texture_size, Some(1024));
    let passes = compress_passes(&input, &output, &tmp, &preset, true, NO_NORMAL);
    assert_eq!(
        args_of(&passes),
        vec![
            vec![
                "etc1s",
                "raw/island.glb",
                "tmp/island.tex.glb",
                "--resize",
                "2048",
                "--slots",
                "@(baseColorTexture|emissiveTexture)"
            ],
            vec![
                "etc1s",
                "tmp/island.tex.glb",
                "tmp/island.tex.data.glb",
                "--resize",
                "1024"
            ],
            vec![
                "meshopt",
                "tmp/island.tex.data.glb",
                "dist/island.glb",
                "--level",
                "high"
            ],
        ]
    );
    assert!(passes[0].label.contains("color textures"));
    assert!(passes[1].label.contains("data map textures"));

    // The same size twice is no split, and no toktx means no texture pass.
    split.data_texture_size = Some(2048);
    let same = resolve(&[split.clone()], "island", DEFAULTS);
    assert_eq!(same.data_texture_size, None);
    assert_eq!(
        compress_passes(&input, &output, &tmp, &same, true, NO_NORMAL).len(),
        2
    );
    let preset = resolve(&[row_with_data(1024)], "island", DEFAULTS);
    assert_eq!(
        compress_passes(&input, &output, &tmp, &preset, false, MapGroups::ALL).len(),
        1
    );

    // The size is checked like the main one, and old files still load.
    split.data_texture_size = Some(8);
    assert!(split.check().is_err());
    let old = r#"{"model":"a","label":"A","textureSize":1024,"codec":"etc1s","meshopt":"high","lock":null}"#;
    let parsed: CompressPreset = serde_json::from_str(old).expect("old preset");
    assert_eq!(parsed.data_texture_size, None);
}

fn row_with_data(data: u32) -> CompressPreset {
    CompressPreset {
        data_texture_size: Some(data),
        ..row("island", 2048, MeshoptLevel::High, None)
    }
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
        MapGroups::ALL,
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

fn with_codecs(map_codecs: MapCodecs) -> ResolvedPreset {
    let row = CompressPreset {
        map_codecs: Some(map_codecs),
        ..row("lamp", 2048, MeshoptLevel::High, None)
    };
    resolve(&[row], "lamp", DEFAULTS)
}

fn texture_args(preset: &ResolvedPreset, groups: MapGroups) -> Vec<Vec<String>> {
    let passes = compress_passes(
        Path::new("raw/lamp.glb"),
        Path::new("dist/lamp.glb"),
        Path::new("tmp/lamp.tex.glb"),
        preset,
        true,
        groups,
    );
    passes[..passes.len() - 1]
        .iter()
        .map(|p| p.args.clone())
        .collect()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn normal_maps_get_their_own_uastc_pass_by_default() {
    let preset = resolve(&[], "lamp", DEFAULTS);
    assert_eq!(
        preset.codecs,
        CodecSet {
            color: TextureCodec::Etc1s,
            normal: TextureCodec::Uastc,
            other: TextureCodec::Etc1s,
        }
    );
    let passes = compress_passes(
        Path::new("raw/lamp.glb"),
        Path::new("dist/lamp.glb"),
        Path::new("tmp/lamp.tex.glb"),
        &preset,
        true,
        MapGroups::ALL,
    );
    assert_eq!(
        args_of(&passes),
        vec![
            vec![
                "uastc",
                "raw/lamp.glb",
                "tmp/lamp.tex.glb",
                "--resize",
                "2048",
                "--slots",
                "normalTexture"
            ],
            vec![
                "etc1s",
                "tmp/lamp.tex.glb",
                "tmp/lamp.tex.data.glb",
                "--resize",
                "2048"
            ],
            vec![
                "meshopt",
                "tmp/lamp.tex.data.glb",
                "dist/lamp.glb",
                "--level",
                "high"
            ],
        ]
    );
    assert!(passes[0].label.contains("normal map textures to UASTC"));
    assert!(passes[1].label.contains("remaining textures to ETC1S"));
    // A model without a normal map pays for no extra pass.
    assert_eq!(texture_args(&preset, NO_NORMAL).len(), 1);
}

#[test]
fn each_map_group_can_pick_its_codec() {
    let etc1s = Some(TextureCodec::Etc1s);
    let uastc = Some(TextureCodec::Uastc);
    // Everything ETC1S, normal maps included: one pass, no slots.
    let all = with_codecs(MapCodecs {
        color: None,
        normal: etc1s,
        other: None,
    });
    assert_eq!(
        texture_args(&all, MapGroups::ALL),
        vec![strings(&[
            "etc1s",
            "raw/lamp.glb",
            "tmp/lamp.tex.glb",
            "--resize",
            "2048"
        ])]
    );
    // UASTC color and normal maps share one extglob pass; the data maps
    // stay ETC1S.
    let crisp = with_codecs(MapCodecs {
        color: uastc,
        normal: uastc,
        other: etc1s,
    });
    let args = texture_args(&crisp, MapGroups::ALL);
    assert_eq!(args.len(), 2);
    assert_eq!(
        args[0][5..],
        strings(&[
            "--slots",
            "@(baseColorTexture|emissiveTexture|normalTexture)"
        ])
    );
    assert_eq!((args[1][0].as_str(), args[1].len()), ("etc1s", 5));
    // ETC1S color maps at 2048 and UASTC data maps at 1024: normal maps
    // match the rest, so they ride along in the last pass.
    let mut split = CompressPreset {
        map_codecs: Some(MapCodecs {
            color: etc1s,
            normal: uastc,
            other: uastc,
        }),
        data_texture_size: Some(1024),
        ..row("lamp", 2048, MeshoptLevel::High, None)
    };
    split.codec = TextureCodec::Etc1s;
    let preset = resolve(&[split], "lamp", DEFAULTS);
    let args = texture_args(&preset, MapGroups::ALL);
    assert_eq!(
        args,
        vec![
            strings(&[
                "etc1s",
                "raw/lamp.glb",
                "tmp/lamp.tex.glb",
                "--resize",
                "2048",
                "--slots",
                "@(baseColorTexture|emissiveTexture)"
            ]),
            strings(&[
                "uastc",
                "tmp/lamp.tex.glb",
                "tmp/lamp.tex.data.glb",
                "--resize",
                "1024"
            ]),
        ]
    );
    // Color, normal and the rest all differ: three passes.
    let three = resolve(
        &[CompressPreset {
            map_codecs: Some(MapCodecs {
                color: uastc,
                normal: etc1s,
                other: uastc,
            }),
            data_texture_size: Some(1024),
            ..row("lamp", 2048, MeshoptLevel::High, None)
        }],
        "lamp",
        DEFAULTS,
    );
    let args = texture_args(&three, MapGroups::ALL);
    assert_eq!(args.len(), 3);
    assert_eq!(args[0][6], "@(baseColorTexture|emissiveTexture)");
    assert_eq!(
        args[1][..3],
        strings(&["etc1s", "tmp/lamp.tex.glb", "tmp/lamp.tex.normal.glb"])
    );
    assert!(args[1].ends_with(&strings(&["--slots", "normalTexture"])));
    assert_eq!(args[2][1], "tmp/lamp.tex.normal.glb");
    assert_eq!(args[2][2], "tmp/lamp.tex.data.glb");
}

#[test]
fn map_codecs_load_from_older_rows_and_round_trip() {
    let old = r#"{"model":"a","label":"A","textureSize":1024,"codec":"uastc","meshopt":"high","lock":null}"#;
    let parsed: CompressPreset = serde_json::from_str(old).expect("old preset");
    assert_eq!(parsed.map_codecs, None);
    let preset = resolve(std::slice::from_ref(&parsed), "a", DEFAULTS);
    assert_eq!(preset.codecs.color, TextureCodec::Uastc);
    let mut row = parsed;
    row.map_codecs = Some(MapCodecs {
        normal: Some(TextureCodec::Etc1s),
        ..MapCodecs::default()
    });
    let text = serde_json::to_string(&row).expect("json");
    assert!(text.contains(r#""mapCodecs":{"color":null,"normal":"etc1s","other":null}"#));
    assert_eq!(
        serde_json::from_str::<CompressPreset>(&text).expect("back"),
        row
    );
}
