use super::*;
use crate::pipeline::template::for_template;
use crate::prepare::{ModelPrep, save_entry};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|a| a.to_string()).collect()
}

/// A diorama project with its pipeline and the current starters.
fn project() -> (tempfile::TempDir, PipelineDef) {
    let dir = tempfile::tempdir().expect("tempdir");
    starter::write(dir.path(), ProjectTemplate::FloatingDiorama).expect("starters");
    (dir, for_template(ProjectTemplate::FloatingDiorama))
}

#[test]
fn a_new_project_bakes_with_the_defaults() {
    let (dir, def) = project();
    let found = view(dir.path(), ProjectTemplate::FloatingDiorama, &def).expect("bake step");
    assert_eq!(found.settings, DEFAULT);
    assert!(found.can_denoise);
    assert_eq!(found.script, StarterState::Current);
    assert!(found.skipped.is_empty());
    // The template writes every flag, so the args say what runs.
    let bake = step(&def).expect("step");
    assert_eq!(read_args(&bake.args, false), DEFAULT);
}

#[test]
fn flags_left_out_read_as_the_script_does_without_them() {
    assert_eq!(read_args(&[], true), DEFAULT);
    let old = read_args(&args(&["--size", "512", "--samples", "16"]), false);
    assert_eq!(
        old,
        BakeSettings {
            size: 512,
            samples: 16,
            denoise: false
        }
    );
    let off = read_args(&args(&["--no-denoise"]), true);
    assert!(!off.denoise);
}

#[test]
fn writing_replaces_the_values_in_place_and_keeps_the_other_args() {
    let mut list = args(&[
        "--out-dir",
        "blender/bake",
        "--size",
        "512",
        "--samples",
        "16",
    ]);
    let want = BakeSettings {
        size: 2048,
        samples: 128,
        denoise: false,
    };
    write_args(&mut list, &want, true).expect("write");
    assert_eq!(
        list,
        args(&[
            "--out-dir",
            "blender/bake",
            "--size",
            "2048",
            "--samples",
            "128",
            "--no-denoise"
        ])
    );
    // Again with denoise on: one flag, never both.
    write_args(&mut list, &DEFAULT, true).expect("again");
    assert_eq!(list.iter().filter(|a| a.contains("denoise")).count(), 1);
    assert_eq!(read_args(&list, true), DEFAULT);
}

#[test]
fn numbers_out_of_range_and_denoise_on_an_old_script_are_refused() {
    let mut list = Vec::new();
    let odd = BakeSettings {
        size: 1000,
        ..DEFAULT
    };
    assert!(write_args(&mut list, &odd, true).is_err());
    let none = BakeSettings {
        samples: 0,
        ..DEFAULT
    };
    assert!(write_args(&mut list, &none, true).is_err());
    let err = write_args(&mut list, &DEFAULT, false).expect_err("old script");
    assert!(err.to_string().contains("cannot denoise"), "{err}");
    assert!(list.is_empty(), "nothing is written when refused");
    // Without the denoiser an older script still takes size and samples.
    let plain = BakeSettings {
        denoise: false,
        ..DEFAULT
    };
    write_args(&mut list, &plain, false).expect("plain");
    assert_eq!(list, args(&["--size", "1024", "--samples", "32"]));
}

#[test]
fn apply_changes_only_the_given_fields() {
    let (dir, mut def) = project();
    let patch = BakePatch {
        samples: Some(256),
        ..BakePatch::default()
    };
    let got = apply(
        dir.path(),
        ProjectTemplate::FloatingDiorama,
        &mut def,
        &patch,
    )
    .expect("apply");
    assert_eq!(
        got,
        BakeSettings {
            samples: 256,
            ..DEFAULT
        }
    );
    let found = view(dir.path(), ProjectTemplate::FloatingDiorama, &def).expect("view");
    assert_eq!(found.settings, got);
}

#[test]
fn an_older_script_keeps_working_without_the_denoise_flag() {
    let (dir, mut def) = project();
    std::fs::write(
        dir.path().join(SCRIPT),
        include_str!("../fixtures/bake_light_first.py"),
    )
    .expect("old script");
    // Args that carry --denoise say the script takes it; the older
    // template's args did not, and then an old script cannot denoise.
    let bake = def.steps.iter_mut().find(|s| is_bake(s)).expect("step");
    bake.args.retain(|a| a != "--denoise");
    let found = view(dir.path(), ProjectTemplate::FloatingDiorama, &def).expect("view");
    assert_ne!(found.script, StarterState::Current);
    assert!(!found.can_denoise);
    assert!(!found.settings.denoise);
    let on = BakePatch {
        denoise: Some(true),
        ..BakePatch::default()
    };
    assert!(apply(dir.path(), ProjectTemplate::FloatingDiorama, &mut def, &on).is_err());
    let bigger = BakePatch {
        size: Some(2048),
        ..BakePatch::default()
    };
    let got = apply(
        dir.path(),
        ProjectTemplate::FloatingDiorama,
        &mut def,
        &bigger,
    )
    .expect("size");
    assert_eq!(got.size, 2048);
    assert!(!got.denoise);
}

#[test]
fn the_view_lists_the_assets_that_skip_the_bake() {
    let (dir, def) = project();
    let skip = ModelPrep {
        skip_bake: Some(true),
        ..ModelPrep::default()
    };
    save_entry(dir.path(), "desk-globe", &skip).expect("skip");
    let bakes = ModelPrep {
        skip_bake: Some(false),
        ..ModelPrep::default()
    };
    save_entry(dir.path(), "desk", &bakes).expect("bake");
    let found = view(dir.path(), ProjectTemplate::FloatingDiorama, &def).expect("view");
    assert_eq!(found.skipped, vec!["desk-globe".to_string()]);
}

#[test]
fn a_pipeline_without_the_starter_bake_has_no_settings() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut def = for_template(ProjectTemplate::CharacterShowcase);
    assert!(view(dir.path(), ProjectTemplate::CharacterShowcase, &def).is_none());
    let err = apply(
        dir.path(),
        ProjectTemplate::CharacterShowcase,
        &mut def,
        &BakePatch::default(),
    )
    .expect_err("no bake");
    assert!(err.to_string().contains("no Bake light step"), "{err}");
}
