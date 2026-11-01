use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::concepts::{mark, send_picks};
use crate::project::{NewProject, ProjectTemplate};

struct Fixture {
    root: tempfile::TempDir,
    drops: tempfile::TempDir,
    layout: FolderLayout,
    id: String,
    dir: PathBuf,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().expect("tempdir");
    let layout = FolderLayout::default();
    let manifest = project::create(
        root.path(),
        &layout,
        NewProject {
            name: "Moon".into(),
            template: ProjectTemplate::Room,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let dir = project::dir(root.path(), &layout, &manifest.id).expect("dir");
    Fixture {
        root,
        drops: tempfile::tempdir().expect("tempdir"),
        layout,
        id: manifest.id,
        dir,
    }
}

impl Fixture {
    fn image(&self, name: &str) -> PathBuf {
        let path = self.drops.path().join(name);
        fs::write(&path, crate::inbox::images::write_png(2, 3, &[90; 18])).expect("png");
        path
    }

    fn import(&self, paths: &[PathBuf]) -> CoreResult<ImportedConcepts> {
        import_images(self.root.path(), &self.layout, &self.id, paths)
    }
}

fn card_stage(imported: &ImportedConcepts, id: &str) -> Stage {
    assets::find(&imported.project, id).expect("card").stage
}

#[test]
fn images_become_a_direction_and_a_card_at_concept() {
    let f = fixture();
    let done = f
        .import(&[f.image("stone_lantern.png"), f.image("other.png")])
        .expect("import");
    assert_eq!(done.direction.name, "Stone lantern");
    assert_eq!(done.direction.images.len(), 2);
    assert_eq!(done.direction.images[0].width, Some(2));
    assert_eq!(done.direction.images[0].height, Some(3));
    for file in &done.added {
        assert!(f.dir.join(file).is_file(), "{file}");
    }
    assert_eq!(card_stage(&done, "stone-lantern"), Stage::Concept);
    assert_eq!(done.asset.thumbnail.as_ref(), Some(&done.added[0]));
    assert!(
        load_direction(&f.dir, "stone-lantern")
            .expect("read")
            .is_some(),
        "the Lab lists it"
    );
}

#[test]
fn a_second_drop_of_the_same_name_adds_to_it_without_overwriting() {
    let f = fixture();
    let first = f.import(&[f.image("lantern.png")]).expect("first");
    let picture = fs::read(f.dir.join(&first.added[0])).expect("read");
    let second = f.import(&[f.image("lantern.png")]).expect("second");
    assert_ne!(first.added, second.added);
    assert_eq!(second.direction.images.len(), 2);
    assert_eq!(
        fs::read(f.dir.join(&first.added[0])).expect("read"),
        picture
    );
    assert_eq!(
        second
            .project
            .assets
            .iter()
            .filter(|a| a.id == "lantern")
            .count(),
        1
    );
}

#[test]
fn it_keeps_the_brief_of_a_direction_of_that_name() {
    let f = fixture();
    let new = crate::concepts::NewConcepts {
        project: f.id.clone(),
        direction: "Lantern".into(),
        prompt: "A lantern.".into(),
        refs: vec![],
        aspect: crate::concepts::Aspect::Wide,
        count: 1,
        use_style: true,
    };
    crate::concepts::prepare(&f.dir, &new).expect("prepare");
    let done = f.import(&[f.image("lantern.png")]).expect("import");
    assert_eq!(done.direction.prompt, "A lantern.");
    assert_eq!(done.direction.aspect, crate::concepts::Aspect::Wide);
}

#[test]
fn nothing_is_copied_when_one_file_is_no_image() {
    let f = fixture();
    let bad = f.drops.path().join("notes.png");
    fs::write(&bad, b"not an image").expect("write");
    let error = f
        .import(&[f.image("lantern.png"), bad])
        .expect_err("refused");
    assert!(error.to_string().contains("not a PNG, JPEG or WebP"));
    assert!(load_direction(&f.dir, "lantern").expect("read").is_none());
}

#[test]
fn the_extension_follows_the_header() {
    let f = fixture();
    let done = f.import(&[f.image("lantern.jpg")]).expect("import");
    assert!(done.added[0].ends_with(".png"), "{}", done.added[0]);
}

#[test]
fn a_name_with_no_letters_still_lands() {
    let f = fixture();
    let done = f.import(&[f.image("___.png")]).expect("import");
    assert_eq!(done.direction.name, FALLBACK_NAME);
}

#[test]
fn a_direction_of_refs_work_is_left_alone() {
    let f = fixture();
    let (mut held, _) = crate::concepts::prepare(
        &f.dir,
        &crate::concepts::NewConcepts {
            project: f.id.clone(),
            direction: "Lantern".into(),
            prompt: String::new(),
            refs: vec![],
            aspect: crate::concepts::Aspect::Square,
            count: 1,
            use_style: false,
        },
    )
    .expect("prepare");
    held.source = Some("concepts/x/x_01.png".into());
    save_direction(&f.dir, &held).expect("save");
    let done = f.import(&[f.image("lantern.png")]).expect("import");
    assert_eq!(done.direction.name, "Lantern 2");
}

#[test]
fn a_card_further_along_stays_where_it_is_with_its_picture() {
    let f = fixture();
    project::modify(f.root.path(), &f.layout, &f.id, |m| {
        let added = assets::add(
            m,
            NewAsset {
                name: "Lantern".into(),
                kind: AssetKind::Character,
                stage: Stage::Refs,
            },
        )?;
        assets::set_ref_thumbnail(m, &added.id, "refs/lantern.png")?;
        Ok(())
    })
    .expect("card");
    let done = f.import(&[f.image("lantern.png")]).expect("import");
    assert_eq!(done.asset.stage, Stage::Refs);
    assert_eq!(done.asset.kind, AssetKind::Character);
    assert_eq!(done.asset.thumbnail.as_deref(), Some("refs/lantern.png"));
}

#[test]
fn sending_a_pick_moves_the_card_up_with_the_kind_chosen_and_a_ref_picture() {
    let f = fixture();
    let done = f.import(&[f.image("lantern.png")]).expect("import");
    mark(&f.dir, "lantern", &done.added[0], None, Some(true)).expect("pick");
    let sent = send_picks(
        f.root.path(),
        &f.layout,
        &f.id,
        "lantern",
        AssetKind::Character,
    )
    .expect("send");
    assert_eq!(sent.asset.id, "lantern");
    assert_eq!(sent.asset.stage, Stage::Refs);
    assert_eq!(
        sent.asset.kind,
        AssetKind::Character,
        "the card was only a guess"
    );
    assert_eq!(sent.asset.thumbnail.as_deref(), Some("refs/lantern_01.png"));
    assert!(Path::new(&f.dir.join("refs/lantern_01.png")).is_file());
}

#[test]
fn a_card_made_by_hand_keeps_its_kind_when_a_pick_is_sent() {
    let f = fixture();
    let done = f.import(&[f.image("lantern.png")]).expect("import");
    project::modify(f.root.path(), &f.layout, &f.id, |m| {
        assets::add(
            m,
            NewAsset {
                name: "Porch".into(),
                kind: AssetKind::Character,
                stage: Stage::Concept,
            },
        )
        .map(|_| ())
    })
    .expect("card");
    drop(done);
    let (_, files) = crate::concepts::prepare(
        &f.dir,
        &crate::concepts::NewConcepts {
            project: f.id.clone(),
            direction: "Porch".into(),
            prompt: "p".into(),
            refs: vec![],
            aspect: crate::concepts::Aspect::Square,
            count: 1,
            use_style: false,
        },
    )
    .expect("prepare");
    let (index, file) = &files[0];
    fs::write(
        f.dir.join(file),
        crate::inbox::images::write_png(2, 2, &[9; 12]),
    )
    .expect("png");
    add_image(&f.dir, "porch", *index, file, None).expect("add");
    mark(&f.dir, "porch", file, None, Some(true)).expect("pick");
    let sent = send_picks(f.root.path(), &f.layout, &f.id, "porch", AssetKind::Prop).expect("send");
    assert_eq!(sent.asset.kind, AssetKind::Character);
}

#[test]
fn names_in_other_scripts_keep_their_name_and_get_their_own_direction() {
    let f = fixture();
    let snow = f.import(&[f.image("\u{96ea}.png")]).expect("snow");
    let moon = f.import(&[f.image("\u{6708}.png")]).expect("moon");
    assert_eq!(snow.direction.name, "\u{96ea}");
    assert_eq!(moon.direction.name, "\u{6708}");
    assert_ne!(snow.direction.id, moon.direction.id);
    assert_ne!(snow.direction.id, "project");
    assert_ne!(snow.asset.id, moon.asset.id);
    let again = f.import(&[f.image("\u{96ea}.png")]).expect("again");
    assert_eq!(again.direction.id, snow.direction.id);
    assert_eq!(again.direction.images.len(), 2);
}

#[test]
fn the_lab_finds_the_same_direction_for_a_name_in_another_script() {
    let f = fixture();
    let done = f.import(&[f.image("\u{96ea}.png")]).expect("import");
    let new = crate::concepts::NewConcepts {
        project: f.id.clone(),
        direction: "\u{96ea}".into(),
        prompt: "Snow.".into(),
        refs: vec![],
        aspect: crate::concepts::Aspect::Square,
        count: 1,
        use_style: false,
    };
    let (direction, _) = crate::concepts::prepare(&f.dir, &new).expect("prepare");
    assert_eq!(direction.id, done.direction.id);
}

#[test]
fn a_stem_of_only_marks_lands_under_the_fallback_name() {
    let f = fixture();
    let done = f.import(&[f.image("!!!.png")]).expect("import");
    assert_eq!(done.direction.name, FALLBACK_NAME);
}

#[test]
fn a_name_sharing_a_slug_with_another_stays_apart() {
    let f = fixture();
    let first = f.import(&[f.image("stone-lantern.png")]).expect("first");
    let other = f.import(&[f.image("Stone lantern!.png")]).expect("other");
    assert_ne!(first.direction.id, other.direction.id);
    assert_eq!(other.direction.name, "Stone lantern! 2");
}

#[test]
fn a_long_name_fits_the_asset_limit_and_a_taken_one_still_gets_a_free_slug() {
    let f = fixture();
    let stem = "a".repeat(120);
    let done = f.import(&[f.image(&format!("{stem}.png"))]).expect("long");
    assert!(done.direction.name.chars().count() <= assets::ASSET_NAME_MAX_CHARS);
    let mut held = done.direction.clone();
    held.source = Some("concepts/x/x_01.png".into());
    save_direction(&f.dir, &held).expect("save");
    let next = f.import(&[f.image(&format!("{stem}.png"))]).expect("next");
    assert_ne!(next.direction.id, done.direction.id);
    assert!(next.direction.name.chars().count() <= assets::ASSET_NAME_MAX_CHARS);
}

#[test]
fn a_reserved_windows_name_does_not_become_a_folder() {
    let f = fixture();
    let done = f.import(&[f.image("con.png")]).expect("import");
    assert_eq!(done.direction.name, "Con");
    assert_eq!(done.direction.id, "con-1");
    assert_eq!(done.asset.id, "con-1");
}

#[test]
fn a_real_image_with_no_extension_lands() {
    let f = fixture();
    let done = f.import(&[f.image("lantern")]).expect("import");
    assert!(done.added[0].ends_with(".png"));
}

#[test]
fn what_is_not_a_regular_file_is_refused_by_name_without_being_opened() {
    let f = fixture();
    let error = f
        .import(&[f.drops.path().to_path_buf()])
        .expect_err("a folder");
    assert!(error.to_string().contains("is not a file"), "{error}");
    #[cfg(unix)]
    {
        let fifo = f.drops.path().join("pipe.png");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo");
        assert!(made.success());
        let error = f.import(&[fifo]).expect_err("a pipe");
        assert!(error.to_string().contains("pipe.png"), "{error}");
    }
}

#[test]
fn a_file_over_the_limit_is_refused_and_the_error_names_it_and_the_limit() {
    let f = fixture();
    let big = f.image("big.png");
    let file = fs::OpenOptions::new()
        .append(true)
        .open(&big)
        .expect("open");
    file.set_len(MAX_IMAGE_BYTES + 1).expect("grow");
    let error = f.import(&[big]).expect_err("too big");
    let text = error.to_string();
    assert!(text.contains("big.png") && text.contains("50 MB"), "{text}");
    assert!(load_direction(&f.dir, "big").expect("read").is_none());
}

#[test]
fn a_drop_over_the_count_limit_is_refused_and_says_the_limit() {
    let f = fixture();
    let one = f.image("one.png");
    let many = vec![one; MAX_IMAGES_PER_DROP + 1];
    let text = f.import(&many).expect_err("too many").to_string();
    assert!(text.contains(&MAX_IMAGES_PER_DROP.to_string()), "{text}");
}

#[test]
fn a_drop_over_the_total_limit_is_refused_and_says_the_limit() {
    let f = fixture();
    let big = f.image("big.png");
    let file = fs::OpenOptions::new()
        .append(true)
        .open(&big)
        .expect("open");
    file.set_len(MAX_IMAGE_BYTES).expect("grow");
    // The file now ends in zeros, so the size check must come before the ending.
    let n = (MAX_DROP_BYTES / MAX_IMAGE_BYTES) as usize + 1;
    let text = f.import(&vec![big; n]).expect_err("too much").to_string();
    assert!(text.contains("512 MB"), "{text}");
}

#[test]
fn a_cut_off_image_with_a_good_header_is_refused() {
    let f = fixture();
    let cut = f.drops.path().join("cut.png");
    let bytes = crate::inbox::images::write_png(2, 3, &[90; 18]);
    fs::write(&cut, &bytes[..bytes.len() - 6]).expect("write");
    let text = f.import(&[cut]).expect_err("cut").to_string();
    assert!(
        text.contains("cut.png") && text.contains("cut off"),
        "{text}"
    );

    let jpeg = f.drops.path().join("cut.jpg");
    fs::write(&jpeg, [0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0]).expect("write");
    assert!(f.import(&[jpeg]).is_err());

    let webp = f.drops.path().join("cut.webp");
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(100u32.to_le_bytes());
    bytes.extend(b"WEBPVP8 ");
    fs::write(&webp, bytes).expect("write");
    assert!(f.import(&[webp]).is_err());
}

#[test]
fn a_whole_jpeg_and_webp_pass_the_ending_check() {
    let f = fixture();
    let jpeg = f.drops.path().join("whole.jpg");
    fs::write(&jpeg, [0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0, 0xFF, 0xD9]).expect("write");
    assert!(f.import(&[jpeg]).expect("jpeg").added[0].ends_with(".jpg"));
    let webp = f.drops.path().join("whole.webp");
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(12u32.to_le_bytes());
    bytes.extend(b"WEBPVP8 ");
    bytes.extend(0u32.to_le_bytes());
    fs::write(&webp, bytes).expect("write");
    assert!(f.import(&[webp]).expect("webp").added[0].ends_with(".webp"));
}
