//! What the bake script reports, and the result the review reads from it:
//! the planted feet's slide (measured as the retarget review measures it),
//! the rig's stretch, and each reach's gap and elbow or knee.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::job::{Prepared, REPORT_FILE};
use super::{RecipeBakeResult, RecipeReachResult, STAGED_FILE};
use crate::animations::retarget::feet::{self, Track};
use crate::error::{CoreError, CoreResult};

/// A hand or foot may end this far from its target, cm.
pub const GAP_LIMIT_CM: f32 = 2.0;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ReachRun {
    pub layer: String,
    pub bone: String,
    /// Metres; none when its end frame was never reached.
    pub gap: Option<f32>,
    pub frame: Option<u32>,
    pub flip_frame: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct StretchRun {
    pub mm: Option<f32>,
    pub frame: Option<u32>,
    pub base: Option<f32>,
}

/// `result.json`, as the bake script writes it.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct BakeRun {
    pub clip: String,
    pub frames: u32,
    pub facing: [f32; 3],
    pub feet: BTreeMap<String, Track>,
    pub base_feet: BTreeMap<String, Track>,
    pub stretch: Option<StretchRun>,
    pub stretch_note: Option<String>,
    pub reaches: Vec<ReachRun>,
    pub seconds: f32,
}

/// Reads the report the run left in `work`.
///
/// # Errors
///
/// Fails when it is missing or not the report's shape.
pub(crate) fn read_run(work: &Path) -> CoreResult<BakeRun> {
    let file = work.join(REPORT_FILE);
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))
}

fn round(value: f32, places: i32) -> f32 {
    let scale = 10f32.powi(places);
    (value * scale).round() / scale
}

fn reach(run: &ReachRun) -> RecipeReachResult {
    let gap_cm = run.gap.map(|m| round(m * 100.0, 2));
    RecipeReachResult {
        layer: run.layer.clone(),
        bone: run.bone.clone(),
        // A reach whose end was never measured did not reach.
        reached: gap_cm.is_some_and(|cm| cm <= GAP_LIMIT_CM),
        gap_cm,
        frame: run.frame.unwrap_or(0),
        flip_frame: run.flip_frame,
    }
}

/// The result of a run whose report is `run`, staged in `work`, for the
/// bake `prepared` describes and that took `seconds` in all.
pub(crate) fn result_of(
    run: &BakeRun,
    prepared: &Prepared,
    work: &Path,
    seconds: f32,
) -> RecipeBakeResult {
    let slide = feet::slide(&run.feet);
    let base = feet::slide(&run.base_feet);
    let planted = slide.cm > crate::animations::retarget::review::PLANTED_CM;
    let stretch = run.stretch.clone().unwrap_or_default();
    let doc = &prepared.doc;
    // A bake on the rest pose has no base clip: it names none.
    let (base_clip, base_hash) = prepared
        .clip
        .as_ref()
        .map(|c| (c.id.clone(), c.hash.clone()))
        .unwrap_or_default();
    // A base known by name only (from a starter) has no hash to differ.
    let recipe_base_hash = (prepared.clip.is_some()
        && !doc.recipe.base.hash.is_empty()
        && doc.recipe.base.hash != base_hash)
        .then(|| doc.recipe.base.hash.clone());
    RecipeBakeResult {
        recipe: doc.id.clone(),
        recipe_name: doc.recipe.name.clone(),
        recipe_version: doc.version.clone(),
        motion_version: doc.motion_version.clone(),
        base_clip,
        base_hash,
        recipe_base_hash,
        mixamo: doc.mixamo,
        asset: prepared.asset.clone(),
        asset_name: prepared.model.name.clone(),
        from_version: prepared.model.version,
        clip: if run.clip.is_empty() {
            prepared.setting.name.clone()
        } else {
            run.clip.clone()
        },
        replaced: prepared.replace.clone(),
        clip_note: prepared.clip_note.clone(),
        frames: run.frames,
        file: work.join(STAGED_FILE).display().to_string(),
        facing: run.facing,
        slide_cm: round(slide.cm, 2),
        slide_frames: planted.then_some([slide.from, slide.to]),
        slide_foot: (!slide.foot.is_empty()).then(|| slide.foot.clone()),
        base_slide_cm: round(base.cm, 2),
        stretch_mm: stretch.mm.map(|v| round(v, 1)),
        stretch_frame: stretch.mm.and(stretch.frame),
        base_stretch_mm: stretch.base.map(|v| round(v, 1)),
        stretch_note: run.stretch_note.clone(),
        reaches: run.reaches.iter().map(reach).collect(),
        blender_seconds: round(run.seconds, 2),
        seconds: round(seconds, 2),
        warnings: prepared.warnings.clone(),
    }
}

/// The mark a finished bake writes into its staged model.
pub fn mark_for(result: &RecipeBakeResult) -> crate::recipes::baked::RecipeBakeMark {
    crate::recipes::baked::RecipeBakeMark {
        recipe: result.recipe.clone(),
        recipe_version: result.recipe_version.clone(),
        base_clip: result.base_clip.clone(),
        clip: result.clip.clone(),
        mixamo: result.mixamo,
    }
}
