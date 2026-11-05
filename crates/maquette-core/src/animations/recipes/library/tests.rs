use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::animations::recipes::BaseStatus;
use crate::animations::tests::library_clip;
use crate::animations::{clip_dir, store};
use crate::recipes::{BlankKind, LayerKind, LayerSettings, RecipeBlank, RecipeChange, RecipeLayer};
use crate::workspace::WorkspaceConfig;

struct Ws {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    config: WorkspaceConfig,
    /// Frostveil, with a Lantern prop and the recipe `reach-out`.
    frost: String,
    lantern: String,
    /// Moon, with no props.
    moon: String,
}

impl Ws {
    fn dir(&self, project: &str) -> PathBuf {
        crate::project::dir(&self.root, &self.config.layout, project).expect("dir")
    }

    fn manifest(&self, project: &str) -> crate::project::ProjectManifest {
        crate::project::load(&self.root, &self.config.layout, project).expect("manifest")
    }

    fn recipe(&self, id: &str) -> RecipeFile {
        recipes::load_file(&recipes::path(&self.dir(&self.frost), id).expect("path")).expect("load")
    }

    fn change(&self, layers: Vec<RecipeLayer>) {
        let version = recipes::version(&self.dir(&self.frost), "reach-out").expect("version");
        recipes::update(
            &self.dir(&self.frost),
            &self.manifest(&self.frost),
            "reach-out",
            RecipeChange {
                layers: Some(layers),
                expected: Some(version),
                ..RecipeChange::default()
            },
        )
        .expect("change");
    }

    fn plan(&self, entry: Option<&str>) -> RecipeSavePlan {
        plan_save(
            &self.root,
            &self.config.layout,
            &self.frost,
            "reach-out",
            entry,
        )
        .expect("plan")
    }

    fn save(&self, plan: &RecipeSavePlan) -> CoreResult<RecipeSaved> {
        save_back(
            &self.root,
            &self.config.layout,
            &RecipeSave {
                project: self.frost.clone(),
                recipe: "reach-out".into(),
                entry: plan.entry.clone(),
                expected: plan.entry_version.clone(),
                recipe_expected: Some(plan.recipe_version.clone()),
            },
        )
    }
}

fn layer(id: &str, kind: LayerKind, bone: &str, target: Option<&str>) -> RecipeLayer {
    RecipeLayer {
        id: id.into(),
        kind,
        bone: bone.into(),
        settings: LayerSettings {
            target: target.map(Into::into),
            ..LayerSettings::default()
        },
    }
}

fn project(root: &Path, config: &WorkspaceConfig, name: &str) -> String {
    crate::project::create(
        root,
        &config.layout,
        crate::project::NewProject {
            name: name.into(),
            template: crate::project::ProjectTemplate::FloatingDiorama,
            target: crate::game::ProjectTarget::Website,
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project")
    .id
}

fn ws() -> Ws {
    let tmp = tempfile::tempdir().expect("temp");
    let root = tmp.path().to_path_buf();
    let config = WorkspaceConfig::default();
    store(
        &root,
        &library_clip("still", "Standing Still", "Singles", "h1"),
    )
    .expect("clip");
    let frost = project(&root, &config, "Frostveil");
    let moon = project(&root, &config, "Moon");
    let mut lantern = String::new();
    crate::project::modify(&root, &config.layout, &frost, |m| {
        let asset = crate::assets::add(
            m,
            crate::assets::NewAsset {
                name: "Lantern".into(),
                kind: crate::assets::AssetKind::Prop,
                stage: crate::assets::Stage::Refs,
            },
        )?;
        lantern = asset.id;
        Ok(())
    })
    .expect("lantern");
    let w = Ws {
        _tmp: tmp,
        root,
        config,
        frost,
        lantern,
        moon,
    };
    crate::animations::recipes::create(&w.root, &w.dir(&w.frost), "still", Some("Reach out"))
        .expect("recipe");
    recipes::update(
        &w.dir(&w.frost),
        &w.manifest(&w.frost),
        "reach-out",
        RecipeChange {
            blanks: Some(vec![
                RecipeBlank {
                    id: "lamp".into(),
                    label: "The lantern".into(),
                    kind: BlankKind::Prop,
                    prop: Some(w.lantern.clone()),
                    point: None,
                },
                RecipeBlank {
                    id: "gate".into(),
                    label: "The gate".into(),
                    kind: BlankKind::Point,
                    prop: None,
                    point: Some([0.0, 1.5, 2.0]),
                },
            ]),
            layers: Some(vec![layer(
                "l-1",
                LayerKind::Reach,
                "rightHand",
                Some("lamp"),
            )]),
            ..RecipeChange::default()
        },
    )
    .expect("fill");
    w
}

#[test]
fn a_save_back_makes_a_library_recipe_at_r1_and_links_the_project_recipe() {
    let w = ws();
    let plan = w.plan(None);
    assert_eq!((plan.entry.as_deref(), plan.revision), (None, 1));
    assert!(
        plan.changes[0].contains("1 change and 2 blanks"),
        "{:?}",
        plan.changes
    );
    let saved = w.save(&plan).expect("save");
    assert_eq!((saved.entry.as_str(), saved.revision), ("reach-out", 1));
    assert!(saved.linked);

    let (entry, version) = load_entry(&w.root, "reach-out").expect("entry");
    assert_eq!(version, saved.entry_version);
    let rev = &entry.revisions[0];
    assert_eq!(rev.file, "r1.recipe.json");
    assert_eq!(rev.project.as_deref(), Some(w.frost.as_str()));
    assert_eq!(rev.recipe.as_deref(), Some("reach-out"));
    let text = fs::read_to_string(
        entry_dir(&w.root, "reach-out")
            .expect("dir")
            .join("r1.recipe.json"),
    )
    .expect("r1");
    assert!(text.contains("\"revision\": 1"), "{text}");
    for word in ["keys", "mesh", "skeleton", "fbx", "\"entry\""] {
        assert!(!text.contains(word), "{word} in {text}");
    }
    let linked = w.recipe("reach-out");
    assert_eq!(linked.revision, 1);
    assert_eq!(
        linked.from.map(|f| (f.entry, f.revision)),
        Some(("reach-out".into(), 1))
    );
}

#[test]
fn each_save_adds_a_revision_says_what_changed_and_never_reuses_a_number() {
    let w = ws();
    w.save(&w.plan(None)).expect("r1");
    let same = w.plan(Some("reach-out"));
    assert!(same.same);
    assert!(
        w.save(&same)
            .expect_err("nothing new")
            .to_string()
            .contains("nothing to save")
    );

    w.change(vec![
        layer("l-1", LayerKind::Reach, "leftHand", Some("lamp")),
        layer("l-2", LayerKind::LookAt, "head", Some("gate")),
    ]);
    let plan = w.plan(Some("reach-out"));
    assert_eq!((plan.latest, plan.revision), (Some(1), 2));
    assert!(
        plan.changes
            .iter()
            .any(|c| c == "Edits the reach on leftHand (l-1): bone."),
        "{:?}",
        plan.changes
    );
    assert!(
        plan.changes
            .iter()
            .any(|c| c == "Adds the look at on head (l-2)."),
        "{:?}",
        plan.changes
    );
    assert_eq!(w.save(&plan).expect("r2").revision, 2);

    let behind = compare(&w.root, &w.config.layout, &w.frost, "reach-out-2");
    assert!(behind.is_err(), "no such recipe");
    let current = compare(&w.root, &w.config.layout, &w.frost, "reach-out").expect("compare");
    assert_eq!((current.revision, current.latest), (2, 2));
    assert!(current.changes.is_empty(), "{:?}", current.changes);
    let entry = remove_revision(&w.root, "reach-out", 2, None).expect("remove r2");
    assert_eq!(entry.revisions.len(), 1);
    assert_eq!(entry.last_revision, 2, "r2 is never handed out again");
    let again = w.plan(Some("reach-out"));
    assert_eq!(again.revision, 3);
    assert_eq!(w.save(&again).expect("r3").revision, 3);
    assert!(
        remove_revision(&w.root, "reach-out", 1, None).is_ok(),
        "two revisions, one can go"
    );
    let only = remove_revision(&w.root, "reach-out", 3, None).expect_err("last one");
    assert!(
        only.to_string().contains("Remove the whole recipe"),
        "{only}"
    );
    let went = remove_entry(&w.root, "reach-out", None).expect("remove entry");
    assert!(went.starts_with(".removed/reach-out-"), "{went}");
    assert!(list(&w.root).expect("list").is_empty());
}

#[test]
fn a_save_to_an_entry_that_moved_since_the_sheet_opened_is_refused_and_writes_nothing() {
    let w = ws();
    w.save(&w.plan(None)).expect("r1");
    w.change(vec![layer(
        "l-1",
        LayerKind::Reach,
        "leftHand",
        Some("lamp"),
    )]);
    let plan = w.plan(Some("reach-out"));
    // Someone else saves a revision meanwhile.
    let mut entry = load_entry(&w.root, "reach-out").expect("entry").0;
    entry.name = "Renamed elsewhere".into();
    crate::json::write_json(
        &entry_dir(&w.root, "reach-out")
            .expect("dir")
            .join(ENTRY_FILE),
        &entry,
    )
    .expect("write");
    let moved = w.save(&plan).expect_err("moved");
    assert!(matches!(moved, CoreError::Moved(_)), "{moved:?}");
    assert!(
        !entry_dir(&w.root, "reach-out")
            .expect("dir")
            .join("r2.recipe.json")
            .exists()
    );
    let mut no_version = plan.clone();
    no_version.entry_version = None;
    assert!(
        w.save(&no_version)
            .expect_err("needs version")
            .to_string()
            .contains("needs the version")
    );
}

#[test]
fn using_a_library_recipe_opens_props_the_project_lacks_and_keeps_points() {
    let w = ws();
    w.save(&w.plan(None)).expect("r1");
    let used = use_recipe(
        &w.root,
        &w.config.layout,
        &UseRecipe {
            project: w.moon.clone(),
            entry: "reach-out".into(),
            revision: None,
            name: None,
        },
    )
    .expect("use");
    let doc = &used.doc;
    assert_eq!(doc.id, "reach-out");
    assert_eq!(doc.base.status, BaseStatus::Found);
    let lamp = doc
        .recipe
        .blanks
        .iter()
        .find(|b| b.id == "lamp")
        .expect("lamp");
    assert_eq!(lamp.prop, None, "Moon has no lantern");
    let gate = doc
        .recipe
        .blanks
        .iter()
        .find(|b| b.id == "gate")
        .expect("gate");
    assert_eq!(gate.point, Some([0.0, 1.5, 2.0]));
    assert!(
        used.notes
            .iter()
            .any(|n| n.contains("The blank lamp") && n.contains("open")),
        "{:?}",
        used.notes
    );
    assert_eq!(doc.recipe.from.as_ref().map(|f| f.revision), Some(1));
    // The same project keeps its own prop, and a second copy takes a fresh id.
    let again = use_recipe(
        &w.root,
        &w.config.layout,
        &UseRecipe {
            project: w.frost.clone(),
            entry: "reach-out".into(),
            revision: Some(1),
            name: None,
        },
    )
    .expect("use in frostveil");
    assert_eq!(again.doc.id, "reach-out-2");
    assert!(
        again
            .notes
            .iter()
            .any(|n| n.contains("so this one is reach-out-2")),
        "{:?}",
        again.notes
    );
    let lamp = again
        .doc
        .recipe
        .blanks
        .iter()
        .find(|b| b.id == "lamp")
        .expect("lamp");
    assert_eq!(lamp.prop.as_deref(), Some(w.lantern.as_str()));
    assert!(
        use_recipe(
            &w.root,
            &w.config.layout,
            &UseRecipe {
                project: w.moon.clone(),
                entry: "reach-out".into(),
                revision: Some(9),
                name: None
            },
        )
        .is_err()
    );
}

#[test]
fn a_used_recipe_says_when_its_base_clip_is_missing_or_has_other_keys() {
    let w = ws();
    w.save(&w.plan(None)).expect("r1");
    let request = UseRecipe {
        project: w.moon.clone(),
        entry: "reach-out".into(),
        revision: None,
        name: Some("Reach again".into()),
    };
    let mut clip = crate::animations::load(&w.root, "still").expect("clip");
    clip.hash = "h2".into();
    store(&w.root, &clip).expect("store");
    let differs = use_recipe(&w.root, &w.config.layout, &request).expect("differs");
    assert_eq!(differs.doc.recipe.name, "Reach again");
    assert_eq!(differs.doc.base.status, BaseStatus::HashDiffers);
    assert!(
        differs.notes.iter().any(|n| n.contains("other keys")),
        "{:?}",
        differs.notes
    );

    fs::remove_dir_all(clip_dir(&w.root, "still").expect("dir")).expect("gone");
    let missing = use_recipe(&w.root, &w.config.layout, &request).expect("still made");
    assert_eq!(missing.doc.base.status, BaseStatus::Missing);
    assert!(
        missing
            .notes
            .iter()
            .any(|n| n.contains("no clip called Standing Still")),
        "{:?}",
        missing.notes
    );
}

fn exported(w: &Ws) -> PathBuf {
    let to = w.root.join("out").join("Reach out.recipe.json");
    fs::create_dir_all(to.parent().expect("parent")).expect("out");
    let source = ExportSource {
        project: Some(w.frost.clone()),
        recipe: Some("reach-out".into()),
        ..ExportSource::default()
    };
    export(&w.root, &w.config.layout, &source, &to).expect("export");
    to
}

#[test]
fn an_export_carries_no_library_link_and_no_path_and_imports_back() {
    let w = ws();
    w.save(&w.plan(None)).expect("r1");
    let to = exported(&w);
    let text = fs::read_to_string(&to).expect("text");
    assert!(!text.contains("\"from\""), "{text}");
    assert!(!text.contains(&w.root.display().to_string()), "{text}");
    assert!(text.contains("\"version\": 1"));

    let into_moon = import_to_project(&w.root, &w.config.layout, &w.moon, &to).expect("import");
    assert_eq!(into_moon.doc.id, "reach-out");
    assert_eq!(into_moon.doc.recipe.from, None);
    let lamp = into_moon
        .doc
        .recipe
        .blanks
        .iter()
        .find(|b| b.id == "lamp")
        .expect("lamp");
    assert_eq!(lamp.prop, None);
    let clash = import_to_project(&w.root, &w.config.layout, &w.frost, &to).expect("clash");
    assert_eq!(clash.doc.id, "reach-out-2");
    assert!(
        clash.notes.iter().any(|n| n.contains("reach-out-2")),
        "{:?}",
        clash.notes
    );

    let first = import_to_library(&w.root, &to).expect("library");
    assert_eq!(first.entry, "reach-out-2", "reach-out is the saved one");
    let doc = get(&w.root, &first.entry, None).expect("get");
    assert_eq!(doc.revision, 1);
    assert_eq!(
        doc.entry.revisions[0].imported.as_deref(),
        Some("Reach out.recipe.json")
    );
    let library_export = w.root.join("out").join("library.recipe.json");
    let source = ExportSource {
        entry: Some(first.entry.clone()),
        ..ExportSource::default()
    };
    export(&w.root, &w.config.layout, &source, &library_export).expect("export revision");
    assert!(
        !fs::read_to_string(&library_export)
            .expect("text")
            .contains("\"from\"")
    );
}

#[test]
fn an_import_is_checked_whole_and_capped_before_it_is_parsed() {
    let w = ws();
    let to = exported(&w);
    let good: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&to).expect("text")).expect("json");
    let out = w.root.join("out");

    let mut many = good.clone();
    let layer = good["layers"][0].clone();
    many["layers"] = serde_json::Value::Array(
        (0..33)
            .map(|n| {
                let mut l = layer.clone();
                l["id"] = serde_json::json!(format!("l-{n}"));
                l
            })
            .collect(),
    );
    let too_many = out.join("many.recipe.json");
    fs::write(&too_many, many.to_string()).expect("write");
    let refused =
        import_to_project(&w.root, &w.config.layout, &w.moon, &too_many).expect_err("cap");
    assert!(
        refused.to_string().contains("at most 32 changes"),
        "{refused}"
    );

    let big = out.join("big.recipe.json");
    fs::write(&big, format!("{}{}", " ".repeat(300 * 1024), good)).expect("write");
    let refused = read_import(&big).expect_err("size");
    assert!(refused.to_string().contains("256 KB"), "{refused}");

    let mut sneaky = good.clone();
    sneaky["from"] = serde_json::json!({ "entry": "reach-out", "revision": 7 });
    sneaky["name"] = serde_json::json!("../../etc/passwd");
    let nasty = out.join("..%2F..%2Fescape.recipe.json");
    fs::write(&nasty, sneaky.to_string()).expect("write");
    let landed = import_to_project(&w.root, &w.config.layout, &w.moon, &nasty).expect("import");
    assert!(recipes::is_recipe_id(&landed.doc.id), "{}", landed.doc.id);
    assert!(
        w.dir(&w.moon)
            .join("recipes")
            .join(format!("{}.recipe.json", landed.doc.id))
            .is_file()
    );
    assert_eq!(landed.doc.recipe.from, None, "an imported link is dropped");
    let in_library = import_to_library(&w.root, &nasty).expect("library");
    assert!(recipes::is_recipe_id(&in_library.entry));
    assert!(
        entry_dir(&w.root, &in_library.entry)
            .expect("dir")
            .starts_with(dir(&w.root))
    );

    let mut differs = good;
    differs["base"]["hash"] = serde_json::json!("not-h1");
    let other = out.join("other.recipe.json");
    fs::write(&other, differs.to_string()).expect("write");
    let warned = import_to_project(&w.root, &w.config.layout, &w.moon, &other).expect("warns");
    assert_eq!(warned.doc.base.status, BaseStatus::HashDiffers);
    assert!(
        warned.notes.iter().any(|n| n.contains("other keys")),
        "{:?}",
        warned.notes
    );

    let not_json = out.join("notes.txt");
    fs::write(&not_json, "{}").expect("write");
    assert!(read_import(&not_json).is_err());
}

#[test]
fn agents_import_only_inside_the_workspace_and_export_only_to_a_fresh_file() {
    let w = ws();
    let to = exported(&w);
    let inside = agent_import_path(&w.root, "out/Reach out.recipe.json").expect("inside");
    assert_eq!(inside, to.canonicalize().expect("real"));
    assert!(agent_import_path(&w.root, &to.display().to_string()).is_ok());
    let outside = tempfile::tempdir().expect("outside");
    let elsewhere = outside.path().join("x.recipe.json");
    fs::write(&elsewhere, "{}").expect("write");
    assert!(agent_import_path(&w.root, &elsewhere.display().to_string()).is_err());
    assert!(agent_import_path(&w.root, "../x.recipe.json").is_err());
    assert!(agent_import_path(&w.root, "out").is_err(), "a folder");

    let source = ExportSource {
        project: Some(w.frost.clone()),
        recipe: Some("reach-out".into()),
        ..ExportSource::default()
    };
    let first = agent_export(&w.root, &w.config.layout, &source, "Reach out").expect("first");
    assert_eq!(first.file, "reach-out.recipe.json");
    let taken = w.root.join("exports/recipes/reach-out-2.recipe.json");
    fs::write(&taken, "taken").expect("write");
    let third = agent_export(
        &w.root,
        &w.config.layout,
        &source,
        "../../Reach out.recipe.json",
    )
    .expect("third");
    assert_eq!(
        third.file, "reach-out-3.recipe.json",
        "a taken name is skipped"
    );
    assert_eq!(fs::read_to_string(&taken).expect("kept"), "taken");
    let device = agent_export(&w.root, &w.config.layout, &source, "CON").expect("con");
    assert_eq!(
        device.file, "con-1.recipe.json",
        "never a name Windows keeps"
    );
}

#[test]
fn agent_exports_at_once_never_replace_each_other() {
    let w = ws();
    let source = ExportSource {
        project: Some(w.frost.clone()),
        recipe: Some("reach-out".into()),
        ..ExportSource::default()
    };
    let files: Vec<String> = std::thread::scope(|s| {
        let runs: Vec<_> = (0..8)
            .map(|_| s.spawn(|| agent_export(&w.root, &w.config.layout, &source, "Wave")))
            .collect();
        runs.into_iter()
            .map(|r| r.join().expect("thread").expect("export").file)
            .collect()
    });
    let mut unique = files.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 8, "{files:?}");
    let on_disk = fs::read_dir(w.root.join("exports/recipes"))
        .expect("folder")
        .count();
    assert_eq!(on_disk, 8);
}

#[test]
fn the_list_reads_every_entry_and_shows_a_broken_one_with_its_problem() {
    let w = ws();
    w.save(&w.plan(None)).expect("r1");
    let broken = dir(&w.root).join("broken");
    fs::create_dir_all(&broken).expect("dir");
    fs::write(broken.join(ENTRY_FILE), "{ not json").expect("write");
    let listed = list(&w.root).expect("list");
    assert_eq!(listed.len(), 2);
    let good = listed.iter().find(|l| l.id == "reach-out").expect("good");
    assert_eq!((good.latest, good.layers, good.blanks), (1, 1, 2));
    assert_eq!(good.base, "Standing Still");
    assert!(
        listed
            .iter()
            .any(|l| l.id == "broken" && l.problem.is_some())
    );
    let mut entry = load_entry(&w.root, "reach-out").expect("entry").0;
    entry.revisions[0].file = "../../escape.recipe.json".into();
    crate::json::write_json(
        &entry_dir(&w.root, "reach-out")
            .expect("dir")
            .join(ENTRY_FILE),
        &entry,
    )
    .expect("write");
    assert!(
        load_entry(&w.root, "reach-out").is_err(),
        "a revision file is only rN.recipe.json"
    );
    assert!(get(&w.root, "../reach-out", None).is_err());
}

#[test]
fn a_recipe_behind_the_library_reads_what_the_newer_revision_changes() {
    let w = ws();
    w.save(&w.plan(None)).expect("r1");
    let used = use_recipe(
        &w.root,
        &w.config.layout,
        &UseRecipe {
            project: w.moon.clone(),
            entry: "reach-out".into(),
            revision: None,
            name: None,
        },
    )
    .expect("use");
    w.change(vec![
        layer("l-1", LayerKind::Reach, "rightHand", Some("lamp")),
        layer("l-2", LayerKind::LookAt, "head", Some("gate")),
    ]);
    let plan = w.plan(Some("reach-out"));
    assert!(!plan.same);
    w.save(&plan).expect("r2");
    let behind = compare(&w.root, &w.config.layout, &w.moon, &used.doc.id).expect("compare");
    assert_eq!((behind.revision, behind.latest), (1, 2));
    assert_eq!(behind.changes, ["Adds the look at on head (l-2)."]);
    let made_here =
        crate::animations::recipes::create(&w.root, &w.dir(&w.moon), "still", Some("Own"))
            .expect("own");
    assert!(compare(&w.root, &w.config.layout, &w.moon, &made_here.id).is_err());
}
