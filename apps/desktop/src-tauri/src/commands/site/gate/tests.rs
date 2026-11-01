use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maquette_core::concepts::Concepts;
use maquette_core::page::{self, PageSettings};
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::site::preview::{CommandSource, ServerState, SitePreview};
use maquette_core::thumbs::Thumbnails;
use maquette_core::workspace;
use maquette_core::{AppState, NewProject, ProjectTemplate, WorkspaceConfig, app_state, project};

use super::*;
use crate::commands::project::{create_in_workspace, set_repo};

/// `git` is on PATH wherever these run, and nothing here ever starts it:
/// the host has no server, so a yes is only remembered.
const COMMAND: &str = "git status";

struct TestHost {
    session: Session,
    runner: PipelineRunner,
    concepts: Concepts,
    thumbs: Thumbnails,
    studio: ProviderJobs,
    preview: Option<Arc<SitePreview>>,
}

impl Host for TestHost {
    fn session(&self) -> &Session {
        &self.session
    }
    fn runner(&self) -> &PipelineRunner {
        &self.runner
    }
    fn concepts(&self) -> &Concepts {
        &self.concepts
    }
    fn thumbs(&self) -> &Thumbnails {
        &self.thumbs
    }
    fn studio(&self) -> &ProviderJobs {
        &self.studio
    }
    fn toolbox(&self) -> Toolbox {
        Toolbox::default()
    }
    fn site_preview(&self) -> Option<Arc<SitePreview>> {
        self.preview.clone()
    }
}

struct Fixture {
    host: Arc<dyn Host>,
    hub: Arc<Hub>,
    root: PathBuf,
    id: String,
    site: PathBuf,
    cfg: PathBuf,
    _dir: tempfile::TempDir,
}

fn host_over(session: Session, preview: Option<Arc<SitePreview>>) -> Arc<dyn Host> {
    Arc::new(TestHost {
        session,
        runner: PipelineRunner::new(Arc::new(NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio: ProviderJobs::new(
            Arc::new(MemoryStore::default()),
            Options::default(),
            maquette_core::inbox::send_ports(),
            |_| {},
        ),
        preview,
    })
}

/// A project whose repo `by` set, running [`COMMAND`] from its settings.
fn fixture(by: RepoSetBy) -> Fixture {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let cfg = dir.path().join("cfg");
    let session = Session::with_state(cfg.clone(), None, AppState::default());
    session.set_workspace(root.clone(), config).expect("open");
    let site = dir.path().join("site");
    std::fs::create_dir_all(&site).expect("site");
    let manifest = create_in_workspace(
        &session,
        NewProject {
            name: "Room".into(),
            template: ProjectTemplate::Room,
            target: Default::default(),
            game: None,
            site_repo: Some(site.display().to_string()),
            source_folder: None,
        },
        by,
    )
    .expect("project");
    let fixture = Fixture {
        host: host_over(session, None),
        hub: Arc::new(Hub::new(|| {})),
        root,
        id: manifest.id,
        site,
        cfg,
        _dir: dir,
    };
    fixture.set_command(Some(COMMAND));
    fixture
}

impl Fixture {
    fn set_command(&self, command: Option<&str>) {
        let layout = WorkspaceConfig::default().layout;
        let dir = project::dir(&self.root, &layout, &self.id).expect("dir");
        let settings = PageSettings {
            dev_command: command.map(str::to_string),
            ..PageSettings::default()
        };
        page::save(&dir, &self.root, &settings).expect("settings");
    }

    fn check(&self, starter: Starter) -> StartGate {
        check(&self.host, &self.hub, &self.id, starter).expect("gate")
    }

    fn set_repo(&self, name: &str, repo: &Path, by: RepoSetBy) {
        set_repo(
            self.host.session(),
            self.host.runner(),
            &self.id,
            name.to_string(),
            Some(repo.display().to_string()),
            by,
        )
        .expect("repo");
    }

    fn card(&self) -> String {
        self.hub.preview_for(&self.id).expect("a card").id
    }

    fn wait_for(&self, what: &str, done: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done() {
            assert!(Instant::now() < deadline, "{what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Says yes to the card and waits until it is remembered.
    fn approve(&self) {
        let id = self.card();
        let ask = self.hub.preview_for(&self.id).and_then(|a| a.preview);
        assert!(self.hub.decide(&id, true));
        let ask = ask.expect("ask");
        self.wait_for("the yes is saved", || {
            self.host
                .session()
                .app_state()
                .preview_approved(&ask.repo, &ask.key())
        });
    }
}

#[test]
fn a_repo_a_person_set_never_asks() {
    let f = fixture(RepoSetBy::Person);
    assert!(matches!(f.check(Starter::Agent), StartGate::Clear(_)));
    assert!(matches!(f.check(Starter::Person), StartGate::Clear(_)));
    assert!(f.hub.approvals().is_empty());
}

#[test]
fn every_starter_of_an_agent_set_repo_gets_the_card() {
    let f = fixture(RepoSetBy::Agent);
    let StartGate::Waiting { id, ask } = f.check(Starter::Agent) else {
        panic!("an agent's start must wait");
    };
    assert_eq!(ask.command, COMMAND);
    assert_eq!(ask.source, CommandSource::Settings);
    let real = std::fs::canonicalize(&f.site).expect("real");
    assert_eq!(ask.repo, real.display().to_string());
    // The Page view and Preview on site join the same card, not a second one.
    let StartGate::Waiting { id: same, .. } = f.check(Starter::Person) else {
        panic!("a start from the app must wait too");
    };
    assert_eq!(id, same);
    assert_eq!(f.hub.approvals().len(), 1);
}

#[test]
fn a_yes_is_remembered_across_restarts_and_for_that_question_only() {
    let f = fixture(RepoSetBy::Agent);
    f.check(Starter::Agent);
    f.approve();
    assert!(matches!(f.check(Starter::Agent), StartGate::Clear(_)));

    // A new app run on the same files keeps both the yes and the mark.
    let host = host_over(reopened(&f), None);
    let hub = Arc::new(Hub::new(|| {}));
    assert!(matches!(
        check(&host, &hub, &f.id, Starter::Agent).expect("gate"),
        StartGate::Clear(_)
    ));

    // A changed command asks again.
    f.set_command(Some("git log"));
    assert!(matches!(f.check(Starter::Agent), StartGate::Waiting { .. }));
    f.approve();
    assert!(matches!(f.check(Starter::Agent), StartGate::Clear(_)));

    // So does a changed path, even with the same command.
    let other = f.site.with_file_name("other");
    std::fs::create_dir_all(&other).expect("other");
    f.set_repo("Room", &other, RepoSetBy::Agent);
    assert!(matches!(f.check(Starter::Agent), StartGate::Waiting { .. }));
}

#[test]
fn a_changed_site_config_or_package_script_asks_again() {
    let f = fixture(RepoSetBy::Agent);
    f.set_command(None);
    let config = f.site.join("maquette.site.json");
    let write = |preview: &str| {
        let text = format!(
            r#"{{ "version": 1, "models": "public/models", "preview": {{ "command": ["{preview}", "status"] }} }}"#
        );
        std::fs::write(&config, text).expect("config");
    };
    write("git");
    let StartGate::Waiting { ask, .. } = f.check(Starter::Agent) else {
        panic!("waits");
    };
    assert_eq!(ask.command, COMMAND);
    f.approve();
    assert!(matches!(f.check(Starter::Agent), StartGate::Clear(_)));
    write("cargo");
    assert!(matches!(f.check(Starter::Agent), StartGate::Waiting { .. }));

    // The line stays "npm run dev" when only the script behind it changes.
    std::fs::remove_file(&config).expect("remove");
    let package = |dev: &str| {
        let text = format!(r#"{{ "scripts": {{ "dev": "{dev}" }} }}"#);
        std::fs::write(f.site.join("package.json"), text).expect("package");
    };
    package("vite");
    let StartGate::Waiting { ask, .. } = f.check(Starter::Agent) else {
        panic!("waits");
    };
    assert_eq!(ask.script_runs.as_deref(), Some("vite"));
    f.approve();
    assert!(matches!(f.check(Starter::Agent), StartGate::Clear(_)));
    package("node evil.js");
    let StartGate::Waiting { ask, .. } = f.check(Starter::Agent) else {
        panic!("a changed script must ask again");
    };
    assert_eq!(ask.script_runs.as_deref(), Some("node evil.js"));
}

#[test]
fn a_no_holds_for_an_agent_until_a_person_asks_again() {
    let f = fixture(RepoSetBy::Agent);
    f.check(Starter::Agent);
    assert!(f.hub.decide(&f.card(), false));
    f.wait_for("the no is recorded", || {
        matches!(f.check_quiet(), Some(StartGate::Refused(_)))
    });
    assert!(f.hub.approvals().is_empty(), "no second card");
    // A person asking from the app gets the card back.
    assert!(matches!(
        f.check(Starter::Person),
        StartGate::Waiting { .. }
    ));
    assert!(matches!(f.check(Starter::Agent), StartGate::Waiting { .. }));
}

impl Fixture {
    /// [`Self::check`] that takes back a card it raised while the no was on
    /// its way, so polling for the no leaves no card behind.
    fn check_quiet(&self) -> Option<StartGate> {
        let found = self.check(Starter::Agent);
        if let StartGate::Waiting { id, .. } = &found {
            self.hub.decide(id, false);
            return None;
        }
        Some(found)
    }
}

#[test]
fn a_command_changed_behind_the_card_does_not_run_on_its_yes() {
    let f = fixture(RepoSetBy::Agent);
    let f = Fixture {
        host: with_server(&f),
        ..f
    };
    f.check(Starter::Agent);
    let first = f.card();
    f.set_command(Some("git log"));
    assert!(f.hub.decide(&first, true));
    f.wait_for("a new card asks about the new command", || {
        f.hub
            .preview_for(&f.id)
            .is_some_and(|a| a.preview.is_some_and(|p| p.command == "git log"))
    });
    let server = f.host.site_preview().expect("server");
    assert_eq!(
        server.status().state,
        ServerState::Idle,
        "only what the person read may run"
    );
}

/// The same session behind a host that has a server, which must stay idle.
fn with_server(f: &Fixture) -> Arc<dyn Host> {
    host_over(reopened(f), Some(Arc::new(SitePreview::default())))
}

/// A new app run on the same files.
fn reopened(f: &Fixture) -> Session {
    let config = workspace::open(&f.root).expect("config");
    let session = Session::with_state(f.cfg.clone(), None, app_state::load(&f.cfg));
    session.set_workspace(f.root.clone(), config).expect("open");
    session
}

#[test]
fn a_state_file_from_before_the_mark_reads_as_a_persons_repo() {
    let dir = tempfile::tempdir().expect("dir");
    std::fs::write(
        dir.path().join(app_state::APP_STATE_FILE),
        r#"{ "version": 1, "lastWorkspace": null, "toolOverrides": {},
             "downloadsWatch": { "enabled": true, "folder": null }, "siteRepos": {} }"#,
    )
    .expect("old file");
    let state = app_state::load(dir.path());
    assert!(!state.repo_set_by_agent("/ws", "p"));
    assert!(state.agent_set_repos.is_empty());
    assert!(state.preview_approvals.is_empty());
}

mod repo_marks;
