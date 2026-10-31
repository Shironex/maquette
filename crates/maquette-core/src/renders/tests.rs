//! Render styles without Blender: the settings, the loop rules, the WebP
//! container, what toon needs, where files go and the plan.

use std::fs;

use super::encode::{frames_of, gif_args, join_webp, margin_of, mp4_args};
use super::plan::has_colour;
use super::record::file_in;
use super::spec::{angles, still_job, turn_job};
use super::*;
use crate::inbox::fixture;
use crate::packs::PackSource;
use crate::workspace::FolderLayout;

/// A library entry with a textured box for a model.
pub(super) fn entry(root: &std::path::Path, id: &str, name: &str) {
    let stats = crate::packs::tests::stats(12, 0, 0);
    let kind = crate::assets::AssetKind::Prop;
    crate::packs::tests::entry(root, id, name, kind, &fixture::clean_prop(), stats);
}

fn u24(b: &[u8]) -> u32 {
    u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16
}

#[test]
fn settings_refuse_what_they_do_not_offer() {
    let ok = RenderSettings::default();
    assert!(ok.check().is_ok());
    assert_eq!(ok.format, LoopFormat::Webp, "the loop default is WebP");
    for bad in [
        RenderSettings {
            size: 300,
            ..ok.clone()
        },
        RenderSettings {
            line_color: "blue".into(),
            ..ok.clone()
        },
        RenderSettings {
            paper_color: Some("#12".into()),
            ..ok.clone()
        },
        RenderSettings {
            line_weight: 0.0,
            ..ok.clone()
        },
        RenderSettings {
            crease: 5,
            ..ok.clone()
        },
        RenderSettings {
            length: 0,
            ..ok.clone()
        },
        RenderSettings {
            length: 13,
            ..ok.clone()
        },
        RenderSettings {
            width: 50,
            ..ok.clone()
        },
        RenderSettings {
            tilt: 80,
            ..ok.clone()
        },
        RenderSettings {
            columns: 0,
            ..ok.clone()
        },
    ] {
        assert!(bad.check().is_err(), "{bad:?}");
    }
}

#[test]
fn the_gif_rules_are_in_the_arguments() {
    let args = gif_args("out.gif", RenderLook::Clay);
    let at = args.iter().position(|a| a == "-framerate").expect("rate");
    assert_eq!(args[at + 1], "20", "20 fps, so each frame is 5/100 s");
    let filter = args
        .iter()
        .find(|a| a.contains("palettegen"))
        .expect("filter");
    assert!(
        filter.contains("stats_mode=full"),
        "one palette over every frame"
    );
    assert!(filter.contains("dither=bayer:bayer_scale=5"), "{filter}");
    assert!(filter.contains("diff_mode=rectangle"), "{filter}");
    assert_eq!(args.last().map(String::as_str), Some("out.gif"));
    assert_eq!(frames_of(3), 60);
    let mp4 = mp4_args("out.mp4");
    assert!(mp4.iter().any(|a| a == "yuv420p"), "plays anywhere");
}

#[test]
fn the_crop_keeps_a_margin_unless_it_is_tight() {
    assert_eq!(margin_of(480, false), 20);
    assert_eq!(margin_of(48, false), 4);
    assert_eq!(margin_of(480, true), 2);
}

#[test]
fn estimates_rank_the_formats_and_scale_with_the_box() {
    let gif = estimate_loop_bytes(LoopFormat::Gif, RenderLook::Clay, (300, 400), 60);
    let webp = estimate_loop_bytes(LoopFormat::Webp, RenderLook::Clay, (300, 400), 60);
    assert!(webp < gif, "{webp} {gif}");
    let half = estimate_loop_bytes(LoopFormat::Gif, RenderLook::Clay, (150, 400), 60);
    assert!((gif / half - 2.0).abs() < 1e-6);
    assert!(
        gif < TARGET_BYTES * 1.5,
        "a usual clay turn is near the target"
    );
}

fn still(payload: &[u8]) -> Vec<u8> {
    let mut out = b"RIFF".to_vec();
    let size = u32::try_from(payload.len() + 12).expect("size");
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(b"WEBPVP8 ");
    let len = u32::try_from(payload.len()).expect("len");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

#[test]
fn a_webp_loop_is_a_well_formed_animation() {
    let joined = join_webp(&[still(&[1, 2, 3, 4]), still(&[5, 6])], 300, 200).expect("join");
    assert_eq!(&joined[0..4], b"RIFF");
    let riff = u32::from_le_bytes(joined[4..8].try_into().expect("size")) as usize;
    assert_eq!(riff + 8, joined.len(), "the RIFF size counts the rest");
    assert_eq!(&joined[8..12], b"WEBP");
    assert_eq!(&joined[12..16], b"VP8X");
    assert_eq!(joined[20] & 0b10, 0b10, "the animation flag");
    assert_eq!(u24(&joined[24..27]), 299, "the canvas width minus one");
    assert_eq!(u24(&joined[27..30]), 199);
    assert_eq!(&joined[30..34], b"ANIM");
    assert_eq!(&joined[44..48], b"ANMF");
    let frame = &joined[52..];
    assert_eq!(u24(&frame[12..15]), 50, "each frame lasts 50 ms");
    assert_eq!(&frame[16..20], b"VP8 ", "the still's own chunk follows");
    let count = joined.windows(4).filter(|w| w == b"ANMF").count();
    assert_eq!(count, 2);
    assert!(join_webp(&[b"not a webp".to_vec()], 1, 1).is_err());
    assert!(join_webp(&[], 1, 1).is_err());
}

#[test]
fn toon_needs_a_texture_or_colours() {
    let dir = tempfile::tempdir().expect("dir");
    let textured = dir.path().join("crate.glb");
    fs::write(&textured, fixture::clean_prop()).expect("write");
    assert!(has_colour(&textured));
    let plain = dir.path().join("plain.gltf");
    fs::write(
        &plain,
        r#"{"materials":[{"pbrMetallicRoughness":{"baseColorFactor":[0.8,0.8,0.8,1]}}]}"#,
    )
    .expect("write");
    assert!(!has_colour(&plain), "one grey has nothing to show");
    let tinted = dir.path().join("tinted.gltf");
    fs::write(
        &tinted,
        r#"{"materials":[{"pbrMetallicRoughness":{"baseColorFactor":[0.8,0.1,0.1,1]}}]}"#,
    )
    .expect("write");
    assert!(has_colour(&tinted), "a red box still reads in toon");
    let blend = dir.path().join("x.blend");
    fs::write(&blend, b"BLENDER").expect("write");
    assert!(has_colour(&blend), "the benefit of the doubt");
}

#[test]
fn a_file_keeps_earlier_renders_unless_told_to_replace() {
    let dir = tempfile::tempdir().expect("dir");
    let d = dir.path();
    assert_eq!(file_in(d, "clay", "png", false), d.join("clay.png"));
    fs::write(d.join("clay.png"), b"x").expect("write");
    assert_eq!(file_in(d, "clay", "png", true), d.join("clay.png"));
    assert_eq!(file_in(d, "clay", "png", false), d.join("clay-2.png"));
    fs::write(d.join("clay-2.png"), b"x").expect("write");
    assert_eq!(file_in(d, "clay", "png", false), d.join("clay-3.png"));
}

#[test]
fn jobs_carry_the_settings() {
    let settings = RenderSettings {
        camera: RenderCamera::Side,
        hidden_dashed: true,
        ..RenderSettings::default()
    };
    let job = still_job(
        RenderStyle::Blueprint,
        &settings,
        std::path::Path::new("o.png"),
        520,
        90,
    );
    assert_eq!(job["style"], "blueprint");
    assert_eq!(job["azimuth"], u64::from(angles(RenderCamera::Side).0 + 90));
    assert_eq!(job["hidden"], true);
    let turn = turn_job(&settings, std::path::Path::new("f"), 240, 8, 10, 72);
    assert_eq!(turn["style"], "clay", "a loop takes the settings' look");
    assert_eq!(turn["elevation"], 12);
    assert_eq!(turn["frames"], 8);
    assert_eq!(turn["crop"], 10);
    assert_eq!(turn["webp"], 72);
}

#[test]
fn library_models_render_to_the_workspace_exports() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    entry(root, "lamp", "Street lamp");
    let source = PackSource::Entries {
        entries: vec!["lamp".into()],
    };
    let planned = plan(root, &FolderLayout::default(), &source).expect("plan");
    assert!(planned.dir.ends_with("renders"), "{}", planned.dir);
    assert!(planned.dir.starts_with(&root.display().to_string()));
    let target = &planned.targets[0];
    assert_eq!(target.key, "lamp");
    assert_eq!(target.folder, "street-lamp");
    assert!(target.has_colour);
    assert!((target.height_m.expect("height") - 0.92).abs() < 0.01);
}
