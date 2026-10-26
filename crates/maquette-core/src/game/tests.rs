use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::inbox::fixture;
use crate::project::{self, NewProject, ProjectTemplate};
use crate::workspace::{self, WorkspaceConfig};

/// A game repo: CMake, cgltf vendored, `.glb` models in props/ and
/// characters/ and a hand made texture next to one.
pub(super) fn glb_game(dir: &Path) -> PathBuf {
    let repo = dir.join("crypt-game");
    let models = repo.join("assets/models");
    fs::create_dir_all(models.join("props")).expect("props");
    fs::create_dir_all(models.join("characters")).expect("characters");
    fs::create_dir_all(repo.join("external/cgltf")).expect("cgltf");
    fs::create_dir_all(repo.join(".git")).expect("git");
    fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n").expect("head");
    fs::write(
        repo.join("CMakeLists.txt"),
        "project(Crypt)\nadd_executable(crypt)\nadd_executable(crypt_tests)\n\
         target_link_libraries(engine PRIVATE cgltf)\n",
    )
    .expect("cmake");
    fs::write(models.join("props/brazier.glb"), fixture::clean_prop()).expect("brazier");
    fs::write(models.join("props/coffin.glb"), fixture::clean_prop()).expect("coffin");
    fs::write(
        models.join("characters/hero.glb"),
        fixture::character(false),
    )
    .expect("hero");
    fs::write(models.join("characters/hero_eyes.png"), b"png").expect("eyes");
    repo
}

/// A game repo with `.obj` models whose textures sit in a folder beside.
pub(super) fn obj_game(dir: &Path) -> PathBuf {
    let repo = dir.join("maze-game");
    let models = repo.join("assets/models");
    fs::create_dir_all(&models).expect("models");
    fs::create_dir_all(repo.join("assets/textures")).expect("textures");
    fs::write(
        repo.join("CMakeLists.txt"),
        "add_executable(maze src/main.cpp)\n",
    )
    .expect("cmake");
    for name in ["stile", "gate"] {
        fs::write(
            models.join(format!("{name}.obj")),
            format!("mtllib {name}.mtl\nv 0 0 0\nv 1 0 0\nv 1 1 0\nusemtl {name}\nf 1 2 3\n"),
        )
        .expect("obj");
        fs::write(
            models.join(format!("{name}.mtl")),
            format!("newmtl {name}\nmap_Kd ../textures/{name}.png\n"),
        )
        .expect("mtl");
        fs::write(
            repo.join(format!("assets/textures/{name}.png")),
            crate::inbox::images::write_png(2, 2, &[0; 12]),
        )
        .expect("png");
    }
    fs::write(models.join("lamp.glb"), fixture::clean_prop()).expect("one glb");
    repo
}

#[test]
fn detection_tells_a_glb_game_an_obj_game_a_website_and_neither_apart() {
    let dir = tempfile::tempdir().expect("dir");
    let glb = inspect_game_repo(&glb_game(dir.path()));
    assert!(glb.exists && glb.is_git && glb.cmake);
    assert_eq!(glb.branch.as_deref(), Some("main"));
    assert_eq!(glb.targets, ["crypt", "crypt_tests"]);
    assert_eq!(glb.models_dir.as_deref(), Some("assets/models"));
    assert_eq!(glb.format, Some(GameFormat::Glb));
    assert_eq!((glb.glb_files, glb.obj_files), (3, 0));
    assert_eq!(glb.folders, ["characters", "props"]);
    assert_eq!(glb.loader.as_deref(), Some("cgltf"));
    assert!(!glb.reads_ktx2 && !glb.reads_meshopt);

    let obj = inspect_game_repo(&obj_game(dir.path()));
    assert_eq!(
        obj.format,
        Some(GameFormat::Obj),
        "two .obj outweigh one .glb"
    );
    assert_eq!((obj.glb_files, obj.obj_files), (1, 2));
    assert_eq!(obj.folders, [""], "models at the top of the model folder");
    assert_eq!(obj.loader, None);
    assert!(!obj.is_git && obj.branch.is_none());

    let site = dir.path().join("site");
    fs::create_dir_all(site.join("public")).expect("site");
    fs::write(site.join("package.json"), "{}").expect("package");
    let web = inspect_game_repo(&site);
    assert!(web.exists && !web.cmake);
    assert_eq!((web.models_dir, web.format), (None, None));
    let as_site = crate::project::inspect_site_repo(&site);
    assert!(!as_site.looks_like_game && !as_site.site.config);
    assert_eq!(as_site.site.models, "public/models");
    assert!(crate::project::inspect_site_repo(&dir.path().join("crypt-game")).looks_like_game);

    let none = inspect_game_repo(&dir.path().join("nowhere"));
    assert!(!none.exists && !none.cmake && none.format.is_none());
}

pub(super) struct Workspace {
    _dir: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) config: WorkspaceConfig,
    pub(super) repo: PathBuf,
}

pub(super) fn game_project(
    make: fn(&Path) -> PathBuf,
    format: GameFormat,
) -> (Workspace, ProjectManifest) {
    let dir = tempfile::tempdir().expect("dir");
    let repo = make(dir.path());
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Crypt".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: ProjectTarget::Game,
            site_repo: Some(repo.display().to_string()),
            source_folder: None,
            game: Some(NewGame {
                format,
                models_dir: DEFAULT_MODELS_DIR.into(),
            }),
        },
    )
    .expect("project");
    (
        Workspace {
            _dir: dir,
            root,
            config,
            repo,
        },
        manifest,
    )
}

#[test]
fn a_game_project_gets_its_settings_and_a_budget_per_folder() {
    let (_ws, manifest) = game_project(glb_game, GameFormat::Glb);
    assert_eq!(manifest.template, ProjectTemplate::GameImport);
    let settings = settings_of(&manifest).expect("game");
    assert!(!settings.bake, "Bake light starts off");
    assert_eq!(settings.export, GameExport::default());
    let folders: Vec<&str> = settings.budgets.iter().map(|b| b.folder.as_str()).collect();
    assert_eq!(folders, ["props", "characters"]);
    assert_eq!(
        (settings.budgets[0].triangles, settings.budgets[0].texture),
        (12_000, 1024)
    );
}

#[test]
fn the_scan_lists_every_model_and_import_makes_the_repo_file_v1() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let settings = settings_of(&manifest).expect("game").clone();
    let scan = scan_models(&ws.repo, &settings).expect("scan");
    let files: Vec<&str> = scan.models.iter().map(|m| m.file.as_str()).collect();
    assert_eq!(
        files,
        [
            "characters/hero.glb",
            "props/brazier.glb",
            "props/coffin.glb"
        ]
    );
    let hero = &scan.models[0];
    assert_eq!(hero.folder, "characters");
    assert!(hero.stats.as_ref().is_some_and(|s| s.joints > 0));
    assert!(scan.models.iter().all(|m| m.asset.is_none()));

    let picked = [
        "props/brazier.glb".to_string(),
        "props/nope.glb".to_string(),
    ];
    let result = import_models(&ws.root, &ws.config.layout, &manifest.id, &picked).expect("import");
    assert_eq!(result.imported, ["brazier"]);
    assert_eq!(result.skipped.len(), 1);
    let brazier = &result.project.assets[0];
    assert_eq!(brazier.name, "Brazier");
    assert_eq!(brazier.versions.len(), 1);
    assert_eq!(brazier.versions[0].file, "models/brazier_v1.glb");
    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    assert_eq!(
        fs::read(project_dir.join("models/brazier_v1.glb")).expect("copy"),
        fs::read(ws.repo.join("assets/models/props/brazier.glb")).expect("repo"),
        "version 1 is the repo file byte for byte"
    );
    let game = result.project.game.as_ref().expect("game");
    assert_eq!(game.models["brazier"].file, "props/brazier.glb");
    assert_eq!(game.models["brazier"].imported_version, Some(1));
    let record = ship::read_record(&project_dir);
    assert_eq!(record.targets["assets/models"], ["props/brazier.glb"]);

    let again =
        import_models(&ws.root, &ws.config.layout, &manifest.id, &picked[..1]).expect("again");
    assert!(
        again.imported.is_empty(),
        "an imported file is not imported twice"
    );
    let rescan = scan_models(&ws.repo, game).expect("scan");
    assert_eq!(rescan.models[1].asset.as_deref(), Some("brazier"));
}

#[test]
fn an_obj_import_keeps_its_mtl_and_textures_beside_it() {
    let (ws, manifest) = game_project(obj_game, GameFormat::Obj);
    let picked = ["stile.obj".to_string()];
    let result = import_models(&ws.root, &ws.config.layout, &manifest.id, &picked).expect("import");
    let stile = &result.project.assets[0];
    assert_eq!(stile.versions[0].file, "models/stile_v1/stile.obj");
    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    for file in ["stile.obj", "stile.mtl", "stile.png"] {
        assert!(
            project_dir.join("models/stile_v1").join(file).is_file(),
            "{file}"
        );
    }
    let record = ship::read_record(&project_dir);
    assert_eq!(record.targets["assets/models"], ["stile.mtl", "stile.obj"]);
    let index = index::build(
        &project_dir,
        &result.project,
        result.project.game.as_ref().expect("game"),
        &crate::prepare::ScenePrep::default(),
    );
    assert!(index.models[0].as_is);
    assert_eq!(
        index.models[0].copy,
        ["models/stile_v1/stile.obj", "models/stile_v1/stile.mtl"]
    );
}

#[test]
fn over_budget_models_are_flagged_and_imported_as_they_are() {
    let (ws, mut manifest) = game_project(glb_game, GameFormat::Glb);
    let mut settings = settings_of(&manifest).expect("game").clone();
    settings.budgets[0].triangles = 10;
    manifest = project::modify(&ws.root, &ws.config.layout, &manifest.id, |m| {
        m.game = Some(Box::new(settings.clone()));
        Ok(())
    })
    .expect("budget");
    let scan = scan_models(&ws.repo, &settings).expect("scan");
    let brazier = scan
        .models
        .iter()
        .find(|m| m.file == "props/brazier.glb")
        .expect("brazier");
    assert_eq!(
        brazier.budget.as_ref().map(|b| b.problems.clone()),
        Some(vec!["Over 10 triangles".to_string()])
    );
    let result = import_models(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        &["props/brazier.glb".to_string()],
    )
    .expect("import");
    let asset = &result.project.assets[0];
    assert_eq!(
        asset.stage,
        crate::Stage::Blender,
        "over budget waits for a fix-up"
    );
    let game = result.project.game.as_ref().expect("game");
    assert!(check_asset(game, asset).is_some_and(|c| c.over()));
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, bytes).expect("write");
}

#[test]
fn the_ship_plan_replaces_only_what_maquette_owns() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let picked = [
        "props/brazier.glb".to_string(),
        "props/coffin.glb".to_string(),
    ];
    let result = import_models(&ws.root, &ws.config.layout, &manifest.id, &picked).expect("import");
    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let export = project_dir.join(ship::EXPORT_DIR);
    let original = fs::read(ws.repo.join("assets/models/props/coffin.glb")).expect("coffin");
    write(&export.join("props/coffin.glb"), &original);
    write(&export.join("props/brazier.glb"), b"fixed brazier");
    write(&export.join("props/candle.glb"), b"new candle");
    write(
        &export.join("characters/hero.glb"),
        b"an export over the hand made hero",
    );

    let plan = ship_plan(&project_dir, &result.project).expect("plan");
    let change = |path: &str| {
        plan.files
            .iter()
            .find(|f| f.path == path)
            .map(|f| (f.change, f.blocked))
    };
    assert_eq!(
        change("props/coffin.glb"),
        Some((ShipChange::Unchanged, false))
    );
    assert_eq!(
        change("props/brazier.glb"),
        Some((ShipChange::Replaced, false))
    );
    assert_eq!(change("props/candle.glb"), Some((ShipChange::New, false)));
    assert_eq!(
        change("characters/hero.glb"),
        Some((ShipChange::HandMade, true))
    );
    let hand: Vec<&str> = plan.hand_made.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(hand, ["characters/hero_eyes.png"]);
    assert_eq!(plan.record_after, 3, "brazier, coffin and the new candle");
    assert_eq!(plan.branch.as_deref(), Some("main"));
    assert!(plan.checks.iter().all(|c| c.ok));
}

#[test]
fn ship_game_writes_only_its_own_files_and_never_removes_one() {
    let Some(node) = crate::toolchain::find_on_path(&["node"], &crate::ProbeEnv::from_system())
    else {
        return;
    };
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    import_models(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        &["props/brazier.glb".to_string()],
    )
    .expect("import");
    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let export = project_dir.join(ship::EXPORT_DIR);
    write(&export.join("props/brazier.glb"), b"fixed brazier");
    write(&export.join("props/candle.glb"), b"new candle");
    write(
        &export.join("characters/hero.glb"),
        b"an export over the hand made hero",
    );
    write(
        &project_dir.join("tools/ship_game.mjs"),
        include_bytes!("../../starter/ship_game.mjs"),
    );
    let out = std::process::Command::new(node)
        .args(["tools/ship_game.mjs", "--from", "web/game", "--game"])
        .arg(&ws.repo)
        .args([
            "--models",
            "assets/models",
            "--record",
            "web/ship-record.json",
        ])
        .current_dir(&project_dir)
        .output()
        .expect("node");
    let log = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let models = ws.repo.join("assets/models");
    assert_eq!(
        fs::read(models.join("props/brazier.glb")).expect("brazier"),
        b"fixed brazier"
    );
    assert_eq!(
        fs::read(models.join("props/candle.glb")).expect("candle"),
        b"new candle"
    );
    assert_eq!(
        fs::read(models.join("characters/hero.glb")).expect("hero"),
        fixture::character(false),
        "a hand made file stays"
    );
    assert!(
        models.join("props/coffin.glb").is_file(),
        "nothing is removed"
    );
    assert!(log.contains("characters/hero.glb is hand made"), "{log}");
    assert!(
        log.contains("1 new, 1 replaced, 0 unchanged, 1 hand made"),
        "{log}"
    );
    let record = ship::read_record(&project_dir);
    assert_eq!(
        record.targets["assets/models"],
        ["props/brazier.glb", "props/candle.glb"]
    );
}

#[test]
fn export_settings_are_checked_and_follow_into_the_pipeline() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let mut settings = settings_of(&manifest).expect("game").clone();
    settings.export.texture_cap = 1000;
    assert!(
        validate(&settings).is_err(),
        "a cap off the list is refused"
    );
    settings.export.texture_cap = 2048;
    settings.models_dir = "../elsewhere".into();
    assert!(
        validate(&settings).is_err(),
        "the model folder stays inside the repo"
    );
    settings.models_dir = DEFAULT_MODELS_DIR.into();
    assert!(validate(&settings).is_ok());

    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let def = crate::pipeline::ensure(&project_dir, &manifest).expect("pipeline");
    let ids: Vec<&str> = def.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        ["build", "export", "game", "ship"],
        "no bake, posters or compress"
    );
    for script in def.steps.iter().filter_map(|s| s.script.as_deref()) {
        assert!(project_dir.join(script).is_file(), "{script} was written");
    }
    let mut def = def;
    settings.bake = true;
    settings.export.textures = GameTextures::Jpg;
    assert!(crate::pipeline::template::fit_game(&mut def, &settings));
    let ids: Vec<&str> = def.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["build", "bake", "export", "game", "ship"]);
    let export = def.step("export").expect("export");
    assert_eq!(export.depends_on, ["bake"]);
    let args = export.args.join(" ");
    assert!(args.contains("blender/game_baked.blend"), "{args}");
    assert!(
        args.contains("--textures jpg --cap 2048 --bake blender/bake"),
        "{args}"
    );
    assert!(
        !crate::pipeline::template::fit_game(&mut def, &settings),
        "once is enough"
    );
    settings.bake = false;
    assert!(crate::pipeline::template::fit_game(&mut def, &settings));
    assert!(def.step("bake").is_none());
    assert_eq!(def.step("export").expect("export").depends_on, ["build"]);
}

#[test]
fn a_new_asset_goes_to_the_folder_of_its_kind() {
    let (_ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let settings = settings_of(&manifest).expect("game");
    let mut m = manifest.clone();
    let prop = crate::assets::add(
        &mut m,
        crate::NewAsset {
            name: "Crypt candelabra".into(),
            kind: AssetKind::Prop,
            stage: crate::Stage::Concept,
        },
    )
    .expect("prop");
    assert_eq!(file_of(settings, &prop), "props/crypt_candelabra.glb");
    let mut obj = settings.clone();
    obj.format = GameFormat::Obj;
    obj.budgets.clear();
    assert_eq!(file_of(&obj, &prop), "crypt_candelabra.obj");
}

#[test]
fn saved_settings_are_checked_and_budget_fixups_bring_a_model_within_its_folder() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    import_models(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        &["props/brazier.glb".to_string()],
    )
    .expect("import");
    let refused = save_settings(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        GameSettingsChange {
            models_dir: Some("../elsewhere".into()),
            ..GameSettingsChange::default()
        },
    );
    assert!(refused.is_err(), "the model folder stays inside the repo");

    let mut budgets = settings_of(&manifest).expect("game").budgets.clone();
    let props = budgets
        .iter_mut()
        .find(|b| b.folder == "props")
        .expect("props");
    props.triangles = 100;
    let first = save_settings(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        GameSettingsChange {
            bake: Some(true),
            budgets: Some(budgets),
            ..GameSettingsChange::default()
        },
    )
    .expect("save");
    assert_eq!(first.game.as_ref().map(|g| g.bake), Some(true));
    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let def = crate::pipeline::load(&project_dir)
        .expect("load")
        .expect("pipeline");
    assert!(
        def.steps.iter().any(|s| s.id == "bake"),
        "Bake light follows the setting"
    );

    // The fixture is tiny; say its model holds more than the budget.
    let saved = project::modify(&ws.root, &ws.config.layout, &manifest.id, |m| {
        if let Some(stats) = m.assets[0].versions[0].stats.as_mut() {
            stats.triangles = 500;
        }
        Ok(())
    })
    .expect("stats");
    assert_eq!(saved.game.as_ref().map(|g| g.bake), Some(true));
    let changed = add_budget_fixups(&project_dir, &saved).expect("fix-ups");
    assert_eq!(changed, ["brazier"]);
    let prep = crate::prepare::load(&project_dir).expect("prepare");
    assert_eq!(
        prep.models["brazier"].fixups,
        [crate::prepare::Fixup::Decimate { triangles: 100 }]
    );
    let check = BudgetCheck {
        folder: "props".into(),
        triangles: 14_600,
        max_triangles: 12_000,
        texture: 1024,
        max_texture: 1024,
        rigged: false,
        problems: vec!["Over 12k triangles".into()],
    };
    assert_eq!(
        settings::fixups_for(&check),
        [crate::prepare::Fixup::Decimate { triangles: 12_000 }]
    );
    let website = project::create(
        &ws.root,
        &ws.config.layout,
        NewProject {
            name: "Site".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: ProjectTarget::Website,
            site_repo: None,
            source_folder: None,
            game: None,
        },
    )
    .expect("website");
    assert!(
        save_settings(
            &ws.root,
            &ws.config.layout,
            &website.id,
            GameSettingsChange::default()
        )
        .is_err()
    );
}

#[test]
fn a_rigged_model_gets_no_decimate_but_still_shrinks_its_textures() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    import_models(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        &["characters/hero.glb".to_string()],
    )
    .expect("import");
    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let over = |triangles: u32, max_texture: u32| {
        project::modify(&ws.root, &ws.config.layout, &manifest.id, |m| {
            if let Some(stats) = m.assets[0].versions[0].stats.as_mut() {
                stats.triangles = triangles;
                stats.max_texture = max_texture;
            }
            Ok(())
        })
        .expect("stats")
    };
    let saved = over(115_000, 4096);
    let game = saved.game.as_ref().expect("game");
    let check = check_asset(game, &saved.assets[0]).expect("check");
    assert!(check.rigged && check.over(), "{check:?}");
    assert_eq!(
        add_budget_fixups(&project_dir, &saved).expect("fix-ups"),
        ["hero"]
    );
    let prep = crate::prepare::load(&project_dir).expect("prepare");
    assert_eq!(
        prep.models["hero"].fixups,
        [crate::prepare::Fixup::ShrinkTextures { max_size: 2048 }],
        "Shrink textures, never Decimate, on a rigged model"
    );

    let saved = over(115_000, 1024);
    let check = check_asset(saved.game.as_ref().expect("game"), &saved.assets[0]).expect("check");
    assert!(settings::fixups_for(&check).is_empty());
    assert!(
        add_budget_fixups(&project_dir, &saved)
            .expect("fix-ups")
            .is_empty(),
        "only its triangles are over: reduced by hand"
    );
}

#[test]
fn the_index_caps_each_model_at_its_folder_budget_and_the_export_cap_elsewhere() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let picked = [
        "props/brazier.glb".to_string(),
        "characters/hero.glb".to_string(),
    ];
    let result = import_models(&ws.root, &ws.config.layout, &manifest.id, &picked).expect("import");
    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let mut settings = result.project.game.as_deref().expect("game").clone();
    settings.export.texture_cap = 512;
    for budget in &mut settings.budgets {
        if budget.folder == "characters" {
            budget.texture = 2048;
        }
    }
    let cap = |s: &GameSettings, asset: &str| {
        index::build(&project_dir, &result.project, s, &Default::default())
            .models
            .iter()
            .find(|m| m.asset == asset)
            .map(|m| m.texture_cap)
    };
    assert_eq!(cap(&settings, "hero"), Some(2048), "its folder allows 2048");
    assert_eq!(cap(&settings, "brazier"), Some(1024));
    settings.budgets.retain(|b| b.folder != "props");
    assert_eq!(
        cap(&settings, "brazier"),
        Some(512),
        "a folder with no budget takes the Game export cap"
    );
    let json = serde_json::to_value(index::build(
        &project_dir,
        &result.project,
        &settings,
        &Default::default(),
    ))
    .expect("json");
    assert_eq!(json["textureCap"], 512);
    assert_eq!(json["models"][1]["textureCap"], 2048, "{json}");
}

#[test]
fn the_ship_plan_says_which_replaced_files_are_the_imported_originals() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let picked = [
        "props/brazier.glb".to_string(),
        "props/coffin.glb".to_string(),
    ];
    let result = import_models(&ws.root, &ws.config.layout, &manifest.id, &picked).expect("import");
    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let export = project_dir.join(ship::EXPORT_DIR);
    write(&export.join("props/brazier.glb"), b"fixed brazier");
    write(&export.join("props/coffin.glb"), b"fixed coffin");
    // A Ship before wrote the coffin, so the game's file is no longer the import.
    write(
        &ws.repo.join("assets/models/props/coffin.glb"),
        b"shipped coffin",
    );
    let plan = ship_plan(&project_dir, &result.project).expect("plan");
    let file = |path: &str| plan.files.iter().find(|f| f.path == path).expect(path);
    assert_eq!(file("props/brazier.glb").change, ShipChange::Replaced);
    assert!(
        file("props/brazier.glb").imported,
        "the repo still holds the original"
    );
    assert_eq!(file("props/coffin.glb").change, ShipChange::Replaced);
    assert!(!file("props/coffin.glb").imported);
    assert_eq!(plan.uncommitted, None, "the plan alone does not run git");
}

struct FakeGit(&'static str);

impl crate::CommandRunner for FakeGit {
    fn run(
        &self,
        _program: &Path,
        args: &[&str],
        _path_env: &std::ffi::OsStr,
        _timeout: std::time::Duration,
    ) -> Result<crate::toolchain::ProbeOutput, crate::toolchain::ProbeError> {
        assert!(
            args.contains(&"--no-optional-locks"),
            "git writes nothing: {args:?}"
        );
        Ok(crate::toolchain::ProbeOutput {
            status_ok: true,
            stdout: self.0.to_string(),
            stderr: String::new(),
        })
    }
}

#[test]
fn uncommitted_counts_what_git_status_lists_and_skips_a_folder_without_git() {
    let dir = tempfile::tempdir().expect("dir");
    let repo = glb_game(dir.path());
    let git = Path::new("git");
    let path = std::ffi::OsStr::new("");
    let dirty = FakeGit(" M src/main.cpp\n?? notes.txt\n");
    assert_eq!(detect::uncommitted(&repo, git, &dirty, path), Some(2));
    assert_eq!(detect::uncommitted(&repo, git, &FakeGit(""), path), Some(0));
    let maze = obj_game(dir.path());
    assert_eq!(detect::uncommitted(&maze, git, &dirty, path), None);
}
