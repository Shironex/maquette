//! Updating an asset from the library: the entry's newest model comes in as
//! the asset's next version. The project's own fix-ups and settings stay,
//! except the ones the new model makes pointless, and the plan says why.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::use_in::target_note;
use super::{LibraryVersion, entry_dir, files, load};
use crate::assets::copy::key;
use crate::assets::{self, Asset, AssetOrigin, ModelStats};
use crate::error::{CoreError, CoreResult};
use crate::inbox::checks::short_count;
use crate::materials::{self, ModelMaterials};
use crate::prepare::{self, Fixup, ModelPrep};
use crate::project::{self, ProjectManifest};
use crate::workspace::FolderLayout;

/// A setting an update drops, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingChange {
    pub what: String,
    pub why: String,
}

/// What Update would do, before it does it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlan {
    pub project: String,
    pub asset: String,
    pub name: String,
    pub entry: String,
    /// The library version the asset is on now.
    pub from: u32,
    /// The library's newest version.
    pub to: u32,
    /// The asset version the new model becomes.
    pub next_version: u32,
    /// Settings that stay as they are.
    pub kept: Vec<String>,
    /// Settings that no longer apply to the new model.
    pub dropped: Vec<SettingChange>,
    /// What the project's target does with it, for a game.
    pub note: Option<String>,
}

/// What Update did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryUpdated {
    pub plan: UpdatePlan,
    pub asset: Asset,
    pub project: ProjectManifest,
}

/// Why a fix-up no longer applies to a model with these numbers, if it
/// does not.
fn pointless(fixup: &Fixup, stats: Option<&ModelStats>) -> Option<String> {
    let stats = stats?;
    match *fixup {
        Fixup::Decimate { triangles } if stats.triangles <= triangles => Some(format!(
            "the new model has {} triangles, already at or under {}",
            short_count(stats.triangles),
            short_count(triangles)
        )),
        Fixup::ShrinkTextures { .. } if stats.textures == 0 => {
            Some("the new model has no textures".to_string())
        }
        Fixup::ShrinkTextures { max_size } if stats.max_texture <= max_size => Some(format!(
            "its largest texture is {} px, already at or under {max_size} px",
            stats.max_texture
        )),
        _ => None,
    }
}

/// The fix-ups and settings to keep and to drop for `stats`.
fn check_prep(
    prep: &ModelPrep,
    stats: Option<&ModelStats>,
    kept: &mut Vec<String>,
    dropped: &mut Vec<SettingChange>,
) -> ModelPrep {
    if let Some(size) = &prep.size {
        kept.push(format!("Real size, {}", size.describe()));
    }
    let mut next = prep.clone();
    next.fixups.retain(|fixup| match pointless(fixup, stats) {
        Some(why) => {
            dropped.push(SettingChange {
                what: fixup.describe(),
                why,
            });
            false
        }
        None => {
            kept.push(fixup.describe());
            true
        }
    });
    if prep.skips_bake() {
        kept.push("Skips the bake".to_string());
    }
    if prep.pads_edges() {
        kept.push("Pads texture edges".to_string());
    }
    next
}

/// The material tweaks to keep: one named after a material the new model
/// does not have would do nothing.
fn check_looks(
    look: &ModelMaterials,
    names: &[String],
    kept: &mut Vec<String>,
    dropped: &mut Vec<SettingChange>,
) -> ModelMaterials {
    if !look.all.is_empty() {
        kept.push("Material tweaks for every material".to_string());
    }
    let mut next = look.clone();
    next.materials.retain(|name, tweak| {
        if tweak.is_empty() {
            return true;
        }
        if names.is_empty() || names.contains(name) {
            kept.push(format!("Material tweak for {name}"));
            return true;
        }
        dropped.push(SettingChange {
            what: format!("Material tweak for {name}"),
            why: format!("the new model has no material called {name}"),
        });
        false
    });
    next
}

struct Prepared {
    plan: UpdatePlan,
    newest: LibraryVersion,
    prep: Option<ModelPrep>,
    looks: Option<ModelMaterials>,
}

fn prepare_update(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    asset_id: &str,
) -> CoreResult<Prepared> {
    let manifest = project::load(root, layout, project_id)?;
    let project_dir = project::dir(root, layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?;
    let origin = asset.origin.clone().ok_or_else(|| {
        CoreError::Invalid(format!("{} did not come from the library.", asset.name))
    })?;
    let entry = load(root, &origin.entry).map_err(|err| match err {
        CoreError::NotFound(_) => CoreError::NotFound(format!(
            "the library entry {} that {} came from",
            origin.entry, asset.name
        )),
        other => other,
    })?;
    let newest = entry
        .latest()
        .cloned()
        .ok_or_else(|| CoreError::Invalid(format!("{} has no model.", entry.name)))?;
    if newest.version <= origin.version {
        return Err(CoreError::Invalid(format!(
            "{} is already on the library's newest version, v{}.",
            asset.name, origin.version
        )));
    }
    let note = target_note(&manifest, asset, &newest.file)?;
    let (mut kept, mut dropped) = (Vec::new(), Vec::new());
    let saved = prepare::load(&project_dir)?;
    let prep = saved
        .models
        .get(asset_id)
        .map(|p| check_prep(p, newest.stats.as_ref(), &mut kept, &mut dropped));
    let names = materials::material_names(&entry_dir(root, &entry.id)?.join(&newest.file));
    let looks = materials::load(&project_dir)?
        .models
        .get(asset_id)
        .map(|l| check_looks(l, &names, &mut kept, &mut dropped));
    Ok(Prepared {
        plan: UpdatePlan {
            project: project_id.to_string(),
            asset: asset_id.to_string(),
            name: asset.name.clone(),
            entry: entry.id.clone(),
            from: origin.version,
            to: newest.version,
            next_version: asset.next_version(),
            kept,
            dropped,
            note,
        },
        newest,
        prep,
        looks,
    })
}

/// What updating `asset_id` would do. Reads only.
///
/// # Errors
///
/// Fails for an asset that did not come from the library, an entry that is
/// gone, an asset already on the newest version or a model the project's
/// target cannot use.
pub fn plan_update(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    asset_id: &str,
) -> CoreResult<UpdatePlan> {
    prepare_update(root, layout, project_id, asset_id).map(|p| p.plan)
}

/// Brings the library entry's newest model into the asset as its next
/// version, with its picture, and drops the settings the plan drops.
///
/// # Errors
///
/// As [`plan_update`], and when a file cannot be copied or is already there.
pub fn update(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    asset_id: &str,
) -> CoreResult<LibraryUpdated> {
    let Prepared {
        plan,
        newest,
        prep,
        looks,
    } = prepare_update(root, layout, project_id, asset_id)?;
    let project_dir = project::dir(root, layout, project_id)?;
    let from = entry_dir(root, &plan.entry)?;
    let asset_key = key(asset_id);
    let number = plan.next_version;
    let file = files::copy_version(&from, &newest.file, &project_dir, &asset_key, number)?;
    let picture = match newest.thumbnail.as_deref() {
        Some(p) => files::copy_picture(&from, p, &project_dir, &asset_key, number).unwrap_or(None),
        None => None,
    };
    let mut updated = None;
    let changed = project::modify(root, layout, project_id, |m| {
        assets::add_version(m, asset_id, file.clone(), newest.stats.clone())?;
        let asset = m
            .assets
            .iter_mut()
            .find(|a| a.id == asset_id)
            .ok_or_else(|| CoreError::NotFound(format!("the asset {asset_id}")))?;
        asset.origin = Some(AssetOrigin {
            entry: plan.entry.clone(),
            version: plan.to,
        });
        if picture.is_some() {
            asset.thumbnail.clone_from(&picture);
        }
        updated = Some(asset.clone());
        Ok(())
    });
    let project = match changed {
        Ok(project) => project,
        Err(err) => {
            files::remove_version(&project_dir, &file);
            return Err(err);
        }
    };
    if !plan.dropped.is_empty() {
        save_settings(&project_dir, asset_id, prep.as_ref(), looks.as_ref())?;
    }
    let asset = updated.ok_or_else(|| CoreError::NotFound(format!("the asset {asset_id}")))?;
    Ok(LibraryUpdated {
        plan,
        asset,
        project,
    })
}

fn save_settings(
    project_dir: &Path,
    asset: &str,
    prep: Option<&ModelPrep>,
    looks: Option<&ModelMaterials>,
) -> CoreResult<()> {
    if let Some(prep) = prep {
        prepare::save_entry(project_dir, asset, prep)?;
    }
    if let Some(look) = looks {
        materials::update(project_dir, |current| {
            let mut next = current.clone();
            next.models.insert(asset.to_string(), look.clone());
            Ok(next)
        })?;
    }
    Ok(())
}
