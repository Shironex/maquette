//! Tests for moving a Lab direction to another project.

use super::*;
use crate::assets::{AssetKind, Stage};
use crate::concepts::{Aspect, NewConcepts, add_image, list_directions, mark, prepare, send_picks};
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
