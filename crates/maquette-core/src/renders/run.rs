//! One file of a render: a still, a turntable or an eight angle sheet, made
//! in a scratch folder and moved into place, with what went wrong said in a
//! way that names the next step.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::encode::{self, TARGET_BYTES, estimate_loop_bytes, frames_of, margin_of};
use super::plan::RenderTarget;
use super::record::{MovingBox, ProblemKind, RenderFile, RenderFileState, RenderProblem};
use super::spec::{self, BlenderFailed};
use super::{LoopFormat, RenderSettings, RenderStyle};
use crate::error::CoreResult;
use crate::pipeline::runner::PipelineRunner;
use crate::pipeline::tools::Toolbox;
use crate::toolchain::ToolId;

/// The runner lane every render file shares, one at a time.
pub(super) const LANE: &str = "renders";
const STILL_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const TURN_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const SHEET_ANGLES: u32 = 8;
const SHEET_TILE: u32 = 512;
const SHEET_COLUMNS: u32 = 4;

pub(super) struct FileRun<'a> {
    pub runner: &'a PipelineRunner,
    pub tools: &'a Toolbox,
    pub settings: &'a RenderSettings,
    /// Scratch space for this file.
    pub work: &'a Path,
}

fn bytes_of(path: &Path) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "render sizes stay below 2^53")]
    fs::metadata(path).map_or(0.0, |m| m.len() as f64)
}

/// Decides what a failed Blender run means. A line pass that dies at a big
/// size on a dense model is out of memory far more often than not.
fn blender_problem(
    style: RenderStyle,
    settings: &RenderSettings,
    target: &RenderTarget,
    failed: &BlenderFailed,
) -> RenderProblem {
    let said = failed.message.to_lowercase();
    let lines = matches!(style, RenderStyle::Blueprint | RenderStyle::Toon);
    let memory = said.contains("memory") || said.contains("malloc");
    let crashed =
        said.contains("signal") || said.contains("exit code -") || said.contains("killed");
    if memory || (lines && crashed && settings.size >= 4096) {
        let edges = u64::from(target.triangles) * 3 / 2 / 1000;
        let size = settings.size;
        return RenderProblem {
            kind: ProblemKind::OutOfMemory,
            message: format!(
                "The line pass ran out of memory at {size} px ({edges}k edges). Try 2048 px or Edges from crease 45°."
            ),
            webp_bytes: None,
        };
    }
    RenderProblem {
        kind: ProblemKind::Failed,
        message: format!("Blender stopped: {}", failed.message),
        webp_bytes: None,
    }
}

fn gif_over(moving: MovingBox, settings: &RenderSettings, bytes: f64) -> RenderProblem {
    let frames = frames_of(settings.length);
    let webp = estimate_loop_bytes(
        LoopFormat::Webp,
        settings.look,
        (moving.w, moving.h),
        frames,
    );
    let mb = |b: f64| format!("{:.1} MB", b / 1024.0 / 1024.0);
    let fills = u64::from(moving.w) * u64::from(moving.h) * 2
        > u64::from(moving.canvas) * u64::from(moving.canvas);
    let why = if fills {
        "The moving area fills the frame."
    } else {
        "A busy texture changes every pixel it covers."
    };
    let smaller = if settings.width > 360 {
        ", drop to 360 px"
    } else {
        ""
    };
    RenderProblem {
        kind: ProblemKind::OverTarget,
        message: format!(
            "Turntable GIF came out at {}. Over the 2 MB target. {why} Crop to the model{smaller}, or save a WebP loop instead (about {}).",
            mb(bytes),
            mb(webp)
        ),
        webp_bytes: Some(webp),
    }
}

/// Moves a finished file out of the scratch folder.
fn place(from: &Path, to: &Path) -> CoreResult<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| crate::error::CoreError::io(parent, e))?;
    }
    fs::copy(from, to).map_err(|e| crate::error::CoreError::io(to, e))?;
    Ok(())
}

fn still(run: &FileRun<'_>, target: &RenderTarget, file: &mut RenderFile) -> CoreResult<()> {
    let out = run.work.join("out.png");
    let job = spec::still_job(file.style, run.settings, &out, run.settings.size, 0);
    match spec::blender(
        run.runner,
        run.tools,
        LANE,
        run.work,
        Some(target),
        vec![job],
        STILL_TIMEOUT,
    )? {
        Ok(_) => {
            place(&out, Path::new(&file.file))?;
            file.state = RenderFileState::Done;
        }
        Err(failed) => {
            file.state = RenderFileState::Failed;
            file.problem = Some(blender_problem(file.style, run.settings, target, &failed));
        }
    }
    Ok(())
}

fn missing_ffmpeg(format: LoopFormat) -> RenderProblem {
    RenderProblem {
        kind: ProblemKind::MissingTool,
        message: format!(
            "A {} loop needs ffmpeg, and it was not found. Set its path in Settings, Toolchain, or save a WebP loop, which needs only Blender.",
            format.extension().to_uppercase()
        ),
        webp_bytes: None,
    }
}

fn turntable(run: &FileRun<'_>, target: &RenderTarget, file: &mut RenderFile) -> CoreResult<()> {
    let settings = run.settings;
    let format = settings.format;
    if format != LoopFormat::Webp && !run.tools.has(ToolId::Ffmpeg) {
        file.state = RenderFileState::Failed;
        file.problem = Some(missing_ffmpeg(format));
        return Ok(());
    }
    let frames = run.work.join("frames");
    // A video keeps its square; a GIF or a WebP loop is cropped to what moves.
    let margin = match format {
        LoopFormat::Mp4 => 0,
        _ => margin_of(settings.width, settings.tight_crop),
    };
    let webp = if format == LoopFormat::Webp {
        encode::WEBP_QUALITY
    } else {
        0
    };
    let job = spec::turn_job(
        settings,
        &frames,
        settings.width,
        frames_of(settings.length),
        margin,
        webp,
    );
    let report = match spec::blender(
        run.runner,
        run.tools,
        LANE,
        run.work,
        Some(target),
        vec![job],
        TURN_TIMEOUT,
    )? {
        Ok(report) => report,
        Err(failed) => {
            file.state = RenderFileState::Failed;
            file.problem = Some(blender_problem(file.style, settings, target, &failed));
            return Ok(());
        }
    };
    let moving = report.jobs.first().and_then(|j| j.moving);
    file.moving = moving;
    let out = run.work.join(format!("out.{}", format.extension()));
    let written = match format {
        LoopFormat::Webp => {
            let (w, h) = moving.map_or((settings.width, settings.width), |m| (m.w, m.h));
            encode::write_webp(&frames, &out, w, h).map(|()| Ok(()))?
        }
        LoopFormat::Gif | LoopFormat::Mp4 => {
            let name = out.display().to_string();
            let args = if format == LoopFormat::Gif {
                encode::gif_args(&name, settings.look)
            } else {
                encode::mp4_args(&name)
            };
            encode::run_ffmpeg(run.runner, run.tools, LANE, &frames, format, args)?
        }
    };
    match written {
        Ok(()) => {
            place(&out, Path::new(&file.file))?;
            file.state = RenderFileState::Done;
            let bytes = bytes_of(&out);
            if let (LoopFormat::Gif, Some(m), true) = (format, moving, bytes > TARGET_BYTES) {
                file.state = RenderFileState::Warn;
                file.problem = Some(gif_over(m, settings, bytes));
            }
        }
        Err(message) => {
            file.state = RenderFileState::Failed;
            file.problem = Some(RenderProblem {
                kind: ProblemKind::MissingTool,
                message,
                webp_bytes: None,
            });
        }
    }
    Ok(())
}

fn angle_sheet(run: &FileRun<'_>, target: &RenderTarget, file: &mut RenderFile) -> CoreResult<()> {
    let frames = run.work.join("frames");
    let out = run.work.join("out.png");
    let tiles: Vec<(PathBuf, String)> = (0..SHEET_ANGLES)
        .map(|n| {
            (
                frames.join(format!("f{:04}.png", n + 1)),
                format!("{}°", n * 360 / SHEET_ANGLES),
            )
        })
        .collect();
    let jobs = vec![
        spec::turn_job(run.settings, &frames, SHEET_TILE, SHEET_ANGLES, 0, 0),
        spec::sheet_job(run.settings, &out, &tiles, SHEET_COLUMNS, SHEET_TILE),
    ];
    match spec::blender(
        run.runner,
        run.tools,
        LANE,
        run.work,
        Some(target),
        jobs,
        STILL_TIMEOUT,
    )? {
        Ok(_) => {
            place(&out, Path::new(&file.file))?;
            file.state = RenderFileState::Done;
        }
        Err(failed) => {
            file.state = RenderFileState::Failed;
            file.problem = Some(blender_problem(file.style, run.settings, target, &failed));
        }
    }
    Ok(())
}

/// Makes one file. Never errors: a file that fails is a failed file.
pub(super) fn run_file(
    run: &FileRun<'_>,
    target: &RenderTarget,
    queued: &RenderFile,
) -> RenderFile {
    let started = Instant::now();
    let mut file = queued.clone();
    file.problem = None;
    let _ = fs::remove_dir_all(run.work);
    let made = match file.style {
        RenderStyle::Turntable => turntable(run, target, &mut file),
        RenderStyle::ContactSheet => angle_sheet(run, target, &mut file),
        _ => still(run, target, &mut file),
    };
    if let Err(err) = made {
        file.state = RenderFileState::Failed;
        file.problem = Some(RenderProblem {
            kind: ProblemKind::Failed,
            message: err.to_string(),
            webp_bytes: None,
        });
    }
    file.seconds = started.elapsed().as_secs_f64();
    file.bytes = bytes_of(Path::new(&file.file));
    let _ = fs::remove_dir_all(run.work);
    file
}

/// One tile of a batch's sheet: the settings' look, three quarter, at the
/// tile's size, into `out`.
pub(super) fn sheet_tile(run: &FileRun<'_>, target: &RenderTarget, out: &Path) -> CoreResult<bool> {
    let look = match run.settings.look {
        super::RenderLook::Blueprint => RenderStyle::Blueprint,
        super::RenderLook::Toon => RenderStyle::Toon,
        super::RenderLook::Clay => RenderStyle::Clay,
        super::RenderLook::Shaded => RenderStyle::Shaded,
    };
    let job = spec::still_job(look, run.settings, out, SHEET_TILE, 0);
    Ok(spec::blender(
        run.runner,
        run.tools,
        LANE,
        run.work,
        Some(target),
        vec![job],
        STILL_TIMEOUT,
    )?
    .is_ok())
}

/// A batch's one contact sheet of `tiles` into the record's sheet file.
pub(super) fn batch_sheet(
    run: &FileRun<'_>,
    tiles: &[(PathBuf, String)],
    queued: &RenderFile,
) -> RenderFile {
    let started = Instant::now();
    let mut file = queued.clone();
    let out = run.work.join("sheet.png");
    let job = spec::sheet_job(run.settings, &out, tiles, run.settings.columns, SHEET_TILE);
    let made = spec::blender(
        run.runner,
        run.tools,
        LANE,
        run.work,
        None,
        vec![job],
        STILL_TIMEOUT,
    )
    .and_then(|outcome| match outcome {
        Ok(_) => place(&out, Path::new(&file.file)).map(|()| None),
        Err(failed) => Ok(Some(failed.message)),
    });
    let trouble = match made {
        Ok(None) => None,
        Ok(Some(message)) => Some(message),
        Err(err) => Some(err.to_string()),
    };
    file.state = if trouble.is_some() {
        RenderFileState::Failed
    } else {
        RenderFileState::Done
    };
    file.problem = trouble.map(|message| RenderProblem {
        kind: ProblemKind::Failed,
        message,
        webp_bytes: None,
    });
    file.seconds = started.elapsed().as_secs_f64();
    file.bytes = bytes_of(Path::new(&file.file));
    file
}
