//! The softening script in a real, headless Blender, on a texture drawn
//! here: dark leather with small grey highlights, small skin and purple
//! dots on it, and a large light area, small and past the size the script
//! measures at. Ignored by default, so CI never
//! needs Blender: run with `--ignored`. `MAQUETTE_AVATAR_OUT` names a
//! folder to copy the textures and their softened copies into for a look,
//! and `MAQUETTE_AVATAR_PROBE` a written `.vrm` to soften into it as well.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::inbox::images::write_png;
use crate::test_blender;

/// The texture's side at scale 1.
const SIZE: usize = 256;
const LEATHER: [u8; 3] = [30, 26, 32];
const SHINE: [u8; 3] = [215, 212, 218];
const SKIN: [u8; 3] = [236, 206, 188];
const PURPLE: [u8; 3] = [170, 130, 235];

/// The centres of the dots of one kind: a 4 by 4 grid in a quarter.
fn dots(scale: usize, x0: usize, y0: usize) -> Vec<(usize, usize)> {
    (0..16)
        .map(|i| {
            (
                scale * (x0 + 12 + (i % 4) * 28),
                scale * (y0 + 12 + (i / 4) * 28),
            )
        })
        .collect()
}

/// The texture, row major from the top: highlights top left, skin top
/// right, purple bottom left, a light area bottom right.
fn texture(scale: usize) -> Vec<u8> {
    let side = SIZE * scale;
    let mut rgb = LEATHER.repeat(side * side);
    let mut paint = |x: usize, y: usize, c: [u8; 3]| {
        let at = (y * side + x) * 3;
        rgb[at..at + 3].copy_from_slice(&c);
    };
    let r = 3 * scale;
    let kinds = [
        (dots(scale, 0, 0), SHINE),
        (dots(scale, 128, 0), SKIN),
        (dots(scale, 0, 128), PURPLE),
    ];
    for (centres, colour) in kinds {
        for (cx, cy) in centres {
            for y in cy - r..=cy + r {
                for x in cx - r..=cx + r {
                    if x.abs_diff(cx).pow(2) + y.abs_diff(cy).pow(2) <= r * r {
                        paint(x, y, colour);
                    }
                }
            }
        }
    }
    for y in 144 * scale..240 * scale {
        for x in 144 * scale..240 * scale {
            paint(x, y, SKIN);
        }
    }
    rgb
}

/// The colours at `points` (x, y from the top) of a PNG, read in Blender.
fn pixels(blender: &Path, png: &Path, points: &[(usize, usize)]) -> Vec<[f64; 3]> {
    let expr = format!(
        "import bpy, json\n\
         im = bpy.data.images.load({png:?})\n\
         w, h = im.size\n\
         px = list(im.pixels)\n\
         pts = {points:?}\n\
         out = [[px[((h - 1 - y) * w + x) * im.channels + c] for c in range(3)] for x, y in pts]\n\
         print('PIXELS:' + json.dumps(out))",
        png = png.display().to_string(),
        points = points.iter().map(|&(x, y)| [x, y]).collect::<Vec<_>>(),
    );
    let text = test_blender::run(blender, &["--python-expr", &expr]);
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("PIXELS:"))
        .expect("pixels");
    serde_json::from_str(line).expect("pixels json")
}

fn luma(c: [f64; 3]) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// Softens the texture at `scale` and checks that only the highlights
/// changed.
fn soften_at(blender: &Path, scale: usize) {
    let dir = tempfile::tempdir().expect("tempdir");
    let side = u32::try_from(SIZE * scale).expect("side");
    let image = BaseColorImage {
        image: 0,
        mime: "image/png".to_string(),
        bytes: write_png(side, side, &texture(scale)),
    };
    let work = dir.path().join(HIGHLIGHTS_DIR);
    let found = run(
        &test_blender::runner(),
        &test_blender::toolbox(blender),
        &work,
        std::slice::from_ref(&image),
    )
    .expect("run");
    let found = &found[0];
    let soft = work.join(found.output.as_deref().expect("a softened copy"));
    if let Some(out) = std::env::var_os("MAQUETTE_AVATAR_OUT").map(PathBuf::from) {
        fs::create_dir_all(&out).expect("out");
        fs::write(
            out.join(format!("highlights_{scale}_before.png")),
            &image.bytes,
        )
        .expect("before");
        fs::copy(&soft, out.join(format!("highlights_{scale}_after.png"))).expect("after");
    }
    // The 16 highlight dots cover 16 * 29 of 256 * 256 pixels, 0.71%.
    println!(
        "coverage {:.4} dark {:.4}",
        found.coverage, found.dark_coverage
    );
    assert!(
        (0.0065..0.0078).contains(&found.coverage),
        "{}",
        found.coverage
    );

    let at = |(x, y): (usize, usize)| (x * scale, y * scale);
    let points = [
        dots(scale, 0, 0)[5],
        dots(scale, 128, 0)[5],
        dots(scale, 0, 128)[5],
        at((190, 190)),
        at((145, 145)),
        at((64, 64)),
    ];
    let before = pixels(blender, &work.join("image_0.png"), &points);
    let after = pixels(blender, &soft, &points);
    let drop = |i: usize| luma(before[i]) - luma(after[i]);
    assert!(
        drop(0) > 0.45,
        "the highlight is toned down: {:.3}",
        drop(0)
    );
    assert!(
        luma(after[0]) > luma(after[5]) + 0.05,
        "but not painted over"
    );
    for (i, what) in [
        (1, "skin dot"),
        (2, "purple dot"),
        (3, "light area"),
        (4, "its corner"),
        (5, "leather"),
    ] {
        assert!(drop(i).abs() < 0.02, "the {what} stays: {:.3}", drop(i));
    }
}

#[test]
#[ignore = "runs Blender"]
fn only_the_grey_highlights_on_dark_leather_are_toned_down() {
    if let Some(blender) = test_blender::blender() {
        soften_at(&blender, 1);
    }
}

#[test]
#[ignore = "runs Blender"]
fn a_texture_past_the_working_size_is_measured_smaller_and_toned_whole() {
    if let Some(blender) = test_blender::blender() {
        soften_at(&blender, 9);
    }
}

#[test]
#[ignore = "runs Blender; MAQUETTE_AVATAR_PROBE names a written .vrm"]
fn soften_a_written_vrm_in_blender() {
    let (Some(blender), Some(probe), Some(out)) = (
        test_blender::blender(),
        std::env::var_os("MAQUETTE_AVATAR_PROBE").map(PathBuf::from),
        std::env::var_os("MAQUETTE_AVATAR_OUT").map(PathBuf::from),
    ) else {
        return;
    };
    let written = Written {
        bytes: fs::read(&probe).expect("read the probe"),
        unresolved: Vec::new(),
    };
    let (softened, report) = apply(
        &test_blender::runner(),
        &test_blender::toolbox(&blender),
        &out,
        MaterialMode::MtoonSoft,
        Highlights::Soften,
        written,
    );
    let report = report.expect("report");
    println!("{}", serde_json::to_string_pretty(&report).expect("json"));
    assert!(report.softened(), "{:?}", report.note);
    fs::write(out.join("softened.vrm"), &softened.bytes).expect("vrm");
}
