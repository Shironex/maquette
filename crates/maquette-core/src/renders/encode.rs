//! The rules of a loop. They are fixed, not settings, because each one is
//! why a turntable stays small and plays right:
//!
//! - one palette for the whole loop, built from all its frames, so colours
//!   do not flicker;
//! - 20 fps, so each frame is exactly 5/100 s: a GIF counts in hundredths
//!   and other rates round and drift;
//! - Bayer dithering at scale 5, since an ordered pattern compresses better
//!   than error diffusion;
//! - a crop box around what moves, since pixels outside it cost nothing.
//!
//! GIF and MP4 go through ffmpeg. A WebP loop is Blender's frames joined
//! here: its container is a small header per frame, and many ffmpeg builds
//! have no WebP encoder.

use std::fs;
use std::path::Path;
use std::time::Duration;

use super::spec::run_step;
use super::{LoopFormat, RenderLook};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::RunState;
use crate::pipeline::runner::PipelineRunner;
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{StepDef, StepKind};

/// Frames per second, for every format.
pub const FPS: u32 = 20;
/// A frame's time in a WebP loop, in milliseconds.
const FRAME_MS: u32 = 1000 / FPS;
/// What a GIF should weigh at most.
pub const TARGET_BYTES: f64 = 2.0 * 1024.0 * 1024.0;
/// WebP frames are written at this quality.
pub const WEBP_QUALITY: u32 = 72;
const ENCODE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Frames of a loop `length` seconds long.
pub fn frames_of(length: u32) -> u32 {
    length * FPS
}

/// The crop margin in pixels around what moves.
pub fn margin_of(width: u32, tight: bool) -> u32 {
    if tight { 2 } else { (width / 24).max(4) }
}

/// A look's colours: flat ones need far fewer than a studio render, and a
/// smaller palette makes a smaller GIF without showing.
fn palette_of(look: RenderLook) -> u32 {
    match look {
        RenderLook::Blueprint => 8,
        RenderLook::Clay => 32,
        RenderLook::Toon => 96,
        RenderLook::Shaded => 128,
    }
}

/// The ffmpeg arguments of a GIF from `f0001.png` and on: one palette over
/// every frame, ordered dither, and only what changed per frame.
pub fn gif_args(out: &str, look: RenderLook) -> Vec<String> {
    let colours = palette_of(look);
    let filter = format!(
        "split[a][b];[a]palettegen=stats_mode=full:max_colors={colours}[p];\
         [b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle"
    );
    let rate = FPS.to_string();
    [
        "-y",
        "-hide_banner",
        "-framerate",
        &rate,
        "-i",
        "f%04d.png",
        "-filter_complex",
        &filter,
        "-loop",
        "0",
        out,
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

/// The ffmpeg arguments of an MP4 that plays anywhere and loops by player.
pub fn mp4_args(out: &str) -> Vec<String> {
    let rate = FPS.to_string();
    [
        "-y",
        "-hide_banner",
        "-framerate",
        &rate,
        "-i",
        "f%04d.png",
        "-c:v",
        "libx264",
        "-crf",
        "20",
        "-pix_fmt",
        "yuv420p",
        "-movflags",
        "+faststart",
        out,
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

/// About what a loop weighs: bytes per changing pixel per frame, measured
/// on a lantern and a windmill at 480 px for each look at the rules above.
pub fn estimate_loop_bytes(
    format: LoopFormat,
    look: RenderLook,
    moving: (u32, u32),
    frames: u32,
) -> f64 {
    let per_pixel = match (format, look) {
        (LoopFormat::Gif, RenderLook::Blueprint) => 0.065,
        (LoopFormat::Gif, RenderLook::Clay) => 0.19,
        (LoopFormat::Gif, RenderLook::Toon) => 0.29,
        (LoopFormat::Gif, RenderLook::Shaded) => 0.28,
        (LoopFormat::Webp, RenderLook::Blueprint) => 0.04,
        (LoopFormat::Webp, RenderLook::Clay) => 0.033,
        (LoopFormat::Webp, RenderLook::Toon) => 0.046,
        (LoopFormat::Webp, RenderLook::Shaded) => 0.042,
        (LoopFormat::Mp4, _) => 0.012,
    };
    f64::from(moving.0) * f64::from(moving.1) * f64::from(frames) * per_pixel
}

fn le24(value: u32) -> [u8; 3] {
    let b = value.to_le_bytes();
    [b[0], b[1], b[2]]
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(kind);
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}

/// The chunks of a still WebP after its `RIFF....WEBP` header.
fn frame_chunks(still: &[u8]) -> Option<&[u8]> {
    if still.len() < 12 || &still[0..4] != b"RIFF" || &still[8..12] != b"WEBP" {
        return None;
    }
    Some(&still[12..])
}

/// One animated WebP of `stills`, each a still WebP of `width` by `height`.
///
/// # Errors
///
/// Fails when a still is not a WebP, or the frame size does not fit.
pub fn join_webp(stills: &[Vec<u8>], width: u32, height: u32) -> CoreResult<Vec<u8>> {
    if stills.is_empty() || width == 0 || height == 0 {
        return Err(CoreError::Invalid(
            "A loop needs at least one frame.".to_string(),
        ));
    }
    let mut body = Vec::new();
    let mut vp8x = vec![0b0000_0010, 0, 0, 0];
    vp8x.extend_from_slice(&le24(width - 1));
    vp8x.extend_from_slice(&le24(height - 1));
    chunk(&mut body, b"VP8X", &vp8x);
    // A white background, looping for ever.
    chunk(&mut body, b"ANIM", &[255, 255, 255, 255, 0, 0]);
    for still in stills {
        let Some(chunks) = frame_chunks(still) else {
            return Err(CoreError::Invalid(
                "A frame of the loop is not a WebP.".to_string(),
            ));
        };
        let mut frame = Vec::with_capacity(chunks.len() + 16);
        frame.extend_from_slice(&le24(0));
        frame.extend_from_slice(&le24(0));
        frame.extend_from_slice(&le24(width - 1));
        frame.extend_from_slice(&le24(height - 1));
        frame.extend_from_slice(&le24(FRAME_MS));
        // Replaces the canvas rather than blending, and keeps it after.
        frame.push(0b0000_0010);
        frame.extend_from_slice(chunks);
        chunk(&mut body, b"ANMF", &frame);
    }
    let mut out = Vec::with_capacity(body.len() + 12);
    out.extend_from_slice(b"RIFF");
    let size = u32::try_from(body.len() + 4)
        .map_err(|_| CoreError::Invalid("The loop came out too big for a WebP.".to_string()))?;
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(&body);
    Ok(out)
}

/// Joins the `f0001.webp` frames in `folder` into the file `out`.
pub(super) fn write_webp(folder: &Path, out: &Path, width: u32, height: u32) -> CoreResult<()> {
    let mut names: Vec<_> = fs::read_dir(folder)
        .map_err(|e| CoreError::io(folder, e))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "webp"))
        .collect();
    names.sort();
    let stills = names
        .iter()
        .map(|p| fs::read(p).map_err(|e| CoreError::io(p, e)))
        .collect::<CoreResult<Vec<_>>>()?;
    let joined = join_webp(&stills, width, height)?;
    crate::fs::write_atomic(out, &joined).map_err(|e| CoreError::io(out, e))
}

/// What ffmpeg's failure means, for the person reading it.
fn ffmpeg_failure(format: LoopFormat, message: &str) -> String {
    let encoder = message.contains("Unknown encoder") || message.contains("not found");
    match (format, encoder) {
        (LoopFormat::Mp4, true) => {
            "This ffmpeg has no H.264 encoder, so it cannot write MP4. Save a GIF or WebP loop instead, or install a full ffmpeg.".to_string()
        }
        _ => format!("ffmpeg could not write the loop. {message}"),
    }
}

/// Runs ffmpeg in `folder` over its frames.
pub(super) fn run_ffmpeg(
    runner: &PipelineRunner,
    tools: &Toolbox,
    lane: &str,
    folder: &Path,
    format: LoopFormat,
    args: Vec<String>,
) -> CoreResult<Result<(), String>> {
    let step = StepDef {
        id: "encode".into(),
        name: format!("Encode {}", format.extension()),
        kind: StepKind::Ffmpeg,
        command: None,
        script: None,
        args,
        inputs: Vec::new(),
        outputs: Vec::new(),
        depends_on: Vec::new(),
        compress: None,
    };
    let record = run_step(runner, tools, lane, folder, step, ENCODE_TIMEOUT)?;
    if record.state == RunState::Done {
        return Ok(Ok(()));
    }
    let message = record
        .steps
        .iter()
        .find_map(|s| s.message.clone())
        .unwrap_or_else(|| "It stopped without saying why.".to_string());
    Ok(Err(ffmpeg_failure(format, &message)))
}
