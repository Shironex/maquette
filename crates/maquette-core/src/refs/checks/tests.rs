//! Tests for the weak ref checks, on images drawn here.

use std::fs;

use super::*;
use crate::inbox::images::write_png;
use crate::refs::AssetRef;

const GREY: [u8; 3] = [200, 200, 204];
const STONE: [u8; 3] = [70, 64, 96];

/// A `size` square of studio grey with a stone block from `from` to `to`
/// (fractions of the side, x then y).
fn block(size: u32, from: (f64, f64), to: (f64, f64)) -> Vec<u8> {
    let mut rgb = Vec::new();
    let s = f64::from(size);
    for y in 0..size {
        for x in 0..size {
            let (fx, fy) = (f64::from(x) / s, f64::from(y) / s);
            let inside = fx >= from.0 && fx < to.0 && fy >= from.1 && fy < to.1;
            rgb.extend_from_slice(if inside { &STONE } else { &GREY });
        }
    }
    write_png(size, size, &rgb)
}

/// Clutter everywhere: a hash of the position for every channel.
fn noise(size: u32) -> Vec<u8> {
    let mut rgb = Vec::new();
    for y in 0..size {
        for x in 0..size {
            let h = (x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663)) % 251;
            rgb.extend_from_slice(&[h as u8, (h * 7 % 251) as u8, (h * 13 % 251) as u8]);
        }
    }
    write_png(size, size, &rgb)
}

fn kinds(dir: &std::path::Path, name: &str, bytes: &[u8]) -> Vec<WarningKind> {
    fs::create_dir_all(dir.join("refs")).expect("refs");
    let file = format!("refs/{name}");
    fs::write(dir.join(&file), bytes).expect("write");
    image_warnings(dir, &file, None)
        .into_iter()
        .map(|w| w.kind)
        .collect()
}

#[test]
fn a_centered_object_on_a_plain_ground_passes() {
    let dir = tempfile::tempdir().expect("dir");
    let good = block(600, (0.3, 0.25), (0.7, 0.8));
    assert_eq!(kinds(dir.path(), "good.png", &good), vec![]);
}

#[test]
fn a_small_image_is_flagged_with_its_size() {
    let dir = tempfile::tempdir().expect("dir");
    let small = block(384, (0.3, 0.3), (0.7, 0.7));
    assert_eq!(
        kinds(dir.path(), "small.png", &small),
        vec![WarningKind::Small]
    );
    let w = image_warnings(dir.path(), "refs/small.png", Some(RefView::Right));
    assert_eq!(w[0].title, "Right is small: 384 × 384 px");
    assert_eq!(w[0].fixes, vec![RefFix::LeaveOut]);
}

#[test]
fn an_object_touching_the_frame_reads_as_cropped() {
    let dir = tempfile::tempdir().expect("dir");
    let low = block(600, (0.3, 0.4), (0.7, 1.0));
    assert_eq!(
        kinds(dir.path(), "low.png", &low),
        vec![WarningKind::Cropped]
    );
    let w = image_warnings(dir.path(), "refs/low.png", Some(RefView::Front));
    assert_eq!(w[0].title, "Front crops the object");
    assert!(w[0].detail.contains("the bottom edge"), "{}", w[0].detail);
    let wide = block(600, (0.0, 0.3), (1.0, 0.7));
    let w = {
        kinds(dir.path(), "wide.png", &wide);
        image_warnings(dir.path(), "refs/wide.png", None)
    };
    assert!(
        w[0].detail.contains("the left and right edges"),
        "{}",
        w[0].detail
    );
}

#[test]
fn clutter_around_the_object_is_a_busy_background() {
    let dir = tempfile::tempdir().expect("dir");
    assert_eq!(
        kinds(dir.path(), "busy.png", &noise(600)),
        vec![WarningKind::BusyBackground]
    );
    // Two halves of different colour: the corners disagree.
    let split = block(600, (0.0, 0.0), (0.5, 1.0));
    assert_eq!(
        kinds(dir.path(), "split.png", &split),
        vec![WarningKind::BusyBackground],
        "busy, so not also called cropped"
    );
}

#[test]
fn images_the_decoder_does_not_read_only_get_the_size_check() {
    let dir = tempfile::tempdir().expect("dir");
    // SOI, an APP0 segment, then SOF0 with height 300 and width 500.
    let jpeg = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x01, 0x2C,
        0x01, 0xF4, 0x01, 0x01, 0x11, 0x00,
    ];
    assert_eq!(
        kinds(dir.path(), "photo.jpg", &jpeg),
        vec![WarningKind::Small]
    );
    assert_eq!(kinds(dir.path(), "broken.png", b"not an image"), vec![]);
}

#[test]
fn one_image_with_multi_view_on_asks_for_the_other_views() {
    let dir = tempfile::tempdir().expect("dir");
    kinds(dir.path(), "front.png", &block(600, (0.3, 0.3), (0.7, 0.7)));
    kinds(dir.path(), "side.png", &block(384, (0.3, 0.3), (0.7, 0.7)));
    let asset: crate::Asset = serde_json::from_str(
        r#"{"id":"frost-lantern","name":"Frost lantern","kind":"prop","stage":"refs",
            "status":"queued","versions":[],"thumbnail":null,"createdAt":"t","updatedAt":"t"}"#,
    )
    .expect("asset");
    let one = ["refs/front.png".to_string()];
    let w = send_warnings(dir.path(), &asset, &one, true);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].kind, WarningKind::OneView);
    assert_eq!(w[0].fixes, vec![RefFix::MakeViews]);
    assert!(send_warnings(dir.path(), &asset, &one, false).is_empty());

    let tagged = crate::Asset {
        refs: vec![
            AssetRef {
                file: "refs/side.png".into(),
                view: RefView::Right,
            },
            AssetRef {
                file: "refs/front.png".into(),
                view: RefView::Front,
            },
        ],
        ..asset
    };
    let both = ["refs/side.png".to_string(), "refs/front.png".to_string()];
    let w = send_warnings(dir.path(), &tagged, &both, true);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].view, Some(RefView::Right), "named by its tag");
    assert_eq!(w[0].file.as_deref(), Some("refs/side.png"));
}

#[test]
fn an_asset_with_a_model_says_the_send_makes_its_next_version() {
    let dir = tempfile::tempdir().expect("dir");
    let mut asset: crate::Asset = serde_json::from_str(
        r#"{"id":"stone-well","name":"Stone well","kind":"prop","stage":"blender",
            "status":"done","versions":[{"version":1,"file":"models/stone_well_v1.glb",
            "createdAt":"t","stats":null}],"thumbnail":null,"createdAt":"t","updatedAt":"t"}"#,
    )
    .expect("asset");
    let w = send_warnings(dir.path(), &asset, &[], false);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].kind, WarningKind::HasModel);
    assert_eq!(w[0].title, "Stone well already has a model (v1)");
    assert_eq!(w[0].detail, "This send makes v2. The older versions stay.");
    assert!(w[0].fixes.is_empty());

    asset.origin = Some(crate::assets::AssetOrigin {
        entry: "stone-well".into(),
        version: 1,
    });
    let w = send_warnings(dir.path(), &asset, &[], false);
    assert_eq!(
        w[0].title,
        "Stone well already has a model (v1, from the library entry stone-well)"
    );
    assert_eq!(
        w[0].detail,
        "This send makes v2; the library entry is not changed. The older versions stay."
    );

    asset.versions.clear();
    assert!(send_warnings(dir.path(), &asset, &[], false).is_empty());
}
