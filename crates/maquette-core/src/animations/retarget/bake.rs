//! The bake itself: a scratch folder with the model, the clips and the
//! job, one headless Blender run, the review, and the draft kept by its
//! token until [`bake`] adds it as the model's next version. A clip with
//! Pin the feet on takes a second run that holds its feet where the first
//! found them planted ([`super::feet`]); the review compares the two. A pin
//! that slides more than the clip did without it comes off again, in a third
//! run, so the draft holds whichever slid less.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{
    Baked, BakedClip, Pending, RetargetDraft, RetargetReview, UseClips, bone_names, bones, feet,
    library_clips, model, rest, review, settings, taken, uses,
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
    /// The clip's own frame rate, which its start and end count in.
    fps: f32,
    loops: bool,
    root_motion: bool,
    speed: f32,
    pairs: Vec<(String, String)>,
    /// Per foot, the frames to hold it still over; empty bakes it as is.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pins: BTreeMap<String, Vec<[u32; 2]>>,
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
    /// Bones only on the model, with the driven bone each moves with.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    follow: Vec<(String, String)>,
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
    let mut settings =
        settings::normalise(&request.settings, &clips, &taken(&model, &entry, &ids))?;
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
            fps: clip.fps,
            loops: setting.loops,
            root_motion: setting.root_motion,
            speed: setting.speed,
            pairs: bones::pairs_for(&bone_names(clip), &map),
            pins: BTreeMap::new(),
        });
    }
    let foot_bones = ["LeftFoot", "RightFoot"]
        .iter()
        .filter_map(|b| map.iter().find(|p| p.bone == *b)?.target.clone())
        .collect();
    let replace = entry
        .baked
        .iter()
        .filter(|b| ids.contains(&b.setting.clip))
        .map(|b| b.setting.name.clone())
        .collect();
    let follow_choices = rest::merged(&entry.follow, &request.follow);
    let follow = rest::follow_pairs(&follow_choices, &rest::model_bones(&model.path), &map);
    let mut job = ScriptJob {
        model: "in/model.glb",
        out: DRAFT_FILE,
        report: REPORT_FILE,
        clips: script_clips,
        keep_rest: bones::keep_rest(&map),
        replace,
        feet: foot_bones,
        follow,
    };
    let mut report = run(runner, tools, &work, &job)?;
    let pinned: Vec<String> = settings
        .iter()
        .filter(|s| s.pin_feet)
        .map(|s| s.clip.clone())
        .collect();
    if !pinned.is_empty() {
        let (with_pins, left_off) = pin_feet(runner, tools, &work, &mut job, report, &pinned)?;
        report = with_pins;
        for setting in settings.iter_mut().filter(|s| left_off.contains(&s.clip)) {
            setting.pin_feet = false;
        }
    }
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
        uses::update(&model.dir, &request.asset, |e| {
            e.map = choices;
            e.follow = follow_choices;
        })?;
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

/// Bakes again with the feet of the `pinned` clips held where `before`
/// found them planted, and once more without the pins that slid more than
/// the clip did without them. The report of the last run, which wrote the
/// draft, and the clips whose pin came off.
fn pin_feet(
    runner: &PipelineRunner,
    tools: &Toolbox,
    work: &Path,
    job: &mut ScriptJob<'_>,
    before: review::RunReport,
    pinned: &[String],
) -> CoreResult<(review::RunReport, Vec<String>)> {
    for clip in job
        .clips
        .iter_mut()
        .filter(|c| pinned.iter().any(|p| p == c.id))
    {
        if let Some(first) = before.clips.iter().find(|r| r.id == clip.id) {
            clip.pins = feet::pins(&first.feet);
        }
    }
    let holding =
        |job: &ScriptJob<'_>, id: &str| job.clips.iter().any(|c| c.id == id && !c.pins.is_empty());
    let mut report = run(runner, tools, work, job)?;
    report.compare_with(&before, |id| holding(job, id));
    let worse = report.pins_made_worse();
    if worse.is_empty() {
        return Ok((report, Vec::new()));
    }
    for clip in job
        .clips
        .iter_mut()
        .filter(|c| worse.iter().any(|(id, _)| id == c.id))
    {
        clip.pins.clear();
    }
    report = run(runner, tools, work, job)?;
    report.compare_with(&before, |id| holding(job, id));
    for clip in &mut report.clips {
        clip.pin_worse_cm = worse
            .iter()
            .find(|(id, _)| *id == clip.id)
            .map(|(_, cm)| *cm);
    }
    Ok((report, worse.into_iter().map(|(id, _)| id).collect()))
}

/// One headless run of the job, and its report with the slides measured.
fn run(
    runner: &PipelineRunner,
    tools: &Toolbox,
    work: &Path,
    job: &ScriptJob<'_>,
) -> CoreResult<review::RunReport> {
    write_json(&work.join(JOB_FILE), job)?;
    let extra = vec!["--job".to_string(), JOB_FILE.to_string()];
    blender::run_script(
        runner,
        tools,
        work,
        (RETARGET_FILE, RETARGET_SCRIPT),
        &extra,
    )?;
    let mut report: review::RunReport = read(&work.join(REPORT_FILE))?;
    for clip in &mut report.clips {
        clip.measure();
    }
    Ok(report)
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

/// The baked copy a draft waits with, for Open in Blender.
///
/// # Errors
///
/// Fails for a token that is not a retarget, or one already baked.
pub fn draft_file(root: &Path, config: &WorkspaceConfig, token: &str) -> CoreResult<PathBuf> {
    Ok(work_of(root, config, token)?.join(DRAFT_FILE))
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
                library_frames: crate::animations::load(root, &c.clip).map_or(0, |l| l.frames),
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
