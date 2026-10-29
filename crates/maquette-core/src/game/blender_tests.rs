//! The game pipeline's starters in a real, headless Blender and node:
//! Build models, Export and Game export on imported fixture models
//! (skipped without Blender or node).

use std::fs;
use std::path::Path;
use std::process::Command;

use super::tests::{game_project, glb_game, obj_game};
use super::*;
use crate::prepare::{self, ModelPrep, RealSize, SizeAxis, SizeUnit};
use crate::project;
use crate::test_blender::{blender, glb_json};
use crate::toolchain::{ProbeEnv, find_on_path};

fn path(p: &Path) -> String {
    p.display().to_string()
}

/// Runs `program` in the project folder, as the pipeline runs a step, and
/// returns what it printed, failing the test when it fails.
fn run_in(project_dir: &Path, program: &Path, args: &[&str]) -> String {
    let out = Command::new(program)
        .args(args)
        .current_dir(project_dir)
        .output()
        .expect("start");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "{} failed:\n{text}",
        program.display()
    );
    text
}

/// Runs the build, export and game export steps the way the pipeline does,
/// in the project folder.
pub(super) fn run_chain(project_dir: &Path, settings: &GameSettings) -> Option<String> {
    let blender = blender()?;
    let node = find_on_path(&["node"], &ProbeEnv::from_system())?;
    let tools = project_dir.join("tools");
    let built = project_dir.join("blender/game_built.blend");
    let headless = ["-b", "--factory-startup", "--python-exit-code", "1"];
    let mut log = run_in(
        project_dir,
        &blender,
        &[
            &headless[..],
            &[
                "--python",
                &path(&tools.join("build_models.py")),
                "--",
                "--index",
                &path(&project_dir.join(index::INDEX_FILE)),
                "--prepare",
                &path(&project_dir.join("prepare.json")),
                "--out",
                &path(&built),
            ],
        ]
        .concat(),
    );
    log += &run_in(
        project_dir,
        &blender,
        &[
            &headless[..],
            &[
                &path(&built),
                "--python",
                &path(&tools.join("export_game.py")),
                "--",
                "--out",
                &path(&project_dir.join("web/raw")),
                "--format",
                settings.format.ext(),
                "--textures",
                settings.export.textures.word(),
                "--cap",
                &settings.export.texture_cap.to_string(),
            ],
        ]
        .concat(),
    );
    log += &run_in(
        project_dir,
        &node,
        &["tools/game_export.mjs", "--index", index::INDEX_FILE],
    );
    Some(log)
}

/// Imports `files`, gives the first one a real size so it needs work, sets
/// the export settings and writes the scripts and the index.
fn prepared(
    make: fn(&Path) -> std::path::PathBuf,
    format: GameFormat,
    files: &[&str],
    export: GameExport,
) -> (tests::Workspace, std::path::PathBuf, ProjectManifest) {
    let (ws, manifest) = game_project(make, format);
    let files: Vec<String> = files.iter().map(|f| (*f).to_string()).collect();
    let result = import_models(&ws.root, &ws.config.layout, &manifest.id, &files).expect("import");
    let project_dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let first = result.imported[0].clone();
    prepare::update(&project_dir, |current| {
        let mut next = current.clone();
        next.models.insert(
            first,
            ModelPrep {
                size: Some(RealSize {
                    axis: SizeAxis::Longest,
                    value: 2.0,
                    unit: SizeUnit::M,
                }),
                ..ModelPrep::default()
            },
        );
        Ok(next)
    })
    .expect("prep");
    let manifest = project::modify(&ws.root, &ws.config.layout, &manifest.id, |m| {
        if let Some(game) = m.game.as_mut() {
            game.export = export;
        }
        Ok(())
    })
    .expect("settings");
    crate::pipeline::ensure(&project_dir, &manifest).expect("pipeline");
    index::write(&project_dir, &manifest).expect("index");
    (ws, project_dir, manifest)
}

#[test]
fn a_glb_game_exports_plain_capped_png_textures_and_copies_untouched_imports() {
    let export = GameExport {
        textures: GameTextures::Png,
        texture_cap: 4096,
    };
    let (ws, project_dir, manifest) = prepared(
        glb_game,
        GameFormat::Glb,
        &["props/brazier.glb", "props/coffin.glb"],
        export,
    );
    // The props budget caps the textures, not the larger Game export cap.
    let manifest = project::modify(&ws.root, &ws.config.layout, &manifest.id, |m| {
        if let Some(game) = m.game.as_mut() {
            for budget in game.budgets.iter_mut().filter(|b| b.folder == "props") {
                budget.texture = 32;
            }
        }
        Ok(())
    })
    .expect("budget");
    index::write(&project_dir, &manifest).expect("index");
    let Some(log) = run_chain(&project_dir, settings_of(&manifest).expect("game")) else {
        return;
    };
    let game = project_dir.join(ship::EXPORT_DIR);
    let brazier = glb_json(&game.join("props/brazier.glb"));
    let text = brazier.to_string();
    assert!(
        !text.contains("maquette"),
        "the tool's tags stay out: {text}"
    );
    for image in brazier["images"].as_array().expect("images") {
        assert_eq!(image["mimeType"], "image/png", "{image}");
    }
    let used = brazier["extensionsUsed"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for forbidden in ship::FORBIDDEN_EXTENSIONS {
        assert!(
            !used.iter().any(|e| e == forbidden),
            "{forbidden} in {used:?}"
        );
    }
    let report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(project_dir.join("web/raw/export.json")).expect("report"),
    )
    .expect("json");
    assert_eq!(report["models"][0]["cap"], 32, "{report}");
    for texture in report["models"][0]["textures"]
        .as_array()
        .expect("textures")
    {
        assert!(texture["width"].as_u64() <= Some(32), "{texture}");
    }
    assert_eq!(
        fs::read(game.join("props/coffin.glb")).expect("coffin"),
        fs::read(ws.repo.join("assets/models/props/coffin.glb")).expect("repo"),
        "an untouched import is copied byte for byte\n{log}"
    );
    let plan = ship_plan(&project_dir, &manifest).expect("plan");
    let change = |p: &str| plan.files.iter().find(|f| f.path == p).map(|f| f.change);
    assert_eq!(change("props/coffin.glb"), Some(ShipChange::Unchanged));
    assert_eq!(change("props/brazier.glb"), Some(ShipChange::Replaced));
}

#[test]
fn an_obj_game_exports_obj_with_its_mtl_and_textures_beside_it() {
    let (_ws, project_dir, manifest) = prepared(
        obj_game,
        GameFormat::Obj,
        &["stile.obj", "gate.obj"],
        GameExport::default(),
    );
    let Some(log) = run_chain(&project_dir, settings_of(&manifest).expect("game")) else {
        return;
    };
    let game = project_dir.join(ship::EXPORT_DIR);
    let obj = fs::read_to_string(game.join("stile.obj")).expect("obj");
    assert!(obj.contains("mtllib stile.mtl"), "{obj}\n{log}");
    let mtl = fs::read_to_string(game.join("stile.mtl")).expect("mtl");
    let texture = mtl
        .lines()
        .find_map(|l| l.strip_prefix("map_Kd "))
        .expect("a colour map")
        .trim()
        .to_string();
    assert!(
        !texture.contains('/'),
        "the .mtl names the texture beside it: {texture}"
    );
    assert!(game.join(&texture).is_file(), "{texture} was saved\n{log}");
    assert!(game.join("gate.obj").is_file() && game.join("gate.mtl").is_file());
}
