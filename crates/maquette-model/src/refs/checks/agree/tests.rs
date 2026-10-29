//! Tests for comparing the views with the front, on lanterns and chests
//! drawn here: views of one object never warn, other looks always do.

use std::fs;

use super::*;
use maquette_glb::images::write_png;

const GREY: [u8; 3] = [200, 200, 204];
const ICE: [u8; 3] = [92, 150, 214];
const BRASS: [u8; 3] = [196, 150, 60];
const EMBER: [u8; 3] = [200, 58, 40];
const PINE: [u8; 3] = [40, 120, 60];

/// A box of colour from `from` to `to`, in fractions of the frame.
type Part = ((f64, f64), (f64, f64), [u8; 3]);

/// A `w` by `h` frame of studio grey with `parts` drawn in order, the
/// object lit by `light` (1 as drawn, less in shadow).
fn draw(w: u32, h: u32, parts: &[Part], light: f64) -> Vec<u8> {
    let mut rgb = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (f64::from(x) / f64::from(w), f64::from(y) / f64::from(h));
            let hit = parts
                .iter()
                .rev()
                .find(|(a, b, _)| fx >= a.0 && fx < b.0 && fy >= a.1 && fy < b.1);
            let px = hit.map_or(GREY, |(_, _, c)| {
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                c.map(|v| (f64::from(v) * light).min(255.0) as u8)
            });
            rgb.extend_from_slice(&px);
        }
    }
    write_png(w, h, &rgb)
}

/// A frost lantern: an ice glass body under a brass cap on a brass base,
/// `width` wide (a fraction of the frame) and standing from `top` to 0.85.
fn lantern(width: f64, top: f64) -> Vec<Part> {
    let (l, r) = (0.5 - width / 2.0, 0.5 + width / 2.0);
    let cap = top + (0.85 - top) * 0.15;
    let base = 0.85 - (0.85 - top) * 0.12;
    vec![
        ((l, top), (r, cap), BRASS),
        ((l + 0.03, cap), (r - 0.03, base), ICE),
        ((l, base), (r, 0.85), BRASS),
    ]
}

/// A rune chest: wide and low, ember red with a pine band.
fn chest() -> Vec<Part> {
    vec![
        ((0.15, 0.5), (0.85, 0.85), EMBER),
        ((0.15, 0.6), (0.85, 0.65), PINE),
    ]
}

fn shape_of(bytes: &[u8]) -> Shape {
    shape(&png::decode(bytes).expect("decode")).expect("an object")
}

fn front() -> Shape {
    shape_of(&draw(600, 600, &lantern(0.3, 0.2), 1.0))
}

#[test]
fn views_of_one_lantern_never_warn() {
    let front = front();
    let matching = [
        // The back as drawn, and in shadow at a larger size.
        (RefView::Back, draw(600, 600, &lantern(0.3, 0.2), 1.0)),
        (RefView::Back, draw(1024, 1024, &lantern(0.32, 0.22), 0.7)),
        // Sides: narrower, framed a little lower or higher, lit softer.
        (RefView::Left, draw(600, 600, &lantern(0.18, 0.25), 0.85)),
        (RefView::Right, draw(800, 800, &lantern(0.22, 0.12), 1.1)),
        // A side much deeper than the front is wide still reads as a side.
        (RefView::Right, draw(600, 600, &lantern(0.6, 0.2), 0.9)),
    ];
    for (view, bytes) in matching {
        let other = shape_of(&bytes);
        assert_eq!(
            compare(&front, &other, view),
            Mismatch::default(),
            "{view:?} {other:?}"
        );
    }
}

#[test]
fn other_colours_or_proportions_warn() {
    let front = front();
    let chest = shape_of(&draw(600, 600, &chest(), 1.0));
    assert_eq!(
        compare(&front, &chest, RefView::Back),
        Mismatch {
            colours: true,
            proportions: true
        }
    );
    // The lantern's own colours, but half its height in the frame.
    let short = shape_of(&draw(600, 600, &lantern(0.3, 0.55), 1.0));
    assert_eq!(
        compare(&front, &short, RefView::Left),
        Mismatch {
            colours: false,
            proportions: true
        }
    );
    // The back with the front's height but twice as wide.
    let wide = shape_of(&draw(600, 600, &lantern(0.6, 0.2), 1.0));
    assert!(compare(&front, &wide, RefView::Back).proportions);
    // The lantern's shape, recoloured.
    let parts: Vec<Part> = lantern(0.3, 0.2)
        .into_iter()
        .map(|(a, b, c)| (a, b, if c == ICE { EMBER } else { PINE }))
        .collect();
    let recoloured = shape_of(&draw(600, 600, &parts, 1.0));
    assert_eq!(
        compare(&front, &recoloured, RefView::Right),
        Mismatch {
            colours: true,
            proportions: false
        }
    );
}

#[test]
fn a_cut_off_object_is_only_compared_on_colour() {
    let front = front();
    let low = shape_of(&draw(600, 600, &lantern(0.3, 0.6), 1.0));
    let mut cut = low.clone();
    cut.cut = true;
    assert!(compare(&front, &low, RefView::Back).proportions);
    assert_eq!(compare(&front, &cut, RefView::Back), Mismatch::default());
}

#[test]
fn a_send_warns_for_the_back_in_plain_words() {
    let dir = tempfile::tempdir().expect("dir");
    fs::create_dir_all(dir.path().join("refs")).expect("refs");
    let files = [
        ("front", draw(600, 600, &lantern(0.3, 0.2), 1.0)),
        ("left", draw(600, 600, &lantern(0.2, 0.2), 0.8)),
        ("back", draw(600, 600, &chest(), 1.0)),
    ];
    let mut sent = Vec::new();
    for (name, bytes) in &files {
        let file = format!("refs/{name}.png");
        fs::write(dir.path().join(&file), bytes).expect("write");
        sent.push(file);
    }
    let views: Vec<String> = files.iter().map(|(v, _)| (*v).to_string()).collect();
    let w = view_warnings(dir.path(), &sent, &views);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].kind, WarningKind::ViewsDisagree);
    assert_eq!(w[0].view, Some(RefView::Back));
    assert_eq!(
        w[0].title,
        "Back looks different from the front: its colours and proportions do not match"
    );
    assert!(
        !w[0].detail.contains("wrong"),
        "never claims another object"
    );
    assert_eq!(w[0].fixes, vec![RefFix::MakeViews, RefFix::LeaveOut]);
    // No front among the views: nothing to compare with.
    assert!(view_warnings(dir.path(), &sent[1..], &views[1..]).is_empty());
}
