//! Ship into site folders an agent set waits for a person whatever "Ask
//! before shipping" says, and so does every other way an agent picks where
//! Ship writes: a real MCP client on a workspace with that rule off, over the
//! one step Ship of `ship_gate`. Temp folders only.

use std::path::{Path, PathBuf};

use maquette_core::project;
use maquette_desktop_lib::mcp::hub::ApprovalKind;
use serde_json::{Value, json};

use super::ship_gate::{World, git_repo, rule_off, rule_off_with};
use super::support::{call, call_error};

/// A site repo a person picked in the app: saved with no agent's mark.
fn persons_repo(world: &World) -> PathBuf {
    let site = git_repo(world.dir.path(), "site");
    let (root, config) = world.host.session.workspace().expect("ws");
    let repo = Some(site.display().to_string());
    project::update(&root, &config.layout, "web", "Web".into(), repo).expect("repo");
    site
}

async fn run(world: &World) -> Value {
    let args = json!({ "project": "web", "waitSeconds": 30 });
    call(&world.client, "run_pipeline", args).await
}

impl World {
    /// `run` waits before Ship on a card that names `repo` and has `shows`
    /// in it, for a reason with `why` in it; a person denies it and nothing
    /// is written.
    async fn denied(&self, run: &Value, repo: &Path, why: &str, shows: &str) {
        assert_eq!(run["status"], json!("waiting for approval"), "{run}");
        let cards = self.hub.approvals();
        assert_eq!(cards.len(), 1, "the app shows the card");
        assert_eq!(cards[0].kind, ApprovalKind::Ship);
        let detail = cards[0].detail.clone().unwrap_or_default();
        let real = maquette_core::fs::real_folder(repo);
        assert!(
            detail.starts_with(&format!("Writes to {}", real.display())),
            "{detail}"
        );
        assert!(detail.contains(shows), "the card names {shows}: {detail}");
        let waiting = self.read(run).await;
        let message = waiting["steps"][0]["message"].as_str().unwrap_or_default();
        assert!(message.contains(why), "{waiting}");
        assert!(self.hub.decide(&cards[0].id, false));
        let denied = self.finished(run).await;
        assert_eq!(denied["status"], json!("cancelled"), "{denied}");
        assert!(!repo.join("shipped.log").exists(), "nothing was written");
    }
}

const FOLDERS: &str = "an agent chose the folders";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_folder_an_agent_sets_waits_for_a_person_with_the_rule_off() {
    let fields = [
        ("models", "public/elsewhere", "models in public/elsewhere"),
        ("posters", "public/stills", "posters in public/stills"),
        (
            "data",
            "src/scene/shipped.json",
            "data in src/scene/shipped.json",
        ),
    ];
    for (field, value, shows) in fields {
        let world = rule_off().await;
        let site = persons_repo(&world);
        let mut args = json!({ "project": "web" });
        args[field] = json!(value);
        let saved = call(&world.client, "set_ship_settings", args).await;
        assert_eq!(saved["target"]["setByAgent"], json!(true), "{saved}");
        let written = std::fs::read_to_string(site.join("maquette.site.json")).expect("file");
        assert!(written.contains(value), "{written}");
        let run = run(&world).await;
        world.denied(&run, &site, FOLDERS, shows).await;
        world.close().await;
    }
}

/// An agent's project `decoy` on `site`, whose folders it then sets.
async fn decoy_sets_folders(world: &World, site: &Path) {
    let args = json!({ "name": "Decoy", "target": "website", "siteRepo": site });
    call(&world.client, "create_project", args).await;
    let args = json!({ "project": "decoy", "models": "public/evil" });
    call(&world.client, "set_ship_settings", args).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn folders_an_agent_set_through_another_project_on_the_repo_wait() {
    let world = rule_off().await;
    let site = persons_repo(&world);
    decoy_sets_folders(&world, &site).await;
    let run = run(&world).await;
    world
        .denied(&run, &site, FOLDERS, "models in public/evil")
        .await;
    world.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_project_a_person_points_at_the_repo_later_waits_too() {
    let world = rule_off().await;
    let site = git_repo(world.dir.path(), "site");
    decoy_sets_folders(&world, &site).await;
    // Only now does a person point their own project at that repo.
    let (root, config) = world.host.session.workspace().expect("ws");
    let repo = Some(site.display().to_string());
    project::update(&root, &config.layout, "web", "Web".into(), repo).expect("repo");
    let run = run(&world).await;
    world
        .denied(&run, &site, FOLDERS, "models in public/evil")
        .await;
    world.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_folders_stay_put_while_a_run_waits() {
    let world = rule_off().await;
    let site = persons_repo(&world);
    let args = json!({ "project": "web", "models": "public/elsewhere" });
    call(&world.client, "set_ship_settings", args).await;
    let run = run(&world).await;
    assert_eq!(run["status"], json!("waiting for approval"), "{run}");
    let args = json!({ "project": "web", "models": "public/other" });
    let refused = call_error(&world.client, "set_ship_settings", args).await;
    assert!(refused.contains("A run is going"), "{refused}");
    let written = std::fs::read_to_string(site.join("maquette.site.json")).expect("file");
    assert!(!written.contains("public/other"), "{written}");
    world
        .denied(&run, &site, FOLDERS, "models in public/elsewhere")
        .await;
    world.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_project_an_agent_made_with_a_repo_waits_with_the_rule_off() {
    let world = rule_off_with(|dir| {
        let site = git_repo(dir, "site");
        json!({ "name": "Web", "target": "website", "siteRepo": site })
    })
    .await;
    let site = world.dir.path().join("site");
    let run = run(&world).await;
    let why = "an agent chose the repo";
    world
        .denied(&run, &site, why, "models in public/models")
        .await;
    world.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_game_ship_after_set_game_settings_still_waits_with_the_rule_off() {
    let world = rule_off_with(|_| json!({ "name": "Web", "target": "game" })).await;
    let repo = git_repo(world.dir.path(), "game");
    let args = json!({ "project": "web", "repo": repo, "modelsDir": "assets/elsewhere" });
    call(&world.client, "set_game_settings", args).await;
    // A game's pipeline is fitted to its settings: start at its own Ship.
    let args = json!({ "project": "web", "from": "ship", "waitSeconds": 30 });
    let run = call(&world.client, "run_pipeline", args).await;
    assert_eq!(run["status"], json!("waiting for approval"), "{run}");
    let cards = world.hub.approvals();
    assert_eq!(cards.len(), 1, "the app shows the card");
    let waiting = world.read(&run).await;
    let steps = waiting["steps"].as_array().cloned().unwrap_or_default();
    let ship = steps
        .iter()
        .find(|s| s["id"] == json!("ship"))
        .expect("ship");
    let message = ship["message"].as_str().unwrap_or_default();
    assert!(message.contains("game repo"), "{waiting}");
    assert!(world.hub.decide(&cards[0].id, false));
    world.finished(&run).await;
    assert!(!repo.join("shipped.log").exists(), "nothing was written");
    world.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_cannot_point_the_data_file_at_a_file_already_in_the_site() {
    let world = rule_off().await;
    let site = persons_repo(&world);
    std::fs::write(site.join("package.json"), "{}\n").expect("site file");
    let args = json!({ "project": "web", "data": "package.json" });
    let refused = call_error(&world.client, "set_ship_settings", args).await;
    assert!(refused.contains("not its data file"), "{refused}");
    assert!(
        !site.join("maquette.site.json").exists(),
        "nothing was written"
    );
    let state = world.host.session.app_state();
    assert!(state.agent_set_folders.is_empty(), "and no mark went down");

    // The site's own data file, already there, can be named again.
    std::fs::create_dir_all(site.join("src/scene")).expect("dir");
    let own = json!({ "version": 1, "models": "public/models", "data": "src/scene/shipped.json" });
    std::fs::write(site.join("maquette.site.json"), own.to_string()).expect("config");
    std::fs::write(site.join("src/scene/shipped.json"), "{}\n").expect("data");
    let args =
        json!({ "project": "web", "models": "public/other", "data": "src/scene/shipped.json" });
    call(&world.client, "set_ship_settings", args).await;
    world.close().await;
}
