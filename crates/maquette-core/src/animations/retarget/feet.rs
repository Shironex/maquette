//! Planted feet: when a foot is down, how far it slides while it is, and
//! where Pin the feet holds it. The Blender run reports each foot's world
//! position per baked frame (Z up); the review measures the slide from
//! those tracks, and Pin the feet bakes again holding each foot still over
//! the same planted runs, so both use one test.
//!
//! A foot is planted on a frame when it is low (within [`PLANTED_HEIGHT`]
//! of its lowest point in the clip) and slow (it moved at most
//! [`PLANTED_STEP`] along the ground since the frame before), for at least
//! [`PLANTED_RUN`] frames in a row. A sliding foot stays planted; a foot
//! swinging through a step is too fast to count.

use std::collections::BTreeMap;

/// A foot this close to its lowest point counts as down, metres.
pub const PLANTED_HEIGHT: f32 = 0.025;
/// The most a planted foot moves along the ground in one frame, metres.
pub const PLANTED_STEP: f32 = 0.08;
/// The fewest frames in a row that make a plant.
pub const PLANTED_RUN: usize = 3;

/// A foot's world position per baked frame, metres, Z up.
pub type Track = Vec<[f32; 3]>;

/// The worst slide of a planted foot in a clip.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Slide {
    pub cm: f32,
    /// The first and last frame of the plant it slides in.
    pub from: u32,
    pub to: u32,
    /// "left" or "right".
    pub foot: String,
}

fn along_ground(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn frame(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

/// The frames a foot is planted on, as first and last frame of each run.
pub fn planted_runs(track: &[[f32; 3]]) -> Vec<[u32; 2]> {
    if track.len() < PLANTED_RUN {
        return Vec::new();
    }
    let floor = track.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
    let planted = |i: usize| {
        let step = match i {
            0 => along_ground(track[0], track[1]),
            _ => along_ground(track[i - 1], track[i]),
        };
        track[i][2] <= floor + PLANTED_HEIGHT && step <= PLANTED_STEP
    };
    let mut runs = Vec::new();
    let mut start = None;
    for i in 0..=track.len() {
        match (i < track.len() && planted(i), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if i - s >= PLANTED_RUN {
                    runs.push([frame(s), frame(i - 1)]);
                }
                start = None;
            }
            _ => {}
        }
    }
    runs
}

/// The worst drift from where a foot was put down, over every plant of
/// every foot.
pub fn slide(feet: &BTreeMap<String, Track>) -> Slide {
    let mut worst = Slide {
        foot: "left".to_string(),
        ..Slide::default()
    };
    for (foot, track) in feet {
        for [from, to] in planted_runs(track) {
            let (a, b) = (from as usize, to as usize);
            let start = track[a];
            let drift = track[a..=b]
                .iter()
                .map(|p| along_ground(start, *p))
                .fold(0.0, f32::max);
            if drift * 100.0 > worst.cm {
                worst = Slide {
                    cm: drift * 100.0,
                    from,
                    to,
                    foot: foot.clone(),
                };
            }
        }
    }
    worst
}

/// Where Pin the feet holds each foot: its planted runs.
pub fn pins(feet: &BTreeMap<String, Track>) -> BTreeMap<String, Vec<[u32; 2]>> {
    feet.iter()
        .map(|(foot, track)| (foot.clone(), planted_runs(track)))
        .filter(|(_, runs)| !runs.is_empty())
        .collect()
}
