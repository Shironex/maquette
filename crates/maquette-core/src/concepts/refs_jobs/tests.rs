//! Tests for the refs jobs' plans: which views get drawn and the prompts.

use std::path::Path;

use super::*;

fn tagged(view: RefView) -> AssetRef {
    AssetRef {
        file: format!("refs/{}.png", view.name()),
        view,
    }
}

fn lantern() -> Asset {
    serde_json::from_str(
        r#"{"id":"frost-lantern","name":"Frost lantern","kind":"prop","stage":"refs",
            "status":"queued","versions":[],"thumbnail":null,"createdAt":"t","updatedAt":"t"}"#,
    )
    .expect("asset")
}

#[test]
fn filled_slots_are_skipped_unless_named() {
    let refs = vec![tagged(RefView::Front), tagged(RefView::Back)];
    assert_eq!(
        views_to_draw(&refs, &[]).expect("empty slots"),
        vec![RefView::Left, RefView::Right]
    );
    assert_eq!(
        views_to_draw(&refs, &[RefView::Back]).expect("redraw"),
        vec![RefView::Back]
    );
    assert!(views_to_draw(&refs, &[RefView::Front]).is_err());
    let full: Vec<AssetRef> = RefView::SLOTS.into_iter().map(tagged).collect();
    let err = views_to_draw(&full, &[]).expect_err("all filled");
    assert!(err.to_string().contains("Every view is filled"));
    assert!(front_of(&[tagged(RefView::Left)]).is_err(), "no front");
    assert_eq!(front_of(&refs).expect("front"), "refs/front.png");
}

#[test]
fn the_prompts_say_the_side_and_what_to_keep() {
    let out = Path::new("/w/frost/concepts/frost-lantern-views/frost-lantern-views_01.png");
    let text = view_prompt(&lantern(), RefView::Right, "", out, None);
    assert!(text.starts_with("Generate an image.\n\nDraw the Frost lantern from its right side"));
    assert!(text.contains("first attached image is its front view"));
    assert!(!text.contains("Extra instructions"));
    assert!(text.trim_end().ends_with(&out.display().to_string()));

    let mut clean = NewCleanRef {
        project: "frost".into(),
        asset: "frost-lantern".into(),
        file: "refs/lantern_kit_crop.png".into(),
        ground: CleanGround::StudioGrey,
        pad: false,
        extend: false,
    };
    let text = clean_prompt(&clean, &lantern(), out);
    assert!(text.contains("studio grey"));
    assert!(text.contains("leave it cut off: invent nothing"));
    clean.ground = CleanGround::Transparent;
    clean.pad = true;
    clean.extend = true;
    let text = clean_prompt(&clean, &lantern(), out);
    assert!(text.contains("transparent"));
    assert!(text.contains("pad the image to a 1:1 square"));
    assert!(text.contains("draw the missing part"));
}

#[test]
fn notes_go_into_every_view_prompt_on_one_line() {
    let out = Path::new("/w/frost/concepts/frost-lantern-views/frost-lantern-views_01.png");
    let notes = view_notes(Some("  cape over the\n\nright shoulder ")).expect("notes");
    assert_eq!(notes, "cape over the right shoulder");
    for view in [RefView::Left, RefView::Back] {
        let text = view_prompt(&lantern(), view, &notes, out, None);
        assert!(
            text.contains("1:1 square composition.\n\nExtra instructions for this view: cape over the right shoulder"),
            "{text}"
        );
    }
    assert_eq!(view_notes(None).expect("none"), "");
    assert_eq!(view_notes(Some(" \n ")).expect("blank"), "");
    let long = "a".repeat(MAX_NOTE_CHARS + 1);
    let err = view_notes(Some(&long)).expect_err("too long");
    assert!(err.to_string().contains("400 characters"), "{err}");
}

#[test]
fn another_assets_direction_of_the_same_name_is_left_alone() {
    let dir = tempfile::tempdir().expect("dir");
    let mut other = lantern();
    other.id = "frost-lantern-old".into();
    let titles = vec!["left".to_string()];
    let (first, _) = prepare(
        dir.path(),
        &other,
        RefsJobKind::Views,
        "refs/a.png",
        &titles,
        None,
    )
    .expect("first");
    let (second, _) = prepare(
        dir.path(),
        &lantern(),
        RefsJobKind::Views,
        "refs/a.png",
        &titles,
        None,
    )
    .expect("second");
    assert_eq!(first.name, "Frost lantern views");
    assert_eq!(second.name, "Frost lantern views 2");
    let (again, planned) = prepare(
        dir.path(),
        &lantern(),
        RefsJobKind::Views,
        "refs/a.png",
        &titles,
        Some(3),
    )
    .expect("again");
    assert_eq!(again.id, second.id, "the asset's own direction is reused");
    assert_eq!(planned[0].index, 2);
    assert_eq!(again.style_revision, Some(3));
}
