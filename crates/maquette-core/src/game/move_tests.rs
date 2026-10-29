//! Moving an asset to a folder of its own in the model folder: the file
//! goes with it, the ship plan shows the old file going, and Ship removes
//! only that old file, only while Maquette owns it.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::tests::{game_project, glb_game};
use super::*;
use crate::project;

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    fs::write(path, bytes).expect("write");
}

fn folders(asset: &str, folder: Option<&str>) -> GameSettingsChange {
    GameSettingsChange {
        asset_folders: Some(BTreeMap::from([(
            asset.to_string(),
            folder.map(str::to_string),
        )])),
        ..GameSettingsChange::default()
    }
}

#[test]
fn a_moved_asset_ships_to_its_folder_and_its_old_file_goes() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let (root, layout) = (&ws.root, &ws.config.layout);
    let files = ["props/brazier.glb".to_string()];
    let brazier = import_models(root, layout, &manifest.id, &files)
        .expect("import")
        .imported[0]
        .clone();
    let project_dir = project::dir(root, layout, &manifest.id).expect("dir");

    for bad in ["../outside", "/etc", "kit/../../up"] {
        let refused = save_settings(root, layout, &manifest.id, folders(&brazier, Some(bad)));
        assert!(refused.is_err(), "{bad} is refused");
    }
    assert!(save_settings(root, layout, &manifest.id, folders("nobody", Some("kit"))).is_err());

    let moved =
        save_settings(root, layout, &manifest.id, folders(&brazier, Some("kit/"))).expect("move");
    let settings = settings_of(&moved).expect("game");
    let asset = moved
        .assets
        .iter()
        .find(|a| a.id == brazier)
        .expect("asset");
    assert_eq!(file_of(settings, asset), "kit/brazier.glb");
    let record = ship::read_record(&project_dir);
    assert_eq!(record.remove["assets/models"], ["props/brazier.glb"]);

    write(
        &project_dir.join("web/game/kit/brazier.glb"),
        b"fixed brazier",
    );
    let plan = ship_plan(&project_dir, &moved).expect("plan");
    let change = |p: &str| plan.files.iter().find(|f| f.path == p).map(|f| f.change);
    assert_eq!(change("kit/brazier.glb"), Some(ShipChange::New));
    assert_eq!(change("props/brazier.glb"), Some(ShipChange::Removed));
    assert_eq!(plan.record_after, 1, "the old file leaves the record");

    let Some(node) = crate::toolchain::find_on_path(&["node"], &crate::ProbeEnv::from_system())
    else {
        return;
    };
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
        fs::read(models.join("kit/brazier.glb")).expect("kit"),
        b"fixed brazier"
    );
    assert!(
        !models.join("props/brazier.glb").exists(),
        "the old file went\n{log}"
    );
    assert!(
        models.join("props/coffin.glb").is_file(),
        "a file it does not own stays"
    );
    assert!(log.contains("1 moved away"), "{log}");
    let record = ship::read_record(&project_dir);
    assert_eq!(record.targets["assets/models"], ["kit/brazier.glb"]);
    assert!(record.remove.is_empty(), "the removal is done");

    // Back where it came from: the shipped kit file is the one to go now.
    save_settings(root, layout, &manifest.id, folders(&brazier, None)).expect("back");
    let record = ship::read_record(&project_dir);
    assert_eq!(record.remove["assets/models"], ["kit/brazier.glb"]);
}
