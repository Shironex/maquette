//! Tests for the style board: validation, copies that stay inside the
//! project, revisions, and the prompt with and without the board.

use super::*;
use crate::assets::AssetKind;
use crate::concepts::{
    Aspect, NewConcepts, assemble, attached, codex_args, objects::object_prompt, prepare_with,
    prompt,
};

fn png() -> Vec<u8> {
    crate::inbox::images::write_png(4, 4, &[90; 4 * 4 * 3])
}

/// A project with one concept, one ref and an image outside it.
struct Lab {
    _dir: tempfile::TempDir,
    project: PathBuf,
    outside: PathBuf,
}

fn lab() -> Lab {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join("frostveil");
    let concept = project.join("concepts/veil-gate-kit/veil-gate-kit_02.png");
    fs::create_dir_all(concept.parent().expect("parent")).expect("mkdir");
    fs::write(&concept, png()).expect("concept");
    fs::create_dir_all(project.join("refs")).expect("refs");
    fs::write(project.join("refs/frozen_lake.png"), png()).expect("ref");
    let outside = dir.path().join("Desktop").join("Frost Glass.PNG");
    fs::create_dir_all(outside.parent().expect("parent")).expect("mkdir");
    fs::write(&outside, png()).expect("outside");
    Lab {
        _dir: dir,
        project,
        outside,
    }
}

fn board(line: &str, palette: &[&str], images: &[&str]) -> StyleBoard {
    StyleBoard {
        revision: 2,
        line: line.into(),
        palette: palette.iter().map(|c| (*c).to_string()).collect(),
        images: images
            .iter()
            .map(|f| StyleImage {
                file: (*f).to_string(),
                source: None,
            })
            .collect(),
        ..StyleBoard::default()
    }
}

fn change(
    line: Option<&str>,
    palette: Option<&[&str]>,
    images: Option<&[&str]>,
) -> StyleBoardChange {
    let strings = |items: &[&str]| items.iter().map(|s| (*s).to_string()).collect();
    StyleBoardChange {
        line: line.map(str::to_string),
        palette: palette.map(strings),
        images: images.map(strings),
    }
}

#[test]
fn colours_are_written_as_six_hex_digits() {
    assert_eq!(normalize_colour("#9FC3FF").as_deref(), Some("#9fc3ff"));
    assert_eq!(normalize_colour(" 1b2440 ").as_deref(), Some("#1b2440"));
    assert_eq!(normalize_colour("#abc").as_deref(), Some("#aabbcc"));
    assert_eq!(normalize_colour("#12345"), None);
    assert_eq!(normalize_colour("frost"), None);
}

#[test]
fn the_board_is_validated() {
    assert!(validate(&StyleBoard::default()).is_ok());
    assert!(validate(&board("frosted glass", &["#9fc3ff"], &["style/a.png"])).is_ok());
    let long = "a".repeat(MAX_LINE_CHARS + 1);
    assert!(validate(&board(&long, &[], &[])).is_err());
    assert!(validate(&board("", &["#9FC3FF"], &[])).is_err());
    assert!(validate(&board("", &["#111111"; MAX_PALETTE + 1], &[])).is_err());
    let five = [
        "style/a.png",
        "style/b.png",
        "style/c.png",
        "style/d.png",
        "style/e.png",
    ];
    assert!(validate(&board("", &[], &five)).is_err());
    for outside in [
        "refs/a.png",
        "style/../a.png",
        "style/sub/a.png",
        "/abs/a.png",
    ] {
        assert!(
            validate(&board("", &[], &[outside])).is_err(),
            "{outside} must be refused"
        );
    }
    let newer = StyleBoard {
        version: STYLE_VERSION + 1,
        ..StyleBoard::default()
    };
    assert!(matches!(
        validate(&newer),
        Err(CoreError::UnsupportedVersion { .. })
    ));
}

#[test]
fn a_project_without_the_file_has_an_empty_board() {
    let l = lab();
    let empty = load(&l.project).expect("load");
    assert!(empty.is_empty());
    assert_eq!(empty.revision, 0);
    assert!(for_job(&l.project, true).is_none());
    fs::write(path(&l.project), "{ not json").expect("broken");
    assert!(load(&l.project).is_err());
    assert!(load_or_default(&l.project).is_empty());
}

#[test]
fn images_are_copied_into_the_project_and_never_linked() {
    let l = lab();
    let outside = l.outside.display().to_string();
    let saved = update(
        &l.project,
        &change(
            Some("  frosted glass,\n cool night light  "),
            Some(&["9FC3FF", "#1b2440", "#9fc3ff"]),
            Some(&[
                "concepts/veil-gate-kit/veil-gate-kit_02.png",
                "refs\\frozen_lake.png",
                &outside,
            ]),
        ),
    )
    .expect("update");
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.line, "frosted glass, cool night light");
    assert_eq!(saved.palette, ["#9fc3ff", "#1b2440"]);
    assert_eq!(
        saved.files(),
        [
            "style/veil_gate_kit_02.png",
            "style/frozen_lake.png",
            "style/frost_glass.png"
        ]
    );
    assert_eq!(
        saved.images[0].source.as_deref(),
        Some("concepts/veil-gate-kit/veil-gate-kit_02.png")
    );
    assert_eq!(
        saved.images[1].source.as_deref(),
        Some("refs/frozen_lake.png")
    );
    assert_eq!(saved.images[2].source, None);
    for image in &saved.images {
        let copy = l.project.join(&image.file);
        assert!(copy.is_file(), "{} was not copied", image.file);
        assert!(copy.starts_with(l.project.join(STYLE_DIR)));
    }
    assert!(l.outside.is_file(), "the original stays where it was");
    assert_eq!(load(&l.project).expect("reload"), saved);
}

#[test]
fn paths_that_leave_the_project_are_refused() {
    let l = lab();
    for bad in [
        "../Desktop/Frost Glass.PNG",
        "concepts/../../Desktop/Frost Glass.PNG",
        "refs/missing.png",
        "C:/Users/frost.png",
    ] {
        assert!(
            update(&l.project, &change(None, None, Some(&[bad]))).is_err(),
            "{bad} must be refused"
        );
    }
    assert!(!l.project.join(STYLE_DIR).exists(), "nothing was copied");
    assert!(!path(&l.project).exists());
}

#[test]
fn an_absolute_path_inside_the_project_is_read_as_its_own_file() {
    let l = lab();
    let inside = l.project.join("refs").join("frozen_lake.png");
    let saved = update(
        &l.project,
        &change(None, None, Some(&[&inside.display().to_string()])),
    )
    .expect("update");
    assert_eq!(
        saved.images[0].source.as_deref(),
        Some("refs/frozen_lake.png")
    );
}

#[test]
fn only_images_join_the_board() {
    let l = lab();
    fs::write(l.project.join("refs/notes.png"), b"not an image").expect("fake");
    assert!(update(&l.project, &change(None, None, Some(&["refs/notes.png"]))).is_err());
    for name in ["a", "b", "c"] {
        fs::write(l.project.join(format!("refs/{name}.png")), png()).expect("ref");
    }
    let five = [
        "refs/frozen_lake.png",
        "concepts/veil-gate-kit/veil-gate-kit_02.png",
        "refs/a.png",
        "refs/b.png",
        "refs/c.png",
    ];
    assert!(update(&l.project, &change(None, None, Some(&five))).is_err());
    assert!(load(&l.project).expect("load").is_empty());
}

#[test]
fn the_revision_goes_up_only_when_the_board_changes() {
    let l = lab();
    let first = update(
        &l.project,
        &change(
            Some("chunky silhouettes"),
            None,
            Some(&["concepts/veil-gate-kit/veil-gate-kit_02.png"]),
        ),
    )
    .expect("first");
    assert_eq!(first.revision, 1);
    // The same again, by the copy or by its source, changes nothing.
    for again in [
        "style/veil_gate_kit_02.png",
        "concepts/veil-gate-kit/veil-gate-kit_02.png",
    ] {
        let same = update(
            &l.project,
            &change(Some("chunky silhouettes"), None, Some(&[again])),
        )
        .expect("same");
        assert_eq!(same, first);
    }
    assert!(!l.project.join("style/veil_gate_kit_02_2.png").exists());
    let second = update(&l.project, &change(None, None, Some(&[]))).expect("second");
    assert_eq!(second.revision, 2);
    assert!(second.images.is_empty());
    assert_eq!(second.line, "chunky silhouettes");
    // A copy that left the board stays, so a job attaching it runs again the same.
    assert!(l.project.join("style/veil_gate_kit_02.png").is_file());
    let refused = update(&l.project, &change(Some(&"x".repeat(300)), None, None));
    assert!(refused.is_err());
    assert_eq!(load(&l.project).expect("load").revision, 2);
}

#[test]
fn a_job_carries_the_board_unless_turned_off() {
    let l = lab();
    update(
        &l.project,
        &change(
            Some("frosted glass"),
            None,
            Some(&[
                "refs/frozen_lake.png",
                "concepts/veil-gate-kit/veil-gate-kit_02.png",
            ]),
        ),
    )
    .expect("update");
    assert!(for_job(&l.project, false).is_none());
    fs::remove_file(l.project.join("style/frozen_lake.png")).expect("remove");
    let carried = for_job(&l.project, true).expect("board");
    assert_eq!(carried.revision, 1);
    assert_eq!(carried.files(), ["style/veil_gate_kit_02.png"]);
}

fn request(refs: &[&str], use_style: bool) -> NewConcepts {
    NewConcepts {
        project: "frostveil".into(),
        direction: "Frozen lake arena".into(),
        prompt: "A frozen lake arena at night.".into(),
        refs: refs.iter().map(|r| (*r).to_string()).collect(),
        aspect: Aspect::Wide,
        count: 2,
        use_style,
    }
}

#[test]
fn without_the_board_the_prompt_is_the_brief_alone() {
    let out = Path::new("/w/frostveil/concepts/frozen-lake-arena/frozen-lake-arena_01.png");
    let plain = prompt(&request(&["refs/a.png"], true), 1, out, None);
    assert!(!plain.contains("Project style"));
    assert!(plain.contains(" The attached images are references:"));
    let empty = StyleBoard::default();
    assert_eq!(
        prompt(&request(&["refs/a.png"], true), 1, out, Some(&empty)),
        plain
    );
    assert_eq!(
        assemble("Lead.", None, out),
        assemble("Lead.", Some(&empty), out)
    );
}

#[test]
fn with_the_board_the_style_follows_the_brief_as_its_own_note() {
    let out = Path::new("/w/frostveil/concepts/frozen-lake-arena/frozen-lake-arena_01.png");
    let style = board(
        "frosted glass, cool night light, chunky silhouettes.",
        &["#1b2440", "#9fc3ff"],
        &["style/veil_gate_kit_02.png", "style/frost_glass.png"],
    );
    let text = prompt(&request(&["refs/a.png"], true), 1, out, Some(&style));
    assert!(text.starts_with("Generate an image.\n\nA frozen lake arena at night.\n"));
    let brief = text.find("A frozen lake arena").expect("brief");
    let note = text.find("\n\nProject style: ").expect("style note");
    let save = text.find("Generate exactly one image").expect("save");
    assert!(brief < note && note < save, "{text}");
    assert!(text.contains("where the two disagree, the brief wins."));
    assert!(text.contains("\nStyle: frosted glass, cool night light, chunky silhouettes.\n"));
    assert!(text.contains("\nPalette: #1b2440, #9fc3ff.\n"));
    assert!(text.contains("The last 2 attached images are the project's style board"));
    assert!(text.contains(" The first attached image is a reference: follow its style"));
    assert!(text.trim_end().ends_with(&out.display().to_string()));
    assert!(!text.contains('\u{2014}') && !text.contains('\u{2013}'));

    // A board of words only names no images, and the refs keep their sentence.
    let words = board("frosted glass", &[], &[]);
    let text = prompt(
        &request(&["refs/a.png", "refs/b.png"], true),
        1,
        out,
        Some(&words),
    );
    assert!(text.contains("Style: frosted glass."));
    assert!(!text.contains("style board:"));
    assert!(text.contains(" The attached images are references:"));
}

#[test]
fn object_refs_keep_the_concept_first_and_the_board_after() {
    let out = Path::new("/w/frostveil/concepts/x/x_01.png");
    let style = board("frosted glass", &[], &["style/frost_glass.png"]);
    let text = object_prompt(
        "lantern post",
        "",
        AssetKind::Prop,
        "Veil gate props",
        Aspect::Square,
        out,
        Some(&style),
    );
    assert!(text.contains(
        "the \"lantern post\" from the first attached image, the concept (the direction \"Veil gate props\")."
    ));
    assert!(text.contains("The last attached image is the project's style board"));
    let plain = object_prompt(
        "lantern post",
        "",
        AssetKind::Prop,
        "Veil gate props",
        Aspect::Square,
        out,
        None,
    );
    assert!(plain.contains("from the attached concept image (the direction"));
    assert!(!plain.contains("Project style"));
}

#[test]
fn the_board_images_are_attached_after_the_briefs_own() {
    let project = Path::new("/w/frostveil");
    let style = board("", &[], &["style/frost_glass.png"]);
    let refs = vec!["refs/a.png".to_string()];
    assert_eq!(
        attached(&refs, Some(&style)),
        ["refs/a.png", "style/frost_glass.png"]
    );
    assert_eq!(attached(&refs, None), ["refs/a.png"]);
    let args = codex_args(project, &attached(&refs, Some(&style)));
    let last = project.join("style/frost_glass.png").display().to_string();
    assert_eq!(args[args.len() - 3], last);
}

#[test]
fn the_direction_records_the_revision_and_the_switch() {
    let l = lab();
    let (styled, _) = prepare_with(&l.project, &request(&[], true), Some(3)).expect("prepare");
    assert_eq!(styled.style_revision, Some(3));
    assert!(!styled.style_off);
    let (off, _) = prepare_with(&l.project, &request(&[], false), None).expect("prepare");
    assert_eq!(off.style_revision, None);
    assert!(off.style_off);
}
