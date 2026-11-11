//! Ship in a run an agent starts: when it waits for a person, and that a
//! repo an agent chose waits whatever "Ask before shipping" says.

use maquette_core::app_state::RepoSetBy;
use maquette_core::concepts::Concepts;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};

use super::*;
use crate::commands::project::{create_in_workspace, set_repo};
use crate::commands::project_trash::{Running, remove, restore};
use crate::mcp::hub::Hub;

/// What the Ship step says while it waits for each reason.
const CHOSE: &str = "an agent chose the repo";
const STARTED: &str = "an agent started this run";

/// A workspace with "Ask before shipping" as `rule`, and a website project
/// whose site repo `by` set.
fn shipping(rule: bool, by: RepoSetBy) -> (tempfile::TempDir, Session, String) {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = WorkspaceConfig::default();
    config.agents.rules.ask_before_shipping = rule;
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &config).expect("ws");
    let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
    session.set_workspace(root, config).expect("open");
    let site = dir.path().join("site");
    std::fs::create_dir_all(site.join(".git")).expect("site");
    let new = NewProject {
        name: "Moon Island".into(),
        template: ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: Some(site.display().to_string()),
        source_folder: None,
    };
    let manifest = create_in_workspace(&session, new, by).expect("project");
    (dir, session, manifest.id)
}

/// What Ship says while it waits in a run an agent starts, into `repo` for
/// this run or else the project's own. None when it does not wait.
fn waits(session: &Session, id: &str, repo: Option<&Path>) -> Option<String> {
    let hub = Arc::new(Hub::new(|| {}));
    let ask = RunAsk {
        repo: repo.map(|r| r.display().to_string()),
        ..RunAsk::default()
    };
    let tools = maquette_core::pipeline::tools::Toolbox::default();
    let (setup, gate) = agent_run(session, &hub, id, ask, tools, |setup, gate| {
        Ok((setup, gate))
    })
    .expect("setup");
    let ship = setup.def.step("ship").expect("ship step");
    gate.map(|gate| {
        assert!(gate.applies(ship));
        gate.waiting_message(ship)
    })
}

#[test]
fn ship_into_a_repo_an_agent_set_waits_whatever_the_rule_says() {
    for rule in [false, true] {
        let (_dir, session, id) = shipping(rule, RepoSetBy::Agent);
        let why = waits(&session, &id, None).expect("an agent's repo asks");
        assert!(why.contains(CHOSE), "rule {rule}: {why}");
    }
}

#[test]
fn ship_into_a_repo_the_run_names_waits_whatever_the_rule_says() {
    let (dir, session, id) = shipping(false, RepoSetBy::Person);
    let worktree = dir.path().join("worktree");
    std::fs::create_dir_all(worktree.join(".git")).expect("worktree");
    let why = waits(&session, &id, Some(&worktree)).expect("a named repo asks");
    assert!(why.contains(CHOSE), "{why}");
    // Naming the person's own repo asks too: the run chose where it ships.
    let own = dir.path().join("site");
    assert!(waits(&session, &id, Some(&own)).is_some());
}

#[test]
fn ship_into_a_repo_a_person_set_asks_only_while_the_rule_is_on() {
    let (_dir, session, id) = shipping(false, RepoSetBy::Person);
    assert_eq!(waits(&session, &id, None), None, "their own repo, unasked");
    let (_dir, session, id) = shipping(true, RepoSetBy::Person);
    let why = waits(&session, &id, None).expect("the rule asks");
    assert!(why.contains(STARTED), "{why}");
}

#[test]
fn an_agents_repo_keeps_asking_through_a_rename_and_a_restore() {
    let (dir, session, id) = shipping(false, RepoSetBy::Agent);
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let save = |name: &str, repo: &Path| {
        let repo = Some(repo.display().to_string());
        set_repo(&session, &runner, &id, name.into(), repo, RepoSetBy::Person).expect("save");
    };
    // Settings saves the same repo beside the new name, which confirms nothing.
    save("New name", &dir.path().join("site"));
    assert!(waits(&session, &id, None).is_some(), "after a rename");

    let lab = Concepts::new(None, 1, |_| {});
    let studio = ProviderJobs::new(
        Arc::new(MemoryStore::default()),
        Options::default(),
        maquette_core::inbox::send_ports(),
        |_| {},
    );
    let running = Running {
        runner: &runner,
        lab: &lab,
        studio: &studio,
        building: Vec::new(),
    };
    let gone = remove(&session, &running, None, None, &id).expect("remove");
    restore(&session, &gone.removed.trash).expect("restore");
    assert!(waits(&session, &id, None).is_some(), "after a restore");

    // A person picking another folder makes the repo their own.
    let other = dir.path().join("other");
    std::fs::create_dir_all(&other).expect("other");
    save("New name", &other);
    assert_eq!(waits(&session, &id, None), None);
    // Picking the first folder again after that keeps it theirs.
    save("New name", &dir.path().join("site"));
    assert_eq!(waits(&session, &id, None), None, "picked again");
}

/// What Ship says while it waits because an agent set the site's folders.
const FOLDERS: &str = "an agent chose the folders";

/// Saves the site's folders in `id` as `by` would, with the models folder
/// `models`, or only a setting when `models` is None.
fn save_folders(session: &Session, id: &str, models: Option<&str>, by: RepoSetBy) {
    use crate::commands::ship::{ShipSave, save};
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let target = models.map(|models| maquette_core::site::SiteTarget {
        models: models.into(),
        posters: None,
        data: Some("src/scene/shipped.json".into()),
    });
    let change = ShipSave {
        target,
        ..ShipSave::default()
    };
    save(session, &runner, id, change, by).expect("save");
}

#[test]
fn folders_an_agent_set_wait_whatever_the_rule_says() {
    for rule in [false, true] {
        let (dir, session, id) = shipping(rule, RepoSetBy::Person);
        save_folders(&session, &id, Some("public/elsewhere"), RepoSetBy::Agent);
        let why = waits(&session, &id, None).expect("an agent's folders ask");
        assert!(why.contains(FOLDERS), "rule {rule}: {why}");
        let written = std::fs::read_to_string(dir.path().join("site/maquette.site.json"));
        assert!(written.expect("site file").contains("public/elsewhere"));
    }
}

#[test]
fn a_persons_save_of_the_folders_takes_the_agents_mark_off() {
    let (_dir, session, id) = shipping(false, RepoSetBy::Person);
    save_folders(&session, &id, Some("public/elsewhere"), RepoSetBy::Agent);
    // A save of settings alone says nothing about the folders.
    save_folders(&session, &id, None, RepoSetBy::Person);
    assert!(waits(&session, &id, None).is_some(), "still the agent's");
    // The person saves the very folders the agent set: now they are theirs.
    save_folders(&session, &id, Some("public/elsewhere"), RepoSetBy::Person);
    assert_eq!(
        waits(&session, &id, None),
        None,
        "their own folders, unasked"
    );

    let (_dir, session, id) = shipping(true, RepoSetBy::Person);
    save_folders(&session, &id, Some("public/elsewhere"), RepoSetBy::Agent);
    save_folders(&session, &id, Some("public/models"), RepoSetBy::Person);
    let why = waits(&session, &id, None).expect("the rule still asks");
    assert!(why.contains(STARTED), "{why}");
}

#[test]
fn an_agents_repo_says_so_over_its_folders() {
    let (_dir, session, id) = shipping(false, RepoSetBy::Agent);
    save_folders(&session, &id, Some("public/elsewhere"), RepoSetBy::Agent);
    let why = waits(&session, &id, None).expect("asks");
    assert!(why.contains(CHOSE), "{why}");
}

#[test]
fn an_agents_folders_keep_asking_through_a_restore() {
    let (dir, session, id) = shipping(false, RepoSetBy::Person);
    save_folders(&session, &id, Some("public/elsewhere"), RepoSetBy::Agent);
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let lab = Concepts::new(None, 1, |_| {});
    let studio = ProviderJobs::new(
        Arc::new(MemoryStore::default()),
        Options::default(),
        maquette_core::inbox::send_ports(),
        |_| {},
    );
    let running = Running {
        runner: &runner,
        lab: &lab,
        studio: &studio,
        building: Vec::new(),
    };
    let gone = remove(&session, &running, None, None, &id).expect("remove");
    restore(&session, &gone.removed.trash).expect("restore");
    // The repo came back unconfirmed: a person picks another folder, whose
    // folders no agent set, then this one again, whose folders still ask.
    let other = dir.path().join("other");
    std::fs::create_dir_all(other.join(".git")).expect("other");
    let pick = |repo: &Path| {
        let repo = Some(repo.display().to_string());
        let name = "Moon Island".to_string();
        set_repo(&session, &runner, &id, name, repo, RepoSetBy::Person).expect("repo");
    };
    pick(&other);
    assert_eq!(waits(&session, &id, None), None, "another repo's folders");
    pick(&dir.path().join("site"));
    let why = waits(&session, &id, None).expect("the folders still ask");
    assert!(why.contains(FOLDERS), "{why}");
}

#[test]
fn a_folder_save_waits_until_the_agents_run_is_queued() {
    use std::sync::mpsc;
    use std::time::Duration;
    let (_dir, session, id) = shipping(false, RepoSetBy::Person);
    let hub = Arc::new(Hub::new(|| {}));
    let tools = maquette_core::pipeline::tools::Toolbox::default();
    let (saved, landed) = mpsc::channel();
    std::thread::scope(|scope| {
        agent_run(&session, &hub, &id, RunAsk::default(), tools, |_, gate| {
            assert!(gate.is_none(), "a person's repo and folders, unasked");
            let (session, id) = (&session, &id);
            scope.spawn(move || {
                save_folders(session, id, Some("public/late"), RepoSetBy::Agent);
                saved.send(()).expect("send");
            });
            // The run is not queued yet: the save waits for it.
            let early = landed.recv_timeout(Duration::from_millis(400));
            assert!(
                early.is_err(),
                "a folder save landed before the run was queued"
            );
            Ok(())
        })
        .expect("run");
        landed
            .recv_timeout(Duration::from_secs(20))
            .expect("the save lands once the run is queued");
    });
    assert!(waits(&session, &id, None).is_some(), "the next run asks");
}
