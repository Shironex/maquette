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
use crate::inbox::analyse::bare_joint_name;
use crate::json::write_json;
use crate::pipeline::tools::Toolbox;
use crate::project::{self, ProjectManifest};
use crate::recipes::{BlankKind, LayerKind, RecipeLayer};
use crate::workspace::WorkspaceConfig;

mod clip_name;
pub(crate) use clip_name::clip_name;
use clip_name::clip_note;

const BAKE_FILE: &str = "bake_recipe.py";
const RETARGET_FILE: &str = "retarget_clips.py";
const SURFACE_FILE: &str = "surface.py";
pub(super) const JOB_FILE: &str = "recipe.json";
pub(super) const REPORT_FILE: &str = "result.json";
pub(super) const PROGRESS_FILE: &str = "progress.json";
/// The step a bake shows and fails as.
const STEP: &str = "Recipe bake";
/// How many frames the rig's stretch is measured on, besides each reach's end.
const STRETCH_FRAMES: u32 = 10;

/// VRM body bones with the humanoid (Mixamo) name the bone map lists each
/// under; `{side}` reads `left` or `right`, `{Side}` `Left` or `Right`.
const BODY: [(&str, &str); 14] = [
    ("hips", "Hips"),
    ("spine", "Spine"),
    ("chest", "Spine1"),
    ("upperChest", "Spine2"),
    ("neck", "Neck"),
    ("head", "Head"),
    ("{side}UpperLeg", "{Side}UpLeg"),
    ("{side}LowerLeg", "{Side}Leg"),
    ("{side}Foot", "{Side}Foot"),
    ("{side}Toes", "{Side}ToeBase"),
    ("{side}Shoulder", "{Side}Shoulder"),
    ("{side}UpperArm", "{Side}Arm"),
    ("{side}LowerArm", "{Side}ForeArm"),
    ("{side}Hand", "{Side}Hand"),
];

/// VRM fingers, their three segments, and Mixamo's finger name (joints 1 to 3).
const FINGERS: [(&str, [&str; 3], &str); 5] = [
    ("Thumb", ["Metacarpal", "Proximal", "Distal"], "Thumb"),
    ("Index", ["Proximal", "Intermediate", "Distal"], "Index"),
    ("Middle", ["Proximal", "Intermediate", "Distal"], "Middle"),
    ("Ring", ["Proximal", "Intermediate", "Distal"], "Ring"),
    ("Little", ["Proximal", "Intermediate", "Distal"], "Pinky"),
];

/// The bones a model's facing is read from, when it has them.
const FACING: [&str; 4] = [
    "leftUpperArm",
    "rightUpperArm",
    "leftUpperLeg",
    "rightUpperLeg",
];

/// The humanoid (Mixamo) name of VRM bone `vrm`: `rightHand` reads
/// `RightHand`, `leftIndexProximal` reads `LeftHandIndex1`.
pub(super) fn mixamo_name(vrm: &str) -> Option<String> {
    for (side, upper) in [("left", "Left"), ("right", "Right")] {
        for (pattern, mixamo) in BODY {
            if pattern.replace("{side}", side) == vrm {
                return Some(mixamo.replace("{Side}", upper));
            }
        }
        for (finger, segments, mixamo) in FINGERS {
            for (i, segment) in segments.iter().enumerate() {
                if format!("{side}{finger}{segment}") == vrm {
                    return Some(format!("{upper}Hand{mixamo}{}", i + 1));
                }
            }
        }
    }
    None
}

/// The upper and lower bones a reach of `end` solves.
pub(super) fn limb_of(end: &str) -> Option<(&'static str, &'static str)> {
    match end {
        "leftHand" => Some(("leftUpperArm", "leftLowerArm")),
        "rightHand" => Some(("rightUpperArm", "rightLowerArm")),
        "leftFoot" => Some(("leftUpperLeg", "leftLowerLeg")),
        "rightFoot" => Some(("rightUpperLeg", "rightLowerLeg")),
        _ => None,
    }
}

/// The model bone for VRM bone `vrm`: the bone map's choice, else a joint
/// with the Mixamo name itself (fingers, which the map leaves out).
pub(super) fn model_bone(vrm: &str, map: &[BonePair], joints: &[String]) -> Option<String> {
    let mixamo = mixamo_name(vrm)?;
    if let Some(pair) = map.iter().find(|p| p.bone == mixamo) {
        return pair.target.clone();
    }
    joints
        .iter()
        .find(|j| bare_joint_name(j).eq_ignore_ascii_case(&mixamo))
        .cloned()
}

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
    pub clip: AnimationClip,
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
    /// The model's clip of the same name an earlier bake of this recipe
    /// added, which the new clip replaces.
    pub(crate) replace: Option<String>,
    /// What the name clash with the model's clips came to, in a sentence.
    pub(crate) clip_note: Option<String>,
    pub warnings: Vec<String>,
}

/// The blanks of a recipe nothing fills yet, by id.
pub fn open_blanks(doc: &RecipeDoc) -> Vec<String> {
    doc.recipe
        .blanks
        .iter()
        .filter(|b| !b.filled())
        .map(|b| b.id.clone())
        .collect()
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
            "{} has no changes yet. Add a reach, nudge, look at or wave first.",
            doc.recipe.name
        ));
    }
    let clip_id = match (&doc.base.status, &doc.base.clip) {
        (BaseStatus::Found | BaseStatus::HashDiffers, Some(id)) => id.clone(),
        _ => {
            return invalid(doc.base.warning.clone().unwrap_or_else(|| {
                "The recipe's base clip is not in the animation library.".to_string()
            }));
        }
    };
    let clip = load_clip(root, &clip_id)?;
    let model = retarget::model(root, config, &request.project, &request.asset)?;
    let entry = retarget::uses::of_asset(&model.dir, &request.asset)?;
    let map = bones::suggest(&retarget::bone_names(&clip), &model.joints, &entry.map, &[]);
    let mut setting = settings::default_for(&clip);
    setting.name.clone_from(&doc.recipe.name);
    let base = settings::snake(&doc.recipe.name);
    // A record that cannot be read names no earlier bake: the older clip
    // then stays and the new one takes a free name.
    let bakes = crate::recipes::baked::of_asset(&model.dir, &request.asset).unwrap_or_default();
    let naming = clip_name(&base, &model.clips, &bakes, &doc.id);
    let setting =
        settings::normalise(&[setting], std::slice::from_ref(&clip), &naming.taken)?.remove(0);
    let note = clip_note(&base, &setting.name, naming.replace.as_deref(), &model.name);
    let hips = model_bone("hips", &map, &model.joints).ok_or_else(|| {
        CoreError::Invalid(format!(
            "{} has no hips in its bone map, so a clip cannot go on it.",
            model.name
        ))
    })?;
    let bones = bones_for(&doc.recipe.layers, &map, &model)?;
    let (targets, props) = targets_of(&doc, &manifest, &dir, &request.asset)?;
    let mut warnings = Vec::new();
    if let Some(warning) = &doc.base.warning {
        warnings.push(warning.clone());
    }
    Ok(Prepared {
        asset: request.asset.clone(),
        follow: entry.follow,
        doc,
        clip,
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

/// Every bone the changes move, by VRM name, with the limbs a reach solves
/// and the bones the facing is read from.
fn bones_for(
    layers: &[RecipeLayer],
    map: &[BonePair],
    model: &Model,
) -> CoreResult<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    let mut missing = Vec::new();
    for layer in layers {
        let mut wanted = vec![layer.bone.as_str()];
        if layer.kind == LayerKind::Reach
            && let Some((upper, lower)) = limb_of(&layer.bone)
        {
            wanted.extend([upper, lower]);
        }
        for vrm in wanted {
            match model_bone(vrm, map, &model.joints) {
                Some(bone) => {
                    out.insert(vrm.to_string(), bone);
                }
                None => missing.push(format!("{vrm} ({})", layer.id)),
            }
        }
    }
    if !missing.is_empty() {
        missing.dedup();
        return invalid(format!(
            "{} has no bone for {}. Map it in Use on model's bone map, or take that change out.",
            model.name,
            missing.join(", ")
        ));
    }
    for vrm in FACING {
        if let Some(bone) = model_bone(vrm, map, &model.joints) {
            out.insert(vrm.to_string(), bone);
        }
    }
    Ok(out)
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
    /// The base clip's library id, as the retarget job names it.
    clip: &'a str,
    hips: &'a str,
    bones: &'a BTreeMap<String, String>,
    layers: &'a [RecipeLayer],
    targets: &'a BTreeMap<String, TargetSpec>,
    stretch_frames: u32,
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
    let job = retarget::script_job(
        root,
        work,
        &prepared.model,
        (
            std::slice::from_ref(&prepared.clip),
            std::slice::from_ref(&prepared.setting),
        ),
        &prepared.map,
        Staged {
            replace: prepared.replace.iter().cloned().collect(),
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
    ] {
        let path = work.join(name);
        crate::fs::write_atomic(&path, text.as_bytes()).map_err(|e| CoreError::io(&path, e))?;
    }
    write_json(
        &work.join(JOB_FILE),
        &RecipeJob {
            retarget: job,
            recipe: RecipeSpec {
                clip: &prepared.clip.id,
                hips: &prepared.hips,
                bones: &prepared.bones,
                layers: &prepared.doc.recipe.layers,
                targets: &prepared.targets,
                stretch_frames: STRETCH_FRAMES,
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
