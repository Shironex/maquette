//! Who set a repo: when the mark goes down and when it clears.

use super::*;

#[test]
fn only_a_person_really_moving_the_repo_clears_the_mark() {
    let f = fixture(RepoSetBy::Agent);
    // Settings and onboarding save the same repo as they click through.
    f.set_repo("New name", &f.site, RepoSetBy::Person);
    assert!(matches!(f.check(Starter::Agent), StartGate::Waiting { .. }));
    f.hub.decide(&f.card(), false);
    let other = f.root.with_file_name("other");
    std::fs::create_dir_all(&other).expect("other");
    f.set_repo("New name", &other, RepoSetBy::Person);
    assert!(matches!(f.check(Starter::Agent), StartGate::Clear(_)));

    // And an agent setting that same repo puts the mark back.
    f.set_repo("New name", &other, RepoSetBy::Agent);
    assert!(!matches!(f.check(Starter::Agent), StartGate::Clear(_)));
}

fn marked(f: &Fixture) -> bool {
    let key = project::repos::root_key(&f.root);
    f.host.session().app_state().repo_set_by_agent(&key, &f.id)
}

#[test]
fn the_mark_goes_down_before_the_repo_and_a_failure_leaves_it() {
    let f = fixture(RepoSetBy::Person);
    assert!(!marked(&f));
    let other = f.root.with_file_name("other");
    let failed = set_repo(
        f.host.session(),
        f.host.runner(),
        &f.id,
        "  ".to_string(), // A name the save refuses, after the mark.
        Some(other.display().to_string()),
        RepoSetBy::Agent,
    );
    assert!(failed.is_err());
    assert!(marked(&f), "a failure in between asks");
}

#[test]
fn an_agent_clearing_a_repo_keeps_the_mark() {
    let f = fixture(RepoSetBy::Agent);
    let cleared = set_repo(
        f.host.session(),
        f.host.runner(),
        &f.id,
        "Room".to_string(),
        None,
        RepoSetBy::Agent,
    );
    cleared.expect("cleared");
    assert!(marked(&f), "no window for a later set to slip through");
}

#[test]
fn a_persons_save_racing_an_agents_cannot_leave_its_repo_unmarked() {
    let f = fixture(RepoSetBy::Person);
    let start = f.root.with_file_name("start");
    let by_agent = f.root.with_file_name("by-agent");
    let by_person = f.root.with_file_name("by-person");
    let session = f.host.session();
    let runner = f.host.runner();
    let save = |repo: &Path, by| {
        let repo = Some(repo.display().to_string());
        set_repo(session, runner, &f.id, "Room".to_string(), repo, by).expect("save");
    };
    let agents = by_agent.display().to_string();
    for round in 0..60 {
        save(&start, RepoSetBy::Person);
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                barrier.wait();
                save(&by_person, RepoSetBy::Person);
            });
            scope.spawn(|| {
                barrier.wait();
                save(&by_agent, RepoSetBy::Agent);
            });
        });
        let (root, config) = session.workspace().expect("workspace");
        let repo = project::load(&root, &config.layout, &f.id)
            .expect("load")
            .site_repo;
        if repo.as_deref() == Some(agents.as_str()) {
            assert!(marked(&f), "round {round}: an agent's repo went unmarked");
        }
    }
}
