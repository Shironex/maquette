//! A bake on the rest pose: no base clip goes on the model, so the bake
//! script holds the rest pose for the recipe's frames at the rest pose's
//! rate and lays the changes over it. Its clip is named like any other.

use serde::Serialize;

use crate::animations::retarget::ClipSetting;
use crate::animations::retarget::settings::free;
use crate::recipes::REST_FPS;

/// What the clip is called when the recipe's name gives nothing.
const FALLBACK: &str = "recipe";

/// The clip a rest pose bake writes: `base` (the recipe's name in
/// snake_case) kept free of `taken`, over `frames` frames.
pub(super) fn setting(base: &str, frames: u32, taken: &[String]) -> ClipSetting {
    let base = if base.is_empty() { FALLBACK } else { base };
    ClipSetting {
        clip: String::new(),
        name: free(base, taken),
        start: 0,
        end: frames,
        loops: false,
        root_motion: false,
        speed: 1.0,
        pin_feet: false,
    }
}

/// The rest pose as the bake script reads it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RestSpec<'a> {
    /// The new clip's name.
    pub name: &'a str,
    pub frames: u32,
    pub fps: f32,
}

impl<'a> RestSpec<'a> {
    pub(super) fn of(setting: &'a ClipSetting) -> Self {
        Self {
            name: &setting.name,
            frames: setting.end,
            fps: REST_FPS,
        }
    }
}
