//! The combat starters: a two cut sword combo and a charged two handed
//! scythe swing, each with a left handed copy. Like every starter they name
//! their clips by name only, a few names tried in order, and never carry a
//! clip's keys; their pose keys, hold and blends are laid as shares of the
//! clips they find, so they fit a clip of any length. Each holds its weapon
//! in a grip (one hand for the sword, both for the scythe).

use crate::animations::AnimationClip;
use crate::animations::recipes::{base_of, clip_named};
use crate::recipes::grips::mirror_pose;
use crate::recipes::{
    BlendWindow, BonePose, KeyEase, LayerKind, LayerMode, LayerSettings, LineUp, PoseKey,
    RECIPE_VERSION, REST_FPS, RecipeBase, RecipeFile, RecipeLayer, TimeEdit, TimeKind,
};

/// One clip of a combo: the names it goes by, tried in order, how long to
/// take it to be when no clip has one of them, and the blend into it.
pub(super) struct Join {
    pub names: &'static [&'static str],
    pub frames: u32,
    pub blend: u32,
}

/// A combat starter: its clips (the first the base) and what it lays on
/// them once their lengths are known.
pub(super) struct Combo {
    pub key: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    pub clips: &'static [Join],
    pub mirror: bool,
    pub shape: fn(&mut RecipeFile),
}

const SLASH: &[&str] = &["Sword And Shield Slash", "Sword Slash", "Slash"];
const SECOND_CUT: &[&str] = &[
    "Sword And Shield Attack",
    "Sword And Shield Slash",
    "Sword Slash",
    "Slash",
];
const SWORD_IDLE: &[&str] = &["Sword And Shield Idle", "Idle", "Breathing Idle"];
const HEAVY_SWING: &[&str] = &[
    "Great Sword Slash",
    "Two Handed Slash",
    "Standing Melee Attack Horizontal",
    "Sword And Shield Slash",
];
const HEAVY_IDLE: &[&str] = &["Great Sword Idle", "Idle", "Breathing Idle"];

const SLASH_CLIPS: &[Join] = &[
    Join {
        names: SLASH,
        frames: 40,
        blend: 0,
    },
    Join {
        names: SECOND_CUT,
        frames: 40,
        blend: 6,
    },
    Join {
        names: SWORD_IDLE,
        frames: 60,
        blend: 10,
    },
];
const SCYTHE_CLIPS: &[Join] = &[
    Join {
        names: HEAVY_SWING,
        frames: 50,
        blend: 0,
    },
    Join {
        names: HEAVY_IDLE,
        frames: 60,
        blend: 10,
    },
];

pub(super) const COMBOS: [Combo; 4] = [
    Combo {
        key: "slash-combo",
        name: "Slash combo",
        about: "Two sword cuts blended into a guard, the chest leaning into each cut, the sword held in the right hand.",
        clips: SLASH_CLIPS,
        mirror: false,
        shape: slash_combo,
    },
    Combo {
        key: "slash-combo-left",
        name: "Slash combo, left handed",
        about: "The slash combo mirrored: the left hand holds the sword and leads each cut.",
        clips: SLASH_CLIPS,
        mirror: true,
        shape: slash_combo,
    },
    Combo {
        key: "scythe-charge",
        name: "Scythe charge",
        about: "Winds a two handed swing back, holds the charge, then lets it go into an idle; both hands on the scythe.",
        clips: SCYTHE_CLIPS,
        mirror: false,
        shape: scythe_charge,
    },
    Combo {
        key: "scythe-charge-left",
        name: "Scythe charge, left handed",
        about: "The scythe charge mirrored: it winds back and swings to the other side.",
        clips: SCYTHE_CLIPS,
        mirror: true,
        shape: scythe_charge,
    },
];

/// The base `join` takes from `clips`: the first clip with one of its
/// names, else its first name, as long as it guesses.
fn clip_of(join: &Join, clips: &[AnimationClip]) -> RecipeBase {
    join.names
        .iter()
        .find_map(|name| clip_named(clips, name))
        .map_or_else(
            || RecipeBase {
                frames: join.frames,
                length: join.frames as f32 / REST_FPS,
                ..RecipeBase::named(join.names[0])
            },
            base_of,
        )
}

/// `combo` as a recipe file called `name` on what `clips` holds.
pub(super) fn recipe_of(combo: &Combo, clips: &[AnimationClip]) -> RecipeFile {
    let mut found: Vec<RecipeBase> = combo.clips.iter().map(|j| clip_of(j, clips)).collect();
    for i in 1..found.len() {
        let most = found[i - 1].kept().min(found[i].kept()) / 2;
        found[i].blend = Some(BlendWindow {
            frames: combo.clips[i].blend.min(most),
            line_up: LineUp::HipsAndFacing,
            ..BlendWindow::default()
        });
    }
    let base = found.remove(0);
    let mut recipe = RecipeFile::new(combo.name, base);
    recipe.version = RECIPE_VERSION;
    recipe.joined = found;
    recipe.mirror = combo.mirror;
    (combo.shape)(&mut recipe);
    recipe
}

/// The frame `share` of the way through clip `clip` of `recipe`.
fn at(recipe: &RecipeFile, clip: u32, share: f32) -> u32 {
    let frames = recipe.clip(clip).map_or(1, |c| c.frames);
    ((frames.saturating_sub(1)) as f32 * share).round() as u32
}

fn key(
    recipe: &RecipeFile,
    clip: u32,
    share: f32,
    ease: KeyEase,
    pose: &[(&str, [f32; 3])],
) -> PoseKey {
    PoseKey {
        clip,
        frame: at(recipe, clip, share),
        ease,
        pose: pose.iter().map(|(b, t)| ((*b).to_string(), *t)).collect(),
    }
}

/// A pose key change laid on top of the clips.
fn keyed(keys: Vec<PoseKey>) -> RecipeLayer {
    let mut keys = keys;
    keys.dedup_by_key(|k| (k.clip, k.frame));
    RecipeLayer {
        mode: Some(LayerMode::Add),
        keys,
        ..RecipeLayer::new("l-1", LayerKind::Pose, "", LayerSettings::default())
    }
}

/// A right hand closed around a handle: each finger joint curled toward
/// the palm by `curl` (the little finger a little more), the thumb folded
/// across and the wrist turned by `wrist`.
fn right_hand(curl: [f32; 3], wrist: [f32; 3]) -> BonePose {
    let mut pose = BonePose::from([("rightHand".to_string(), wrist)]);
    for (f, finger) in ["Index", "Middle", "Ring", "Little"].iter().enumerate() {
        for (j, joint) in ["Proximal", "Intermediate", "Distal"].iter().enumerate() {
            pose.insert(
                format!("right{finger}{joint}"),
                [0.0, 0.0, curl[j] + f as f32 * 3.0],
            );
        }
    }
    pose.insert("rightThumbMetacarpal".into(), [18.0, -24.0, 10.0]);
    pose.insert(
        "rightThumbProximal".into(),
        [0.0, -16.0, (curl[0] / 3.0).round()],
    );
    pose.insert(
        "rightThumbDistal".into(),
        [0.0, -12.0, (curl[1] / 4.0).round()],
    );
    pose
}

/// A grip change holding `pose` across the whole recipe, in place of the
/// clips' own hands, easing in and out over 4 frames.
fn grip(pose: BonePose, hand: &str) -> RecipeLayer {
    RecipeLayer {
        mode: Some(LayerMode::Replace),
        pose: Some(pose),
        ..RecipeLayer::new(
            "l-2",
            LayerKind::Grip,
            hand,
            LayerSettings {
                ease_in: Some(4),
                ease_out: Some(4),
                ..LayerSettings::default()
            },
        )
    }
}

fn slash_combo(r: &mut RecipeFile) {
    let keys = vec![
        key(r, 0, 0.15, KeyEase::Smooth, &[("chest", [0.0, 0.0, 0.0])]),
        key(r, 0, 0.5, KeyEase::Out, &[("chest", [8.0, -14.0, 0.0])]),
        key(r, 1, 0.1, KeyEase::Smooth, &[("chest", [0.0, 0.0, 0.0])]),
        key(r, 1, 0.5, KeyEase::Out, &[("chest", [6.0, 16.0, 0.0])]),
        key(r, 2, 0.3, KeyEase::Smooth, &[("chest", [0.0, 0.0, 0.0])]),
    ];
    r.layers = vec![
        keyed(keys),
        grip(right_hand([62.0, 74.0, 48.0], [0.0, 0.0, 6.0]), "rightHand"),
    ];
}

fn scythe_charge(r: &mut RecipeFile) {
    let wound = at(r, 0, 0.3);
    let held = at(r, 0, 0.55).saturating_sub(wound).max(1);
    r.time = vec![TimeEdit {
        kind: TimeKind::HoldOver,
        clip: 0,
        from: None,
        to: None,
        speed: None,
        ease: None,
        at: Some(wound),
        frames: Some(held),
        breathe: Some(0.3),
        ease_in: Some(4.min(held)),
        ease_out: Some(4.min(held)),
    }];
    let keys = vec![
        key(r, 0, 0.1, KeyEase::Smooth, &[("spine", [0.0, 0.0, 0.0])]),
        key(r, 0, 0.3, KeyEase::Out, &[("spine", [-10.0, 20.0, 0.0])]),
        key(r, 0, 0.7, KeyEase::In, &[("spine", [8.0, -24.0, 0.0])]),
        key(r, 1, 0.2, KeyEase::Smooth, &[("spine", [0.0, 0.0, 0.0])]),
    ];
    let right = right_hand([58.0, 70.0, 44.0], [0.0, 8.0, 0.0]);
    let both: BonePose = mirror_pose(&right).into_iter().chain(right).collect();
    r.layers = vec![keyed(keys), grip(both, "rightHand")];
}
