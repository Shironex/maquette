use super::*;

#[test]
fn edits_turn_into_core_patches_and_bad_words_are_refused() {
    let args: SetMaterialsArgs = serde_json::from_value(json!({
        "project": "bench",
        "asset": "laptop",
        "all": { "tint": "#3a3936", "roughness": { "mode": "Override", "value": 0.62 }, "emissiveStrength": null },
        "materials": { "Screen": null }
    }))
    .expect("args");
    let patch = args.patch().expect("patch");
    let all = patch.all.expect("all");
    assert_eq!(all.tint, Some(Some("#3a3936".to_string())));
    assert_eq!(
        all.roughness.flatten().map(|r| r.mode),
        Some(FactorMode::Override)
    );
    assert_eq!(all.metalness, None, "left out stays");
    assert_eq!(all.emissive_strength, Some(None), "null clears");
    assert_eq!(patch.materials.get("Screen"), Some(&None));

    let bad: SetMaterialsArgs = serde_json::from_value(json!({
        "project": "bench",
        "asset": "laptop",
        "all": { "roughness": { "mode": "lerp", "value": 0.5 } }
    }))
    .expect("args");
    assert!(
        bad.patch()
            .expect_err("mode")
            .contains("override or multiply")
    );
    assert!(
        parse_codec("normal", "bc7")
            .expect_err("codec")
            .contains("codecs.normal")
    );
    assert_eq!(parse_codec("color", "UASTC"), Ok(TextureCodec::Uastc));
}

fn row(model: &str, codec: TextureCodec) -> CompressPreset {
    CompressPreset {
        model: model.to_string(),
        label: model.to_string(),
        texture_size: 1024,
        data_texture_size: None,
        codec,
        map_codecs: None,
        meshopt: maquette_core::MeshoptLevel::default(),
        lock: None,
    }
}

#[test]
fn a_tweak_that_fails_puts_the_old_presets_back() {
    let before = vec![row("crate", TextureCodec::Etc1s)];
    let after = vec![row("crate", TextureCodec::Uastc)];
    let change = || PresetChange {
        next: after.clone(),
        previous: before.clone(),
    };
    let disk = std::cell::RefCell::new(before.clone());
    let save = |rows: Vec<CompressPreset>| {
        *disk.borrow_mut() = rows;
        Ok(())
    };

    let err = presets_then_tweaks(Some(change()), save, || {
        Err::<(), _>("The disk is full.".to_string())
    })
    .expect_err("tweaks fail");
    assert_eq!(err, "The disk is full.");
    assert_eq!(*disk.borrow(), before, "the old presets are back");

    presets_then_tweaks(Some(change()), save, || Ok(())).expect("both");
    assert_eq!(*disk.borrow(), after);

    // Presets that cannot be written stop before the tweaks.
    let tweaked = std::cell::Cell::new(false);
    let err = presets_then_tweaks(
        Some(change()),
        |_| Err("read only".to_string()),
        || {
            tweaked.set(true);
            Ok(())
        },
    )
    .expect_err("presets fail");
    assert_eq!(err, "read only");
    assert!(!tweaked.get());

    // When putting them back fails too, the answer says so.
    let mut calls = 0;
    let err = presets_then_tweaks(
        Some(change()),
        |_| {
            calls += 1;
            if calls == 1 {
                Ok(())
            } else {
                Err("locked".to_string())
            }
        },
        || Err::<(), _>("The disk is full.".to_string()),
    )
    .expect_err("both fail");
    assert!(err.contains("could not be put back (locked)"), "{err}");
}
