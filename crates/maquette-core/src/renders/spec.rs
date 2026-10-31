//! What `render_styles.py` is told: the JSON spec of a Blender run, and the
//! run itself on the pipeline's runner, always headless.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};

use super::plan::RenderTarget;
use super::record::MovingBox;
use super::{RenderCamera, RenderLook, RenderSettings, RenderStyle};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::{RunRecord, RunState};
use crate::pipeline::runner::{PipelineRunner, RunOptions, RunSetup};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::workspace::{MeshoptLevel, TextureCodec};

pub(super) const SCRIPT: &str = include_str!("render_styles.py");
const SCRIPT_FILE: &str = "render_styles.py";
const SPEC_FILE: &str = "spec.json";
const REPORT_FILE: &str = "report.json";

/// The camera of a still: its turn and how far it looks down, in degrees.
pub(super) fn angles(camera: RenderCamera) -> (u32, u32) {
    match camera {
        RenderCamera::Front => (0, 6),
        RenderCamera::ThreeQuarter => (38, 14),
        RenderCamera::Side => (90, 6),
    }
}

/// The look of a style that is a look of its own; a loop or a sheet takes
/// the settings' look.
pub(super) fn look_name(style: RenderStyle, settings: &RenderSettings) -> &'static str {
    match style {
        RenderStyle::Blueprint => RenderLook::Blueprint.name(),
        RenderStyle::Toon => RenderLook::Toon.name(),
        RenderStyle::Clay => RenderLook::Clay.name(),
        RenderStyle::Shaded => RenderLook::Shaded.name(),
        RenderStyle::Turntable | RenderStyle::ContactSheet => settings.look.name(),
    }
}

/// The fields every job of these settings shares.
fn base(style: &str, settings: &RenderSettings, out: &Path, size: u32) -> Value {
    json!({
        "style": style,
        "out": out,
        "size": size,
        "weight": settings.line_weight,
        "crease": settings.crease,
        "lineColor": settings.line_color,
        "paperColor": settings.paper_color,
        "clayColor": settings.clay_color,
        "hidden": settings.hidden_dashed,
        "marks": settings.marks,
        "titleBlock": settings.title_block,
    })
}

fn merged(mut job: Value, more: Value) -> Value {
    if let (Value::Object(target), Value::Object(fields)) = (&mut job, more) {
        target.extend(fields);
    }
    job
}

/// One still of `style`, turned `turn` degrees from the settings' camera.
pub(super) fn still_job(
    style: RenderStyle,
    settings: &RenderSettings,
    out: &Path,
    size: u32,
    turn: u32,
) -> Value {
    let (azimuth, elevation) = angles(settings.camera);
    merged(
        base(look_name(style, settings), settings, out, size),
        json!({
            "azimuth": azimuth + turn,
            "elevation": elevation,
            "transparent": settings.transparent,
        }),
    )
}

/// `frames` frames of one full turn into the folder `out`.
pub(super) fn turn_job(
    settings: &RenderSettings,
    out: &Path,
    size: u32,
    frames: u32,
    crop_margin: u32,
    webp: u32,
) -> Value {
    merged(
        base(settings.look.name(), settings, out, size),
        json!({
            "azimuth": 0,
            "elevation": settings.tilt,
            "frames": frames,
            "still": settings.still_background,
            "crop": crop_margin,
            "webp": webp,
        }),
    )
}

/// `tiles` laid out on one PNG, each `(file, name)`.
pub(super) fn sheet_job(
    settings: &RenderSettings,
    out: &Path,
    tiles: &[(PathBuf, String)],
    columns: u32,
    tile: u32,
) -> Value {
    let tiles: Vec<Value> = tiles
        .iter()
        .map(|(file, name)| json!({ "file": file, "name": name }))
        .collect();
    json!({
        "out": out,
        "sheet": {
            "tiles": tiles,
            "columns": columns,
            "tile": tile,
            "names": settings.names,
        },
    })
}

/// A job's line of the report.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct JobReport {
    pub seconds: f64,
    pub moving: Option<MovingBox>,
}

/// What Blender found and did.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct Report {
    pub jobs: Vec<JobReport>,
    pub textured: bool,
    pub tinted: bool,
    pub triangles: u64,
    pub edges: u64,
    pub width_m: Option<f64>,
    pub height_m: Option<f64>,
}

/// A failed run's last words, as the runner kept them.
#[derive(Debug)]
pub(super) struct BlenderFailed {
    pub message: String,
}

pub(super) fn defaults() -> CompressDefaults {
    CompressDefaults {
        texture_size: 2048,
        codec: TextureCodec::Etc1s,
        meshopt: MeshoptLevel::Medium,
    }
}

/// Runs one step on `lane`, in `work`, and answers its record.
pub(super) fn run_step(
    runner: &PipelineRunner,
    tools: &Toolbox,
    lane: &str,
    work: &Path,
    step: StepDef,
    timeout: Duration,
) -> CoreResult<RunRecord> {
    runner.run_blocking_with(
        RunSetup {
            project: lane.to_string(),
            project_dir: work.to_path_buf(),
            cache_dir: work.join("runs"),
            def: PipelineDef {
                version: PIPELINE_VERSION,
                root: None,
                steps: vec![step],
                presets: Vec::new(),
            },
            tools: tools.clone(),
            defaults: defaults(),
            vars: Default::default(),
            shipped_names: Default::default(),
            materials: Default::default(),
            from: None,
            force: true,
            textures: Default::default(),
            quality: crate::ship::DEFAULT_QUALITY,
            build_only: false,
        },
        RunOptions {
            timeout: Some(timeout),
            ..RunOptions::default()
        },
    )
}

/// One headless Blender run of `jobs` over `target`, or with no model for
/// sheets alone, in the folder `work`. Answers the report, or `Ok(Err)`
/// when Blender ran and failed.
pub(super) fn blender(
    runner: &PipelineRunner,
    tools: &Toolbox,
    lane: &str,
    work: &Path,
    target: Option<&RenderTarget>,
    jobs: Vec<Value>,
    timeout: Duration,
) -> CoreResult<Result<Report, BlenderFailed>> {
    fs::create_dir_all(work).map_err(|e| CoreError::io(work, e))?;
    for (name, text) in [
        (SCRIPT_FILE, SCRIPT),
        (crate::thumbs::SCRIPT_FILE, crate::thumbs::SCRIPT),
    ] {
        let path = work.join(name);
        crate::fs::write_atomic(&path, text.as_bytes()).map_err(|e| CoreError::io(&path, e))?;
    }
    let report_file = work.join(REPORT_FILE);
    let _ = fs::remove_file(&report_file);
    let spec = json!({
        "model": target.map(|t| &t.file),
        "name": target.map(|t| &t.name),
        "heightM": target.and_then(|t| t.height_m),
        "version": target.map(|t| t.version),
        "report": report_file,
        "jobs": jobs,
    });
    let spec_file = work.join(SPEC_FILE);
    crate::json::write_json(&spec_file, &spec)?;
    let mut args: Vec<String> = ["-b", "--factory-startup", "--python-exit-code", "1"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let is_blend = target.is_some_and(|t| {
        Path::new(&t.file)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("blend"))
    });
    if let (true, Some(t)) = (is_blend, target) {
        args.push(t.file.clone());
    }
    args.extend(
        ["--python", "{script}", "--", "--spec", SPEC_FILE]
            .iter()
            .map(|s| (*s).to_string()),
    );
    let step = StepDef {
        id: "render".into(),
        name: match target {
            Some(t) => format!("Render {}", t.name),
            None => "Lay out a contact sheet".into(),
        },
        kind: StepKind::Blender,
        command: None,
        script: Some(SCRIPT_FILE.into()),
        args,
        inputs: Vec::new(),
        outputs: Vec::new(),
        depends_on: Vec::new(),
        compress: None,
    };
    let record = run_step(runner, tools, lane, work, step, timeout)?;
    if record.state != RunState::Done {
        let message = record
            .steps
            .iter()
            .find_map(|s| s.message.clone())
            .unwrap_or_else(|| "Blender stopped without saying why.".to_string());
        return Ok(Err(BlenderFailed { message }));
    }
    let text = fs::read_to_string(&report_file).map_err(|e| CoreError::io(&report_file, e))?;
    let report = serde_json::from_str(&text).map_err(|e| CoreError::json(&report_file, e))?;
    Ok(Ok(report))
}
