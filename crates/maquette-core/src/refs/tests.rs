//! Tests for ref views: storage, the order an untagged asset reads, swaps,
//! send order, By hand names and taking Lab candidates into the refs.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::assets::{AssetKind, NewAsset, Stage};
use crate::concepts::refs_jobs::{self, RefsJobKind};
use crate::concepts::{add_image, load_direction};
use crate::project::{self, NewProject, ProjectTemplate};

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

#[test]
fn tags_save_with_the_asset_and_older_files_load_without_them() {
    let f = Fixture::new();
    assert!(f.asset().refs.is_empty(), "a new asset has no tags");
    let front = f.image("refs/frost_lantern.png");
    let side = f.image("refs/lantern_side.png");
    f.set(&[(&front, RefView::Front), (&side, RefView::Left)])
        .expect("set");
    assert_eq!(
        f.asset().refs,
        vec![tagged(&front, RefView::Front), tagged(&side, RefView::Left)]
    );
    let text = fs::read_to_string(f.dir.join("project.json")).expect("read");
    assert!(text.contains("\"view\": \"left\""), "{text}");

    let old: Asset = serde_json::from_str(
        r#"{"id":"a","name":"A","kind":"prop","stage":"refs","status":"queued",
            "versions":[],"thumbnail":null,"createdAt":"t","updatedAt":"t"}"#,
    )
    .expect("an older asset");
    assert!(old.refs.is_empty());
}

#[test]
fn an_untagged_asset_reads_its_views_from_the_order() {
    let f = Fixture::new();
    let files = strings(&[
        "refs/a.png",
        "refs/b.png",
        "refs/c.png",
        "refs/d.png",
        "refs/e.png",
    ]);
    let views: Vec<RefView> = inferred(&files).iter().map(|r| r.view).collect();
    assert_eq!(
        views,
        [
            RefView::Front,
            RefView::Left,
            RefView::Back,
            RefView::Right,
            RefView::Other
        ]
    );
    let asset = f.asset();
    let refs = views_of(&f.dir, &asset, &files);
    assert_eq!(slot_files(&refs), files[..4].to_vec(), "sends as before");
    let (sent, names) = send_order(&asset, &files[..2]);
    assert_eq!(sent, files[..2].to_vec());
    assert_eq!(names, strings(&["front", "left"]));
}

#[test]
fn tags_win_and_new_refs_join_as_other() {
    let f = Fixture::new();
    let a = f.image("refs/frost_lantern.png");
    let b = f.image("refs/frost_lantern_2.png");
    let c = f.image("refs/frost_lantern_3.png");
    f.set(&[(&b, RefView::Front), (&a, RefView::Back)])
        .expect("set");
    let refs = views_of(&f.dir, &f.asset(), &[a.clone(), b.clone(), c.clone()]);
    assert_eq!(
        refs,
        vec![
            tagged(&b, RefView::Front),
            tagged(&a, RefView::Back),
            tagged(&c, RefView::Other)
        ]
    );
    fs::remove_file(f.dir.join(&a)).expect("remove");
    let refs = views_of(&f.dir, &f.asset(), &[]);
    assert_eq!(refs, vec![tagged(&b, RefView::Front)], "a gone file drops");
}

#[test]
fn picking_a_taken_view_swaps_the_two() {
    let mut refs = vec![
        tagged("refs/a.png", RefView::Front),
        tagged("refs/b.png", RefView::Left),
        tagged("refs/c.png", RefView::Other),
    ];
    assign(&mut refs, "refs/b.png", RefView::Front);
    assert_eq!(refs[0].view, RefView::Left, "the old front takes b's view");
    assert_eq!(refs[1].view, RefView::Front);
    assign(&mut refs, "refs/c.png", RefView::Front);
    assert_eq!(refs[1].view, RefView::Other, "b takes c's other");
    assert_eq!(refs[2].view, RefView::Front);
    assign(&mut refs, "refs/new.png", RefView::Left);
    assert_eq!(
        refs[0].view,
        RefView::Other,
        "a new file leaves other behind"
    );
    assert_eq!(refs[3], tagged("refs/new.png", RefView::Left));
    assign(&mut refs, "refs/new.png", RefView::Other);
    assign(&mut refs, "refs/a.png", RefView::Other);
    assert_eq!(slot_files(&refs), strings(&["refs/c.png"]));
}

#[test]
fn a_set_of_tags_is_checked_before_it_saves() {
    let f = Fixture::new();
    let a = f.image("refs/a.png");
    let b = f.image("refs/b.png");
    let twice = f.set(&[(&a, RefView::Front), (&b, RefView::Front)]);
    assert!(
        twice
            .expect_err("two fronts")
            .to_string()
            .contains("both tagged Front")
    );
    assert!(f.set(&[(&a, RefView::Front), (&a, RefView::Left)]).is_err());
    assert!(f.set(&[("refs/missing.png", RefView::Front)]).is_err());
    assert!(f.set(&[("../escape.png", RefView::Front)]).is_err());
    let concept = f.image("concepts/x/x_01.png");
    assert!(f.set(&[(&concept, RefView::Front)]).is_err(), "refs only");
    assert!(f.asset().refs.is_empty(), "nothing saved");
    let windows = f
        .set(&[("refs\\a.png", RefView::Other), (&b, RefView::Other)])
        .expect("two others");
    let asset = assets::find(&windows, "frost-lantern").expect("asset");
    assert_eq!(asset.refs[0].file, a, "a Windows separator reads as /");
}

#[test]
fn a_send_orders_by_tag_and_leaves_other_refs_home() {
    let f = Fixture::new();
    let names = ["refs/r.png", "refs/f.png", "refs/o.png", "refs/b.png"];
    for n in names {
        f.image(n);
    }
    f.set(&[
        (names[0], RefView::Right),
        (names[1], RefView::Front),
        (names[2], RefView::Other),
        (names[3], RefView::Back),
    ])
    .expect("set");
    let asset = f.asset();
    let (sent, views) = send_order(&asset, &strings(&names));
    assert_eq!(sent, strings(&["refs/f.png", "refs/b.png", "refs/r.png"]));
    assert_eq!(views, strings(&["front", "back", "right"]));
    // Only other refs picked: they go as picked, the first as the front.
    let (sent, views) = send_order(&asset, &strings(&["refs/o.png"]));
    assert_eq!(sent, strings(&["refs/o.png"]));
    assert_eq!(views, strings(&["front"]));
}

#[test]
fn by_hand_files_and_tripo_inputs_are_named_from_the_tags() {
    use crate::providers::handoff::view_names;
    use crate::providers::presets::TripoPreset;
    use crate::providers::tripo::generation_request;
    let images = vec![PathBuf::from("refs/f.png"), PathBuf::from("refs/b.jpg")];
    let views = strings(&["front", "back"]);
    assert_eq!(
        view_names(&images, &views),
        strings(&["front.png", "back.jpg"])
    );
    assert_eq!(
        view_names(&images, &[]),
        strings(&["front.png", "left.jpg"])
    );
    let mut preset = TripoPreset::prop();
    preset.multi_view = true;
    let (path, body) = generation_request(&preset, &strings(&["t1", "t2"]), &views);
    assert_eq!(path, "generation/multiview-to-model");
    assert_eq!(body["inputs"][1]["back"], "t2");
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
