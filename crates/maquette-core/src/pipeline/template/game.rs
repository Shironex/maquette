//! The game pipeline: build each model with its fix-ups, bake its light
//! when the project asks for it, export it in the engine's format, check
//! and lay out the files the way the game keeps them, ship them into the
//! game's model folder. No camera stops and no posters.
//!
//! The steps read the game index (`web/game-index.json`, see
//! [`crate::game::index`]) and take the Game export settings as args, which
//! [`fit_game`] keeps in step with the project's game settings.

use super::{blender, s};
use crate::game::GameSettings;
use crate::game::index::INDEX_FILE;
use crate::pipeline::bake::DEFAULT as BAKE;
use crate::pipeline::{PipelineDef, StepDef, StepKind};

pub(crate) const BUILD_SCRIPT: &str = "tools/build_models.py";
pub(crate) const EXPORT_SCRIPT: &str = "tools/export_game.py";
pub(crate) const GAME_EXPORT_SCRIPT: &str = "tools/game_export.mjs";
pub(crate) const SHIP_SCRIPT: &str = "tools/ship_game.mjs";
const BAKE_SCRIPT: &str = crate::pipeline::bake::SCRIPT;
const BUILT: &str = "blender/game_built.blend";
const BAKED: &str = "blender/game_baked.blend";
/// The version files: `.glb` ones at the top, an `.obj` in its own folder.
const MODEL_INPUTS: [&str; 3] = ["models/*.glb", "models/*.obj", "models/*/*"];

fn node(id: &str, name: &str, script: &str, args: Vec<String>, deps: &[&str]) -> StepDef {
    StepDef {
        id: id.into(),
        name: name.into(),
        kind: StepKind::Shell,
        command: Some("node".into()),
        script: Some(script.into()),
        args,
        inputs: Vec::new(),
        outputs: Vec::new(),
        depends_on: s(deps),
        compress: None,
    }
}

/// The steps of a game project with `settings`.
pub fn steps(settings: &GameSettings) -> Vec<StepDef> {
    let bake = settings.bake;
    let mut build_args = vec![
        "--index",
        INDEX_FILE,
        "--prepare",
        crate::prepare::PREPARE_FILE,
        "--out",
        BUILT,
    ];
    if bake {
        // Models baked together must not stand in each other's light.
        build_args.push("--spread");
    }
    let mut build_inputs = vec![
        INDEX_FILE,
        crate::prepare::PREPARE_FILE,
        crate::prepare::SCRIPT_PATH,
    ];
    build_inputs.extend(MODEL_INPUTS);
    let mut out = vec![blender(
        "build",
        "Build models",
        BUILD_SCRIPT,
        None,
        &build_args,
        &build_inputs,
        &[BUILT],
        &[],
    )];
    if bake {
        let (size, samples) = (BAKE.size.to_string(), BAKE.samples.to_string());
        out.push(blender(
            "bake",
            "Bake light",
            BAKE_SCRIPT,
            Some(BUILT),
            &[
                "--out-dir",
                "blender/bake",
                "--out-blend",
                BAKED,
                "--size",
                &size,
                "--samples",
                &samples,
                "--denoise",
            ],
            &[BUILT],
            &["blender/bake/bake.json", BAKED],
            &["build"],
        ));
    }
    let cap = settings.export.texture_cap.to_string();
    let format = settings.format.ext();
    let mut export_args = vec![
        "--index",
        INDEX_FILE,
        "--out",
        "web/raw",
        "--format",
        format,
        "--textures",
        settings.export.textures.word(),
        "--cap",
        &cap,
    ];
    let blend = if bake { BAKED } else { BUILT };
    let mut export_inputs = vec![blend, INDEX_FILE];
    if bake {
        export_args.extend(["--bake", "blender/bake"]);
        export_inputs.push("blender/bake/*");
    }
    let name = match settings.format {
        crate::game::GameFormat::Glb => "Export GLB",
        crate::game::GameFormat::Obj => "Export OBJ",
    };
    out.push(blender(
        "export",
        name,
        EXPORT_SCRIPT,
        Some(blend),
        &export_args,
        &export_inputs,
        &["web/raw/export.json"],
        &[if bake { "bake" } else { "build" }],
    ));
    let mut game = node(
        "game",
        "Game export",
        GAME_EXPORT_SCRIPT,
        s(&[
            "{script}",
            "--index",
            INDEX_FILE,
            "--raw",
            "web/raw",
            "--out",
            crate::game::ship::EXPORT_DIR,
            "--report",
            "web/game-export.json",
        ]),
        &["export"],
    );
    game.inputs = s(&[INDEX_FILE, "web/raw/**/*"]);
    game.inputs.extend(s(&MODEL_INPUTS));
    game.outputs = s(&["web/game-export.json"]);
    out.push(game);
    // Ship writes outside the pipeline root, so it runs every time.
    out.push(node(
        "ship",
        "Ship",
        SHIP_SCRIPT,
        s(&[
            "{script}",
            "--from",
            crate::game::ship::EXPORT_DIR,
            "--game",
            "{siteRepo}",
            "--models",
            &settings.models_dir,
            "--record",
            crate::game::ship::RECORD_FILE,
        ]),
        &["game"],
    ));
    out
}

/// Keeps a game pipeline in step with its settings: the bake step in or
/// out, the export's format and texture args, Ship's model folder. A step
/// that runs a script of the person's own is left as it is, and so are the
/// bake's own size, samples and denoiser. Returns true when `def` changed.
pub fn fit_game(def: &mut PipelineDef, settings: &GameSettings) -> bool {
    let wanted = steps(settings);
    let mut next: Vec<StepDef> = Vec::new();
    for step in &wanted {
        match def.step(&step.id) {
            Some(current) if current.script != step.script => next.push(current.clone()),
            Some(current) if step.id == "bake" => next.push(current.clone()),
            _ => next.push(step.clone()),
        }
    }
    for current in &def.steps {
        let ours = wanted.iter().any(|w| w.id == current.id);
        let dropped_bake = current.id == "bake" && current.script.as_deref() == Some(BAKE_SCRIPT);
        if !ours && !dropped_bake {
            next.push(current.clone());
        }
    }
    if next == def.steps {
        return false;
    }
    def.steps = next;
    true
}
