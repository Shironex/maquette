//! Packs without Blender: settings, the plan, the out folder rules,
//! presets, what a check's report becomes, the contact sheet and finishing
//! a pack into its README, manifest and zip.

use std::fs;
use std::io::Read as _;
use std::path::Path;

use super::finish::{manifest, readme};
use super::model_run::{Check, Numbers, Report, plan_for, verdict};
use super::record::{MARKER, PackItem, PackItemState, PackRecord, PackState};
use super::*;
use crate::assets::ModelStats;
use crate::inbox::fixture;
use crate::library::{LibraryEntry, LibraryVersion, VersionFrom};
use crate::prepare::{Fixup, ModelPrep, RealSize, SizeAxis, SizeUnit};
use crate::workspace::FolderLayout;

pub(crate) fn stats(triangles: u32, joints: u32, animations: u32) -> ModelStats {
    ModelStats {
        triangles,
        bytes: 1000.0,
        textures: 1,
        max_texture: 2048,
        joints,
        animations,
    }
}

/// A library entry `id` of `kind` whose newest model is `bytes`.
pub(crate) fn entry(
    root: &Path,
    id: &str,
    name: &str,
    kind: AssetKind,
    bytes: &[u8],
    stats: ModelStats,
) {
    let dir = root.join("library").join(id);
    fs::create_dir_all(dir.join("models")).expect("models");
    let file = format!("models/{}_v1.glb", id.replace('-', "_"));
    fs::write(dir.join(&file), bytes).expect("model");
    let entry = LibraryEntry {
        version: 1,
        id: id.to_string(),
        name: name.to_string(),
        kind,
        tags: Vec::new(),
        refs: Vec::new(),
        versions: vec![LibraryVersion {
            version: 1,
            file,
            created_at: "2026-10-30T10:00:00Z".into(),
            stats: Some(stats),
            source: crate::library::LibrarySource::Tripo,
            credits: None,
            hash: format!("hash-{id}"),
            from: VersionFrom {
                project: "gone".into(),
                asset: id.into(),
                version: 1,
            },
            thumbnail: None,
            prep: Some(ModelPrep {
                size: Some(RealSize {
                    axis: SizeAxis::Height,
                    value: 92.0,
                    unit: SizeUnit::Cm,
                }),
                fixups: vec![Fixup::Decimate { triangles: 2_000 }],
                ..ModelPrep::default()
            }),
            materials: None,
        }],
        created_at: "2026-10-30T10:00:00Z".into(),
        updated_at: "2026-10-30T10:00:00Z".into(),
    };
    crate::json::write_json(&dir.join("entry.json"), &entry).expect("entry");
}

fn no_source(
    _: &Path,
    _: &crate::assets::Asset,
    _: u32,
) -> (crate::library::LibrarySource, Option<f64>) {
    (crate::library::LibrarySource::Imported, None)
}

#[test]
fn settings_check_the_name_textures_and_budgets() {
    let ok = PackSettings {
        name: "Village Props!".into(),
        ..PackSettings::default()
    };
    assert_eq!(ok.checked().expect("ok").name, "village-props");
    let empty = PackSettings {
        name: "  ".into(),
        ..PackSettings::default()
    };
    assert!(empty.checked().is_err());
    let odd = PackSettings {
        texture: 1000,
        ..PackSettings::default()
    };
    assert!(
        odd.checked()
            .expect_err("odd size")
            .to_string()
            .contains("1024")
    );
    let mut low = PackSettings::default();
    low.budgets[0].triangles = Some(40);
    assert!(
        low.checked()
            .expect_err("low")
            .to_string()
            .contains("props budget")
    );
    assert_eq!(PackSettings::default().budget(AssetKind::Character), None);
}

#[test]
fn a_plan_reads_entries_with_their_kinds_keys_and_clips() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    entry(
        root,
        "stove",
        "Cast iron stove",
        AssetKind::Prop,
        &fixture::clean_prop(),
        stats(33_700, 0, 0),
    );
    entry(
        root,
        "stove-2",
        "Cast iron stove",
        AssetKind::Prop,
        &fixture::clean_prop(),
        stats(12, 0, 0),
    );
    entry(
        root,
        "hero",
        "Hero",
        AssetKind::Character,
        &fixture::character(false),
        stats(4_000, 65, 1),
    );
    let source = PackSource::Entries {
        entries: vec![
            "stove".into(),
            "stove-2".into(),
            "hero".into(),
            "nope".into(),
        ],
    };
    let plan = plan(
        root,
        &FolderLayout::default(),
        &source,
        &PackSettings::default(),
        &no_source,
    )
    .expect("plan");
    assert_eq!(plan.label, "3 models from the library");
    let keys: Vec<&str> = plan.models.iter().map(|m| m.key.as_str()).collect();
    assert_eq!(keys, ["cast_iron_stove", "cast_iron_stove_2", "hero"]);
    assert_eq!(
        plan.skipped.len(),
        1,
        "the unknown entry is left out with why"
    );
    assert!(
        plan.models[0]
            .height_m
            .is_some_and(|h| (h - 0.92).abs() < 1e-4)
    );
    let hero = &plan.models[2];
    assert!(hero.rigged);
    // A Tripo model on Mixamo's bone names keeps its clips: Tripo made them.
    assert!(!hero.mixamo);
    assert_eq!(plan.mixamo, 0);
    let props = &plan.kinds[0];
    assert_eq!(
        (
            props.label.as_str(),
            props.count,
            props.biggest,
            props.budget
        ),
        ("Props", 2, 33_700, Some(3_000))
    );
    assert_eq!(plan.kinds[1].budget, None);
    assert!(plan.estimate_bytes > 0.0);
    let none = PackSource::Entries {
        entries: vec!["nope".into()],
    };
    assert!(
        super::plan(
            root,
            &FolderLayout::default(),
            &none,
            &PackSettings::default(),
            &no_source
        )
        .is_err()
    );
}

#[test]
fn mixamo_clips_are_told_apart_by_source_and_bakes() {
    use crate::inbox::analyse::RigKind;
    use crate::library::LibrarySource as S;
    assert!(licence::may_be_mixamo(
        Some(S::ByHand),
        Some(RigKind::Mixamo),
        false
    ));
    assert!(licence::may_be_mixamo(
        Some(S::Imported),
        Some(RigKind::Mixamo),
        false
    ));
    assert!(!licence::may_be_mixamo(
        Some(S::Tripo),
        Some(RigKind::Mixamo),
        false
    ));
    assert!(licence::may_be_mixamo(
        Some(S::Tripo),
        Some(RigKind::Other),
        true
    ));
    assert!(!licence::may_be_mixamo(
        Some(S::Meshy),
        Some(RigKind::Other),
        false
    ));
    assert!(licence::note(Some(S::Tripo)).contains("Tripo plan"));
}

fn item(triangles: u32, rigged: bool, budget: Option<u32>) -> PackItem {
    let dir = tempfile::tempdir().expect("dir");
    entry(
        dir.path(),
        "m",
        "Model",
        AssetKind::Prop,
        &fixture::clean_prop(),
        stats(triangles, 0, 0),
    );
    let source = PackSource::Entries {
        entries: vec!["m".into()],
    };
    let plan = plan(
        dir.path(),
        &FolderLayout::default(),
        &source,
        &PackSettings::default(),
        &no_source,
    )
    .expect("plan");
    let mut model = plan.models[0].clone();
    model.rigged = rigged;
    model.stats = Some(stats(triangles, u32::from(rigged), 0));
    PackItem::new(model, budget)
}

#[test]
fn the_pack_adds_its_fixups_to_the_models_own() {
    let settings = PackSettings::default();
    let plan = plan_for(&item(30_000, false, Some(3_000)), &settings);
    assert!(
        plan.fixups.contains(&Fixup::Decimate { triangles: 2_000 }),
        "the smaller of its own and the budget"
    );
    assert!(
        plan.fixups
            .contains(&Fixup::ShrinkTextures { max_size: 1024 })
    );
    assert!(plan.fixups.contains(&Fixup::OriginBottom));
    assert!(plan.size.is_some());
    let rigged = plan_for(&item(30_000, true, Some(3_000)), &settings);
    assert!(
        rigged
            .fixups
            .contains(&Fixup::Decimate { triangles: 2_000 }),
        "its own stays"
    );
    let plain = PackSettings {
        real_size: false,
        pivot_bottom: false,
        texture: 0,
        ..settings
    };
    let kept = plan_for(&item(30_000, false, None), &plain);
    assert_eq!(kept.size, None);
    assert_eq!(kept.fixups, [Fixup::Decimate { triangles: 2_000 }]);
}

fn report(before: u32, after: u32, thin: bool, check: Check) -> Report {
    Report {
        before: Numbers { triangles: before },
        after: Numbers { triangles: after },
        thin,
        clips_removed: 0,
        files: vec!["m.obj".into()],
        checks: vec![check],
        pivot_slack: 0.01,
    }
}

fn good_check() -> Check {
    Check {
        file: "m.obj".into(),
        triangles: 1_500,
        height_m: Some(0.92),
        floor_m: Some(0.0),
        centre: Some([0.0, 0.0]),
        ..Check::default()
    }
}

#[test]
fn a_check_report_becomes_pass_warn_or_fail() {
    let s = PackSettings::default();
    let base = item(16_000, false, Some(1_500));
    let pass = verdict(&base, &report(16_000, 1_500, false, good_check()), &s);
    assert_eq!(pass.state, PackItemState::Pass);
    assert_eq!(
        pass.note.as_deref(),
        Some("Re-imported, Y up, on the floor")
    );
    let thin = verdict(&base, &report(16_000, 1_500, true, good_check()), &s);
    assert_eq!(thin.state, PackItemState::Warn);
    assert_eq!(thin.suggest, Some(3_000));
    assert!(
        thin.note
            .as_deref()
            .unwrap_or_default()
            .contains("Thin parts lost at 1.5k")
    );
    let missing = Check {
        missing: vec!["basket.png".into()],
        ..good_check()
    };
    let fail = verdict(&base, &report(16_000, 1_500, false, missing), &s);
    assert_eq!(fail.state, PackItemState::Fail);
    assert!(
        fail.note
            .as_deref()
            .unwrap_or_default()
            .contains("basket.png")
    );
    let off = Check {
        floor_m: Some(0.2),
        ..good_check()
    };
    assert_eq!(
        verdict(&base, &report(16_000, 1_500, false, off), &s).state,
        PackItemState::Warn
    );
    let tall = Check {
        height_m: Some(1.4),
        ..good_check()
    };
    assert!(
        verdict(&base, &report(16_000, 1_500, false, tall), &s)
            .note
            .unwrap_or_default()
            .contains("0.92 m")
    );
    let small = item(800, false, Some(1_500));
    let under = verdict(&small, &report(800, 800, false, good_check()), &s);
    assert_eq!(under.note.as_deref(), Some("Under budget, left as is"));
    let mut mixamo = item(800, false, None);
    mixamo.model.mixamo = true;
    let clips = Check {
        clips: 2,
        ..good_check()
    };
    assert_eq!(
        verdict(&mixamo, &report(800, 800, false, clips), &s).state,
        PackItemState::Fail
    );
    let rigged = item(9_000, true, Some(1_500));
    let over = verdict(&rigged, &report(9_000, 9_000, false, good_check()), &s);
    assert_eq!(over.state, PackItemState::Warn);
    assert!(over.note.unwrap_or_default().contains("not decimated"));
}

#[test]
fn an_agent_writes_only_inside_the_workspace_and_never_over_foreign_files() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path().join("ws");
    let outside = dir.path().join("elsewhere");
    fs::create_dir_all(&root).expect("root");
    fs::create_dir_all(&outside).expect("outside");
    let (pack, zip) = record::out_dir(&root, None, "village", false).expect("default");
    assert_eq!(pack, root.join("exports").join("village"));
    assert_eq!(zip, root.join("exports").join("village.zip"));
    let (inside, _) =
        record::out_dir(&root, Some("deliveries"), "village", false).expect("relative");
    assert_eq!(inside, root.join("deliveries").join("village"));
    let refused = record::out_dir(
        &root,
        Some(&outside.display().to_string()),
        "village",
        false,
    );
    assert!(
        refused
            .expect_err("outside")
            .to_string()
            .contains("outside the workspace")
    );
    let (picked, _) = record::out_dir(&root, Some(&outside.display().to_string()), "village", true)
        .expect("picked");
    assert_eq!(picked, outside.join("village"));
    let gone = dir.path().join("not-there");
    assert!(record::out_dir(&root, Some(&gone.display().to_string()), "village", true).is_err());
    fs::create_dir_all(outside.join("village")).expect("folder");
    fs::write(outside.join("village").join("notes.txt"), "mine").expect("file");
    assert!(record::out_dir(&root, Some(&outside.display().to_string()), "village", true).is_err());
    fs::write(outside.join("village").join(MARKER), "village-1").expect("marker");
    assert!(record::out_dir(&root, Some(&outside.display().to_string()), "village", true).is_ok());
}

#[test]
fn presets_keep_the_built_in_one_first_and_count_uses() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    let first = presets(root).expect("presets");
    assert_eq!(first.len(), 1);
    assert!(first[0].built_in);
    let glb = PackSettings {
        format: PackFormat::Glb,
        ..PackSettings::default()
    };
    let saved = save_preset(root, "Forest kit", &glb).expect("save");
    assert_eq!(saved[1].name, "Forest kit");
    assert_eq!(saved[1].settings.format, PackFormat::Glb);
    assert!(save_preset(root, "Plain OBJ pack", &glb).is_err());
    presets::mark_used(root, "forest-kit");
    presets::mark_used(root, presets::PLAIN_OBJ);
    let counted = presets(root).expect("presets");
    assert_eq!((counted[0].used, counted[1].used), (1, 1));
}

#[test]
fn the_contact_sheet_tiles_four_wide_and_reads_back() {
    assert_eq!(
        (
            sheet::columns(1),
            sheet::columns(4),
            sheet::columns(12),
            sheet::columns(40)
        ),
        (1, 3, 4, 6)
    );
    let png = sheet::encode(2, 1, &[255, 0, 0, 0, 0, 255]).expect("encode");
    let read = crate::refs::png::decode(&png).expect("decode");
    assert_eq!((read.width, read.height), (2, 1));
    assert_eq!(read.at(1, 0), [0, 0, 255, 255]);
}

pub(super) fn done_record(dir: &Path) -> PackRecord {
    let pack = dir.join("village-props");
    let mut items = Vec::new();
    for (key, state) in [
        ("stove", PackItemState::Pass),
        ("basket", PackItemState::Fail),
        ("rake", PackItemState::Warn),
    ] {
        let mut it = item(16_000, false, Some(3_000));
        it.model.key = key.into();
        it.model.name = key.into();
        it.state = state;
        it.triangles = Some(3_000);
        fs::create_dir_all(pack.join(key)).expect("folder");
        fs::write(pack.join(key).join(format!("{key}.obj")), "v 0 0 0\n").expect("obj");
        it.files = vec![format!("{key}/{key}.obj")];
        let png = sheet::encode(4, 4, &[90; 48]).expect("png");
        let picture = dir.join(format!("{key}.png"));
        fs::write(&picture, png).expect("picture");
        it.picture = Some(picture.display().to_string());
        items.push(it);
    }
    items[0].clips_left_out = 2;
    PackRecord {
        version: 1,
        id: "village-props-1".into(),
        name: "village-props".into(),
        label: "3 models from the library".into(),
        source: PackSource::Collection {
            collection: "village".into(),
        },
        settings: PackSettings {
            name: "village-props".into(),
            ..PackSettings::default()
        },
        dir: pack.display().to_string(),
        zip: Some(dir.join("village-props.zip").display().to_string()),
        state: PackState::Waiting,
        items,
        created_at: "2026-10-31T14:02:00Z".into(),
        finished_at: None,
        bytes: None,
        message: None,
        preset: None,
    }
}

#[test]
fn finishing_writes_the_readme_manifest_sheet_and_zip_of_what_passes() {
    let dir = tempfile::tempdir().expect("dir");
    let mut record = done_record(dir.path());
    let taken: Vec<&PackItem> = record.items.iter().take(1).collect();
    let text = readme(&record, &taken);
    assert!(text.starts_with("village-props, 1 models"));
    assert!(text.contains("Format    OBJ, Y up, metres, real size"));
    assert!(text.contains("Mixamo clips left out"));
    let json = manifest(&record, &taken);
    assert_eq!(json["up"], "Y");
    assert_eq!(json["models"][0]["files"][0], "stove.obj");
    assert_eq!(json["models"][0]["source"], "Tripo");
    finish(&mut record, ZipChoice::Passing).expect("finish");
    assert_eq!(record.state, PackState::Done);
    assert_eq!(
        record.message.as_deref(),
        Some("2 models left out of the pack.")
    );
    let pack = Path::new(&record.dir);
    assert!(pack.join("README.txt").is_file());
    assert!(pack.join("contact-sheet.png").is_file());
    assert!(
        !pack.join("basket").exists(),
        "a failed model's folder goes"
    );
    assert!(!pack.join("rake").exists(), "a warning not kept goes too");
    let zip = fs::File::open(record.zip.as_deref().expect("zip")).expect("open");
    let mut archive = zip::ZipArchive::new(zip).expect("archive");
    let mut names: Vec<String> = (0..archive.len())
        .map(|i| {
            archive
                .by_index(i)
                .expect("entry")
                .name()
                .expect("name")
                .to_string()
        })
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "village-props/README.txt",
            "village-props/contact-sheet.png",
            "village-props/manifest.json",
            "village-props/stove/stove.obj"
        ]
    );
    let mut text = String::new();
    archive
        .by_name("village-props/manifest.json")
        .expect("manifest")
        .read_to_string(&mut text)
        .expect("read");
    let read: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(read["models"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        read["notes"][0].as_str().map(|n| n.starts_with("Mixamo")),
        Some(true)
    );
    assert!(record.bytes.unwrap_or_default() > 0.0);
}

#[test]
fn finishing_with_all_keeps_the_warnings() {
    let dir = tempfile::tempdir().expect("dir");
    let mut record = done_record(dir.path());
    record.settings.zip = false;
    finish(&mut record, ZipChoice::All).expect("finish");
    assert!(Path::new(&record.dir).join("rake").is_dir());
    assert_eq!(record.zip, None);
    assert!(!dir.path().join("village-props.zip").exists());
}
