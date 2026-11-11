//! Undoing a replace: the model the asset had is current again, with the
//! real size, fix-ups, material tweaks, picture and library origin it had,
//! and the built asset's own card comes back from the asset trash. The
//! versions added since (the built model and every rebuild) leave the asset,
//! and their files go to a folder in the workspace trash: nothing is deleted.
//! The layout was never touched, so it needs nothing.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::apply::{set_looks, set_prep};
use super::swaps::undone_record;
use super::{ReplaceEnv, invalid};
use crate::assets::copy::{key, thumb_of};
use crate::assets::remove;
use crate::assets::{self, Asset, AssetVersion};
use crate::built::{ReplacedModel, UndoneReplace, load, update};
use crate::error::CoreResult;
use crate::layout::stem;
use crate::materials;
use crate::pipeline::cache::project_cache_dir;
use crate::pipeline::history;
use crate::pipeline::template::CHARACTER_LOCK;
use crate::prepare;
use crate::project::{self, ProjectManifest, now_rfc3339};

/// One version of the asset, by its file and numbers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelRef {
    pub version: u32,
    /// Relative to the project.
    pub file: String,
    pub triangles: Option<u32>,
    /// Bytes as a JS safe number.
    pub bytes: Option<f64>,
}

impl ModelRef {
    fn of(version: &AssetVersion) -> Self {
        Self {
            version: version.version,
            file: version.file.clone(),
            triangles: version.stats.as_ref().map(|s| s.triangles),
            bytes: version.stats.as_ref().map(|s| s.bytes),
        }
    }
}

/// What undoing a replace changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UndoReport {
    pub project: String,
    pub asset: String,
    pub name: String,
    /// The script that stops building this asset.
    pub script: String,
    /// The model that is current again.
    pub back: ModelRef,
    /// The versions that leave the asset, oldest first: the built model and
    /// every version added since.
    pub taken_off: Vec<ModelRef>,
    /// The versions among them that no build of the script made (an Inbox
    /// accept, say). An agent's undo is refused while there are any.
    pub not_built: Vec<u32>,
    /// Where their files went, relative to the workspace, once it is done.
    pub set_aside: Option<String>,
    /// The built asset whose card comes back from the asset trash, by name.
    pub restores: Option<String>,
    /// What comes back with the model: its real size, fix-ups and the rest.
    pub settings: Vec<String>,
    pub warnings: Vec<String>,
}

/// What an undo did, with the project as it is now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UndoDone {
    pub report: UndoReport,
    pub project: ProjectManifest,
}

struct Planned {
    report: UndoReport,
    project_dir: PathBuf,
    record: ReplacedModel,
    /// The files and folders to set aside, relative to the project.
    paths: Vec<String>,
    /// The trash id the built asset's card comes back from.
    trash: Option<String>,
    /// The stem the locked character row goes back to, when it names a
    /// version that leaves.
    preset: Option<String>,
    /// What the next Ship names, when the built model shipped.
    undone: Option<UndoneReplace>,
}

/// The file of a version, or the folder an `.obj` version keeps its files in.
fn version_root(file: &str, key: &str) -> String {
    let parts: Vec<&str> = file.split('/').collect();
    match parts.as_slice() {
        [models, folder, _] if *models == "models" && folder.starts_with(key) => {
            format!("{models}/{folder}")
        }
        _ => file.to_string(),
    }
}

/// What comes back with the old model, in words.
fn settings_back(record: &ReplacedModel) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(prep) = &record.prep {
        if let Some(size) = &prep.size {
            out.push(format!("Real size, {}", size.describe()));
        }
        out.extend(prep.fixups.iter().map(prepare::Fixup::describe));
        if prep.skips_bake() {
            out.push("Skips the bake".to_string());
        }
        if prep.pads_edges() {
            out.push("Pads texture edges".to_string());
        }
        if prep.parts.as_ref().is_some_and(|p| !p.is_empty()) {
            out.push("Part names and joins".to_string());
        }
    }
    if record.materials.as_ref().is_some_and(|m| !m.is_empty()) {
        out.push("Material tweaks".to_string());
    }
    if let Some(origin) = &record.origin {
        out.push(format!(
            "Library origin, {} v{}",
            origin.entry, origin.version
        ));
    }
    out
}

/// The stem the locked character compression row goes back to: the row
/// follows its asset's newest model (`template::fit_presets`), so after the
/// replace it may name a version that leaves, which no file matches any more.
fn preset_back(project_dir: &Path, taken: &[&AssetVersion], back: &AssetVersion) -> Option<String> {
    let def = crate::pipeline::load(project_dir).ok().flatten()?;
    let row = def
        .presets
        .iter()
        .find(|p| p.lock.as_deref() == Some(CHARACTER_LOCK))?;
    taken
        .iter()
        .any(|v| stem(&v.file).eq_ignore_ascii_case(&row.model))
        .then(|| stem(&back.file))
}

/// Whether the built asset's card can come back, and what to say if not.
fn card_back(
    env: &ReplaceEnv<'_>,
    manifest: &ProjectManifest,
    record: &ReplacedModel,
) -> CoreResult<(Option<String>, Option<String>)> {
    let by = &record.by;
    if manifest.assets.iter().any(|a| a.id == by.asset) {
        return Ok((None, None));
    }
    let listed = remove::list_trash(env.root, &env.config.layout, env.project)?;
    let found = by
        .trash
        .as_ref()
        .filter(|id| listed.iter().any(|entry| &entry.id == *id));
    Ok(match found {
        Some(id) => (Some(id.clone()), None),
        None => (
            None,
            Some(format!(
                "{} is not in the asset trash any more, so its card does not come back. Its script stays in built/.",
                by.name
            )),
        ),
    })
}

fn planned(env: &ReplaceEnv<'_>, asset_id: &str) -> CoreResult<Planned> {
    let layout = &env.config.layout;
    let project_dir = project::dir(env.root, layout, env.project)?;
    let manifest = project::load(env.root, layout, env.project)?;
    remove::refuse_while_running(&manifest, &env.busy)?;
    let asset = assets::find(&manifest, asset_id)?.clone();
    if env.busy.building.iter().any(|b| b == &asset.id) {
        return Err(invalid(format!(
            "{} is building in Blender now. Bring the old model back once the build is done.",
            asset.name
        )));
    }
    let built = load(&project_dir)?;
    let Some(source) = built.assets.get(&asset.id) else {
        return Err(invalid(format!("{} is not a built asset.", asset.name)));
    };
    let Some(record) = source.replaced.clone() else {
        return Err(invalid(format!(
            "{} was added as a built asset and took no other model's place, so there is no older model to bring back.",
            asset.name
        )));
    };
    let back = record
        .version()
        .and_then(|n| asset.versions.iter().find(|v| v.version == n))
        .ok_or_else(|| {
            invalid(format!(
                "The model {} had before is no longer among its versions, so it cannot come back.",
                asset.name
            ))
        })?;
    if !project_dir.join(&back.file).exists() {
        return Err(invalid(format!(
            "{} is not in the project any more, so v{} cannot come back.",
            back.file, back.version
        )));
    }
    let taken: Vec<&AssetVersion> = asset
        .versions
        .iter()
        .filter(|v| !record.versions.contains(&v.version))
        .collect();
    let stem = key(&asset.id);
    let mut paths = Vec::new();
    for version in &taken {
        paths.push(version_root(&version.file, &stem));
        paths.push(thumb_of(&asset.id, version.version));
    }
    let (trash, gone) = card_back(env, &manifest, &record)?;
    let mut warnings: Vec<String> = gone.into_iter().collect();
    let not_built: Vec<u32> = taken
        .iter()
        .map(|v| v.version)
        .filter(|n| !record.built.contains(n))
        .collect();
    if !not_built.is_empty() {
        let list: Vec<String> = not_built.iter().map(|n| format!("v{n}")).collect();
        warnings.push(format!(
            "{} did not come from a build of {} (an Inbox accept, say). The undo takes it off with the rest, into the workspace trash.",
            list.join(", "),
            source.script
        ));
    }
    let cache = project_cache_dir(&env.root.join(&layout.cache), env.project);
    let runs = history::load_runs(&cache);
    let undone = undone_record(
        &project_dir,
        &manifest,
        &asset,
        &source.script,
        &record,
        &runs,
    );
    if undone.is_some() {
        warnings.push(format!(
            "The built model of {} has shipped since the replace. The next Ship puts the old model back, and its approval names it.",
            asset.name
        ));
    }
    let preset = preset_back(&project_dir, &taken, back);
    let mut settings = settings_back(&record);
    if let Some(stem) = &preset {
        settings.push(format!(
            "Locked character compression preset, back on {stem}"
        ));
    }
    Ok(Planned {
        report: UndoReport {
            project: manifest.id.clone(),
            asset: asset.id.clone(),
            name: asset.name.clone(),
            script: source.script.clone(),
            back: ModelRef::of(back),
            taken_off: taken.into_iter().map(ModelRef::of).collect(),
            not_built,
            set_aside: None,
            restores: trash.as_ref().map(|_| record.by.name.clone()),
            settings,
            warnings,
        },
        project_dir,
        record,
        paths,
        trash,
        preset,
        undone,
    })
}

/// What undoing the replace of `asset` would change. Reads only.
///
/// # Errors
///
/// Refuses while a pipeline run is going in the project or the asset is
/// building, for an asset no replace made built, and when the model it had
/// is gone from its versions or from the project.
pub fn undo_plan(env: &ReplaceEnv<'_>, asset: &str) -> CoreResult<UndoReport> {
    planned(env, asset).map(|p| p.report)
}

/// The asset's versions, picture, origin, status and stage as the record
/// has them, and the built asset's card back where it stood.
fn restore_entry(asset: &mut Asset, record: &ReplacedModel) {
    let mut kept = Vec::new();
    for number in &record.versions {
        if let Some(version) = asset.versions.iter().find(|v| v.version == *number) {
            kept.push(version.clone());
        }
    }
    asset.versions = kept;
    asset.thumbnail.clone_from(&record.thumbnail);
    asset.origin.clone_from(&record.origin);
    asset.status = record.status;
    asset.stage = record.stage;
    asset.updated_at = now_rfc3339();
}

/// Brings back the model `asset` had before a built asset took its place.
///
/// # Errors
///
/// As [`undo_plan`], and when a file cannot be written.
pub fn undo(env: &ReplaceEnv<'_>, asset: &str) -> CoreResult<UndoDone> {
    let Planned {
        mut report,
        project_dir,
        record,
        paths,
        trash,
        preset,
        undone,
    } = planned(env, asset)?;
    let layout = &env.config.layout;
    let id = report.asset.clone();
    // The built asset's card first, so its script is in use the whole time.
    let mut card = false;
    if let Some(trash) = &trash {
        match remove::restore(env.root, layout, env.project, trash, &env.busy) {
            Ok(done) => {
                card = true;
                report.warnings.extend(done.warnings);
            }
            Err(err) => {
                report.restores = None;
                report.warnings.push(format!(
                    "{} could not come back from the asset trash: {err} Its script stays in built/.",
                    record.by.name
                ));
            }
        }
    }
    // What the settings and the source are now goes with the files, so an
    // edit made since the replace is not lost without a trace.
    let note = json!({
        "asset": id,
        "undoneAt": now_rfc3339(),
        "why": "These versions were added after a built asset took the place of this asset's model. Undoing the replace took them off.",
        "source": load(&project_dir)?.assets.get(&id),
        "prep": prepare::load(&project_dir)?.models.get(&id),
        "materials": materials::load(&project_dir)?.models.get(&id),
        "versions": report.taken_off,
    });
    project::modify(env.root, layout, env.project, |m| {
        restore_entry(assets::find_mut(m, &id)?, &record);
        let at = m.assets.iter().position(|a| a.id == record.by.asset);
        if let Some(at) = at.filter(|_| card) {
            let moved = m.assets.remove(at);
            let to = (record.by.index as usize).min(m.assets.len());
            m.assets.insert(to, moved);
        }
        Ok(())
    })?;
    match remove::set_aside(&env.place()?, &id, &paths, &note) {
        Ok(folder) => report.set_aside = folder,
        Err(err) => report.warnings.push(format!(
            "The versions are off the asset, but their files could not be moved to the trash: {err} Move {} out of the project by hand, or the next build stops at a file that is already there.",
            paths.join(", ")
        )),
    }
    update(&project_dir, |file| {
        file.assets.remove(&id);
        if let Some(undone) = undone {
            file.undone.insert(id.clone(), undone);
        }
        Ok(())
    })?;
    set_prep(&project_dir, &id, record.prep.as_ref())?;
    set_looks(&project_dir, &id, record.materials.as_ref())?;
    let manifest = project::load(env.root, layout, env.project)?;
    if let Some(stem) = preset {
        crate::pipeline::update(&project_dir, &manifest, |def| {
            let row = def
                .presets
                .iter_mut()
                .find(|p| p.lock.as_deref() == Some(CHARACTER_LOCK));
            if let Some(row) = row {
                row.model = stem;
            }
            Ok(())
        })?;
    }
    Ok(UndoDone {
        report,
        project: manifest,
    })
}
