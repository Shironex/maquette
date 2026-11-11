//! What a recipe bake starts from: the checks that refuse it, the model's
//! bones by VRM name, where each blank's target sits, and the job the bake
//! script reads.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::RecipeBakeRequest;
use crate::animations::recipes::{self as library_recipes, BaseStatus, RecipeDoc};
use crate::animations::retarget::{self, BonePair, ClipSetting, Model, Staged, bones, settings};
use crate::animations::{AnimationClip, load as load_clip};
use crate::base::ports::{ScriptRunner, Watch};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::pipeline::tools::Toolbox;
use crate::project::{self, ProjectManifest};
use crate::recipes::timeline::{self, Timeline};
use crate::recipes::{BlankKind, RecipeLayer};
use crate::workspace::WorkspaceConfig;

mod clip_name;
mod grips;
mod keys;
mod parts;
mod ready;
mod rest;
mod vrm;
pub(crate) use clip_name::clip_name;
use clip_name::clip_note;
pub use ready::open_blanks;
use rest::RestSpec;
use vrm::bones_for;
pub(crate) use vrm::model_bone;
#[cfg(test)]
pub(crate) use vrm::{limb_of, mixamo_name};

const BAKE_FILE: &str = "bake_recipe.py";
const RETARGET_FILE: &str = "retarget_clips.py";
const SURFACE_FILE: &str = "surface.py";
const MOTION_FILE: &str = "recipe_motion.py";
const PINS_FILE: &str = "recipe_pins.py";
const KEYS_FILE: &str = "recipe_keys.py";
pub(super) const JOB_FILE: &str = "recipe.json";
pub(super) const REPORT_FILE: &str = "result.json";
pub(super) const PROGRESS_FILE: &str = "progress.json";
/// The step a bake shows and fails as.
const STEP: &str = "Recipe bake";
/// How many frames the rig's stretch is measured on, besides each reach's end.
const STRETCH_FRAMES: u32 = 10;

/// Where a blank's target sits, as the bake script reads it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub(crate) enum TargetSpec {
    /// In the model's own frame, from the spot it stands on.
    Point { point: [f32; 3] },
    /// The rough preview's slot in front of the model.
    Slot { slot: u32 },
    /// Where the layout places the prop, seen from where it places the model.
    Prop {
        prop: String,
        #[serde(rename = "propMatrix")]
        prop_matrix: [f32; 16],
        #[serde(rename = "modelMatrix")]
        model_matrix: [f32; 16],
    },
}

/// A bake that passed its checks, ready to stage.
pub struct Prepared {
    pub doc: RecipeDoc,
    /// The base clip; none for a recipe on the rest pose.
    pub clip: Option<AnimationClip>,
    /// The joined clips, in order (clip 2 first).
    pub joined: Vec<AnimationClip>,
    /// The frames the bake builds from its clips, for a recipe that joins
    /// clips or edits its base; none for one that plays its clip whole.
    pub(crate) timeline: Option<Timeline>,
    /// The model's left and right bones a mirror swaps.
    pub(crate) mirror_pairs: Vec<[String; 2]>,
    /// The pins on the bake's frames, solved last.
    pub(crate) pins: Vec<crate::recipes::pins::PinSpan>,
    /// Each grip on the bake's frames with its pose, by change id.
    pub(crate) grips: BTreeMap<String, grips::GripSpec>,
    /// Each grip as the review reads it: the bones the model has and lacks.
    pub(crate) grip_results: Vec<super::grips::RecipeGripResult>,
    pub asset: String,
    pub(crate) model: Model,
    pub(crate) setting: ClipSetting,
    pub(crate) map: Vec<BonePair>,
    pub(crate) follow: Vec<retarget::FollowChoice>,
    pub(crate) hips: String,
    pub(crate) bones: BTreeMap<String, String>,
    pub(crate) targets: BTreeMap<String, TargetSpec>,
    /// Prop asset ids to the files to copy in, absolute.
    pub(crate) props: BTreeMap<String, PathBuf>,
    /// The model's clips earlier bakes of this recipe added, which the new
    /// clip replaces.
    pub(crate) replace: Vec<String>,
    /// What the name clash with the model's clips came to, in a sentence.
    pub(crate) clip_note: Option<String>,
    pub warnings: Vec<String>,
}

fn invalid<T>(message: String) -> CoreResult<T> {
    Err(CoreError::Invalid(message))
}

/// Checks a bake and gathers what it needs: the recipe as it is on disk (at
/// `expected`, when given), no open blanks, at least one change, its base
/// clip in the library, a rigged model with a bone for every change.
///
/// # Errors
///
/// `moved` for a recipe that changed since `expected`; `invalid` naming
/// what to fix otherwise; as reading the project, recipe and model do.
pub fn prepare(
    root: &Path,
    config: &WorkspaceConfig,
    request: &RecipeBakeRequest,
) -> CoreResult<Prepared> {
    let manifest = project::load(root, &config.layout, &request.project)?;
    let dir = project::dir(root, &config.layout, &request.project)?;
    let doc = library_recipes::get(root, &dir, &manifest, &request.recipe)?;
    if let Some(expected) = &request.expected
        && *expected != doc.version
    {
        return Err(CoreError::Moved(format!(
            "{} changed since it was read, so it was not baked. Read it again and bake what it holds now.",
            doc.recipe.name
        )));
    }
    let open = open_blanks(&doc);
    if !open.is_empty() {
        return invalid(format!(
            "Fill the open blanks of {} first: {}. A bake needs every target.",
            doc.recipe.name,
            open.join(", ")
        ));
    }
    if doc.recipe.layers.is_empty() {
        return invalid(format!(
            "{} has no changes yet. Add one first: a reach, a pin, pose keys or a grip, say.",
            doc.recipe.name
        ));
    }
    let clip = match (&doc.base.status, &doc.base.clip) {
        (BaseStatus::Rest, _) => None,
        (BaseStatus::Found | BaseStatus::HashDiffers, Some(id)) => Some(load_clip(root, id)?),
        _ => {
            return invalid(doc.base.warning.clone().unwrap_or_else(|| {
                "The recipe's base clip is not in the animation library.".to_string()
            }));
        }
    };
    let joined = parts::joined_clips(root, &doc)?;
    let model = retarget::model(root, config, &request.project, &request.asset)?;
    if let Some(blank) =
        doc.recipe.blanks.iter().find(|b| {
            b.kind == BlankKind::Prop && b.prop.as_deref() == Some(request.asset.as_str())
        })
    {
        return invalid(format!(
            "The blank {} holds {}, the model this bakes onto, which cannot reach for itself. Fill it with another prop.",
            blank.id, model.name
        ));
    }
    let entry = retarget::uses::of_asset(&model.dir, &request.asset)?;
    let names = clip.as_ref().map(retarget::bone_names).unwrap_or_default();
    let map = bones::suggest(&names, &model.joints, &entry.map, &[]);
    let base = settings::snake(&doc.recipe.name);
    // A record that cannot be read names no earlier bake: the older clip
    // then stays and the new one takes a free name.
    let bakes = crate::recipes::baked::of_asset(&model.dir, &request.asset).unwrap_or_default();
    let naming = clip_name(&model.clips, &bakes, &doc.id);
    let setting = match &clip {
        Some(clip) => {
            let mut setting = settings::default_for(clip);
            setting.name.clone_from(&doc.recipe.name);
            settings::normalise(&[setting], std::slice::from_ref(clip), &naming.taken)?.remove(0)
        }
        None => rest::setting(&base, doc.recipe.base.frames, &naming.taken),
    };
    let note = clip_note(&base, &setting.name, &naming.replace, &model.name);
    let hips = model_bone("hips", &map, &model.joints).ok_or_else(|| {
        CoreError::Invalid(format!(
            "{} has no hips in its bone map, so a clip cannot go on it.",
            model.name
        ))
    })?;
    let mut bones = bones_for(&doc.recipe.layers, &map, &model)?;
    let (keyed, unkeyed) = keys::pose_bones(&doc.recipe, &map, &model);
    bones.extend(keyed);
    let whole = timeline::timeline(&doc.recipe);
    let (gripped, grip_results) = grips::grip_bones(&doc.recipe, &whole, &map, &model);
    bones.extend(gripped);
    let (targets, props) = targets_of(&doc, &manifest, &dir, &request.asset)?;
    let warnings = std::iter::once(&doc.base)
        .chain(&doc.joined)
        .filter_map(|m| m.warning.clone())
        .chain(unkeyed)
        .collect();
    let pins = crate::recipes::pins::spans(&doc.recipe, &whole);
    let grips = grips::grip_specs(&doc.recipe, &whole);
    let timeline = (!timeline::plain(&doc.recipe)).then_some(whole);
    let mirror_pairs = if doc.recipe.mirror {
        vrm::mirror_pairs(&map, &model.joints)
    } else {
        Vec::new()
    };
    Ok(Prepared {
        asset: request.asset.clone(),
        follow: entry.follow,
        doc,
        clip,
        joined,
        timeline,
        mirror_pairs,
        pins,
        grips,
        grip_results,
        model,
        setting,
        map,
        hips,
        bones,
        targets,
        props,
        replace: naming.replace,
        clip_note: note,
        warnings,
    })
}

fn flat(m: &crate::layout::Mat4) -> [f32; 16] {
    std::array::from_fn(|i| m[i / 4][i % 4])
}

/// Where each filled blank's target sits, and the prop files to copy in. A
/// prop takes the preview's next slot in front of the model, unless the
/// layout places both it and the model.
fn targets_of(
    doc: &RecipeDoc,
    manifest: &ProjectManifest,
    project_dir: &Path,
    asset: &str,
) -> CoreResult<(BTreeMap<String, TargetSpec>, BTreeMap<String, PathBuf>)> {
    let layout = crate::layout::load(project_dir)?;
    let placed_model = layout.models.get(asset);
    let mut targets = BTreeMap::new();
    let mut props = BTreeMap::new();
    let mut slot = 0;
    for blank in doc.recipe.blanks.iter().filter(|b| b.filled()) {
        if blank.kind == BlankKind::Point {
            if let Some(point) = blank.point {
                targets.insert(blank.id.clone(), TargetSpec::Point { point });
            }
            continue;
        }
        let Some(prop) = &blank.prop else { continue };
        let file = crate::assets::find(manifest, prop)
            .ok()
            .and_then(|a| a.latest())
            .map(|v| project_dir.join(&v.file))
            .filter(|f| f.is_file());
        let spec = match (layout.models.get(prop), placed_model, file) {
            (Some(at), Some(model), Some(file)) => {
                props.insert(prop.clone(), file);
                TargetSpec::Prop {
                    prop: format!("in/props/{prop}.glb"),
                    prop_matrix: flat(&crate::layout::matrix(at)),
                    model_matrix: flat(&crate::layout::matrix(model)),
                }
            }
            _ => TargetSpec::Slot { slot },
        };
        slot += 1;
        targets.insert(blank.id.clone(), spec);
    }
    Ok((targets, props))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecipeSpec<'a> {
    /// The base clip's library id, as the retarget job names it; none on
    /// the rest pose.
    clip: Option<&'a str>,
    /// The rest pose to hold instead of a base clip.
    rest: Option<RestSpec<'a>>,
    hips: &'a str,
    bones: &'a BTreeMap<String, String>,
    layers: &'a [RecipeLayer],
    targets: &'a BTreeMap<String, TargetSpec>,
    stretch_frames: u32,
    /// The new clip's name.
    name: &'a str,
    /// What to build the motion from, frame by frame, when the recipe joins
    /// clips or edits its base; none plays its clip whole.
    timeline: Option<&'a Timeline>,
    /// Each recipe clip's part, in order.
    parts: Vec<parts::PartSpec>,
    mirror_pairs: &'a [[String; 2]],
    /// The pins on the bake's frames, solved after everything else.
    pins: &'a [crate::recipes::pins::PinSpan],
    /// Each pose key change's keys with where they stand, by change id.
    pose_keys: BTreeMap<String, keys::PoseLayerSpec>,
    /// Each grip change on the bake's frames with its pose, by change id.
    grips: &'a BTreeMap<String, grips::GripSpec>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecipeJob<'a> {
    retarget: retarget::ScriptJob,
    recipe: RecipeSpec<'a>,
    out: &'static str,
    report: &'static str,
    progress: &'static str,
}

/// Copies the model, the base clip and the props into `work` and writes the
/// job and the scripts the bake script imports.
///
/// # Errors
///
/// Fails when a file cannot be copied or written.
pub(crate) fn stage(root: &Path, prepared: &Prepared, work: &Path) -> CoreResult<()> {
    let (clips, settings, parts) = if prepared.timeline.is_some() {
        parts::parts_of(prepared.clip.as_ref(), &prepared.joined)
    } else {
        let clips: Vec<AnimationClip> = prepared.clip.iter().cloned().collect();
        let settings = clips.iter().map(|_| prepared.setting.clone()).collect();
        (clips, settings, Vec::new())
    };
    let job = retarget::script_job(
        root,
        work,
        &prepared.model,
        (&clips, &settings),
        &prepared.map,
        Staged {
            replace: prepared.replace.clone(),
            follow: prepared.follow.clone(),
        },
    )?;
    for (id, file) in &prepared.props {
        let to = work.join("in").join("props").join(format!("{id}.glb"));
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        fs::copy(file, &to).map_err(|e| CoreError::io(file, e))?;
    }
    let surface = crate::rig::scripts::text(crate::rig::scripts::HUMANOID, SURFACE_FILE)?;
    for (name, text) in [
        (RETARGET_FILE, retarget::RETARGET_SCRIPT),
        (SURFACE_FILE, surface),
        (MOTION_FILE, super::MOTION_SCRIPT),
        (PINS_FILE, super::PINS_SCRIPT),
        (KEYS_FILE, super::KEYS_SCRIPT),
    ] {
        let path = work.join(name);
        crate::fs::write_atomic(&path, text.as_bytes()).map_err(|e| CoreError::io(&path, e))?;
    }
    write_json(
        &work.join(JOB_FILE),
        &RecipeJob {
            retarget: job,
            recipe: RecipeSpec {
                clip: prepared.clip.as_ref().map(|c| c.id.as_str()),
                rest: prepared
                    .clip
                    .is_none()
                    .then(|| RestSpec::of(&prepared.setting)),
                hips: &prepared.hips,
                bones: &prepared.bones,
                layers: &prepared.doc.recipe.layers,
                targets: &prepared.targets,
                stretch_frames: STRETCH_FRAMES,
                name: &prepared.setting.name,
                timeline: prepared.timeline.as_ref(),
                parts,
                mirror_pairs: &prepared.mirror_pairs,
                pins: &prepared.pins,
                pose_keys: keys::pose_specs(&prepared.doc.recipe),
                grips: &prepared.grips,
            },
            out: super::STAGED_FILE,
            report: REPORT_FILE,
            progress: PROGRESS_FILE,
        },
    )
}

/// Runs the staged bake in `work`, headless, watched.
///
/// # Errors
///
/// Without Blender, when the run fails, or once it is cancelled.
pub(crate) fn run(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    work: &Path,
    watch: &mut Watch<'_>,
) -> CoreResult<()> {
    let extra = vec!["--job".to_string(), JOB_FILE.to_string()];
    runner.run_watched(
        tools,
        work,
        (STEP, (BAKE_FILE, super::BAKE_SCRIPT)),
        &extra,
        watch,
    )
}
