//! The bake itself: a scratch folder with the model, the clips and the
//! job, one headless Blender run, the review, and the draft kept by its
//! token until [`bake`] adds it as the model's next version.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{
    Baked, BakedClip, Pending, RetargetDraft, RetargetReview, UseClips, bone_names, bones,
    library_clips, model, review, settings, taken, uses,
};
use crate::animations::{blender, clip_dir};
use crate::assets;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::pipeline::runner::PipelineRunner;
use crate::pipeline::tools::Toolbox;
use crate::project;
use crate::workspace::WorkspaceConfig;

/// Bakes library clips onto a model, headless.
pub const RETARGET_SCRIPT: &str = include_str!("../retarget_clips.py");
const RETARGET_FILE: &str = "retarget_clips.py";
const JOB_FILE: &str = "retarget.json";
const REQUEST_FILE: &str = "request.json";
const REPORT_FILE: &str = "report.json";
const REVIEW_FILE: &str = "review.json";
const DRAFT_FILE: &str = "draft.glb";
const TOKEN_PREFIX: &str = "retarget-";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScriptClip<'a> {
    id: &'a str,
    file: String,
    name: &'a str,
    start: u32,
    end: u32,
    loops: bool,
    root_motion: bool,
    speed: f32,
    pairs: Vec<(String, String)>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScriptJob<'a> {
    model: &'static str,
    out: &'static str,
    report: &'static str,
    clips: Vec<ScriptClip<'a>>,
    keep_rest: Vec<String>,
    /// Model clips the bake replaces.
    replace: Vec<String>,
    feet: Vec<String>,
}

fn read<T: serde::de::DeserializeOwned>(file: &Path) -> CoreResult<T> {
    let text = fs::read_to_string(file).map_err(|e| CoreError::io(file, e))?;
    serde_json::from_str(&text).map_err(|e| CoreError::json(file, e))
}

fn copy(from: &Path, to: &Path) -> CoreResult<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    fs::copy(from, to).map_err(|e| CoreError::io(from, e))?;
    Ok(())
}

/// Bakes the clips onto a copy of the model's newest version, headless,
/// and reviews them. The project is not changed, apart from the bone map
/// when `save_map` asks for it.
///
/// # Errors
///
/// As [`plan`], for no clips, without Blender, or when the run fails.
pub fn draft(
    root: &Path,
    config: &WorkspaceConfig,
    (runner, tools): (&PipelineRunner, &Toolbox),
    request: &UseClips,
) -> CoreResult<RetargetDraft> {
    if request.settings.is_empty() {
        return Err(CoreError::Invalid("Choose at least one clip.".to_string()));
    }
    let model = model(root, config, &request.project, &request.asset)?;
    let ids: Vec<String> = request.settings.iter().map(|s| s.clip.clone()).collect();
    let clips = library_clips(root, &ids)?;
    let entry = uses::of_asset(&model.dir, &request.asset)?;
    let settings = settings::normalise(&request.settings, &clips, &taken(&model, &entry, &ids))?;
    let source = clips.first().map(bone_names).unwrap_or_default();
    let map = bones::suggest(&source, &model.joints, &entry.map, &request.map);

    let work = blender::scratch(root, config, "retarget")?;
    copy(&model.path, &work.join("in").join("model.glb"))?;
    let mut script_clips = Vec::new();
    for (setting, clip) in settings.iter().zip(&clips) {
        let rel = format!("in/clips/{}.glb", clip.id);
        copy(
            &clip_dir(root, &clip.id)?.join(&clip.file),
            &work
                .join("in")
                .join("clips")
                .join(format!("{}.glb", clip.id)),
        )?;
        script_clips.push(ScriptClip {
            id: &clip.id,
            file: rel,
            name: &setting.name,
            start: setting.start,
            end: setting.end,
            loops: setting.loops,
            root_motion: setting.root_motion,
            speed: setting.speed,
            pairs: bones::pairs_for(&bone_names(clip), &map),
        });
    }
    let feet = ["LeftFoot", "RightFoot"]
        .iter()
        .filter_map(|b| map.iter().find(|p| p.bone == *b)?.target.clone())
        .collect();
    let replace = entry
        .baked
        .iter()
        .filter(|b| ids.contains(&b.setting.clip))
        .map(|b| b.setting.name.clone())
        .collect();
    write_json(
        &work.join(JOB_FILE),
        &ScriptJob {
            model: "in/model.glb",
            out: DRAFT_FILE,
            report: REPORT_FILE,
            clips: script_clips,
            keep_rest: bones::keep_rest(&map),
            replace,
            feet,
        },
    )?;
    let extra = vec!["--job".to_string(), JOB_FILE.to_string()];
    blender::run_script(
        runner,
        tools,
        &work,
        (RETARGET_FILE, RETARGET_SCRIPT),
        &extra,
    )?;
    let report: review::RunReport = read(&work.join(REPORT_FILE))?;
    let at_rest = map
        .iter()
        .filter(|p| p.target.is_none())
        .map(|p| p.bone.clone())
        .collect();
    let review = review::review(
        &report,
        |id| settings.iter().any(|s| s.clip == id && s.root_motion),
        at_rest,
    );
    if request.save_map {
        let choices = bones::choices(&map);
        uses::update(&model.dir, &request.asset, |e| e.map = choices)?;
    }
    let token = work
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    write_json(
        &work.join(REQUEST_FILE),
        &Pending {
            project: request.project.clone(),
            asset: request.asset.clone(),
            name: model.name.clone(),
            from_version: model.version,
            settings: settings.clone(),
            library: clips
                .iter()
                .map(|c| (c.id.clone(), c.revision, c.name.clone()))
                .collect(),
        },
    )?;
    write_json(&work.join(REVIEW_FILE), &review)?;
    Ok(RetargetDraft {
        token,
        project: request.project.clone(),
        asset: request.asset.clone(),
        name: model.name,
        from_version: model.version,
        file: work.join(DRAFT_FILE).display().to_string(),
        review,
        settings,
    })
}

fn work_of(root: &Path, config: &WorkspaceConfig, token: &str) -> CoreResult<PathBuf> {
    if !token.starts_with(TOKEN_PREFIX) || !crate::workspace::is_single_segment(token) {
        return Err(CoreError::Invalid(format!("{token:?} is not a retarget.")));
    }
    let work = root
        .join(&config.layout.cache)
        .join(crate::animations::ANIMATIONS_DIR)
        .join(token);
    if !work.join(DRAFT_FILE).is_file() {
        return Err(CoreError::NotFound(
            "that retarget; run it again".to_string(),
        ));
    }
    Ok(work)
}

/// Adds a reviewed draft as the model's next version, through the checks
/// a new version gets, and records the clips it baked.
///
/// # Errors
///
/// Fails for an unknown or finished draft, when every clip failed, or when
/// the model got a newer version since the draft was made.
pub fn bake(root: &Path, config: &WorkspaceConfig, token: &str) -> CoreResult<Baked> {
    let work = work_of(root, config, token)?;
    let pending: Pending = read(&work.join(REQUEST_FILE))?;
    let review: RetargetReview = read(&work.join(REVIEW_FILE))?;
    if review.baked().next().is_none() {
        return Err(CoreError::Invalid(
            "Every clip failed in review, so there is nothing to bake.".to_string(),
        ));
    }
    let dir = project::dir(root, &config.layout, &pending.project)?;
    let manifest = project::load(root, &config.layout, &pending.project)?;
    let asset = assets::find(&manifest, &pending.asset)?.clone();
    if asset.latest().map(|v| v.version) != Some(pending.from_version) {
        return Err(CoreError::Invalid(format!(
            "{} has a newer version than v{}. Retarget the clips again onto it.",
            asset.name, pending.from_version
        )));
    }
    let draft = work.join(DRAFT_FILE);
    let (warnings, stats) = crate::built::checked(&draft, &asset, config);
    let mut made = None;
    let manifest = project::modify(root, &config.layout, &pending.project, |m| {
        made = Some(crate::built::add_model_version(
            m, &dir, &asset.id, &draft, stats,
        )?);
        Ok(())
    })?;
    let version = made.ok_or_else(|| CoreError::Invalid("No version was made.".to_string()))?;
    let baked: Vec<BakedClip> = review
        .baked()
        .filter_map(|c| {
            let setting = pending.settings.iter().find(|s| s.clip == c.clip)?.clone();
            let (_, revision, library_name) = pending
                .library
                .iter()
                .find(|(id, _, _)| *id == c.clip)?
                .clone();
            Some(BakedClip {
                setting,
                revision,
                library_name,
                version: version.version,
            })
        })
        .collect();
    uses::update(&dir, &pending.asset, |e| {
        uses::record_bake(e, baked, review.clone(), version.version);
    })?;
    if let Err(err) = fs::remove_dir_all(&work) {
        tracing::warn!(error = %err, "could not clear the retarget's scratch folder");
    }
    Ok(Baked {
        project: manifest,
        version,
        review,
        warnings,
    })
}
