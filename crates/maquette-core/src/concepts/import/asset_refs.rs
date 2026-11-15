//! Images someone already has, added straight to one asset's refs: copied
//! into `refs/`, never linked, and tagged with the view each shows. The
//! asset is found by id, or by name and made at Refs when there is none.

use std::path::{Path, PathBuf};

use super::refs::import_ref_files;
use crate::assets::{self, Asset, AssetKind, Stage};
use crate::concepts::picks::place_at_refs;
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest};
use crate::refs::{self, AssetRef, RefView};
use crate::workspace::FolderLayout;

/// One image to add, with the view it shows when that is known.
#[derive(Debug, Clone)]
pub struct NewRef {
    pub path: PathBuf,
    pub view: Option<RefView>,
}

/// The asset the images are for.
#[derive(Debug, Clone)]
pub enum RefsFor {
    /// An asset of the project, by id.
    Asset(String),
    /// The asset of this name, or a new one of `kind` at Refs.
    Named { name: String, kind: AssetKind },
}

/// What adding the images did.
#[derive(Debug, Clone)]
pub struct AddedRefs {
    pub asset: Asset,
    /// Each image's ref, relative to the project folder, in the order given.
    pub files: Vec<String>,
    /// True when the asset was made by this call.
    pub created: bool,
    pub project: ProjectManifest,
}

/// Checks what can be checked before a file is copied: the asset, its name
/// and that no two images ask for the same view slot.
fn check_first(manifest: &ProjectManifest, target: &RefsFor, images: &[NewRef]) -> CoreResult<()> {
    match target {
        RefsFor::Asset(id) => {
            assets::find(manifest, id)?;
        }
        RefsFor::Named { name, .. } => {
            assets::clean_name(name)?;
        }
    }
    for (i, image) in images.iter().enumerate() {
        let Some(view) = image.view.filter(|v| v.is_slot()) else {
            continue;
        };
        if images[..i].iter().any(|o| o.view == Some(view)) {
            return Err(CoreError::Invalid(format!(
                "Two images are tagged {}. A view holds one ref.",
                view.label()
            )));
        }
    }
    Ok(())
}

/// The asset's refs with `added` laid in: a named view takes its slot (the
/// ref that held it stays as other), an image already a ref keeps its view
/// unless one is named, and the rest join as other. When the asset still has
/// no front, the first image given without a view becomes it.
fn merged(mut out: Vec<AssetRef>, added: &[(String, Option<RefView>)]) -> Vec<AssetRef> {
    for (file, view) in added {
        if let Some(view) = view.filter(|v| v.is_slot()) {
            for r in out.iter_mut().filter(|r| r.view == view && &r.file != file) {
                r.view = RefView::Other;
            }
        }
        match out.iter_mut().find(|r| &r.file == file) {
            Some(r) => {
                if let Some(view) = view {
                    r.view = *view;
                }
            }
            None => out.push(AssetRef {
                file: file.clone(),
                view: view.unwrap_or(RefView::Other),
            }),
        }
    }
    if !out.iter().any(|r| r.view == RefView::Front)
        && let Some((first, _)) = added.iter().find(|(_, v)| v.is_none())
        && let Some(r) = out
            .iter_mut()
            .find(|r| &r.file == first && r.view == RefView::Other)
    {
        r.view = RefView::Front;
    }
    out
}

/// Copies `images` into the project's `refs/` and makes them refs of the
/// asset `target` names, which ends at Refs or later: one still at Concept
/// moves up, and a name no asset has makes a new one. Every image is checked
/// before anything is written; an image already there with the same bytes
/// is reused, never replaced. The card shows the front when it shows no
/// model yet.
///
/// # Errors
///
/// Fails for an unknown project or asset, a bad name, two images tagged
/// with one view, or an image the import refuses (type, size, count).
pub fn add_refs(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    target: &RefsFor,
    images: &[NewRef],
) -> CoreResult<AddedRefs> {
    let manifest = project::load(workspace_root, layout, project_id)?;
    check_first(&manifest, target, images)?;
    let project_dir = project::dir(workspace_root, layout, project_id)?;
    let paths: Vec<PathBuf> = images.iter().map(|i| i.path.clone()).collect();
    let files = import_ref_files(&project_dir, &paths)?;
    let added: Vec<(String, Option<RefView>)> = files
        .iter()
        .cloned()
        .zip(images.iter().map(|i| i.view))
        .collect();
    let mut placed: Option<(String, bool)> = None;
    let manifest = project::modify(workspace_root, layout, project_id, |manifest| {
        let (asset, created) = match target {
            RefsFor::Asset(id) => {
                let asset = assets::find(manifest, id)?.clone();
                let asset = if asset.stage < Stage::Refs {
                    assets::move_to(manifest, &asset.id, Stage::Refs)?
                } else {
                    asset
                };
                (asset, false)
            }
            RefsFor::Named { name, kind } => {
                let created = assets::find_by_name(manifest, name).is_none();
                (place_at_refs(manifest, name, *kind)?, created)
            }
        };
        let ref_files = refs::ref_files(&project_dir);
        let now = refs::views_of(
            &project_dir,
            &asset,
            &refs::suggested(&project_dir, &asset, &ref_files),
        );
        let tags = refs::check(&project_dir, &merged(now, &added))?;
        let picture = tags
            .iter()
            .find(|r| r.view == RefView::Front)
            .or_else(|| tags.first())
            .map(|r| r.file.clone());
        let found = assets::find_mut(manifest, &asset.id)?;
        found.refs = tags;
        found.updated_at = project::now_rfc3339();
        if let Some(picture) = picture {
            assets::set_ref_thumbnail(manifest, &asset.id, &picture)?;
        }
        placed = Some((asset.id, created));
        Ok(())
    })?;
    let (id, created) =
        placed.ok_or_else(|| CoreError::Unavailable("The asset was not saved.".into()))?;
    Ok(AddedRefs {
        asset: assets::find(&manifest, &id)?.clone(),
        files,
        created,
        project: manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::merged;
    use crate::refs::{AssetRef, RefView};

    fn r(file: &str, view: RefView) -> AssetRef {
        AssetRef {
            file: file.into(),
            view,
        }
    }

    #[test]
    fn the_first_image_without_a_view_becomes_the_front_of_an_asset_with_none() {
        let out = merged(
            vec![],
            &[("refs/a.png".into(), None), ("refs/b.png".into(), None)],
        );
        assert_eq!(
            out,
            vec![
                r("refs/a.png", RefView::Front),
                r("refs/b.png", RefView::Other)
            ]
        );
    }

    #[test]
    fn a_named_view_takes_its_slot_and_the_old_ref_stays_as_other() {
        let out = merged(
            vec![r("refs/old.png", RefView::Front)],
            &[("refs/new.png".into(), Some(RefView::Front))],
        );
        assert_eq!(
            out,
            vec![
                r("refs/old.png", RefView::Other),
                r("refs/new.png", RefView::Front)
            ]
        );
    }

    #[test]
    fn an_image_already_a_ref_keeps_its_view_unless_one_is_named() {
        let now = vec![r("refs/a.png", RefView::Left)];
        assert_eq!(merged(now.clone(), &[("refs/a.png".into(), None)]), now);
        assert_eq!(
            merged(now, &[("refs/a.png".into(), Some(RefView::Back))]),
            vec![r("refs/a.png", RefView::Back)]
        );
    }
}
