//! The page light on real pixels, in a real, headless Blender. Skipped where
//! no Blender is found. Set `MAQUETTE_KEEP_RENDERS` to a folder to keep the
//! pictures there and look.

use std::fs;
use std::path::Path;

use super::*;
use crate::refs::png;

/// What the reference page shows for a flat, sunlit surface of each colour:
/// the swatch tiles built, baked and exported by the starter scripts, then
/// shown as the docs say (the lightmap as a light map) through the site
/// scene's renderer settings, three.js ACES Filmic at exposure 1.15. Sampled
/// from that page on 2026-11-10 (three.js 0.186).
const ON_THE_PAGE: [(&str, [u8; 3]); 7] = [
    ("lavender", [196, 186, 211]),
    ("cream", [217, 213, 203]),
    ("blush", [217, 196, 197]),
    ("mint", [194, 210, 201]),
    ("sky", [189, 204, 214]),
    ("paper", [218, 218, 216]),
    ("white", [221, 221, 221]),
];
/// How far a channel may sit from the page. The page's light map carries no
/// sheen; Blender's surface keeps a little, most seen from straight above.
const CLOSE: f64 = 8.0;

fn keep(file: &Path) {
    if let Some(dir) = std::env::var_os("MAQUETTE_KEEP_RENDERS")
        && let Some(name) = file.file_name()
    {
        let dir = Path::new(&dir);
        fs::create_dir_all(dir).expect("keep");
        let _ = fs::copy(file, dir.join(name));
    }
}

fn scripts(dir: &Path) {
    for (name, text) in [
        (SCRIPT_FILE, SCRIPT),
        (RIG_FILE, RIG_SCRIPT),
        (SWATCH_FILE, SWATCH_SCRIPT),
    ] {
        fs::write(dir.join(name), text).expect("script");
    }
}

fn rgb(value: &serde_json::Value) -> [f64; 3] {
    std::array::from_fn(|i| value[i].as_f64().expect("a channel"))
}

fn hex(text: &str) -> [f64; 3] {
    std::array::from_fn(|i| {
        f64::from(u8::from_str_radix(&text[1 + 2 * i..3 + 2 * i], 16).expect("hex"))
    })
}

#[test]
fn the_swatch_sheet_shows_pale_colours_as_the_page_does() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    scripts(dir.path());
    let sheet = dir.path().join("swatch-sheet.png");
    let report = dir.path().join("swatch-sheet.json");
    crate::test_blender::run(
        &blender,
        &[
            "--python",
            &dir.path().join(SWATCH_FILE).display().to_string(),
            "--",
            "--out",
            &sheet.display().to_string(),
            "--report",
            &report.display().to_string(),
            "--width",
            "800",
            "--samples",
            "32",
        ],
    );
    keep(&sheet);
    keep(&report);
    let picture = png::decode(&fs::read(&sheet).expect("sheet")).expect("png");
    assert_eq!(picture.width, 800);
    let found: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report).expect("report")).expect("json");
    // The rig says which light made it.
    assert_eq!(found["sun"], serde_json::json!(3.0));
    assert_eq!(found["fill"], serde_json::json!(0.18));
    assert_eq!(found["exposure"], serde_json::json!(1.15));
    let swatches = found["swatches"].as_array().expect("swatches");
    assert_eq!(swatches.len(), 15);
    let swatch = |name: &str| {
        swatches
            .iter()
            .find(|s| s["name"] == name)
            .unwrap_or_else(|| panic!("no {name}"))
    };
    for (name, page) in ON_THE_PAGE {
        let shown = rgb(&swatch(name)["shown"]);
        for c in 0..3 {
            assert!(
                (shown[c] - f64::from(page[c])).abs() <= CLOSE,
                "{name} shows {shown:?}, the page {page:?}"
            );
        }
    }
    for s in swatches {
        // No colour clips, white least of all, and no hue is lost on the way:
        // lavender stays more blue than red, cream more red than blue.
        let shown = rgb(&s["shown"]);
        assert!(
            shown.iter().all(|c| *c < 250.0),
            "{} clips: {shown:?}",
            s["name"]
        );
        // The strip is the colour as written, untouched by the tone curve.
        let written = rgb(&s["written"]);
        let asked = hex(s["hex"].as_str().expect("hex"));
        for c in 0..3 {
            assert!(
                (written[c] - asked[c]).abs() <= 2.5,
                "{}: {written:?}",
                s["name"]
            );
        }
    }
    let lavender = rgb(&swatch("lavender")["shown"]);
    assert!(
        lavender[2] > lavender[0] + 8.0 && lavender[0] > lavender[1] + 4.0,
        "{lavender:?}"
    );
    let cream = rgb(&swatch("cream")["shown"]);
    assert!(cream[0] > cream[2] + 8.0, "{cream:?}");
}

/// The share of a picture that is blown out to white.
fn clipped(file: &Path) -> f64 {
    let pixels = png::decode(&fs::read(file).expect("read")).expect("png");
    let mut white = 0u64;
    for y in 0..pixels.height {
        for x in 0..pixels.width {
            let p = pixels.at(x, y);
            white += u64::from(p[0] >= 250 && p[1] >= 250 && p[2] >= 250);
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "pixel counts")]
    let share = white as f64 / (pixels.width * pixels.height) as f64;
    share
}

#[test]
fn the_page_light_keeps_the_pale_colours_the_studio_light_clips() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    scripts(dir.path());
    let model = dir.path().join("pale_prop.glb");
    fs::write(&model, crate::inbox::fixture::pale_prop()).expect("model");
    let script = dir.path().join(SCRIPT_FILE).display().to_string();
    let render = |name: &str, look: ThumbLook| {
        let out = dir.path().join(name);
        let mut args = vec![
            "--python".to_string(),
            script.clone(),
            "--".to_string(),
            "--model".to_string(),
            model.display().to_string(),
            "--out".to_string(),
            out.display().to_string(),
        ];
        args.extend(look.args().iter().map(|a| (*a).to_string()));
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        crate::test_blender::run(&blender, &args);
        keep(&out);
        out
    };
    let studio = render("pale-studio.png", ThumbLook::Studio);
    let page = render("pale-page.png", ThumbLook::Page);
    // The studio's key washes the lavender, the cream and the white out.
    assert!(clipped(&studio) > 0.02, "studio clips {}", clipped(&studio));
    // The page light leaves no pixel blown out, and the picture is not dark.
    assert!(clipped(&page) == 0.0, "page clips {}", clipped(&page));
    let pixels = png::decode(&fs::read(&page).expect("read")).expect("png");
    assert_eq!((pixels.width, pixels.height), (256, 256));
    let middle = pixels.at(128, 128);
    assert!(middle[0] > 120, "the model is lit: {middle:?}");
    // The backdrop is the app's graphite, exactly, whatever the tone curve.
    let corner = pixels.at(3, 3);
    for (c, want) in [0x31u8, 0x2f, 0x2b].into_iter().enumerate() {
        assert!(corner[c].abs_diff(want) <= 2, "backdrop {corner:?}");
    }
}
