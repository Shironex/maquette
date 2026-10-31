//! Render styles: pictures of a model for a page, a post or a friend,
//! headless in Blender with the runner the pipeline uses and no second
//! renderer. Blueprint, toon and clay use Workbench with Freestyle lines,
//! shaded is the thumbnail's studio look in Cycles, a turntable is one full
//! turn as a looping file, and a contact sheet is eight angles on one PNG.
//!
//! ```text
//! <project>/exports/renders/street-lamp/blueprint.png, turntable.webp
//! <project>/exports/renders/winter-spire_contact.png
//! <workspace>/exports/.renders/<id>.json
//! ```
//!
//! A library model has no project, so its renders go to
//! `<workspace>/exports/renders/`. The rendering is [`render_styles.py`];
//! [`encode`] holds the rules of a loop; [`job`] runs a batch one file at a
//! time, keeps its record and goes on from where it stopped; [`preview`]
//! makes the small pictures the Render tab shows, cached by their settings.
//!
//! A WebP loop needs no other tool: Blender writes each frame and [`encode`]
//! joins them, since many ffmpeg builds have no WebP encoder. GIF and MP4 go
//! through ffmpeg from the toolchain.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

pub mod encode;
mod job;
mod plan;
mod preview;
mod record;
mod run;
mod spec;

pub use encode::{TARGET_BYTES, estimate_loop_bytes};
pub use job::{RenderEvent, Renders};
pub use plan::{RenderPlan, RenderTarget, plan};
pub use preview::{PreviewShot, RenderPreview, RenderShot};
pub use record::{
    MovingBox, ProblemKind, RenderFile, RenderFileState, RenderItem, RenderProblem, RenderRecord,
    RenderState,
};

/// What a file is drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RenderStyle {
    Blueprint,
    Toon,
    Clay,
    Shaded,
    Turntable,
    ContactSheet,
}

impl RenderStyle {
    pub const ALL: [RenderStyle; 6] = [
        Self::Blueprint,
        Self::Toon,
        Self::Clay,
        Self::Shaded,
        Self::Turntable,
        Self::ContactSheet,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Blueprint => "Blueprint",
            Self::Toon => "Toon",
            Self::Clay => "Clay",
            Self::Shaded => "Shaded",
            Self::Turntable => "Turntable",
            Self::ContactSheet => "Contact sheet",
        }
    }

    /// The file's name without its extension.
    pub fn stem(self) -> &'static str {
        match self {
            Self::Blueprint => "blueprint",
            Self::Toon => "toon",
            Self::Clay => "clay",
            Self::Shaded => "shaded",
            Self::Turntable => "turntable",
            Self::ContactSheet => "contact-sheet",
        }
    }
}

/// How a turntable or a contact sheet looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RenderLook {
    Blueprint,
    Toon,
    Clay,
    Shaded,
}

impl RenderLook {
    pub fn name(self) -> &'static str {
        match self {
            Self::Blueprint => "blueprint",
            Self::Toon => "toon",
            Self::Clay => "clay",
            Self::Shaded => "shaded",
        }
    }
}

/// A turntable's file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum LoopFormat {
    Webp,
    Gif,
    Mp4,
}

impl LoopFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Webp => "webp",
            Self::Gif => "gif",
            Self::Mp4 => "mp4",
        }
    }
}

/// Where a still's camera stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RenderCamera {
    Front,
    ThreeQuarter,
    Side,
}

/// A camera stop's view of a model: where the stop stands around it and how
/// far it looks down, in degrees, so a still can be taken from there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderStop {
    /// The stop's name, for the file and the picture's tag.
    pub name: String,
    /// The turn from the front, 0 to 359.
    pub azimuth: u32,
    /// How far it looks down, 0 to 80.
    pub elevation: u32,
}

/// How the files of a render are made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct RenderSettings {
    pub camera: RenderCamera,
    /// A camera stop's view, which a still takes in place of `camera`.
    pub stop: Option<RenderStop>,
    /// A still's edge in pixels.
    pub size: u32,
    /// Leaves the paper out of a still.
    pub transparent: bool,
    /// The blueprint's line colour.
    pub line_color: String,
    /// The paper behind a still; `None` keeps each style's own.
    pub paper_color: Option<String>,
    /// Line weight in pixels on a 1024 px picture.
    pub line_weight: f32,
    /// How sharp a fold must be, in degrees, to get a line.
    pub crease: u32,
    pub hidden_dashed: bool,
    pub marks: bool,
    pub title_block: bool,
    pub clay_color: String,
    pub format: LoopFormat,
    /// A loop's length in seconds, at the fixed [`encode::FPS`].
    pub length: u32,
    /// A loop's frame edge in pixels, before the crop.
    pub width: u32,
    pub look: RenderLook,
    /// How far a loop's camera looks down, in degrees.
    pub tilt: u32,
    /// Keeps the ground out of a loop, so only the model's pixels change.
    pub still_background: bool,
    /// Crops a loop right up to the model, no margin.
    pub tight_crop: bool,
    /// Columns of a batch's contact sheet.
    pub columns: u32,
    /// A name under each picture of a contact sheet.
    pub names: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            camera: RenderCamera::ThreeQuarter,
            stop: None,
            size: 2048,
            transparent: false,
            line_color: "#b3d9ff".to_string(),
            paper_color: None,
            line_weight: 1.6,
            crease: 30,
            hidden_dashed: false,
            marks: true,
            title_block: true,
            clay_color: "#a89e92".to_string(),
            format: LoopFormat::Webp,
            length: 3,
            width: 480,
            look: RenderLook::Clay,
            tilt: 12,
            still_background: true,
            tight_crop: false,
            columns: 3,
            names: true,
        }
    }
}

/// The still sizes the settings offer.
pub const SIZES: [u32; 4] = [512, 1024, 2048, 4096];
/// The longest loop, in seconds.
pub const MAX_LENGTH: u32 = 12;

fn is_hex(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl RenderSettings {
    /// Checks every value a caller could get wrong.
    ///
    /// # Errors
    ///
    /// Refuses a size, colour, weight, crease, length, width or tilt the
    /// settings do not offer, each with what would do.
    pub fn check(&self) -> CoreResult<()> {
        let bad = |message: &str| Err(CoreError::Invalid(message.to_string()));
        if !SIZES.contains(&self.size) {
            return bad("Pick a size of 512, 1024, 2048 or 4096 px.");
        }
        let colours = [
            Some(&self.line_color),
            self.paper_color.as_ref(),
            Some(&self.clay_color),
        ];
        if colours.into_iter().flatten().any(|c| !is_hex(c)) {
            return bad("Colours are written like #b3d9ff.");
        }
        if !(0.25..=8.0).contains(&self.line_weight) {
            return bad("Keep the line weight between 0.25 and 8 px.");
        }
        if !(10..=90).contains(&self.crease) {
            return bad("Keep the crease angle between 10 and 90 degrees.");
        }
        if !(1..=MAX_LENGTH).contains(&self.length) {
            return bad("Keep a loop between 1 and 12 seconds.");
        }
        if !(120..=1024).contains(&self.width) {
            return bad("Keep a loop's width between 120 and 1024 px.");
        }
        if self.tilt > 60 {
            return bad("Keep the camera tilt at 60 degrees or under.");
        }
        if let Some(stop) = &self.stop
            && (stop.azimuth >= 360 || stop.elevation > 80)
        {
            return bad("Keep a camera stop's turn under 360 degrees and its tilt at 80 or under.");
        }
        if !(1..=8).contains(&self.columns) {
            return bad("Keep a contact sheet between 1 and 8 columns.");
        }
        Ok(())
    }
}

/// The styles one model gets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderRow {
    /// The asset id in a project, the entry id in the library.
    pub key: String,
    pub styles: Vec<RenderStyle>,
}

/// Everything a render needs to start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderRequest {
    /// A project's models or library entries; a collection works too.
    pub source: crate::packs::PackSource,
    pub rows: Vec<RenderRow>,
    pub settings: RenderSettings,
    /// Writes over earlier renders; else the new files get a number.
    pub replace: bool,
    /// One contact sheet of all the models at the end, in the settings' look.
    pub sheet: bool,
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_blender;
#[cfg(test)]
mod tests_job;

/// Registers this module's IPC types with the shell's TypeScript export;
/// `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<RenderStyle>()
        .register::<RenderLook>()
        .register::<LoopFormat>()
        .register::<RenderCamera>()
        .register::<RenderStop>()
        .register::<RenderSettings>()
        .register::<RenderRow>()
        .register::<RenderRequest>()
        .register::<RenderTarget>()
        .register::<RenderPlan>()
        .register::<MovingBox>()
        .register::<ProblemKind>()
        .register::<RenderProblem>()
        .register::<RenderFileState>()
        .register::<RenderFile>()
        .register::<RenderItem>()
        .register::<RenderState>()
        .register::<RenderRecord>()
        .register::<RenderShot>()
        .register::<PreviewShot>()
        .register::<RenderPreview>()
}
