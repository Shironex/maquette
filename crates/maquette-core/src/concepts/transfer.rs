//! Moving a Lab direction to another project of the workspace: its folder
//! (images, contact sheet, `direction.json` with the stars and picks), the
//! refs its picks were copied to and, when nothing else holds on to it, the
//! Board asset of the same name.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    CONCEPTS_DIR, DIRECTION_FILE, Direction, REFS_DIR, direction_dir, load_direction, write_lock,
};
use crate::assets::{self, Asset};
use crate::error::{CoreError, CoreResult};
use crate::fs::move_tree;
use crate::json::write_json;
use crate::project::{self, ProjectManifest, now_rfc3339};
use crate::workspace::FolderLayout;

/// What Move to project did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MovedDirection {
    /// The direction as it is now, in the other project. Its id changes
    /// only when that project already had one with the same id.
    pub direction: Direction,
    /// The project it left.
    pub from: ProjectManifest,
    /// The project it is in now.
    pub to: ProjectManifest,
    /// The Board asset that went along, when one did.
    pub asset: Option<Asset>,
    /// Its ref files in the other project's `refs/`, relative to that project.
    pub refs: Vec<String>,
}

/// A name in `dir` that is free: `name` itself, else `stem-2.ext`,
/// `stem-3.ext` and so on. A file with the same bytes counts as free, so a
/// ref that is already there is reused instead of copied twice.
fn free_name(dir: &Path, name: &str, same_as: &Path) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, Some(ext)),
        _ => (name, None),
    };
    let candidate = |n: u32| match (n, ext) {
        (1, _) => name.to_string(),
        (n, Some(ext)) => format!("{stem}-{n}.{ext}"),
        (n, None) => format!("{stem}-{n}"),
    };
    let same = |path: &Path| fs::read(path).ok() == fs::read(same_as).ok();
    (1u32..)
        .map(candidate)
        .find(|c| {
            let path = dir.join(c);
            !path.exists() || same(&path)
        })
        .unwrap_or_else(|| name.to_string())
}

fn free_direction_id(to_dir: &Path, id: &str) -> String {
    let taken = |c: &str| to_dir.join(CONCEPTS_DIR).join(c).exists();
    if !taken(id) {
        return id.to_string();
    }
    (2u32..)
        .map(|n| format!("{id}-{n}"))
        .find(|c| !taken(c))
        .unwrap_or_else(|| id.to_string())
}

/// Copies the project relative file `rel` from one project's `refs/` to the
/// other's and returns its new relative path. `None` when the file is gone.
fn carry_ref(from_dir: &Path, to_dir: &Path, rel: &str) -> CoreResult<Option<String>> {
    let source = from_dir.join(rel);
    if !source.is_file() {
        return Ok(None);
    }
    let Some(name) = source.file_name().map(|n| n.to_string_lossy().into_owned()) else {
        return Ok(None);
    };
    let refs = to_dir.join(REFS_DIR);
    fs::create_dir_all(&refs).map_err(|e| CoreError::io(&refs, e))?;
    let name = free_name(&refs, &name, &source);
    let target = refs.join(&name);
    if !target.exists() {
        fs::copy(&source, &target).map_err(|e| CoreError::io(&source, e))?;
    }
    Ok(Some(format!("{REFS_DIR}/{name}")))
}

/// True for a brief reference that points into the project's `refs/`.
fn is_project_ref(reference: &str) -> bool {
    reference.starts_with("refs/") || reference.starts_with("refs\\")
}

/// Moves the direction `direction_id` from project `from_id` to `to_id`.
///
/// The Board asset named like the direction moves along when it has no
/// model versions yet and `keep_asset` is false (the shell passes true
/// while a Send to 3D job still needs it where it is). The refs its picks
/// were copied to move with it then; otherwise they are copied, so the
/// asset left behind keeps its pictures. Brief references in `refs/` are
/// copied.
pub fn move_direction(
    workspace_root: &Path,
    layout: &FolderLayout,
    from_id: &str,
    direction_id: &str,
    to_id: &str,
    keep_asset: bool,
) -> CoreResult<MovedDirection> {
    if from_id == to_id {
        return Err(CoreError::Invalid(
            "The direction is already in this project.".to_string(),
        ));
    }
    let from_manifest = project::load(workspace_root, layout, from_id)?;
    project::load(workspace_root, layout, to_id)?;
    let from_dir = project::dir(workspace_root, layout, from_id)?;
    let to_dir = project::dir(workspace_root, layout, to_id)?;

    let _guard = write_lock();
    let mut direction = load_direction(&from_dir, direction_id)?
        .ok_or_else(|| CoreError::NotFound(format!("the direction {direction_id}")))?;
    let asset = assets::find_by_name(&from_manifest, &direction.name).cloned();
    let asset_moves = !keep_asset && asset.as_ref().is_some_and(|a| a.versions.is_empty());

    let new_id = free_direction_id(&to_dir, &direction.id);
    let source = direction_dir(&from_dir, &direction.id)?;
    let target = direction_dir(&to_dir, &new_id)?;
    let parent = to_dir.join(CONCEPTS_DIR);
    fs::create_dir_all(&parent).map_err(|e| CoreError::io(&parent, e))?;
    move_tree(&source, &target).map_err(|e| CoreError::io(&source, e))?;

    let carried = carry_files(&from_dir, &to_dir, &mut direction, &new_id);
    let written = carried
        .and_then(|refs| write_json(&target.join(DIRECTION_FILE), &direction).map(|()| refs));
    let (refs, renamed) = match written {
        Ok(found) => found,
        Err(err) => {
            // Put the folder back so the direction stays where it was.
            if let Err(back) = move_tree(&target, &source) {
                tracing::warn!(error = %back, "could not put a direction back");
            }
            return Err(err);
        }
    };

    let mut moved_asset = None;
    let (from, to) = if let (true, Some(asset)) = (asset_moves, asset) {
        let thumbnail = asset
            .thumbnail
            .as_ref()
            .map(|t| renamed_ref(&renamed, t).unwrap_or_else(|| t.clone()));
        let to = project::modify(workspace_root, layout, to_id, |m| {
            let placed = assets::adopt(
                m,
                Asset {
                    thumbnail,
                    ..asset.clone()
                },
            );
            moved_asset = Some(placed);
            Ok(())
        })?;
        let from = project::modify(workspace_root, layout, from_id, |m| {
            m.assets.retain(|a| a.id != asset.id);
            Ok(())
        })?;
        // The asset went along, so its refs did too: nothing here uses them.
        for (old, _) in &renamed {
            let path = from_dir.join(old);
            if let Err(err) = fs::remove_file(&path) {
                tracing::warn!(file = %path.display(), error = %err, "could not remove a moved ref");
            }
        }
        (from, to)
    } else {
        (
            project::load(workspace_root, layout, from_id)?,
            project::load(workspace_root, layout, to_id)?,
        )
    };
    Ok(MovedDirection {
        direction,
        from,
        to,
        asset: moved_asset,
        refs,
    })
}

type Renamed = Vec<(String, String)>;

fn renamed_ref(renamed: &Renamed, old: &str) -> Option<String> {
    renamed
        .iter()
        .find(|(o, _)| o == old)
        .map(|(_, n)| n.clone())
}

/// Points the direction at its new folder and copies its refs over.
/// Returns the refs now in the other project and each old to new path.
fn carry_files(
    from_dir: &Path,
    to_dir: &Path,
    direction: &mut Direction,
    new_id: &str,
) -> CoreResult<(Vec<String>, Renamed)> {
    let old_prefix = format!("{CONCEPTS_DIR}/{}/", direction.id);
    let new_prefix = format!("{CONCEPTS_DIR}/{new_id}/");
    let mut refs = Vec::new();
    let mut renamed: Renamed = Vec::new();
    for image in &mut direction.images {
        if let Some(rest) = image.file.strip_prefix(&old_prefix) {
            image.file = format!("{new_prefix}{rest}");
        }
        let Some(old) = image.ref_file.take() else {
            continue;
        };
        let new = match renamed_ref(&renamed, &old) {
            Some(new) => Some(new),
            None => carry_ref(from_dir, to_dir, &old)?,
        };
        if let Some(new) = &new {
            renamed.push((old, new.clone()));
            refs.push(new.clone());
        }
        image.ref_file = new;
    }
    let mut brief = Vec::new();
    for reference in &direction.refs {
        let kept = if is_project_ref(reference) {
            carry_ref(from_dir, to_dir, reference)?
        } else {
            Some(reference.clone())
        };
        brief.extend(kept);
    }
    direction.refs = brief;
    refs.dedup();
    direction.id = new_id.to_string();
    direction.updated_at = now_rfc3339();
    Ok((refs, renamed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{AssetKind, Stage};
    use crate::concepts::{
        Aspect, NewConcepts, add_image, list_directions, mark, prepare, send_picks,
    };
    use crate::project::{NewProject, ProjectTemplate};
    use std::path::PathBuf;

    struct Fixture {
        root: tempfile::TempDir,
        layout: FolderLayout,
        moon: PathBuf,
        landing: PathBuf,
    }

    fn new_project(root: &Path, layout: &FolderLayout, name: &str) -> PathBuf {
        let m = project::create(
            root,
            layout,
            NewProject {
                name: name.into(),
                template: ProjectTemplate::FloatingDiorama,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        project::dir(root, layout, &m.id).expect("dir")
    }

    /// Writes a direction of `count` images into `project_dir`; image 1 is
    /// starred and the given ones are picked.
    fn direction(project_dir: &Path, name: &str, count: u32, picks: &[usize], refs: Vec<String>) {
        let request = NewConcepts {
            project: String::new(),
            direction: name.into(),
            prompt: "A desk with a laptop showing the site".into(),
            refs,
            aspect: Aspect::Wide,
            count,
        };
        let (d, files) = prepare(project_dir, &request).expect("prepare");
        for (n, (index, file)) in files.iter().enumerate() {
            fs::write(project_dir.join(file), format!("png {name} {index}")).expect("png");
            add_image(project_dir, &d.id, *index, file, None).expect("add");
            if picks.contains(&n) {
                mark(project_dir, &d.id, file, None, Some(true)).expect("pick");
            }
        }
        mark(project_dir, &d.id, &files[0].1, Some(true), None).expect("star");
    }

    /// Moon Island holds the "Maquette landing studio" direction, its pick
    /// sent to Refs; "Maquette landing" is the new, empty project.
    fn fixture() -> Fixture {
        let root = tempfile::tempdir().expect("tempdir");
        let layout = FolderLayout::default();
        let moon = new_project(root.path(), &layout, "Moon Island");
        let landing = new_project(root.path(), &layout, "Maquette landing");
        fs::write(moon.join("refs/brief.png"), b"brief").expect("brief ref");
        direction(
            &moon,
            "Maquette landing studio",
            4,
            &[3],
            vec!["refs/brief.png".into()],
        );
        send_picks(
            root.path(),
            &layout,
            "moon-island",
            "maquette-landing-studio",
            AssetKind::Scene,
        )
        .expect("send");
        Fixture {
            root,
            layout,
            moon,
            landing,
        }
    }

    #[test]
    fn the_landing_studio_moves_with_its_picks_refs_and_board_card() {
        let f = fixture();
        let moved = move_direction(
            f.root.path(),
            &f.layout,
            "moon-island",
            "maquette-landing-studio",
            "maquette-landing",
            false,
        )
        .expect("move");

        assert!(!f.moon.join("concepts/maquette-landing-studio").exists());
        assert!(!f.moon.join("refs/maquette-landing-studio_04.png").exists());
        assert!(
            f.moon.join("refs/brief.png").is_file(),
            "brief refs are copied"
        );
        let folder = f.landing.join("concepts/maquette-landing-studio");
        assert!(folder.join(DIRECTION_FILE).is_file());

        let d = &moved.direction;
        assert_eq!(d.id, "maquette-landing-studio");
        assert_eq!(d.images.len(), 4);
        for image in &d.images {
            assert!(f.landing.join(&image.file).is_file(), "{}", image.file);
        }
        assert!(d.images[0].starred && !d.images[0].picked);
        assert!(d.images[3].picked);
        assert_eq!(
            d.images[3].ref_file.as_deref(),
            Some("refs/maquette-landing-studio_04.png")
        );
        assert_eq!(d.refs, vec!["refs/brief.png".to_string()]);
        assert!(
            f.landing
                .join("refs/maquette-landing-studio_04.png")
                .is_file()
        );
        assert!(f.landing.join("refs/brief.png").is_file());
        assert_eq!(moved.refs, vec!["refs/maquette-landing-studio_04.png"]);

        // The Lab of each project reads it from its new place.
        assert!(list_directions(&f.moon).is_empty());
        assert_eq!(list_directions(&f.landing), vec![d.clone()]);

        let asset = moved.asset.expect("asset moved");
        assert_eq!(asset.id, "maquette-landing-studio");
        assert_eq!(asset.stage, Stage::Refs);
        assert_eq!(
            asset.thumbnail.as_deref(),
            Some("refs/maquette-landing-studio_04.png")
        );
        assert!(moved.from.assets.is_empty());
        assert_eq!(moved.to.assets, vec![asset]);
        let saved = project::load(f.root.path(), &f.layout, "maquette-landing").expect("load");
        assert_eq!(saved.assets.len(), 1);
    }

    #[test]
    fn an_asset_a_send_still_needs_stays_and_keeps_its_refs() {
        let f = fixture();
        let moved = move_direction(
            f.root.path(),
            &f.layout,
            "moon-island",
            "maquette-landing-studio",
            "maquette-landing",
            true,
        )
        .expect("move");
        assert!(moved.asset.is_none());
        assert_eq!(moved.from.assets.len(), 1);
        assert!(moved.to.assets.is_empty());
        assert!(f.moon.join("refs/maquette-landing-studio_04.png").is_file());
        assert!(
            f.landing
                .join("refs/maquette-landing-studio_04.png")
                .is_file()
        );
    }

    #[test]
    fn an_asset_with_models_stays_where_its_versions_are() {
        let f = fixture();
        project::modify(f.root.path(), &f.layout, "moon-island", |m| {
            assets::add_version(
                m,
                "maquette-landing-studio",
                "models/studio_v1.glb".into(),
                None,
            )
            .map(|_| ())
        })
        .expect("version");
        let moved = move_direction(
            f.root.path(),
            &f.layout,
            "moon-island",
            "maquette-landing-studio",
            "maquette-landing",
            false,
        )
        .expect("move");
        assert!(moved.asset.is_none());
        assert_eq!(moved.from.assets[0].versions.len(), 1);
    }

    #[test]
    fn a_taken_id_gets_a_suffix_and_the_paths_follow() {
        let f = fixture();
        direction(&f.landing, "Maquette landing studio", 1, &[], vec![]);
        // A different ref file under the same name is already there.
        fs::write(
            f.landing.join("refs/maquette-landing-studio_04.png"),
            b"other",
        )
        .expect("ref");
        let moved = move_direction(
            f.root.path(),
            &f.layout,
            "moon-island",
            "maquette-landing-studio",
            "maquette-landing",
            false,
        )
        .expect("move");
        let d = &moved.direction;
        assert_eq!(d.id, "maquette-landing-studio-2");
        assert_eq!(
            d.images[0].file,
            "concepts/maquette-landing-studio-2/maquette-landing-studio_01.png"
        );
        for image in &d.images {
            assert!(f.landing.join(&image.file).is_file(), "{}", image.file);
        }
        assert_eq!(
            d.images[3].ref_file.as_deref(),
            Some("refs/maquette-landing-studio_04-2.png")
        );
        assert_eq!(
            fs::read(f.landing.join("refs/maquette-landing-studio_04.png")).expect("read"),
            b"other"
        );
        assert_eq!(
            moved.asset.expect("asset").thumbnail.as_deref(),
            Some("refs/maquette-landing-studio_04-2.png")
        );
        assert_eq!(list_directions(&f.landing).len(), 2);
    }

    #[test]
    fn bad_moves_change_nothing() {
        let f = fixture();
        let move_to = |to: &str, direction: &str| {
            move_direction(
                f.root.path(),
                &f.layout,
                "moon-island",
                direction,
                to,
                false,
            )
        };
        assert_eq!(
            move_to("moon-island", "maquette-landing-studio")
                .expect_err("same")
                .code(),
            "invalid"
        );
        assert_eq!(
            move_to("nope", "maquette-landing-studio")
                .expect_err("no project")
                .code(),
            "not-found"
        );
        assert_eq!(
            move_to("maquette-landing", "nope")
                .expect_err("no direction")
                .code(),
            "not-found"
        );
        assert!(move_to("maquette-landing", "../x").is_err());
        assert_eq!(list_directions(&f.moon).len(), 1);
        assert!(list_directions(&f.landing).is_empty());
    }
}
