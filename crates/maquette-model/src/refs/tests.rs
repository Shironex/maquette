//! Tests for ref views: storage, the order an untagged asset reads, swaps
//! and send order.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::assets::{AssetKind, NewAsset, Stage};
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
        fs::write(&path, maquette_glb::images::write_png(4, 4, &[120; 48])).expect("png");
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
