//! Copy to project with what the asset took from the animation library:
//! its entry in `animations.json` (bone map, baked clips, last review) and
//! its accepted recipe bakes in `recipe-bakes.json`, carried to the copy
//! under its new id. Without them, the copy's clips would read as its own:
//! export packs would put a Mixamo bake's clips in, and Save clips to the
//! library would offer them.

use std::path::Path;

use super::retarget::uses;
use crate::assets::copy::{self, CopiedAsset};
use crate::error::{CoreError, CoreResult};
use crate::project;
use crate::recipes::baked;
use crate::workspace::FolderLayout;

/// Why a copy is refused when a record cannot be read.
fn unreadable(file: &str, name: &str, err: &CoreError) -> CoreError {
    CoreError::Invalid(format!(
        "{file} of {name} cannot be read, so the copy could lose which of its clips came from the library or a Mixamo bake. Fix or remove that file, then copy again. {err}"
    ))
}

/// Copies the asset `asset_id` of project `from_id` to project `to_id`, as
/// [`copy::copy_to_project`] does, with its animation records. Both records
/// are read before anything is copied, so one that cannot be read refuses
/// the copy and writes nothing, and written before the copy is listed, so
/// one that cannot be written leaves it out.
///
/// # Errors
///
/// As [`copy::copy_to_project`], and when a record cannot be read or
/// written.
pub fn copy_to_project(
    workspace_root: &Path,
    layout: &FolderLayout,
    from_id: &str,
    asset_id: &str,
    to_id: &str,
    sent: &[String],
) -> CoreResult<CopiedAsset> {
    let from_dir = project::dir(workspace_root, layout, from_id)?;
    let from = project::load(workspace_root, layout, from_id)?;
    let asset = crate::assets::find(&from, asset_id)?;
    let bakes = baked::of_asset(&from_dir, &asset.id)
        .map_err(|e| unreadable(baked::BAKES_FILE, &from.name, &e))?;
    let taken = uses::load(&from_dir)
        .map_err(|e| unreadable(uses::USES_FILE, &from.name, &e))?
        .assets
        .remove(&asset.id);
    let to_dir = project::dir(workspace_root, layout, to_id)?;
    // Written before the copy is listed, so it never shows without them,
    // and under the manifest's lock with the id checked free: entries for
    // that id are left over from a copy that was never listed, so they go
    // first, and go again when a write fails.
    copy::copy_to_project_then(
        workspace_root,
        layout,
        (from_id, asset_id, to_id),
        sent,
        |id| {
            let wrote = forget_records(&to_dir, id).and_then(|()| {
                baked::record_all(&to_dir, id, bakes)?;
                match taken {
                    Some(entry) => uses::update(&to_dir, id, |slot| *slot = entry),
                    None => Ok(()),
                }
            });
            if wrote.is_err() {
                let _ = forget_records(&to_dir, id);
            }
            wrote
        },
    )
}

/// Drops whatever the records hold for `id` in the project at `dir`.
fn forget_records(dir: &Path, id: &str) -> CoreResult<()> {
    baked::forget(dir, id)?;
    uses::forget(dir, id)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::animations::retarget::uses::AssetAnimations;
    use crate::assets::{AssetKind, NewAsset, Stage};
    use crate::project::{NewProject, ProjectTemplate};
    use crate::recipes::baked::RecipeBakeUse;

    struct Two {
        _root: tempfile::TempDir,
        root: std::path::PathBuf,
        layout: FolderLayout,
        from: String,
        to: String,
    }

    fn new_project(root: &Path, layout: &FolderLayout, name: &str) -> String {
        project::create(
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
        .expect("project")
        .id
    }

    /// Moon Island holds Elsheria with one model; Harbour is empty.
    fn two() -> Two {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        let layout = FolderLayout::default();
        let from = new_project(&root, &layout, "Moon Island");
        let to = new_project(&root, &layout, "Harbour");
        let from_dir = project::dir(&root, &layout, &from).expect("dir");
        fs::create_dir_all(from_dir.join("models")).expect("models");
        fs::write(from_dir.join("models/elsheria_v1.glb"), b"glb").expect("model");
        project::modify(&root, &layout, &from, |m| {
            crate::assets::add(
                m,
                NewAsset {
                    name: "Elsheria".into(),
                    kind: AssetKind::Character,
                    stage: Stage::Blender,
                },
            )?;
            crate::assets::add_version(m, "elsheria", "models/elsheria_v1.glb".into(), None)?;
            Ok(())
        })
        .expect("asset");
        Two {
            _root: dir,
            root,
            layout,
            from,
            to,
        }
    }

    fn bake(mixamo: bool) -> RecipeBakeUse {
        RecipeBakeUse {
            recipe: "wave".into(),
            recipe_version: "abc".into(),
            base_clip: "idle".into(),
            clip: "wave".into(),
            mixamo,
            version: 1,
        }
    }

    #[test]
    fn a_copy_carries_its_recipe_bakes_and_its_clip_uses_under_the_new_id() {
        let t = two();
        let from_dir = project::dir(&t.root, &t.layout, &t.from).expect("dir");
        let to_dir = project::dir(&t.root, &t.layout, &t.to).expect("dir");
        baked::record(&from_dir, "elsheria", bake(true)).expect("bake");
        uses::update(&from_dir, "elsheria", |e| {
            e.review_version = Some(1);
        })
        .expect("uses");
        // The target already has an Elsheria, so the copy gets another id.
        project::modify(&t.root, &t.layout, &t.to, |m| {
            crate::assets::add(
                m,
                NewAsset {
                    name: "Elsheria".into(),
                    kind: AssetKind::Character,
                    stage: Stage::Tripo,
                },
            )
            .map(|_| ())
        })
        .expect("taken");
        let copied =
            copy_to_project(&t.root, &t.layout, &t.from, "elsheria", &t.to, &[]).expect("copy");
        assert_eq!(copied.asset.id, "elsheria-2");
        assert_eq!(
            baked::of_asset(&to_dir, "elsheria-2").expect("read"),
            vec![bake(true)]
        );
        assert!(
            baked::took_mixamo(&to_dir, "elsheria-2"),
            "packs leave it out"
        );
        assert!(
            !baked::took_mixamo(&to_dir, "elsheria"),
            "not the other asset"
        );
        assert_eq!(
            uses::of_asset(&to_dir, "elsheria-2").expect("uses"),
            AssetAnimations {
                review_version: Some(1),
                ..AssetAnimations::default()
            }
        );
        assert_eq!(
            baked::of_asset(&from_dir, "elsheria").expect("read").len(),
            1,
            "the original keeps its record"
        );
    }

    #[test]
    fn an_asset_with_no_records_copies_without_writing_any() {
        let t = two();
        copy_to_project(&t.root, &t.layout, &t.from, "elsheria", &t.to, &[]).expect("copy");
        let to_dir = project::dir(&t.root, &t.layout, &t.to).expect("dir");
        assert!(!to_dir.join(baked::BAKES_FILE).exists());
        assert!(!to_dir.join(uses::USES_FILE).exists());
    }

    #[test]
    fn an_unreadable_record_refuses_the_copy_before_anything_is_copied() {
        let t = two();
        let from_dir = project::dir(&t.root, &t.layout, &t.from).expect("dir");
        fs::write(
            from_dir.join(baked::BAKES_FILE),
            r#"{"version": 99, "assets": {}}"#,
        )
        .expect("newer");
        let err = copy_to_project(&t.root, &t.layout, &t.from, "elsheria", &t.to, &[])
            .expect_err("refused");
        assert!(err.to_string().contains("recipe-bakes.json"), "{err}");
        let to = project::load(&t.root, &t.layout, &t.to).expect("to");
        assert!(to.assets.is_empty(), "nothing copied");
    }

    #[test]
    fn a_record_the_target_cannot_take_leaves_the_copy_unlisted() {
        let t = two();
        let from_dir = project::dir(&t.root, &t.layout, &t.from).expect("dir");
        let to_dir = project::dir(&t.root, &t.layout, &t.to).expect("dir");
        baked::record(&from_dir, "elsheria", bake(true)).expect("bake");
        // The target's record is a newer format this build cannot write over.
        fs::write(
            to_dir.join(baked::BAKES_FILE),
            r#"{"version": 99, "assets": {}}"#,
        )
        .expect("newer");
        let err = copy_to_project(&t.root, &t.layout, &t.from, "elsheria", &t.to, &[])
            .expect_err("refused");
        assert!(err.to_string().contains("newer Maquette"), "{err}");
        let to = project::load(&t.root, &t.layout, &t.to).expect("to");
        assert!(to.assets.is_empty(), "never listed without its bakes");
    }

    #[test]
    fn records_left_for_an_unlisted_id_give_way_to_the_copy() {
        let t = two();
        let from_dir = project::dir(&t.root, &t.layout, &t.from).expect("dir");
        let to_dir = project::dir(&t.root, &t.layout, &t.to).expect("dir");
        baked::record(&from_dir, "elsheria", bake(true)).expect("bake");
        // Left by a copy that was never listed, and a listed asset's own.
        let stale = RecipeBakeUse {
            recipe: "stale".into(),
            ..bake(false)
        };
        baked::record(&to_dir, "elsheria", stale).expect("stale");
        uses::update(&to_dir, "elsheria", |e| e.review_version = Some(7)).expect("uses");
        baked::record(&to_dir, "lantern", bake(false)).expect("other");
        let copied =
            copy_to_project(&t.root, &t.layout, &t.from, "elsheria", &t.to, &[]).expect("copy");
        assert_eq!(copied.asset.id, "elsheria");
        assert_eq!(
            baked::of_asset(&to_dir, "elsheria").expect("read"),
            vec![bake(true)],
            "only the copy's own"
        );
        assert_eq!(
            uses::of_asset(&to_dir, "elsheria").expect("uses"),
            AssetAnimations::default()
        );
        assert_eq!(
            baked::of_asset(&to_dir, "lantern").expect("read").len(),
            1,
            "another asset's records stay"
        );
    }
}
