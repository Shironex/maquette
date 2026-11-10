//! What `render_views.py` is told and what it says back: the JSON spec of
//! one run, the names of its pictures and the report it leaves.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::cameras::{StandardView, ViewTarget};
use super::{
    PageLight, RECORD_VERSION, SAMPLES, SCRIPT, SHEET_FILE, SHEET_TILE, ViewSet, ViewShot,
    ViewedModel, ViewsRequest,
};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::cache::FileHasher;
use crate::project::now_rfc3339;

pub(super) const SPEC_FILE: &str = "spec.json";
pub(super) const REPORT_FILE: &str = "report.json";
/// The folder of a run's pictures inside its scratch folder.
pub(super) const OUT_DIR: &str = "out";

/// A picture's file name: the view's id, with the model's in front when two
/// models are rendered side by side.
pub fn file_name(models: usize, model: &str, view: &str) -> String {
    if models > 1 {
        format!("{model}_{view}.png")
    } else {
        format!("{view}.png")
    }
}

fn target_fields(target: &ViewTarget) -> Value {
    match target {
        ViewTarget::Whole => json!({}),
        ViewTarget::Half(half) => json!({ "half": half }),
        ViewTarget::Node(name) => json!({ "node": name }),
        ViewTarget::Box(corners) => json!({ "box": corners }),
    }
}

fn view_json(view: &StandardView, request: &ViewsRequest, out: &Path) -> Value {
    let outs: serde_json::Map<String, Value> = request
        .models
        .iter()
        .map(|m| {
            let name = file_name(request.models.len(), &m.id, &view.id);
            (m.id.clone(), json!(out.join(name)))
        })
        .collect();
    let mut fields = json!({
        "id": view.id,
        "azimuth": view.azimuth,
        "elevation": view.elevation,
        "fill": view.fill,
        "outs": outs,
    });
    if let (Value::Object(all), Value::Object(more)) = (&mut fields, target_fields(&view.target)) {
        all.extend(more);
    }
    fields
}

/// The spec of one run in the scratch folder `work`.
pub(super) fn spec_json(request: &ViewsRequest, work: &Path) -> Value {
    let out = work.join(OUT_DIR);
    json!({
        "size": request.size,
        "samples": SAMPLES,
        "report": work.join(REPORT_FILE),
        "models": request
            .models
            .iter()
            .map(|m| json!({ "id": m.id, "file": m.file }))
            .collect::<Vec<_>>(),
        "views": request
            .views
            .iter()
            .map(|v| view_json(v, request, &out))
            .collect::<Vec<_>>(),
        "sheet": {
            "out": out.join(SHEET_FILE),
            "tile": SHEET_TILE.min(request.size),
            "refs": request.refs,
        },
    })
}

/// What decides the pictures: the scripts, the size, the views and the
/// content of every model and ref. The same key is the same set.
///
/// # Errors
///
/// Fails when a model cannot be read.
pub(super) fn key_of(request: &ViewsRequest) -> CoreResult<String> {
    fs::create_dir_all(&request.work).map_err(|e| CoreError::io(&request.work, e))?;
    let mut files = FileHasher::with_memo(&request.work);
    let mut hasher = Sha256::new();
    for script in [SCRIPT, crate::thumbs::SCRIPT, crate::thumbs::RIG_SCRIPT] {
        hasher.update(script.replace("\r\n", "\n").as_bytes());
        hasher.update(b"\n");
    }
    hasher.update(format!("{} {SAMPLES} {SHEET_TILE}\n", request.size).as_bytes());
    let views =
        serde_json::to_string(&request.views).map_err(|e| CoreError::Invalid(e.to_string()))?;
    hasher.update(views.as_bytes());
    for model in &request.models {
        let hash = files
            .hash(&model.file)
            .map_err(|e| CoreError::io(&model.file, e))?;
        hasher.update(format!("\n{} {hash}", model.id).as_bytes());
    }
    for file in &request.refs {
        // A ref that is gone only leaves the sheet; it does not stop a render.
        let hash = files.hash(file).unwrap_or_default();
        hasher.update(format!("\nref {hash}").as_bytes());
    }
    files.flush();
    Ok(hex::encode(hasher.finalize())[..32].to_string())
}

#[derive(Debug, Deserialize)]
struct ReportModel {
    id: String,
    min: [f32; 3],
    max: [f32; 3],
    triangles: u64,
}

#[derive(Debug, Deserialize)]
struct ReportView {
    id: String,
    azimuth: f32,
    elevation: f32,
    frames: String,
    position: [f32; 3],
    target: [f32; 3],
    fov: f32,
}

/// The script's `report.json`.
#[derive(Debug, Deserialize)]
struct Report {
    size: u32,
    models: Vec<ReportModel>,
    views: Vec<ReportView>,
    light: PageLight,
    seconds: f64,
}

/// The finished set of a run, its pictures named where they end up
/// (`request.dir`).
///
/// # Errors
///
/// Fails when the report cannot be read or leaves a model out.
pub(super) fn read_set(request: &ViewsRequest, key: &str, work: &Path) -> CoreResult<ViewSet> {
    let file = work.join(REPORT_FILE);
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    let report: Report = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    let at = |name: String| request.dir.join(name).display().to_string();
    let mut models = Vec::new();
    for model in &request.models {
        let seen = report
            .models
            .iter()
            .find(|m| m.id == model.id)
            .ok_or_else(|| CoreError::Invalid(format!("Blender rendered no {}.", model.id)))?;
        let shots = report
            .views
            .iter()
            .map(|v| ViewShot {
                id: v.id.clone(),
                file: at(file_name(request.models.len(), &model.id, &v.id)),
                azimuth: v.azimuth,
                elevation: v.elevation,
                frames: v.frames.clone(),
                position: v.position,
                target: v.target,
                fov: v.fov,
            })
            .collect();
        models.push(ViewedModel {
            id: model.id.clone(),
            version: model.version,
            file: model.file.display().to_string(),
            min: seen.min,
            max: seen.max,
            triangles: seen.triangles,
            shots,
        });
    }
    Ok(ViewSet {
        version: RECORD_VERSION,
        key: key.to_string(),
        dir: request.dir.display().to_string(),
        size: report.size,
        models,
        sheet: at(SHEET_FILE.to_string()),
        light: report.light,
        seconds: report.seconds,
        rendered_at: now_rfc3339(),
    })
}

/// Every file a set names, the sheet among them.
pub(super) fn files_of(set: &ViewSet) -> Vec<PathBuf> {
    set.models
        .iter()
        .flat_map(|m| m.shots.iter().map(|s| PathBuf::from(&s.file)))
        .chain(std::iter::once(PathBuf::from(&set.sheet)))
        .collect()
}
