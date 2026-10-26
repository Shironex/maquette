//! Copying a Board asset to another project of the workspace: its refs,
//! every model version and their thumbnails, under a name and id that are
//! free there. The asset stays where it was.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{Asset, AssetStatus, find, slugify};
use crate::concepts::REFS_DIR;
use crate::concepts::transfer::carry_ref;
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest, now_rfc3339};
use crate::workspace::FolderLayout;

/// What Copy to project did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CopiedAsset {
    /// The copy, in the other project.
    pub asset: Asset,
    /// The other project as it is now.
    pub to: ProjectManifest,
    /// Each old ref path and where it landed, relative to the projects.
    pub refs: Vec<(String, String)>,
}

/// The file stem a version or a thumbnail of the asset `id` starts with.
pub(crate) fn key(id: &str) -> String {
    id.replace('-', "_")
}

/// True for a ref file of the asset: `clay_bowl.png` or `clay_bowl_02.png`
/// belong to "clay-bowl", `clay_bowl_lid.png` does not.
pub(crate) fn is_ref_of(stem: &str, key: &str) -> bool {
    let stem = stem.to_lowercase().replace('-', "_");
    match stem.strip_prefix(key) {
        Some("") => true,
        Some(rest) => rest
            .strip_prefix('_')
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())),
        None => false,
    }
}

/// The asset's refs in `project_dir`: the ones it was sent with, its card
/// picture and the files named after it, each once.
fn refs_of(project_dir: &Path, asset: &Asset, sent: &[String]) -> Vec<String> {
    let mut refs: Vec<String> = sent.to_vec();
    refs.extend(asset.thumbnail.iter().cloned());
    let wanted = key(&asset.id);
    if let Ok(entries) = fs::read_dir(project_dir.join(REFS_DIR)) {
        let mut named: Vec<String> = entries
            .flatten()
            .filter(|e| e.path().is_file())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let stem = Path::new(&name).file_stem()?.to_string_lossy().into_owned();
                is_ref_of(&stem, &wanted).then(|| format!("{REFS_DIR}/{name}"))
            })
            .collect();
        named.sort();
        refs.extend(named);
    }
    let mut seen = Vec::new();
    refs.retain(|r| {
        let keep = r.starts_with("refs/") && !seen.contains(r);
        seen.push(r.clone());
        keep
    });
    refs
}

/// `models/clay_bowl_v2.glb` for the copy named `clay-bowl-2`.
fn renamed_version(file: &str, id: &str, version: u32) -> String {
    let ext = Path::new(file)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_else(|| "glb".to_string());
    let dir = file.rsplit_once('/').map_or("models", |(dir, _)| dir);
    format!("{dir}/{}_v{version}.{ext}", key(id))
}

fn thumb_of(id: &str, version: u32) -> String {
    format!("thumbs/{}_v{version}.png", key(id))
}

/// A name and id free in `manifest` whose model files and thumbnails are
/// free on disk too: the name itself, else "Clay bowl 2", "Clay bowl 3".
fn free_name(manifest: &ProjectManifest, to_dir: &Path, asset: &Asset) -> (String, String) {
    let taken = |name: &str| {
        let id = slugify(name);
        manifest
            .assets
            .iter()
            .any(|a| a.id == id || slugify(&a.name) == id)
            || asset.versions.iter().any(|v| {
                to_dir
                    .join(renamed_version(&v.file, &id, v.version))
                    .exists()
                    || to_dir.join(thumb_of(&id, v.version)).exists()
            })
    };
    let name = (1u32..)
        .map(|n| match n {
            1 => asset.name.clone(),
            n => format!("{} {n}", asset.name),
        })
        .find(|name| !taken(name))
        .unwrap_or_else(|| asset.name.clone());
    let id = slugify(&name);
    (name, id)
}

fn copy_file(from: &Path, to: &Path) -> CoreResult<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    fs::copy(from, to).map_err(|e| CoreError::io(from, e))?;
    Ok(())
}

/// Copies the asset `asset_id` of project `from_id` to project `to_id`.
/// `sent` lists the ref images its Send to 3D jobs used, so those come
/// along even when their names do not say whose they are.
pub fn copy_to_project(
    workspace_root: &Path,
    layout: &FolderLayout,
    from_id: &str,
    asset_id: &str,
    to_id: &str,
    sent: &[String],
) -> CoreResult<CopiedAsset> {
    if from_id == to_id {
        return Err(CoreError::Invalid(
            "Pick another project to copy the asset to.".to_string(),
        ));
    }
    let from = project::load(workspace_root, layout, from_id)?;
    let target = project::load(workspace_root, layout, to_id)?;
    let from_dir = project::dir(workspace_root, layout, from_id)?;
    let to_dir = project::dir(workspace_root, layout, to_id)?;
    let asset = find(&from, asset_id)?.clone();
    if let Some(missing) = asset
        .versions
        .iter()
        .find(|v| !from_dir.join(&v.file).is_file())
    {
        return Err(CoreError::NotFound(format!(
            "the model file {}",
            missing.file
        )));
    }

    let mut refs = Vec::new();
    for old in refs_of(&from_dir, &asset, sent) {
        if let Some(new) = carry_ref(&from_dir, &to_dir, &old)? {
            refs.push((old, new));
        }
    }

    let (name, id) = free_name(&target, &to_dir, &asset);
    let mut versions = Vec::new();
    let mut thumbnail = None;
    for v in &asset.versions {
        let file = renamed_version(&v.file, &id, v.version);
        copy_file(&from_dir.join(&v.file), &to_dir.join(&file))?;
        let old_thumb = thumb_of(&asset.id, v.version);
        if from_dir.join(&old_thumb).is_file() {
            let new_thumb = thumb_of(&id, v.version);
            copy_file(&from_dir.join(&old_thumb), &to_dir.join(&new_thumb))?;
            if asset.thumbnail.as_deref() == Some(old_thumb.as_str()) {
                thumbnail = Some(new_thumb);
            }
        }
        versions.push(super::AssetVersion { file, ..v.clone() });
    }
    if thumbnail.is_none() {
        thumbnail = asset.thumbnail.as_ref().and_then(|t| {
            refs.iter()
                .find(|(old, _)| old == t)
                .map(|(_, new)| new.clone())
        });
    }

    let now = now_rfc3339();
    let copy = Asset {
        id,
        name,
        versions,
        thumbnail,
        status: match asset.status {
            AssetStatus::Running | AssetStatus::Queued => AssetStatus::Queued,
            other => other,
        },
        created_at: now.clone(),
        updated_at: now,
        ..asset
    };
    let to = project::modify(workspace_root, layout, to_id, |m| {
        if m.assets.iter().any(|a| a.id == copy.id) {
            return Err(CoreError::Invalid(format!(
                "{} already has an asset called {}.",
                m.name, copy.name
            )));
        }
        m.assets.push(copy.clone());
        Ok(())
    })?;
    Ok(CopiedAsset {
        asset: copy,
        to,
        refs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{AssetKind, NewAsset, Stage, add, add_version};
    use crate::project::{NewProject, ProjectTemplate};
    use std::path::PathBuf;

    fn new_project(root: &Path, layout: &FolderLayout, name: &str) -> PathBuf {
        let m = project::create(
            root,
            layout,
            NewProject {
                name: name.into(),
                template: ProjectTemplate::FloatingDiorama,
                target: Default::default(),
                game: None,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        project::dir(root, layout, &m.id).expect("dir")
    }

    /// Moon Island has a clay bowl with two versions, a render of v2 on its
    /// card, two refs of its own, a lid ref that is not its and a sent ref.
    fn fixture() -> (tempfile::TempDir, FolderLayout, PathBuf, PathBuf) {
        let root = tempfile::tempdir().expect("tempdir");
        let layout = FolderLayout::default();
        let moon = new_project(root.path(), &layout, "Moon Island");
        let landing = new_project(root.path(), &layout, "Maquette landing");
        for (file, bytes) in [
            ("refs/clay_bowl.png", "front"),
            ("refs/clay_bowl_02.png", "side"),
            ("refs/clay_bowl_lid.png", "lid"),
            ("refs/desk_04.png", "sent"),
            ("models/clay_bowl_v1.glb", "glb 1"),
            ("models/clay_bowl_v2.glb", "glb 2"),
            ("thumbs/clay_bowl_v2.png", "render 2"),
        ] {
            let path = moon.join(file);
            fs::create_dir_all(path.parent().expect("parent")).expect("dir");
            fs::write(path, bytes).expect("write");
        }
        project::modify(root.path(), &layout, "moon-island", |m| {
            let bowl = add(
                m,
                NewAsset {
                    name: "Clay bowl".into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Blender,
                },
            )?;
            add_version(m, &bowl.id, "models/clay_bowl_v1.glb".into(), None)?;
            add_version(m, &bowl.id, "models/clay_bowl_v2.glb".into(), None)?;
            m.assets[0].thumbnail = Some("thumbs/clay_bowl_v2.png".into());
            Ok(())
        })
        .expect("bowl");
        (root, layout, moon, landing)
    }

    #[test]
    fn ref_names_belong_to_the_asset_they_start_with() {
        assert!(is_ref_of("clay_bowl", "clay_bowl"));
        assert!(is_ref_of("Clay-Bowl_02", "clay_bowl"));
        assert!(!is_ref_of("clay_bowl_lid", "clay_bowl"));
        assert!(!is_ref_of("clay_bowls", "clay_bowl"));
        assert!(!is_ref_of("clay_bowl_", "clay_bowl"));
    }

    #[test]
    fn the_copy_brings_refs_versions_and_thumbnails_and_the_source_stays() {
        let (root, layout, moon, landing) = fixture();
        let copied = copy_to_project(
            root.path(),
            &layout,
            "moon-island",
            "clay-bowl",
            "maquette-landing",
            &["refs/desk_04.png".to_string()],
        )
        .expect("copy");
        assert_eq!(copied.asset.id, "clay-bowl");
        assert_eq!(copied.asset.versions.len(), 2);
        assert_eq!(copied.asset.versions[1].file, "models/clay_bowl_v2.glb");
        assert_eq!(
            copied.asset.thumbnail.as_deref(),
            Some("thumbs/clay_bowl_v2.png")
        );
        let landed: Vec<&str> = copied.refs.iter().map(|(_, n)| n.as_str()).collect();
        assert_eq!(
            landed,
            [
                "refs/desk_04.png",
                "refs/clay_bowl.png",
                "refs/clay_bowl_02.png"
            ]
        );
        assert!(!landing.join("refs/clay_bowl_lid.png").exists());
        assert_eq!(
            fs::read_to_string(landing.join("models/clay_bowl_v1.glb")).expect("v1"),
            "glb 1"
        );
        assert!(landing.join("thumbs/clay_bowl_v2.png").is_file());
        assert_eq!(copied.to.assets.len(), 1);

        let source = project::load(root.path(), &layout, "moon-island").expect("moon");
        assert_eq!(source.assets.len(), 1);
        assert!(moon.join("models/clay_bowl_v2.glb").is_file());
    }

    #[test]
    fn a_second_copy_gets_a_free_name_and_its_files_follow() {
        let (root, layout, _moon, landing) = fixture();
        let copy =
            |to: &str| copy_to_project(root.path(), &layout, "moon-island", "clay-bowl", to, &[]);
        copy("maquette-landing").expect("first");
        let second = copy("maquette-landing").expect("second");
        assert_eq!(second.asset.name, "Clay bowl 2");
        assert_eq!(second.asset.id, "clay-bowl-2");
        assert_eq!(second.asset.versions[0].file, "models/clay_bowl_2_v1.glb");
        assert_eq!(
            second.asset.thumbnail.as_deref(),
            Some("thumbs/clay_bowl_2_v2.png")
        );
        assert!(landing.join("models/clay_bowl_2_v2.glb").is_file());
        // The refs are the same bytes, so they are reused, not copied twice.
        assert!(!landing.join("refs/clay_bowl-2.png").exists());
        assert_eq!(second.to.assets.len(), 2);
    }

    #[test]
    fn bad_copies_change_nothing() {
        let (root, layout, moon, landing) = fixture();
        let same = copy_to_project(
            root.path(),
            &layout,
            "moon-island",
            "clay-bowl",
            "moon-island",
            &[],
        );
        assert!(matches!(same, Err(CoreError::Invalid(_))));
        let unknown = copy_to_project(
            root.path(),
            &layout,
            "moon-island",
            "nope",
            "maquette-landing",
            &[],
        );
        assert!(matches!(unknown, Err(CoreError::NotFound(_))));
        fs::remove_file(moon.join("models/clay_bowl_v1.glb")).expect("rm");
        let missing = copy_to_project(
            root.path(),
            &layout,
            "moon-island",
            "clay-bowl",
            "maquette-landing",
            &[],
        );
        assert!(matches!(missing, Err(CoreError::NotFound(_))));
        assert!(!landing.join("refs/clay_bowl.png").exists());
        let target = project::load(root.path(), &layout, "maquette-landing").expect("landing");
        assert!(target.assets.is_empty());
    }
}
