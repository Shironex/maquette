use super::*;

fn defaults() -> FixupDefaults {
    FixupDefaults {
        triangles: 60_000,
        max_texture: 2048,
        loose_share: 0.02,
    }
}

#[test]
fn edits_fill_in_the_defaults_and_bad_words_are_refused() {
    let edit: FixupEdit = serde_json::from_value(json!({ "op": "Decimate" })).expect("args");
    assert_eq!(
        edit.fixup(&defaults()),
        Ok(Fixup::Decimate { triangles: 60_000 })
    );
    let edit: FixupEdit =
        serde_json::from_value(json!({ "op": "shrink-textures", "maxSize": 512 })).expect("args");
    assert_eq!(
        edit.fixup(&defaults()),
        Ok(Fixup::ShrinkTextures { max_size: 512 })
    );
    let split: FixupEdit = serde_json::from_value(json!({ "op": "split-parts" })).expect("args");
    assert_eq!(
        split.fixup(&defaults()),
        Ok(Fixup::SplitParts {
            by: SplitBy::Loose,
            merge_under: 0.01,
        }),
        "by loose pieces, bits under 1% merged in"
    );
    let split: FixupEdit = serde_json::from_value(
        json!({ "op": "split-parts", "by": "Material", "mergeUnder": 0.05 }),
    )
    .expect("args");
    assert_eq!(
        split.fixup(&defaults()),
        Ok(Fixup::SplitParts {
            by: SplitBy::Material,
            merge_under: 0.05,
        })
    );
    let odd: FixupEdit =
        serde_json::from_value(json!({ "op": "split-parts", "by": "colour" })).expect("args");
    assert!(
        odd.fixup(&defaults())
            .expect_err("by")
            .contains("loose or material")
    );
    let bad: FixupEdit = serde_json::from_value(json!({ "op": "smooth" })).expect("args");
    assert!(
        bad.fixup(&defaults())
            .expect_err("op")
            .contains("origin-bottom")
    );

    let size = SizeEdit {
        axis: "longest".to_string(),
        value: 1.5,
        unit: "M".to_string(),
    };
    assert_eq!(
        size.size(),
        Ok(RealSize {
            axis: SizeAxis::Longest,
            value: 1.5,
            unit: SizeUnit::M,
        })
    );
    let inch = SizeEdit {
        unit: "in".to_string(),
        ..size
    };
    assert!(inch.size().expect_err("unit").contains("cm or m"));
}

#[test]
fn a_removal_names_kinds_or_all() {
    let none = RemoveFixupsArgs {
        project: "bench".to_string(),
        asset: "lamp".to_string(),
        ops: Vec::new(),
        all: false,
    };
    assert!(none.patch().is_err());
    let some = RemoveFixupsArgs {
        ops: vec!["merge".to_string()],
        ..none
    };
    assert_eq!(some.patch().expect("patch").remove, vec![FixupKind::Merge]);
}
