//! One model of a pack: written by `export_pack.py` (with the project's
//! fix-up script beside it), compressed when the pack asks, and rendered
//! from what was written by the thumbnail script, each headless in Blender.
//! What the check found becomes the item's state and note ([`verdict`]).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::record::{PackItem, PackItemState};
use super::{PackCompression, PackSettings};
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::pipeline::runner::{PipelineRunner, RunOptions, RunSetup};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{
    CompressDefaults, CompressSpec, PIPELINE_VERSION, PipelineDef, StepDef, StepKind,
};
use crate::prepare::{Fixup, FixupKind, ModelPrep};

pub(super) const SCRIPT: &str = include_str!("export_pack.py");
const SCRIPT_FILE: &str = "export_pack.py";
const FIXUPS_FILE: &str = "fixups.py";
/// Scratch space inside the pack folder; it never goes in the zip.
pub(super) const WORK_DIR: &str = ".work";
const COMPRESS_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// How far the measured height may drift from the real size.
const HEIGHT_SLACK: f32 = 0.02;

/// What one model's run needs.
pub(super) struct ModelRun<'a> {
    pub runner: &'a PipelineRunner,
    pub tools: &'a Toolbox,
    /// The pack folder.
    pub dir: &'a Path,
    /// Where the check renders are kept.
    pub pictures: &'a Path,
    pub settings: &'a PackSettings,
    pub defaults: CompressDefaults,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Numbers {
    pub triangles: u32,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct Check {
    pub file: String,
    pub triangles: u32,
    pub height_m: Option<f32>,
    pub floor_m: Option<f32>,
    pub centre: Option<[f32; 2]>,
    pub unloaded: Vec<String>,
    pub missing: Vec<String>,
    pub clips: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Report {
    pub before: Numbers,
    pub after: Numbers,
    pub thin: bool,
    pub clips_removed: u32,
    pub files: Vec<String>,
    pub checks: Vec<Check>,
    pub pivot_slack: f32,
}

fn put(fixups: &mut Vec<Fixup>, fixup: Fixup) {
    fixups.retain(|f| f.kind() != fixup.kind());
    fixups.push(fixup);
}

/// The model's own real size and fix-ups with the pack's added.
pub(super) fn plan_for(item: &PackItem, settings: &PackSettings) -> ModelPrep {
    let mut prep = item.model.prep.clone().unwrap_or_default();
    if !settings.real_size {
        prep.size = None;
    }
    let own_decimate = prep.fixup(FixupKind::Decimate).and_then(|f| match f {
        Fixup::Decimate { triangles } => Some(*triangles),
        _ => None,
    });
    if let Some(budget) = item.budget.filter(|_| !item.model.rigged) {
        let target = own_decimate.map_or(budget, |own| own.min(budget));
        put(&mut prep.fixups, Fixup::Decimate { triangles: target });
    }
    if settings.texture > 0 {
        let own = prep.fixup(FixupKind::ShrinkTextures).and_then(|f| match f {
            Fixup::ShrinkTextures { max_size } => Some(*max_size),
            _ => None,
        });
        let max_size = own.map_or(settings.texture, |o| o.min(settings.texture));
        put(&mut prep.fixups, Fixup::ShrinkTextures { max_size });
    }
    if settings.pivot_bottom {
        put(&mut prep.fixups, Fixup::OriginBottom);
    }
    prep
}

fn bytes_of(path: &Path) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "model sizes stay below 2^53")]
    fs::metadata(path).map_or(0.0, |m| m.len() as f64)
}

fn compress(run: &ModelRun<'_>, key: &str, work: &Path) -> CoreResult<()> {
    let from = format!("{WORK_DIR}/{key}/glb");
    let step = StepDef {
        id: "compress".into(),
        name: format!("Compress {key}"),
        kind: StepKind::GltfTransform,
        command: None,
        script: None,
        args: Vec::new(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        depends_on: Vec::new(),
        compress: Some(CompressSpec {
            from,
            to: key.to_string(),
        }),
    };
    let record = run.runner.run_blocking_with(
        RunSetup {
            project: super::job::LANE.to_string(),
            project_dir: run.dir.to_path_buf(),
            cache_dir: work.join("runs"),
            def: PipelineDef {
                version: PIPELINE_VERSION,
                root: None,
                steps: vec![step],
                presets: Vec::new(),
            },
            tools: run.tools.clone(),
            defaults: run.defaults,
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
            timeout: Some(COMPRESS_TIMEOUT),
            ..RunOptions::default()
        },
    )?;
    if record.state != crate::pipeline::history::RunState::Done {
        let why = record
            .steps
            .iter()
            .find_map(|s| s.message.clone())
            .unwrap_or_else(|| "gltf-transform stopped without saying why.".to_string());
        return Err(CoreError::Invalid(format!("Compress failed: {why}")));
    }
    Ok(())
}

fn render(run: &ModelRun<'_>, work: &Path, file: &Path, out: &Path) -> Option<String> {
    let extra = [
        "--model".to_string(),
        file.display().to_string(),
        "--out".to_string(),
        out.display().to_string(),
        "--size".to_string(),
        crate::thumbs::THUMB_SIZE.to_string(),
    ];
    let script = (crate::thumbs::SCRIPT_FILE, crate::thumbs::SCRIPT);
    match run
        .runner
        .run_step(run.tools, work, "Check render", script, &extra)
    {
        Ok(()) if out.is_file() => Some(out.display().to_string()),
        Ok(()) => None,
        Err(err) => {
            tracing::info!(error = %err, "a pack check render failed");
            None
        }
    }
}

/// The item after its run: written, checked and rendered, or failed with
/// why. Never errors: a model that fails is a failed item.
pub(super) fn run_one(run: &ModelRun<'_>, item: &PackItem) -> PackItem {
    let started = Instant::now();
    let mut next = item.clone();
    match write_and_check(run, item) {
        Ok(done) => next = done,
        Err(err) => {
            next.state = PackItemState::Fail;
            next.note = Some(err.to_string());
            next.files.clear();
        }
    }
    next.took_ms = started.elapsed().as_secs_f64() * 1000.0;
    next
}

fn write_and_check(run: &ModelRun<'_>, item: &PackItem) -> CoreResult<PackItem> {
    let key = &item.model.key;
    let work = run.dir.join(WORK_DIR).join(key);
    if work.exists() {
        fs::remove_dir_all(&work).map_err(|e| CoreError::io(&work, e))?;
    }
    fs::create_dir_all(&work).map_err(|e| CoreError::io(&work, e))?;
    let out = run.dir.join(key);
    if out.exists() {
        fs::remove_dir_all(&out).map_err(|e| CoreError::io(&out, e))?;
    }
    let fixups = work.join(FIXUPS_FILE);
    crate::fs::write_atomic(&fixups, crate::prepare::SCRIPT.as_bytes())
        .map_err(|e| CoreError::io(&fixups, e))?;
    let settings = run.settings;
    let glb = settings.format.glb()
        || (settings.format.obj() && settings.rigged_glb && item.model.rigged);
    let web = glb && settings.compression == PackCompression::Web;
    let glb_out = web.then(|| work.join("glb").join(format!("{key}.glb")));
    let job = serde_json::json!({
        "input": item.model.file,
        "key": key,
        "out": out,
        "plan": plan_for(item, settings),
        "obj": settings.format.obj(),
        "glb": glb,
        "glbOut": glb_out,
        "rename": settings.one_material,
        "stripClips": item.model.mixamo,
        "texture": settings.texture,
    });
    let job_file = work.join("job.json");
    crate::json::write_json(&job_file, &job)?;
    let report_file = work.join("report.json");
    let extra = [
        "--job".to_string(),
        job_file.display().to_string(),
        "--report".to_string(),
        report_file.display().to_string(),
    ];
    let name = format!("Pack {}", item.model.name);
    run.runner
        .run_step(run.tools, &work, &name, (SCRIPT_FILE, SCRIPT), &extra)?;
    let text = fs::read_to_string(&report_file).map_err(|e| CoreError::io(&report_file, e))?;
    let report: Report =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&report_file, e))?;
    if web {
        compress(run, key, &work)?;
    }
    let mut next = verdict(item, &report, settings);
    next.files = written(run.dir, key);
    next.bytes = next.files.iter().map(|f| bytes_of(&run.dir.join(f))).sum();
    if settings.refs {
        copy_refs(&item.model.refs, &out.join("refs"));
    }
    let main = report
        .files
        .iter()
        .find(|f| f.ends_with(".obj") || f.ends_with(".glb"))
        .map(|f| match &glb_out {
            Some(path) if f.ends_with(".glb") => path.clone(),
            _ => out.join(f),
        });
    fs::create_dir_all(run.pictures).map_err(|e| CoreError::io(run.pictures, e))?;
    let picture = run.pictures.join(format!("{key}.png"));
    let _ = fs::remove_file(&picture);
    next.picture = main.and_then(|file| render(run, &work, &file, &picture));
    Ok(next)
}

/// The files under the model's folder, relative to the pack folder.
fn written(dir: &Path, key: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir.join(key)];
    while let Some(folder) = stack.pop() {
        let Ok(read) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in read.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(rel) = path.strip_prefix(dir) {
                found.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found.sort();
    found
}

fn copy_refs(refs: &[String], to: &Path) {
    if refs.is_empty() || fs::create_dir_all(to).is_err() {
        return;
    }
    for file in refs {
        let from = Path::new(file);
        if let Some(name) = from.file_name()
            && let Err(err) = fs::copy(from, to.join(name))
        {
            tracing::warn!(error = %err, file = %from.display(), "could not copy a ref into a pack");
        }
    }
}

/// The item's state and note from what the check found.
pub(super) fn verdict(item: &PackItem, report: &Report, settings: &PackSettings) -> PackItem {
    let mut next = item.clone();
    next.triangles = Some(report.after.triangles);
    next.clips_left_out = report.clips_removed;
    next.suggest = None;
    let main = report.checks.first();
    next.height_m = main.and_then(|c| c.height_m);
    let (state, note) = judge(item, report, settings);
    next.state = state;
    next.note = Some(note);
    if state == PackItemState::Warn && report.thin {
        let budget = item.budget.unwrap_or(report.after.triangles);
        next.suggest = Some(report.before.triangles.min(budget.saturating_mul(2)));
    }
    next
}

fn judge(item: &PackItem, report: &Report, settings: &PackSettings) -> (PackItemState, String) {
    let Some(main) = report.checks.first() else {
        return (PackItemState::Fail, "Nothing was written.".to_string());
    };
    if let Some(check) = report
        .checks
        .iter()
        .find(|c| !c.missing.is_empty() || !c.unloaded.is_empty())
    {
        let gone = check
            .missing
            .first()
            .or(check.unloaded.first())
            .cloned()
            .unwrap_or_default();
        return (
            PackItemState::Fail,
            format!(
                "Texture missing: {} names {gone}, which did not load. Export it again.",
                check.file
            ),
        );
    }
    if item.model.mixamo && report.checks.iter().any(|c| c.clips > 0) {
        return (
            PackItemState::Fail,
            "Mixamo clips are still in the file, so it is held back.".to_string(),
        );
    }
    let count = crate::assets::short_count;
    if report.thin {
        let budget = item.budget.unwrap_or(report.after.triangles);
        let keeps = report.before.triangles.min(budget.saturating_mul(2));
        return (
            PackItemState::Warn,
            format!(
                "Thin parts lost at {} tris; {} keeps more of them.",
                count(budget),
                count(keeps)
            ),
        );
    }
    if let Some(budget) = item
        .budget
        .filter(|b| item.model.rigged && report.after.triangles > *b)
    {
        return (
            PackItemState::Warn,
            format!(
                "Rigged, so not decimated: {} tris, over the {} budget.",
                count(report.after.triangles),
                count(budget)
            ),
        );
    }
    let slack = report.pivot_slack.max(0.001);
    let off_floor = main.floor_m.is_some_and(|f| f.abs() > slack);
    let off_centre = main
        .centre
        .is_some_and(|[x, z]| x.abs() > slack || z.abs() > slack);
    if settings.pivot_bottom && (off_floor || off_centre) {
        return (
            PackItemState::Warn,
            "The pivot is not at the bottom centre after import.".to_string(),
        );
    }
    let wanted = item.model.height_m.filter(|_| settings.real_size);
    if let (Some(wanted), Some(got)) = (wanted, main.height_m)
        && wanted > 0.0
        && ((got - wanted) / wanted).abs() > HEIGHT_SLACK
    {
        return (
            PackItemState::Warn,
            format!("Measures {got:.2} m high; its real size says {wanted:.2} m."),
        );
    }
    let mut note = if report.after.triangles >= report.before.triangles && item.budget.is_some() {
        "Under budget, left as is".to_string()
    } else if settings.pivot_bottom {
        "Re-imported, Y up, on the floor".to_string()
    } else {
        "Re-imported, Y up".to_string()
    };
    if report.clips_removed > 0 {
        let n = report.clips_removed;
        note = format!(
            "{note}; {n} Mixamo clip{} left out",
            if n == 1 { "" } else { "s" }
        );
    }
    (PackItemState::Pass, note)
}
