//! The review's pose sheet: the frames of a bake worth a look, picked from
//! the recipe instead of every tenth frame. Each seam where clips meet, each
//! pose key and where each freeze or hold over eases in and out; a recipe
//! with none of them shows where each change starts and ends. A frame that
//! a check warns on comes first, and at most [`SHEET_FRAMES`] are kept.

use serde::{Deserialize, Serialize};

use crate::recipes::joint_limits::KeyedLimits;
use crate::recipes::pins::frames_within;
use crate::recipes::pose_keys::key_frame;
use crate::recipes::seams::{SeamCheck, SeamKind};
use crate::recipes::timeline::{Timeline, plain};
use crate::recipes::{LayerKind, RecipeFile};

/// The most frames a sheet shows.
pub const SHEET_FRAMES: usize = 8;

/// Why a frame is on the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SheetKind {
    Seam,
    Key,
    Hold,
    Change,
}

/// One frame of the sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSheetFrame {
    /// The bake frame, from 0.
    pub frame: u32,
    pub kind: SheetKind,
    /// What is there: "seam 1", "chest key", "hold ends".
    pub label: String,
    /// What a check saw there: "hips jump 3.1 cm", "eases out, 9 deg".
    pub detail: Option<String>,
    /// A check warns on it.
    pub warn: bool,
}

fn seam_detail(seam: &SeamCheck) -> String {
    if seam.hips_worse() && seam.hips_over_cm > 0.0 {
        format!("hips jump {:.1} cm", seam.hips_cm)
    } else if seam.turn_over_degrees > 0.0 {
        format!(
            "{} turns {:.0} deg",
            seam.bone.as_deref().unwrap_or("a bone"),
            seam.turn_degrees
        )
    } else {
        match seam.blend {
            Some(0) => "cuts straight".to_string(),
            Some(n) => format!("blends over {n} f"),
            None => "smooth".to_string(),
        }
    }
}

fn frame(t: &Timeline, recipe: &RecipeFile, clip: u32, at: u32) -> Option<u32> {
    if plain(recipe) {
        (at <= t.frames).then_some(at)
    } else {
        key_frame(t, clip, at)
    }
}

/// Every frame worth a look, before the cap.
fn candidates(
    recipe: &RecipeFile,
    t: &Timeline,
    seams: &[SeamCheck],
    joints: Option<&KeyedLimits>,
) -> Vec<RecipeSheetFrame> {
    let mut out = Vec::new();
    let mut n = 0;
    for seam in seams {
        let (kind, label) = match seam.kind {
            SeamKind::Join => {
                n += 1;
                (SheetKind::Seam, format!("seam {n}"))
            }
            SeamKind::HoldIn | SeamKind::HoldOut => {
                let what = if seam.freeze { "freeze" } else { "hold" };
                let way = if seam.kind == SeamKind::HoldIn {
                    "starts"
                } else {
                    "ends"
                };
                (SheetKind::Hold, format!("{what} {way}"))
            }
        };
        let detail = if kind == SheetKind::Hold {
            let way = if seam.kind == SeamKind::HoldIn {
                "eases in"
            } else {
                "eases out"
            };
            format!("{way}, {:.0} deg", seam.turn_degrees)
        } else {
            seam_detail(seam)
        };
        out.push(RecipeSheetFrame {
            frame: seam.frame,
            kind,
            label,
            detail: Some(detail),
            warn: seam.pops,
        });
    }
    for layer in recipe.layers.iter().filter(|l| l.kind == LayerKind::Pose) {
        for key in &layer.keys {
            let Some(at) = frame(t, recipe, key.clip, key.frame) else {
                continue;
            };
            let bone = match key.pose.len() {
                1 => key.pose.keys().next().cloned().unwrap_or_default(),
                count => format!("{count} bones"),
            };
            let hit = joints.and_then(|j| {
                j.hits
                    .iter()
                    .find(|h| h.layer == layer.id && h.clip == key.clip && h.frame == key.frame)
            });
            out.push(RecipeSheetFrame {
                frame: at,
                kind: SheetKind::Key,
                label: format!("{bone} key"),
                detail: hit.map(|h| {
                    format!(
                        "{} {:.0} of {:.0} deg",
                        h.hit.word,
                        h.hit.degrees.abs(),
                        h.hit.limit.abs()
                    )
                }),
                warn: hit.is_some(),
            });
        }
    }
    out
}

/// Where each change starts and ends, for a recipe with no seam, key or
/// hold to show.
fn change_frames(recipe: &RecipeFile, t: &Timeline) -> Vec<RecipeSheetFrame> {
    let mut out = Vec::new();
    for layer in &recipe.layers {
        let s = &layer.settings;
        let Some((first, last)) = frames_within(t, s.start, s.end) else {
            continue;
        };
        for (at, way) in [(first, "starts"), (last, "ends")] {
            out.push(RecipeSheetFrame {
                frame: at,
                kind: SheetKind::Change,
                label: format!("{} {way}", layer.bone),
                detail: None,
                warn: false,
            });
        }
    }
    out
}

/// The sheet of `recipe` baked on timeline `t`: the frames worth a look,
/// in frame order, at most [`SHEET_FRAMES`], those a check warns on kept
/// first and no two within a frame of each other.
pub(super) fn sheet_of(
    recipe: &RecipeFile,
    t: &Timeline,
    seams: &[SeamCheck],
    joints: Option<&KeyedLimits>,
) -> Vec<RecipeSheetFrame> {
    let mut found = candidates(recipe, t, seams, joints);
    if found.is_empty() {
        found = change_frames(recipe, t);
    }
    // Warnings first, then seams, keys, holds and changes, each in order.
    found.sort_by_key(|f| (!f.warn, f.kind as u8, f.frame));
    let mut kept: Vec<RecipeSheetFrame> = Vec::new();
    for f in found {
        if kept.len() >= SHEET_FRAMES || kept.iter().any(|k| k.frame.abs_diff(f.frame) <= 1) {
            continue;
        }
        kept.push(f);
    }
    kept.sort_by_key(|f| f.frame);
    kept
}
