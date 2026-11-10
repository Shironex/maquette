//! The writes of a replace, in an order that can be taken back: the model
//! comes in as the asset's next version, the asset gains the built source
//! with its `replaced` record, its settings become the merged ones and, last,
//! the built asset's own card leaves the Board through the asset trash. The
//! script stays in `built/` because the asset names it by then.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::{Planned, ReplaceDone, ReplaceEnv, invalid};
use crate::assets::copy::thumb_of;
use crate::assets::remove::{self, RemoveState};
use crate::assets::{self, AssetStatus, AssetVersion, Stage};
use crate::built::{add_model_version, update};
use crate::error::CoreResult;
use crate::materials::{self, ModelMaterials};
use crate::prepare::{self, ModelPrep};
use crate::project;

/// Makes `want` the asset's `prepare.json` entry, writing only when it
/// differs from what is saved, so a file nothing changed in stays as it is.
pub(super) fn set_prep(dir: &Path, id: &str, want: Option<&ModelPrep>) -> CoreResult<()> {
    let saved = prepare::load(dir)?.models.get(id).cloned();
    let want = want.filter(|p| !p.is_empty());
    if saved.as_ref() == want {
        return Ok(());
    }
    prepare::save_entry(dir, id, &want.cloned().unwrap_or_default()).map(|_| ())
}

/// Makes `want` the asset's material tweaks, writing only when they differ.
pub(super) fn set_looks(dir: &Path, id: &str, want: Option<&ModelMaterials>) -> CoreResult<()> {
    let saved = materials::load(dir)?.models.get(id).cloned();
    let want = want.filter(|l| !l.is_empty());
    if saved.as_ref() == want {
        return Ok(());
    }
    let look = want.cloned().unwrap_or_default();
    materials::save_looks(dir, &BTreeMap::from([(id.to_string(), look)])).map(|_| ())
}

/// Brings the built asset's model and picture in as the asset's next version.
fn add_version(env: &ReplaceEnv<'_>, planned: &Planned) -> CoreResult<AssetVersion> {
    let dir = &planned.project_dir;
    let id = &planned.report.asset;
    let from = dir.join(&planned.model);
    let mut added: Option<AssetVersion> = None;
    let written = project::modify(env.root, &env.config.layout, env.project, |m| {
        let version = add_model_version(m, dir, id, &from, planned.stats.clone())?;
        added = Some(version.clone());
        let asset = assets::find_mut(m, id)?;
        if let Some(picture) = &planned.picture {
            let to = thumb_of(id, version.version);
            if fs::copy(dir.join(picture), dir.join(&to)).is_ok() {
                asset.thumbnail = Some(to);
            }
        }
        asset.origin = None;
        if asset.stage < Stage::Blender {
            asset.stage = Stage::Blender;
        }
        asset.status = AssetStatus::Done;
        Ok(())
    });
    match (written, added) {
        (Ok(_), Some(version)) => Ok(version),
        (Ok(_), None) => Err(invalid("The model was not added.")),
        (Err(err), added) => {
            // The copy landed and project.json did not: take the copy back.
            if let Some(version) = added {
                remove_copies(dir, id, &version);
            }
            Err(err)
        }
    }
}

/// Removes the model and picture a replace copied in for `version`.
fn remove_copies(dir: &Path, id: &str, version: &AssetVersion) {
    for file in [version.file.clone(), thumb_of(id, version.version)] {
        let path = dir.join(file);
        if path.is_file()
            && let Err(err) = fs::remove_file(&path)
        {
            tracing::warn!(error = %err, "could not remove a file of a replace that stopped part way");
        }
    }
}

/// The asset becomes built and takes its merged settings.
fn write_settings(planned: &Planned, version: u32) -> CoreResult<()> {
    let dir = &planned.project_dir;
    let id = &planned.report.asset;
    let mut source = planned.source.clone();
    if let Some(last) = source.last.as_mut() {
        last.version = Some(version);
    }
    source.replaced = Some(planned.record.clone());
    update(dir, |file| {
        if file.assets.contains_key(id) {
            return Err(invalid(format!(
                "{} is already a built asset.",
                planned.report.name
            )));
        }
        file.assets.insert(id.clone(), source);
        Ok(())
    })?;
    set_prep(dir, id, Some(&planned.prep))?;
    set_looks(dir, id, planned.looks.as_ref())
}

/// Takes back a replace that stopped part way: the built source, the
/// settings and the new version all go.
fn roll_back(env: &ReplaceEnv<'_>, planned: &Planned, version: &AssetVersion) {
    let dir = &planned.project_dir;
    let id = &planned.report.asset;
    let record = &planned.record;
    let undone = update(dir, |file| {
        if file.assets.get(id).is_some_and(|s| s.replaced.is_some()) {
            file.assets.remove(id);
        }
        Ok(())
    })
    .and_then(|()| set_prep(dir, id, planned.old_prep.as_ref()))
    .and_then(|()| set_looks(dir, id, planned.old_looks.as_ref()))
    .and_then(|()| {
        project::modify(env.root, &env.config.layout, env.project, |m| {
            let asset = assets::find_mut(m, id)?;
            asset.versions.retain(|v| v.version != version.version);
            asset.thumbnail.clone_from(&record.thumbnail);
            asset.origin.clone_from(&record.origin);
            asset.status = record.status;
            asset.stage = record.stage;
            Ok(())
        })
    });
    if let Err(err) = undone {
        tracing::warn!(error = %err, "could not take back a replace that stopped part way");
    }
    remove_copies(dir, id, version);
}

/// Takes the built asset's own card off the Board. Returns its trash id, or
/// why it stayed.
fn retire(env: &ReplaceEnv<'_>, planned: &Planned) -> (Option<String>, Vec<String>) {
    let name = &planned.report.built_name;
    let stays = |why: &str| {
        format!(
            "{name} could not be taken off the Board: {why} Remove it from the project yourself, or the scene builds it as a second model."
        )
    };
    let ids = [planned.report.built.clone()];
    let layout = &env.config.layout;
    match remove::remove_assets(env.root, layout, env.project, &ids, &env.busy, env.sends) {
        Ok(done) => match done.outcomes.into_iter().next() {
            Some(out) if out.state == RemoveState::Removed => (out.trash, out.warnings),
            Some(out) => (None, vec![stays(&out.reason.unwrap_or_default())]),
            None => (None, vec![stays("the remove answered nothing.")]),
        },
        Err(err) => (None, vec![stays(&err.to_string())]),
    }
}

/// Runs a planned replace.
pub(super) fn run(env: &ReplaceEnv<'_>, planned: Planned) -> CoreResult<ReplaceDone> {
    let version = add_version(env, &planned)?;
    if let Err(err) = write_settings(&planned, version.version) {
        roll_back(env, &planned, &version);
        return Err(err);
    }
    let (trash, notes) = retire(env, &planned);
    let id = planned.report.asset.clone();
    if let Some(trash) = &trash {
        update(&planned.project_dir, |file| {
            if let Some(record) = file.assets.get_mut(&id).and_then(|s| s.replaced.as_mut()) {
                record.by.trash = Some(trash.clone());
            }
            Ok(())
        })?;
    }
    let mut report = planned.report;
    report.new.file.clone_from(&version.file);
    report.new.version = version.version;
    report.trash = trash;
    report.warnings.extend(notes);
    Ok(ReplaceDone {
        report,
        project: project::load(env.root, &env.config.layout, env.project)?,
    })
}
