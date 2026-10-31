//! The small pictures the Render tab shows: stills at a low size, and a
//! turn's eight sample frames with the box they move in and what the loop
//! would weigh. Cached in the workspace cache by the model's content and
//! the settings, so going back to a style is instant.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::encode::{estimate_loop_bytes, frames_of, margin_of};
use super::plan::{self, RenderTarget};
use super::record::MovingBox;
use super::spec;
use super::{RenderSettings, RenderStyle};
use crate::error::{CoreError, CoreResult};
use crate::packs::PackSource;
use crate::pipeline::runner::PipelineRunner;
use crate::pipeline::tools::Toolbox;
use crate::toolchain::ToolId;
use crate::workspace::WorkspaceConfig;

/// The lane previews share, apart from the files being made.
const LANE: &str = "render-previews";
const TIMEOUT: Duration = Duration::from_secs(5 * 60);
const SAMPLE_FRAMES: u32 = 8;
/// The most shots one call may ask for.
const MAX_SHOTS: usize = 16;

/// A picture the tab wants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderShot {
    pub style: RenderStyle,
    /// Degrees on from the settings' camera.
    pub turn: u32,
    pub size: u32,
}

/// A picture made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewShot {
    pub style: RenderStyle,
    pub turn: u32,
    pub size: u32,
    /// The picture; a turn's first frame.
    pub file: String,
    pub seconds: f64,
    /// A turn's eight sample frames.
    pub frames: Vec<String>,
    /// A turn's moving area, at its size.
    pub moving: Option<MovingBox>,
    /// What the whole loop would weigh, about, with the settings' format.
    pub estimate_bytes: Option<f64>,
}

/// A model's pictures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderPreview {
    pub key: String,
    pub name: String,
    pub version: u32,
    pub height_m: Option<f32>,
    pub has_colour: bool,
    pub folder: String,
    /// Where the finished files go, an absolute path.
    pub dir: String,
    pub shots: Vec<PreviewShot>,
}

fn is_turn(style: RenderStyle) -> bool {
    matches!(style, RenderStyle::Turntable | RenderStyle::ContactSheet)
}

fn digest(
    target: &RenderTarget,
    settings: &RenderSettings,
    shot: &RenderShot,
) -> CoreResult<String> {
    let model = file_digest(Path::new(&target.file))?;
    let mut hasher = Sha256::new();
    hasher.update(model.as_bytes());
    hasher.update(spec::SCRIPT.as_bytes());
    let parts = serde_json::to_string(&(settings, shot, &target.name, target.height_m))
        .map_err(|e| CoreError::Invalid(e.to_string()))?;
    hasher.update(parts.as_bytes());
    Ok(hex::encode(&hasher.finalize()[..12]))
}

fn file_digest(file: &Path) -> CoreResult<String> {
    crate::pipeline::cache::hash_file(file).map_err(|e| CoreError::io(file, e))
}

/// What a shot leaves behind in its folder.
fn stored(dir: &Path, shot: &RenderShot) -> Option<(PathBuf, Vec<PathBuf>, Option<MovingBox>)> {
    if is_turn(shot.style) {
        let frames: Vec<PathBuf> = (1..=SAMPLE_FRAMES)
            .map(|n| dir.join(format!("f{n:04}.png")))
            .collect();
        if !frames.iter().all(|f| f.is_file()) {
            return None;
        }
        let text = fs::read_to_string(dir.join("moving.json")).ok()?;
        let moving = serde_json::from_str(&text).ok()?;
        return Some((frames[0].clone(), frames, moving));
    }
    let file = dir.join("picture.png");
    file.is_file().then(|| (file, Vec::new(), None))
}

fn job_of(settings: &RenderSettings, shot: &RenderShot, dir: &Path) -> Value {
    if is_turn(shot.style) {
        let margin = margin_of(shot.size, settings.tight_crop);
        // The frames stay whole: the tab draws the crop box over them.
        let mut job = spec::turn_job(settings, dir, shot.size, SAMPLE_FRAMES, 0, 0);
        if let Value::Object(fields) = &mut job {
            fields.insert("boxMargin".to_string(), json!(margin));
        }
        return job;
    }
    spec::still_job(
        shot.style,
        settings,
        &dir.join("picture.png"),
        shot.size,
        shot.turn,
    )
}

fn answer(
    settings: &RenderSettings,
    shot: &RenderShot,
    made: (PathBuf, Vec<PathBuf>, Option<MovingBox>),
    seconds: f64,
) -> PreviewShot {
    let (file, frames, moving) = made;
    let estimate_bytes = moving
        .filter(|_| shot.style == RenderStyle::Turntable)
        .map(|m| {
            // The sample's box, scaled up to the width the loop would have.
            let scale = f64::from(settings.width) / f64::from(m.canvas.max(1));
            let (w, h) = (f64::from(m.w) * scale, f64::from(m.h) * scale);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "pixel counts"
            )]
            let size = (w.round() as u32, h.round() as u32);
            estimate_loop_bytes(
                settings.format,
                settings.look,
                size,
                frames_of(settings.length),
            )
        });
    PreviewShot {
        style: shot.style,
        turn: shot.turn,
        size: shot.size,
        file: file.display().to_string(),
        seconds,
        frames: frames.iter().map(|f| f.display().to_string()).collect(),
        moving,
        estimate_bytes,
    }
}

/// Makes what is not cached in one Blender run, then answers every shot.
pub(super) fn make(
    runner: &PipelineRunner,
    root: &Path,
    config: &WorkspaceConfig,
    tools: &Toolbox,
    source: &PackSource,
    key: &str,
    settings: &RenderSettings,
    shots: &[RenderShot],
) -> CoreResult<RenderPreview> {
    settings.check()?;
    if shots.is_empty() || shots.len() > MAX_SHOTS {
        return Err(CoreError::Invalid(format!(
            "Ask for 1 to {MAX_SHOTS} pictures at a time."
        )));
    }
    let planned = plan::plan(root, &config.layout, source)?;
    let target = plan::target(&planned, key)?;
    if let Some(why) = &target.unreadable {
        return Err(CoreError::Invalid(why.clone()));
    }
    let cache = crate::thumbs::thumbs_dir(root, config).with_file_name("render-previews");
    let folders: Vec<PathBuf> = shots
        .iter()
        .map(|s| Ok(cache.join(digest(target, settings, s)?)))
        .collect::<CoreResult<_>>()?;
    let mut jobs = Vec::new();
    let mut fresh = Vec::new();
    for (n, (shot, dir)) in shots.iter().zip(&folders).enumerate() {
        if stored(dir, shot).is_none() {
            fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
            jobs.push(job_of(settings, shot, dir));
            fresh.push(n);
        }
    }
    let mut seconds = vec![0.0; shots.len()];
    if !jobs.is_empty() {
        if !tools.has(ToolId::Blender) {
            return Err(CoreError::Unavailable(
                "Blender was not found. Render styles use headless Blender. Set its path in Settings, Toolchain, then render again."
                    .to_string(),
            ));
        }
        let work = cache.join("run");
        let report = spec::blender(runner, tools, LANE, &work, Some(target), jobs, TIMEOUT)?
            .map_err(|failed| CoreError::Invalid(format!("Blender stopped: {}", failed.message)))?;
        for (at, line) in fresh.iter().zip(&report.jobs) {
            seconds[*at] = line.seconds;
            if is_turn(shots[*at].style) {
                let moving = serde_json::to_string(&line.moving)
                    .map_err(|e| CoreError::Invalid(e.to_string()))?;
                let path = folders[*at].join("moving.json");
                crate::fs::write_atomic(&path, moving.as_bytes())
                    .map_err(|e| CoreError::io(&path, e))?;
            }
        }
    }
    let made = shots
        .iter()
        .zip(&folders)
        .enumerate()
        .map(|(n, (shot, dir))| {
            let found = stored(dir, shot).ok_or_else(|| {
                CoreError::Invalid(format!(
                    "Blender wrote no picture for {}.",
                    shot.style.label()
                ))
            })?;
            Ok(answer(settings, shot, found, seconds[n]))
        })
        .collect::<CoreResult<Vec<_>>>()?;
    Ok(RenderPreview {
        key: target.key.clone(),
        name: target.name.clone(),
        version: target.version,
        height_m: target.height_m,
        has_colour: target.has_colour,
        folder: target.folder.clone(),
        dir: planned.dir.clone(),
        shots: made,
    })
}
