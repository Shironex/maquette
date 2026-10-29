//! Tests for taking Lab candidates into an asset's refs: a drawn view into
//! its slot, a clean copy in place of the ref it cleans.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::assets::{AssetKind, NewAsset, Stage};
use crate::concepts::refs_jobs::{self, RefsJobKind};
use crate::concepts::{add_image, load_direction};
use crate::project::{self, NewProject, ProjectTemplate};
use crate::refs::{AssetRef, set_views};

struct Fixture {
    root: tempfile::TempDir,
    layout: FolderLayout,
    project: String,
    dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("tempdir");
        let layout = FolderLayout::default();
        let manifest = project::create(
            root.path(),
            &layout,
            NewProject {
                name: "Frostveil".into(),
                template: ProjectTemplate::Room,
                target: Default::default(),
                game: None,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        let dir = project::dir(root.path(), &layout, &manifest.id).expect("dir");
        project::modify(root.path(), &layout, &manifest.id, |m| {
            assets::add(
                m,
                NewAsset {
                    name: "Frost lantern".into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Refs,
                },
            )
            .map(|_| ())
        })
        .expect("asset");
        Self {
            root,
            layout,
            project: manifest.id,
            dir,
        }
    }

    fn image(&self, rel: &str) -> String {
        let path = self.dir.join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        fs::write(&path, crate::inbox::images::write_png(4, 4, &[120; 48])).expect("png");
        rel.to_string()
    }

    fn asset(&self) -> Asset {
        let m = project::load(self.root.path(), &self.layout, &self.project).expect("load");
        assets::find(&m, "frost-lantern").expect("asset").clone()
    }

    fn set(&self, refs: &[(&str, RefView)]) -> CoreResult<ProjectManifest> {
        let refs: Vec<AssetRef> = refs.iter().map(|(f, v)| tagged(f, *v)).collect();
        set_views(
            self.root.path(),
            &self.layout,
            &self.project,
            "frost-lantern",
            &refs,
        )
    }
}

fn tagged(file: &str, view: RefView) -> AssetRef {
    AssetRef {
        file: file.to_string(),
        view,
    }
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

/// A views direction for the lantern with one candidate per title.
fn candidates(f: &Fixture, kind: RefsJobKind, source: &str, titles: &[&str]) -> Vec<String> {
    let asset = f.asset();
    let (direction, planned) =
        refs_jobs::prepare(&f.dir, &asset, kind, source, &strings(titles), None).expect("prep");
    planned
        .iter()
        .map(|p| {
            f.image(&p.file);
            add_image(&f.dir, &direction.id, p.index, &p.file, Some(&p.title)).expect("add");
            p.file.clone()
        })
        .collect()
}

#[test]
fn use_as_left_copies_the_candidate_into_the_left_slot() {
    let f = Fixture::new();
    let front = f.image("refs/frost_lantern.png");
    let made = candidates(&f, RefsJobKind::Views, &front, &["left", "left", "back"]);
    let manifest = use_view_image(
        f.root.path(),
        &f.layout,
        &f.project,
        "frost-lantern",
        &made[0],
        None,
        OldView::KeepAsOther,
    )
    .expect("use")
    .manifest;
    let asset = assets::find(&manifest, "frost-lantern").expect("asset");
    assert_eq!(
        asset.refs[0],
        tagged(&front, RefView::Front),
        "inferred, kept"
    );
    assert_eq!(asset.refs[1].view, RefView::Left);
    let copy = asset.refs[1].file.clone();
    assert_eq!(copy, "refs/frost_lantern_left.png");
    assert!(f.dir.join(&copy).is_file());
    // The second left takes the slot; the first becomes other.
    let again = use_view_image(
        f.root.path(),
        &f.layout,
        &f.project,
        "frost-lantern",
        &made[1],
        None,
        OldView::KeepAsOther,
    )
    .expect("again")
    .manifest;
    let asset = assets::find(&again, "frost-lantern").expect("asset");
    assert_eq!(asset.refs[1], tagged(&copy, RefView::Other));
    assert_eq!(asset.refs[2].view, RefView::Left);
    let d = load_direction(&f.dir, "frost-lantern-views")
        .expect("load")
        .expect("direction");
    assert_eq!(d.images.len(), 3, "accepting never deletes candidates");
    let front_again = use_view_image(
        f.root.path(),
        &f.layout,
        &f.project,
        "frost-lantern",
        &made[2],
        Some(RefView::Front),
        OldView::KeepAsOther,
    );
    assert!(front_again.is_err(), "the front is never replaced");
    let elsewhere = f.image("concepts/porch/porch_01.png");
    let foreign = use_view_image(
        f.root.path(),
        &f.layout,
        &f.project,
        "frost-lantern",
        &elsewhere,
        Some(RefView::Left),
        OldView::KeepAsOther,
    );
    assert!(foreign.is_err(), "only the asset's own candidates");
}

/// Uses `file` in its own view, replacing the ref that held it.
fn replace(f: &Fixture, file: &str, sent: &[String]) -> UsedView {
    use_view_image(
        f.root.path(),
        &f.layout,
        &f.project,
        "frost-lantern",
        file,
        None,
        OldView::Replace { sent },
    )
    .expect("replace")
}

#[test]
fn a_new_view_replaces_the_old_one_and_the_old_file_moves_aside() {
    let f = Fixture::new();
    let front = f.image("refs/frost_lantern.png");
    let left = f.image("refs/lantern_left_old.png");
    f.set(&[(&front, RefView::Front), (&left, RefView::Left)])
        .expect("set");
    let made = candidates(&f, RefsJobKind::Views, &front, &["left", "back"]);
    let used = replace(&f, &made[0], &[]);
    let asset = assets::find(&used.manifest, "frost-lantern").expect("asset");
    let copy = "refs/frost_lantern_left.png";
    assert_eq!(
        asset.refs,
        vec![tagged(&front, RefView::Front), tagged(copy, RefView::Left)],
        "the old left left the list"
    );
    let moved = "refs/originals/lantern_left_old.png";
    assert_eq!(
        used.replaced,
        Some(Replaced {
            file: left.clone(),
            view: RefView::Left,
            moved_to: Some(moved.to_string()),
            kept_because: None,
        })
    );
    assert!(!f.dir.join(&left).exists());
    assert!(f.dir.join(moved).is_file());
    let refs = views_of(
        &f.dir,
        &f.asset(),
        &suggested(&f.dir, &f.asset(), &ref_files(&f.dir)),
    );
    assert!(!refs.iter().any(|r| r.file == left), "{refs:?}");
    // An empty slot replaces nothing.
    let back = replace(&f, &made[1], &[]);
    assert_eq!(back.replaced, None);
}

#[test]
fn a_replaced_ref_another_asset_or_a_send_uses_stays_in_place() {
    let f = Fixture::new();
    let front = f.image("refs/frost_lantern.png");
    let left = f.image("refs/lantern_left_old.png");
    let back = f.image("refs/lantern_back_old.png");
    f.set(&[
        (&front, RefView::Front),
        (&left, RefView::Left),
        (&back, RefView::Back),
    ])
    .expect("set");
    project::modify(f.root.path(), &f.layout, &f.project, |m| {
        let porch = assets::add(
            m,
            NewAsset {
                name: "Porch lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )?
        .id;
        assets::find_mut(m, &porch)?.refs = vec![tagged(&left, RefView::Front)];
        Ok(())
    })
    .expect("porch");
    let made = candidates(&f, RefsJobKind::Views, &front, &["left", "back"]);
    let used = replace(&f, &made[0], &[]);
    let replaced = used.replaced.expect("replaced");
    assert_eq!(replaced.moved_to, None);
    assert_eq!(
        replaced.kept_because.as_deref(),
        Some("Porch lantern uses it too")
    );
    assert!(f.dir.join(&left).is_file(), "the porch keeps its front");
    let asset = assets::find(&used.manifest, "frost-lantern").expect("asset");
    assert!(!asset.refs.iter().any(|r| r.file == left));
    let porch = assets::find(&used.manifest, "porch-lantern").expect("porch");
    assert_eq!(porch.refs, vec![tagged(&left, RefView::Front)]);

    let used = replace(&f, &made[1], std::slice::from_ref(&back));
    let replaced = used.replaced.expect("replaced");
    assert_eq!(replaced.kept_because.as_deref(), Some("a send lists it"));
    assert!(f.dir.join(&back).is_file(), "the send still finds it");
    let asset = assets::find(&used.manifest, "frost-lantern").expect("asset");
    assert!(!asset.refs.iter().any(|r| r.file == back));
}

#[test]
fn the_clean_copy_keeps_the_view_and_the_original_moves_aside() {
    let f = Fixture::new();
    let front = f.image("refs/frost_lantern.png");
    let kit = f.image("refs/lantern_kit_crop.png");
    f.set(&[(&front, RefView::Front), (&kit, RefView::Left)])
        .expect("set");
    let made = candidates(&f, RefsJobKind::Clean, &kit, &[&kit]);
    let manifest = use_clean_copy(
        f.root.path(),
        &f.layout,
        &f.project,
        "frost-lantern",
        &made[0],
    )
    .expect("use");
    let asset = assets::find(&manifest, "frost-lantern").expect("asset");
    let clean = "refs/lantern_kit_crop_clean.png";
    assert!(
        asset.refs.contains(&tagged(clean, RefView::Left)),
        "{:?}",
        asset.refs
    );
    assert!(!asset.refs.iter().any(|r| r.file == kit));
    assert!(!f.dir.join(&kit).exists(), "the original left refs/");
    assert!(
        Path::new(&f.dir.join(ORIGINALS_DIR).join("lantern_kit_crop.png")).is_file(),
        "and is kept in refs/originals"
    );
    assert!(f.dir.join(clean).is_file());
    assert!(!ref_files(&f.dir).iter().any(|r| r.contains("originals")));
}
