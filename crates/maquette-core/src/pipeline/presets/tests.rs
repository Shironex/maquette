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
        data_texture_size: None,
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
fn a_data_map_size_splits_the_texture_pass_in_two() {
    let input = PathBuf::from("raw/island.glb");
    let output = PathBuf::from("dist/island.glb");
    let tmp = PathBuf::from("tmp/island.tex.glb");
    let mut split = row("island", 2048, MeshoptLevel::High, None);
    split.data_texture_size = Some(1024);
    let preset = resolve(&[split.clone()], "island", DEFAULTS);
    assert_eq!(preset.data_texture_size, Some(1024));
    let passes = compress_passes(&input, &output, &tmp, &preset, true);
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
    assert_eq!(compress_passes(&input, &output, &tmp, &same, true).len(), 2);
    let preset = resolve(&[row_with_data(1024)], "island", DEFAULTS);
    assert_eq!(
        compress_passes(&input, &output, &tmp, &preset, false).len(),
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
