//! Replacing a model with a built asset, in place: "this built asset now
//! stands where that generated model stood" as one action.
//!
//! The asset that is replaced keeps its id and becomes the built one. It
//! gains the built asset's script, helpers and root in `built.json`, the
//! built asset's newest model lands as its next version and its older
//! versions stay below as history. Everything keyed by its id is therefore
//! untouched: its placement in `layout.json`, the camera stops and path ends
//! that name it, its object states, explode track and clip choices, its ship
//! choice and file name in `ship.json` and its game file. The next run
//! builds, bakes and ships the new model like any other built piece, Ship
//! writes the new file name by itself, and a later run cannot bring the old
//! model back, since the newest version is the built one.
//!
//! What was recorded for the old model itself is looked at setting by
//! setting ([`settings`]): the real size carries over, fix-ups and switches
//! that repaired a scan are reset to the built asset's own, and what names a
//! part or clip the new model lacks is reported and left alone ([`links`]).
//! Nothing is aligned: the report gives the old and new bounds so a model
//! built in another orientation or around another origin shows.
//!
//! The built asset's own card leaves the Board through the asset trash, and
//! what the replaced asset was is kept on its built source
//! ([`super::ReplacedModel`]), so [`undo`] puts the old model back as it was.
//! Website and game projects both work: nothing here depends on the target.

mod apply;
mod links;
mod report;
mod settings;
mod swaps;
mod undo;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{BuiltFile, BuiltSource, BuiltState, ReplacedBy, ReplacedModel, load, state_of};
use crate::assets::copy::{key, thumb_of};
use crate::assets::remove::{self, Busy, Place, Sends, SentState};
use crate::assets::{self, Asset, ModelStats};
use crate::error::{CoreError, CoreResult};
use crate::materials::{self, ModelMaterials};
use crate::prepare::{self, ModelPrep};
use crate::project::{self, ProjectManifest, now_rfc3339};
use crate::workspace::WorkspaceConfig;

pub use report::{NOTE, ReplaceReport, SwapAction, SwapSetting, SwapSide};
pub use swaps::{ShipSwap, swaps};
pub use undo::{ModelRef, UndoDone, UndoReport, undo, undo_plan};

/// Where a replace works, and what else is going on there.
pub struct ReplaceEnv<'a> {
    pub root: &'a Path,
    pub config: &'a WorkspaceConfig,
    pub project: &'a str,
    /// The pipeline run going in the project and the assets building now.
    pub busy: Busy<'a>,
    /// The project's sends: the report names the ones still making a model
    /// for the asset, and the built asset's own are stopped as a remove does.
    pub sends: &'a dyn Sends,
}

impl ReplaceEnv<'_> {
    fn place(&self) -> CoreResult<Place<'_>> {
        Place::new(self.root, &self.config.layout, self.project)
    }

    /// The fix-up previews, where the bounds after a rotate are kept.
    fn previews(&self) -> PathBuf {
        prepare::preview::previews_dir(&self.root.join(&self.config.layout.cache))
    }
}

/// What a replace did, with the project as it is now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceDone {
    pub report: ReplaceReport,
    pub project: ProjectManifest,
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

/// A replace that passed every check, with what it will write.
struct Planned {
    report: ReplaceReport,
    project_dir: PathBuf,
    /// The built asset's source, which moves to the asset.
    source: BuiltSource,
    /// The built asset's newest model and its picture, relative to the project.
    model: String,
    stats: Option<ModelStats>,
    picture: Option<String>,
    /// The asset's settings as they are, and as they become.
    old_prep: Option<ModelPrep>,
    prep: ModelPrep,
    old_looks: Option<ModelMaterials>,
    looks: Option<ModelMaterials>,
    record: ReplacedModel,
}

/// Why the built asset `by` cannot take a place right now, if it cannot.
fn not_ready(
    env: &ReplaceEnv<'_>,
    project_dir: &Path,
    by: &Asset,
    source: &BuiltSource,
) -> Option<String> {
    let building = env.busy.building.iter().any(|b| b == &by.id);
    let (state, missing) = state_of(project_dir, by, source, building);
    match state {
        BuiltState::Current => None,
        BuiltState::Building => {
            Some("is building in Blender now. Replace once the build is done.".to_string())
        }
        BuiltState::New => Some("has not built yet. Build it first.".to_string()),
        BuiltState::Stale => Some(
            "has a script that changed since its last build. Rebuild it first, so the model that takes the place is the one its script makes."
                .to_string(),
        ),
        BuiltState::Failed => Some(format!(
            "did not build: {}. Fix its script in built/ and rebuild it first.",
            source
                .last
                .as_ref()
                .and_then(|l| l.error.clone())
                .unwrap_or_default()
        )),
        BuiltState::Missing => Some(format!(
            "is missing {}. Put the files back in built/ and rebuild it first.",
            missing.join(", ")
        )),
    }
}

/// The built asset's source, once the pair may be swapped.
fn check_pair(
    env: &ReplaceEnv<'_>,
    project_dir: &Path,
    asset: &Asset,
    by: &Asset,
    built: &BuiltFile,
) -> CoreResult<BuiltSource> {
    if asset.id == by.id {
        return Err(invalid(
            "Pick two assets: the built asset, and the one whose model it replaces.",
        ));
    }
    if built.assets.contains_key(&asset.id) {
        return Err(invalid(format!(
            "{} is already a built asset. Edit its script in built/ and rebuild it.",
            asset.name
        )));
    }
    let Some(source) = built.assets.get(&by.id) else {
        return Err(invalid(format!(
            "{} is not a built asset. Add its script as a built asset first.",
            by.name
        )));
    };
    if asset.latest().is_none() {
        return Err(invalid(format!(
            "{} has no model yet, so there is nothing to replace. Use {} as it is.",
            asset.name, by.name
        )));
    }
    if let Some(why) = not_ready(env, project_dir, by, source) {
        return Err(invalid(format!("{} {why}", by.name)));
    }
    let jobs = env.sends.jobs(env.project);
    if let Some(job) = jobs
        .iter()
        .find(|j| j.asset == by.id && j.state == SentState::Running)
    {
        return Err(invalid(format!(
            "{} is making a model of {} now (send {}). Wait for it or cancel it in Sends, then replace.",
            job.provider, by.name, job.id
        )));
    }
    Ok(source.clone())
}

/// What to know that no setting says: sends still open for the asset, two
/// kinds, a placement of the built asset that is not used.
fn pair_warnings(
    env: &ReplaceEnv<'_>,
    layout: &crate::layout::SceneLayout,
    asset: &Asset,
    by: &Asset,
) -> Vec<String> {
    let mut out = Vec::new();
    let open: Vec<String> = env
        .sends
        .jobs(env.project)
        .into_iter()
        .filter(|j| j.asset == asset.id && j.state != SentState::Done)
        .map(|j| j.id)
        .collect();
    if !open.is_empty() {
        out.push(format!(
            "{} still open for {}: {}. A model accepted from one becomes the newest version, over the built one. Cancel it in Sends if it is no longer wanted.",
            if open.len() == 1 {
                "A send is"
            } else {
                "Sends are"
            },
            asset.name,
            open.join(", ")
        ));
    }
    if asset.kind != by.kind {
        out.push(format!(
            "{} is a {} and {} a {}. The asset keeps its kind.",
            by.name,
            format!("{:?}", by.kind).to_lowercase(),
            asset.name,
            format!("{:?}", asset.kind).to_lowercase()
        ));
    }
    let placed = layout.models.contains_key(&by.id)
        || layout.states.contains_key(&by.id)
        || layout.timeline.explode.contains_key(&by.id);
    if placed {
        out.push(format!(
            "{}'s own placement and states are not used: they leave the Board with its card and come back with an undo.",
            by.name
        ));
    }
    out
}

/// The file stem of an asset's newest model, as the presets name it.
fn newest_stem(asset: &Asset) -> Vec<String> {
    asset
        .latest()
        .map(|v| crate::layout::stem(&v.file))
        .into_iter()
        .collect()
}

fn planned(env: &ReplaceEnv<'_>, asset_id: &str, built_id: &str) -> CoreResult<Planned> {
    let layout = &env.config.layout;
    let project_dir = project::dir(env.root, layout, env.project)?;
    let manifest = project::load(env.root, layout, env.project)?;
    remove::refuse_while_running(&manifest, &env.busy)?;
    let asset = assets::find(&manifest, asset_id)?.clone();
    let by = assets::find(&manifest, built_id)?.clone();
    let source = check_pair(env, &project_dir, &asset, &by, &load(&project_dir)?)?;
    let (Some(old), Some(from)) = (asset.latest().cloned(), by.latest().cloned()) else {
        return Err(invalid(format!("{} has no model to bring.", by.name)));
    };
    if !project_dir.join(&from.file).is_file() {
        return Err(invalid(format!(
            "{} is not in the project, so {} has no model to bring. Rebuild it first.",
            from.file, by.name
        )));
    }
    let number = asset.next_version();
    let file = format!("models/{}_v{number}.glb", key(&asset.id));
    if project_dir.join(&file).exists() {
        return Err(invalid(format!(
            "{file} already exists. Rename it or remove it first."
        )));
    }

    let saved = prepare::load(&project_dir)?;
    let looks = materials::load(&project_dir)?;
    let scene = crate::layout::load(&project_dir)?;
    let ship = crate::ship::load(&project_dir)?;
    let old_prep = saved.models.get(&asset.id).cloned();
    let own_prep = saved.models.get(&by.id).cloned();
    let old_looks = looks.models.get(&asset.id).cloned();
    let own_looks = looks.models.get(&by.id).cloned();

    let from_path = project_dir.join(&from.file);
    let found = crate::parts::read(&from_path).ok();
    let file_parts: Vec<String> = found.iter().flatten().map(|p| p.name.clone()).collect();
    let clips = crate::inbox::analyse(&from_path).ok().map(|r| r.animations);

    // The project once replaced: the built asset's card gone, its model the
    // asset's newest version. Ship names the file from this.
    let mut after = manifest.clone();
    after.assets.retain(|a| a.id != by.id);
    assets::add_version(&mut after, &asset.id, file.clone(), from.stats.clone())?;

    let empty = ModelPrep::default();
    let basis = settings::next_prep(
        old_prep.as_ref().unwrap_or(&empty),
        own_prep.as_ref().unwrap_or(&empty),
        None,
    );
    let previews = env.previews();
    let new_side = report::side(report::SideFile {
        project_dir: &project_dir,
        previews: Some(&previews),
        read: &from.file,
        version: number,
        file: file.clone(),
        stats: from.stats.as_ref(),
        entry: Some(&basis),
        ships_as: report::ships_as(&after, &ship, &asset.id),
    });
    let old_side = report::side(report::SideFile {
        project_dir: &project_dir,
        previews: Some(&previews),
        read: &old.file,
        version: old.version,
        file: old.file.clone(),
        stats: old.stats.as_ref(),
        entry: old_prep.as_ref(),
        ships_as: report::ships_as(&manifest, &ship, &asset.id),
    });
    let merged = settings::merge(
        old_prep.as_ref(),
        own_prep.as_ref(),
        (old_looks.as_ref(), own_looks.as_ref()),
        &file_parts,
        new_side.scale,
    );
    let parts: Vec<String> = crate::parts::rows(
        found.as_deref().unwrap_or_default(),
        merged.prep.parts.as_ref(),
    )
    .into_iter()
    .map(|row| row.name)
    .collect();
    let linked = links::of_asset(
        &project_dir,
        &manifest,
        &scene,
        &ship,
        &asset,
        found.is_some().then_some(parts.as_slice()),
        clips.as_deref(),
    );
    let presets = crate::pipeline::load(&project_dir)
        .ok()
        .flatten()
        .map(|def| def.presets)
        .unwrap_or_default();
    let (preset_lines, preset_warnings) = settings::preset_lines(
        &presets,
        &newest_stem(&asset),
        &newest_stem(&by),
        &crate::layout::stem(&file),
    );

    let mut warnings =
        report::standing_warnings(&asset.name, &old_side, &new_side, merged.prep.size.as_ref());
    warnings.extend(linked.warnings);
    warnings.extend(preset_warnings);
    warnings.extend(pair_warnings(env, &scene, &asset, &by));
    let mut lines = merged.lines;
    lines.extend(preset_lines);
    lines.extend(settings::origin_line(asset.origin.as_ref()));
    lines.extend(linked.kept);

    // A website bakes its light; a game only when its Bake setting is on.
    let bake_step = crate::game::settings_of(&manifest)
        .map(|game| game.bake)
        .unwrap_or(true);
    let bakes = bake_step && !merged.prep.skips_bake();
    let index = manifest
        .assets
        .iter()
        .position(|a| a.id == by.id)
        .unwrap_or(manifest.assets.len());
    let record = ReplacedModel {
        at: now_rfc3339(),
        by: ReplacedBy {
            asset: by.id.clone(),
            name: by.name.clone(),
            index: u32::try_from(index).unwrap_or(u32::MAX),
            trash: None,
        },
        versions: asset.versions.iter().map(|v| v.version).collect(),
        thumbnail: asset.thumbnail.clone(),
        origin: asset.origin.clone(),
        status: asset.status,
        stage: asset.stage,
        prep: old_prep.clone(),
        materials: old_looks.clone(),
    };
    let picture = Some(thumb_of(&by.id, from.version)).filter(|t| project_dir.join(t).is_file());
    Ok(Planned {
        report: ReplaceReport {
            project: manifest.id.clone(),
            asset: asset.id.clone(),
            name: asset.name.clone(),
            built: by.id.clone(),
            built_name: by.name.clone(),
            script: source.script.clone(),
            old: old_side,
            new: new_side,
            bakes,
            parts,
            settings: lines,
            unresolved: linked.unresolved,
            warnings,
            note: NOTE.to_string(),
            trash: None,
        },
        project_dir,
        source,
        model: from.file,
        stats: from.stats,
        picture,
        old_prep,
        prep: merged.prep,
        old_looks,
        looks: merged.looks,
        record,
    })
}

/// What replacing the model of `asset` with the built asset `built` would
/// change. Reads only.
///
/// # Errors
///
/// Refuses while a pipeline run is going in the project, for an asset that
/// is already built or has no model, and for a built asset that is not one
/// or whose newest model is not what its script makes (never built, stale,
/// failed, missing a file, building).
pub fn plan(env: &ReplaceEnv<'_>, asset: &str, built: &str) -> CoreResult<ReplaceReport> {
    planned(env, asset, built).map(|p| p.report)
}

/// Replaces the model of `asset` with the built asset `built`: see the
/// module text. Returns the report with where the built asset's card went.
///
/// # Errors
///
/// As [`plan`], and when a file cannot be written. A replace that stops
/// part way takes back what it wrote.
pub fn replace(env: &ReplaceEnv<'_>, asset: &str, built: &str) -> CoreResult<ReplaceDone> {
    apply::run(env, planned(env, asset, built)?)
}

/// Registers the replace's IPC types with the shell's TypeScript export.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<ReplaceReport>()
        .register::<ReplaceDone>()
        .register::<SwapSide>()
        .register::<SwapSetting>()
        .register::<SwapAction>()
        .register::<UndoReport>()
        .register::<UndoDone>()
        .register::<ModelRef>()
        .register::<ShipSwap>()
}
