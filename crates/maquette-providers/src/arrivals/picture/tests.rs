//! The picture scorer on images drawn here: a red block (the download's
//! render) against a red block ref framed apart, a red disc and a blue disc.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::arrivals::matching::tests::{T, waiting};
use maquette_glb::images::write_png;
use maquette_model::refs::{picture, png};

const GROUND: [u8; 3] = [212, 212, 216];
const RED: [u8; 3] = [196, 40, 36];
const RED_LIT: [u8; 3] = [226, 70, 60];
const BLUE: [u8; 3] = [40, 70, 200];

/// A `size` square of studio grey with a shape drawn by `inside`, which
/// takes the fractions (x, y) of the side and returns the colour there.
fn draw(size: u32, inside: impl Fn(f64, f64) -> Option<[u8; 3]>) -> Vec<u8> {
    let mut rgb = Vec::new();
    let s = f64::from(size);
    for y in 0..size {
        for x in 0..size {
            let (fx, fy) = (f64::from(x) / s, f64::from(y) / s);
            rgb.extend_from_slice(&inside(fx, fy).unwrap_or(GROUND));
        }
    }
    write_png(size, size, &rgb)
}

/// A red block, its top face lit, from `from` to `to` (fractions).
fn block(from: (f64, f64), to: (f64, f64)) -> Vec<u8> {
    draw(600, |x, y| {
        let inside = x >= from.0 && x < to.0 && y >= from.1 && y < to.1;
        let top = y < from.1 + (to.1 - from.1) * 0.2;
        inside.then_some(if top { RED_LIT } else { RED })
    })
}

fn disc(colour: [u8; 3]) -> Vec<u8> {
    draw(600, |x, y| {
        (((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt() < 0.3).then_some(colour)
    })
}

fn object(bytes: &[u8]) -> Object {
    picture::object(&png::decode(bytes).expect("png")).expect("an object")
}

#[test]
fn a_red_block_looks_most_like_the_red_block_ref() {
    let render = object(&block((0.3, 0.3), (0.7, 0.7)));
    // The same block, drawn bigger and off centre, as a ref often is.
    let same = score(&render, &object(&block((0.1, 0.15), (0.75, 0.8)))).total();
    let red_disc = score(&render, &object(&disc(RED))).total();
    let blue_disc = score(&render, &object(&disc(BLUE))).total();
    assert!(same > 0.9, "the same block scores {same}");
    assert!(
        same > red_disc + 0.05,
        "{same} against a red disc at {red_disc}"
    );
    assert!(
        red_disc > blue_disc + 0.2,
        "{red_disc} against a blue disc at {blue_disc}"
    );
    assert!(
        blue_disc < LOOKS_LIKE,
        "a blue disc never looks like it: {blue_disc}"
    );
}

#[test]
fn a_tall_block_does_not_fit_a_wide_one() {
    let tall = object(&block((0.4, 0.1), (0.6, 0.9)));
    let wide = object(&block((0.1, 0.4), (0.9, 0.6)));
    let found = score(&tall, &wide);
    assert!(found.ratio < 0.1, "{found:?}");
    assert!(found.shape < 0.2, "{found:?}");
    assert!(found.colour > 0.9, "same paint: {found:?}");
}

/// A project folder with one ref per waiting send.
fn sends(dir: &std::path::Path) -> Vec<crate::jobs::ProviderJob> {
    let refs = dir.join("refs");
    fs::create_dir_all(&refs).expect("refs");
    fs::write(refs.join("crate.png"), block((0.2, 0.2), (0.8, 0.8))).expect("crate");
    fs::write(refs.join("ball.png"), disc(BLUE)).expect("ball");
    let mut red = waiting("red-crate", "Red crate", 1, T);
    red.images = vec!["refs/crate.png".into()];
    let mut blue = waiting("blue-ball", "Blue ball", 1, T);
    // Tagged views: the front is the second image.
    blue.images = vec!["refs/missing.png".into(), "refs/ball.png".into()];
    blue.views = vec!["left".into(), "front".into()];
    vec![blue, red]
}

#[test]
fn every_send_is_ranked_by_its_picture_and_the_best_is_only_suggested() {
    let dir = tempfile::tempdir().expect("dir");
    let jobs = sends(dir.path());
    let root = dir.path().to_path_buf();
    let project_dir = move |_: &str| -> Option<PathBuf> { Some(root.clone()) };
    let render = object(&block((0.3, 0.3), (0.7, 0.7)));

    let ranked = rank("model (3).glb", None, Some(&render), &jobs, &project_dir);
    let order: Vec<&str> = ranked.iter().map(|s| s.asset.as_str()).collect();
    assert_eq!(order, ["red-crate", "blue-ball"]);
    assert_eq!(
        ranked[1].image.as_deref(),
        Some("refs/ball.png"),
        "the front ref"
    );
    let best = looks_like(&ranked).expect("a clear best");
    assert_eq!(best.asset, "red-crate");
    assert!(best.picture.expect("scored by picture") > 0.85, "{best:?}");

    // Without a render the name ranks them, and nothing is suggested by picture.
    let named = rank("blue ball.glb", None, None, &jobs, &project_dir);
    assert_eq!(named[0].asset, "blue-ball");
    assert_eq!(named[0].picture, None);
    assert_eq!(looks_like(&named), None);

    // A name only breaks ties: the picture still wins against a close name.
    let misnamed = rank("blue ball.glb", None, Some(&render), &jobs, &project_dir);
    assert_eq!(misnamed[0].asset, "red-crate");
}

#[test]
fn two_sends_that_look_alike_suggest_neither() {
    let dir = tempfile::tempdir().expect("dir");
    let refs = dir.path().join("refs");
    fs::create_dir_all(&refs).expect("refs");
    fs::write(refs.join("a.png"), block((0.2, 0.2), (0.8, 0.8))).expect("a");
    fs::write(refs.join("b.png"), block((0.25, 0.25), (0.75, 0.75))).expect("b");
    let mut a = waiting("crate-a", "Crate", 1, T);
    a.images = vec!["refs/a.png".into()];
    let mut b = waiting("crate-b", "Crate", 1, T);
    b.images = vec!["refs/b.png".into()];
    let root = dir.path().to_path_buf();
    let project_dir = move |_: &str| -> Option<PathBuf> { Some(root.clone()) };
    let render = object(&block((0.3, 0.3), (0.7, 0.7)));
    let ranked = rank("model.glb", None, Some(&render), &[a, b], &project_dir);
    assert_eq!(ranked.len(), 2);
    assert_eq!(looks_like(&ranked), None, "{ranked:?}");
}
