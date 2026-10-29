//! How each library clip goes onto one model: its name there, the frames
//! it keeps, whether it loops and travels, its speed and whether its feet
//! are pinned while planted. Settings belong to that model only and are
//! never saved back into the library.

use serde::{Deserialize, Serialize};

use crate::animations::AnimationClip;
use crate::error::{CoreError, CoreResult};

/// The slowest and fastest a clip may play.
pub const SPEED_MIN: f32 = 0.25;
pub const SPEED_MAX: f32 = 4.0;

/// Words a clip's name in a model leaves out besides its pack's.
const FILLER: [&str; 3] = ["standing", "pro", "pack"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipSetting {
    /// The library clip's id.
    pub clip: String,
    /// Its name in the model, snake_case.
    pub name: String,
    /// The first and last frame kept, of the library clip's own frames: a
    /// clip of `frames` frames runs from frame 0 to frame `frames`, so its
    /// whole length is `frames / fps` seconds.
    pub start: u32,
    pub end: u32,
    pub loops: bool,
    /// Keeps the root's travel in the clip; off plays it in place.
    pub root_motion: bool,
    pub speed: f32,
    /// Holds each foot still while it is planted and solves the leg back
    /// to it, for a clip whose feet slide.
    #[serde(default)]
    pub pin_feet: bool,
}

/// `Sword And Shield Idle` (pack `Pro Sword and Shield Pack`) reads
/// `sword_idle`: snake_case, with the words the pack shares dropped after
/// the first, and a few filler words dropped everywhere.
pub fn default_name(name: &str, pack: &str) -> String {
    let words = |s: &str| -> Vec<String> {
        s.split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_ascii_lowercase)
            .collect()
    };
    let pack_words = words(pack);
    let kept: Vec<String> = words(name)
        .into_iter()
        .filter(|w| !FILLER.contains(&w.as_str()))
        .enumerate()
        .filter(|(i, w)| *i == 0 || !pack_words.contains(w))
        .map(|(_, w)| w)
        .collect();
    let out = kept.join("_");
    if out.is_empty() {
        words(name).join("_")
    } else {
        out
    }
}

/// `Sword Run!` reads `sword_run`; what is left of a name typed by hand.
fn snake(name: &str) -> String {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("_")
}

/// `name`, or `name_2` and on when the model or an earlier clip has it.
fn free(name: &str, taken: &[String]) -> String {
    if !taken.iter().any(|t| t == name) {
        return name.to_string();
    }
    (2u32..)
        .map(|n| format!("{name}_{n}"))
        .find(|n| !taken.contains(n))
        .unwrap_or_else(|| name.to_string())
}

/// The settings a clip starts with on a model: its whole length, its own
/// loop and root motion, normal speed.
pub fn default_for(clip: &AnimationClip) -> ClipSetting {
    ClipSetting {
        clip: clip.id.clone(),
        name: default_name(&clip.name, &clip.pack),
        start: 0,
        end: clip.frames,
        loops: clip.loops,
        root_motion: clip.root_motion.is_some(),
        speed: 1.0,
        pin_feet: false,
    }
}

/// `settings` made safe to bake: names in snake_case and unique against
/// the model's own clips (`taken`) and each other, frames inside the clip
/// (a trim that keeps nothing keeps it all), speed inside its range.
///
/// # Errors
///
/// `Invalid` for a clip the library does not have, or the same clip twice.
pub fn normalise(
    settings: &[ClipSetting],
    clips: &[AnimationClip],
    taken: &[String],
) -> CoreResult<Vec<ClipSetting>> {
    let mut used: Vec<String> = taken.to_vec();
    let mut seen: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for s in settings {
        let clip = clips
            .iter()
            .find(|c| c.id == s.clip)
            .ok_or_else(|| CoreError::Invalid(format!("The library has no clip {}.", s.clip)))?;
        if seen.contains(&s.clip.as_str()) {
            return Err(CoreError::Invalid(format!(
                "{} is chosen twice. Choose each clip once.",
                clip.name
            )));
        }
        seen.push(&s.clip);
        let last = clip.frames;
        let (start, end) = if s.start < s.end && s.end <= last {
            (s.start, s.end)
        } else if s.start < s.end.min(last) {
            (s.start, last)
        } else {
            (0, last)
        };
        let typed = snake(&s.name);
        let base = if typed.is_empty() {
            default_name(&clip.name, &clip.pack)
        } else {
            typed
        };
        let name = free(&base, &used);
        used.push(name.clone());
        let speed = if s.speed.is_finite() {
            s.speed.clamp(SPEED_MIN, SPEED_MAX)
        } else {
            1.0
        };
        out.push(ClipSetting {
            clip: s.clip.clone(),
            name,
            start,
            end,
            loops: s.loops,
            root_motion: s.root_motion,
            speed,
            pin_feet: s.pin_feet,
        });
    }
    Ok(out)
}
