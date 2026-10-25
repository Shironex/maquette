//! Tests for the Concept lab: directions, marks and picks sent to refs.

use super::*;
use crate::assets::{self, NewAsset, Stage};
use crate::project;
use crate::workspace::FolderLayout;

fn request(direction: &str, count: u32, refs: Vec<String>) -> NewConcepts {
    NewConcepts {
        project: "moon".into(),
        direction: direction.into(),
        prompt: "  A cosy porch at dusk, lanterns lit.  ".into(),
        refs,
        aspect: Aspect::Wide,
        count,
    }
}

#[test]
fn the_prompt_starts_and_ends_as_codex_needs() {
    let new = request("Porch at dusk", 3, vec!["refs/a.png".into()]);
    let out = Path::new("/w/projects/moon/concepts/porch-at-dusk/porch-at-dusk_02.png");
    let text = prompt(&new, 2, out);
    assert!(text.starts_with("Generate an image.\n\nA cosy porch at dusk, lanterns lit.\n"));
    assert!(text.contains("16:9 landscape"));
    assert!(text.contains("image 2 of 3"));
    assert!(text.contains("references"));
    assert!(text.contains("~/.codex/generated_images/"));
    assert!(text.trim_end().ends_with(&out.display().to_string()));
}

#[test]
fn refs_come_before_the_end_of_options_and_the_stdin_dash() {
    let args = codex_args(
        Path::new("/w/moon"),
        &["refs/a.png".into(), "/abs/b.png".into()],
    );
    assert_eq!(
        args,
        [
            "exec",
            "--skip-git-repo-check",
            "-s",
            "workspace-write",
            "-i",
            "/w/moon/refs/a.png",
            "-i",
            "/abs/b.png",
            "--",
            "-"
        ]
    );
    let bare = codex_args(Path::new("/w/moon"), &[]);
    assert_eq!(bare.last().map(String::as_str), Some("-"));
}

#[test]
fn requests_are_checked_and_cleaned() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("refs")).expect("mkdir");
    fs::write(dir.path().join("refs/a.png"), b"png").expect("ref");
    let ok = clean_request(
        dir.path(),
        &request(" Porch ", 2, vec!["refs/a.png".into(), "refs/a.png".into()]),
    )
    .expect("clean");
    assert_eq!(ok.direction, "Porch");
    assert_eq!(ok.prompt, "A cosy porch at dusk, lanterns lit.");
    assert_eq!(ok.refs, vec!["refs/a.png".to_string()]);
    assert!(clean_request(dir.path(), &request("", 2, vec![])).is_err());
    assert!(clean_request(dir.path(), &request("P", 0, vec![])).is_err());
    assert!(clean_request(dir.path(), &request("P", 13, vec![])).is_err());
    assert!(
        clean_request(
            dir.path(),
            &request("P", 1, vec!["refs/missing.png".into()])
        )
        .is_err()
    );
}

#[test]
fn directions_keep_their_images_marks_and_numbering() {
    let dir = tempfile::tempdir().expect("tempdir");
    let new = request("Porch at dusk", 2, vec![]);
    let (direction, files) = prepare(dir.path(), &new).expect("prepare");
    assert_eq!(direction.id, "porch-at-dusk");
    assert_eq!(
        files,
        vec![
            (1, "concepts/porch-at-dusk/porch-at-dusk_01.png".to_string()),
            (2, "concepts/porch-at-dusk/porch-at-dusk_02.png".to_string()),
        ]
    );
    let png = crate::inbox::images::write_png(4, 2, &[200; 4 * 2 * 3]);
    fs::write(dir.path().join(&files[0].1), &png).expect("png");
    let d = add_image(dir.path(), "porch-at-dusk", 1, &files[0].1, None).expect("add");
    assert_eq!(d.images.len(), 1);
    assert_eq!((d.images[0].width, d.images[0].height), (Some(4), Some(2)));
    // Job 2 is still out, its file not there yet; a second batch must
    // not reuse index 2 once its file lands.
    fs::write(dir.path().join(&files[1].1), &png).expect("png");
    let (_, more) = prepare(dir.path(), &request("Porch at dusk", 1, vec![])).expect("more");
    assert_eq!(more[0].0, 3);
    let d = mark(dir.path(), "porch-at-dusk", &files[0].1, Some(true), None).expect("star");
    assert!(d.images[0].starred && !d.images[0].picked);
    let d = mark(dir.path(), "porch-at-dusk", &files[0].1, None, Some(true)).expect("pick");
    assert!(d.images[0].starred && d.images[0].picked);
    assert!(mark(dir.path(), "porch-at-dusk", "nope.png", Some(true), None).is_err());
    let listed = list_directions(dir.path());
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "Porch at dusk");
    assert!(direction_dir(dir.path(), "../up").is_err());
}

#[test]
fn send_picks_copies_into_refs_and_advances_the_asset() {
    let root = tempfile::tempdir().expect("tempdir");
    let layout = FolderLayout::default();
    let manifest = project::create(
        root.path(),
        &layout,
        crate::project::NewProject {
            name: "Moon".into(),
            template: crate::project::ProjectTemplate::Room,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(root.path(), &layout, &manifest.id).expect("dir");
    let (_, files) = prepare(&project_dir, &request("Stone lantern", 2, vec![])).expect("prep");
    let png = crate::inbox::images::write_png(2, 2, &[90; 12]);
    for (index, file) in &files {
        fs::write(project_dir.join(file), &png).expect("png");
        add_image(&project_dir, "stone-lantern", *index, file, None).expect("add");
    }
    let none = send_picks(
        root.path(),
        &layout,
        &manifest.id,
        "stone-lantern",
        AssetKind::Prop,
    );
    assert!(none.is_err(), "nothing picked yet");
    mark(&project_dir, "stone-lantern", &files[1].1, None, Some(true)).expect("pick");
    project::modify(root.path(), &layout, &manifest.id, |m| {
        assets::add(
            m,
            NewAsset {
                name: "Stone lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Concept,
            },
        )
        .map(|_| ())
    })
    .expect("asset");
    let sent = send_picks(
        root.path(),
        &layout,
        &manifest.id,
        "stone-lantern",
        AssetKind::Prop,
    )
    .expect("send");
    assert_eq!(sent.copied, vec!["refs/stone-lantern_02.png".to_string()]);
    assert!(project_dir.join("refs/stone-lantern_02.png").is_file());
    assert_eq!(sent.asset.stage, Stage::Refs);
    assert_eq!(
        sent.direction.images[1].ref_file.as_deref(),
        Some("refs/stone-lantern_02.png")
    );
    // A second send keeps the one asset.
    let again = send_picks(
        root.path(),
        &layout,
        &manifest.id,
        "stone-lantern",
        AssetKind::Prop,
    )
    .expect("again");
    assert_eq!(again.asset.id, sent.asset.id);
    let m = project::load(root.path(), &layout, &manifest.id).expect("load");
    assert_eq!(m.assets.len(), 1);
}

/// A project with a "Stone lantern" direction of two images on disk.
fn lantern_project() -> (
    tempfile::TempDir,
    FolderLayout,
    String,
    PathBuf,
    Vec<String>,
) {
    let root = tempfile::tempdir().expect("tempdir");
    let layout = FolderLayout::default();
    let manifest = project::create(
        root.path(),
        &layout,
        crate::project::NewProject {
            name: "Moon".into(),
            template: crate::project::ProjectTemplate::Room,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(root.path(), &layout, &manifest.id).expect("dir");
    let (_, files) = prepare(&project_dir, &request("Stone lantern", 2, vec![])).expect("prep");
    let png = crate::inbox::images::write_png(2, 2, &[90; 12]);
    for (index, file) in &files {
        fs::write(project_dir.join(file), &png).expect("png");
        add_image(&project_dir, "stone-lantern", *index, file, None).expect("add");
    }
    let files = files.into_iter().map(|(_, f)| f).collect();
    (root, layout, manifest.id, project_dir, files)
}

#[test]
fn send_picks_shows_the_first_pick_on_the_card_but_never_over_a_render() {
    let (root, layout, id, project_dir, files) = lantern_project();
    for file in &files {
        mark(&project_dir, "stone-lantern", file, None, Some(true)).expect("pick");
    }
    let sent =
        send_picks(root.path(), &layout, &id, "stone-lantern", AssetKind::Prop).expect("send");
    assert_eq!(
        sent.asset.thumbnail.as_deref(),
        Some("refs/stone-lantern_01.png")
    );
    assert_eq!(
        sent.project.assets[0].thumbnail.as_deref(),
        Some("refs/stone-lantern_01.png")
    );

    // A model render on the card stays when the picks go again.
    project::modify(root.path(), &layout, &id, |m| {
        m.assets[0].thumbnail = Some("thumbs/stone_lantern_v1.png".into());
        Ok(())
    })
    .expect("render");
    let again =
        send_picks(root.path(), &layout, &id, "stone-lantern", AssetKind::Prop).expect("again");
    assert_eq!(
        again.asset.thumbnail.as_deref(),
        Some("thumbs/stone_lantern_v1.png")
    );
}

#[test]
fn heal_gives_a_stuck_refs_card_its_newest_ref() {
    let (root, layout, id, project_dir, files) = lantern_project();
    for file in &files {
        mark(&project_dir, "stone-lantern", file, None, Some(true)).expect("pick");
    }
    send_picks(root.path(), &layout, &id, "stone-lantern", AssetKind::Prop).expect("send");
    // What an older build left behind: the asset at Refs without a picture.
    project::modify(root.path(), &layout, &id, |m| {
        m.assets[0].thumbnail = None;
        Ok(())
    })
    .expect("stuck");
    // The first ref copy went missing, so the newest one that exists is used.
    fs::remove_file(project_dir.join("refs/stone-lantern_01.png")).expect("rm");

    let healed = heal_ref_thumbnails(root.path(), &layout, &id)
        .expect("heal")
        .expect("changed");
    assert_eq!(
        healed.assets[0].thumbnail.as_deref(),
        Some("refs/stone-lantern_02.png")
    );
    let saved = project::load(root.path(), &layout, &id).expect("load");
    assert_eq!(
        saved.assets[0].thumbnail.as_deref(),
        Some("refs/stone-lantern_02.png")
    );
    assert!(
        heal_ref_thumbnails(root.path(), &layout, &id)
            .expect("again")
            .is_none(),
        "nothing left to heal"
    );
}

#[test]
fn heal_leaves_assets_with_models_or_without_sent_refs_alone() {
    let (root, layout, id, _, _) = lantern_project();
    project::modify(root.path(), &layout, &id, |m| {
        let a = assets::add(
            m,
            NewAsset {
                name: "Stone lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )?;
        assets::add_version(m, &a.id, "models/stone_lantern_v1.glb".into(), None)?;
        assets::add(
            m,
            NewAsset {
                name: "Torii".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )
        .map(|_| ())
    })
    .expect("assets");
    assert!(
        heal_ref_thumbnails(root.path(), &layout, &id)
            .expect("heal")
            .is_none()
    );
}
