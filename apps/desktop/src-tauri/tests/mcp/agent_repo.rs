//! The repo an agent names: where it may point Ship, and that a refused or
//! bad call leaves everything as it was. Temp folders only.

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use maquette_core::pipeline;
use maquette_core::project;
use serde_json::{Value, json};

use super::start::{serve, session_in};
use super::support::{call, call_error};

fn git_repo(path: &Path) {
    std::fs::create_dir_all(path.join(".git")).expect("repo");
}

/// Temp folders sit behind a link on macOS, and a repo is saved as its real folder.
fn real(path: &Path) -> String {
    path.canonicalize().expect("real").display().to_string()
}

fn git(repo: &Path, args: &[&str]) {
    let done = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git");
    assert!(done.status.success(), "{args:?}: {done:?}");
}

/// A real repo whose config runs a command on `git status`, with a changed
/// file so that it does. The marker appears when it ran.
fn repo_with_a_clean_filter(repo: &Path) -> PathBuf {
    std::fs::create_dir_all(repo).expect("repo");
    git(repo, &["init", "-q"]);
    std::fs::write(repo.join("a.txt"), "one\n").expect("file");
    git(repo, &["add", "a.txt"]);
    let marker = repo.join("filter-ran");
    let clean = format!("touch '{}'; cat", marker.display());
    git(repo, &["config", "filter.bad.clean", &clean]);
    std::fs::write(repo.join(".gitattributes"), "*.txt filter=bad\n").expect("attributes");
    // The same size, or git calls it changed without reading it.
    std::fs::write(repo.join("a.txt"), "two\n").expect("change");
    marker
}

#[tokio::test(flavor = "multi_thread")]
async fn only_a_real_repo_outside_the_workspace_is_taken() {
    let dir = tempfile::tempdir().expect("temp");
    let (client, server, host) = serve(session_in(dir.path(), true, |_| {})).await;
    call(
        &client,
        "create_project",
        json!({ "name": "Web", "kind": "website" }),
    )
    .await;
    let (root, config) = host.session.workspace().expect("ws");
    let web = project::dir(&root, &config.layout, "web").expect("dir");

    let plain = dir.path().join("plain");
    std::fs::create_dir_all(&plain).expect("plain");
    git_repo(&root);
    git_repo(&web);
    // The temp folder holds the workspace, as a home folder would.
    git_repo(dir.path());
    let good = dir.path().join("site");
    git_repo(&good);

    let wrong: [Value; 7] = [
        json!(""),
        json!("relative/site"),
        json!(dir.path().join("nope")),
        json!(plain),
        json!(root),
        json!(web),
        json!(dir.path()),
    ];
    let repo = || {
        project::load(&root, &config.layout, "web")
            .expect("load")
            .site_repo
    };
    let projects = || {
        project::summaries(&root, &config.layout)
            .expect("list")
            .len()
    };
    for path in wrong {
        call_error(
            &client,
            "set_ship_settings",
            json!({ "project": "web", "siteRepo": path }),
        )
        .await;
        assert_eq!(repo(), None, "{path}");
        let before = projects();
        for kind in ["website", "game"] {
            call_error(
                &client,
                "create_project",
                json!({ "name": "Other", "kind": kind, "siteRepo": path }),
            )
            .await;
            assert_eq!(projects(), before, "{kind} {path}");
        }
    }

    call(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": good }),
    )
    .await;
    assert_eq!(repo(), Some(real(&good)));
    // Only null clears it.
    call(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": null }),
    )
    .await;
    assert_eq!(repo(), None);

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bad_value_leaves_the_new_repo_and_the_old_one_alone() {
    let dir = tempfile::tempdir().expect("temp");
    let (client, server, host) = serve(session_in(dir.path(), true, |_| {})).await;
    call(
        &client,
        "create_project",
        json!({ "name": "Web", "kind": "website" }),
    )
    .await;
    let (root, config) = host.session.workspace().expect("ws");
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    git_repo(&first);
    git_repo(&second);
    call(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": first }),
    )
    .await;

    // The folders are good and the quality is not: site_config passes first.
    for bad in [
        json!({ "project": "web", "siteRepo": second, "models": "public/m", "quality": 500 }),
        json!({ "project": "web", "siteRepo": second, "models": "../outside" }),
    ] {
        call_error(&client, "set_ship_settings", bad).await;
        assert!(
            !second.join("maquette.site.json").exists(),
            "a refused call leaves no file"
        );
        assert_eq!(
            project::load(&root, &config.layout, "web")
                .expect("load")
                .site_repo,
            Some(real(&first))
        );
    }

    // The same call with good values writes to the new repo.
    call(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": second, "models": "public/m" }),
    )
    .await;
    assert!(second.join("maquette.site.json").is_file());

    client.cancel().await.expect("close");
    server.stop();
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn the_repo_stays_put_while_a_run_is_going() {
    let dir = tempfile::tempdir().expect("temp");
    let (client, server, host) = serve(session_in(dir.path(), true, |_| {})).await;
    call(
        &client,
        "create_project",
        json!({ "name": "Web", "kind": "website" }),
    )
    .await;
    let (root, config) = host.session.workspace().expect("ws");
    let site = dir.path().join("site");
    git_repo(&site);

    let web = project::dir(&root, &config.layout, "web").expect("dir");
    let manifest = project::load(&root, &config.layout, "web").expect("load");
    let mut def = pipeline::ensure(&web, &manifest).expect("pipeline");
    def.steps = vec![pipeline::StepDef {
        id: "slow".into(),
        name: "Slow".into(),
        kind: pipeline::StepKind::Shell,
        command: Some("/bin/sh".into()),
        script: Some("slow.sh".into()),
        args: vec!["{script}".into()],
        inputs: Vec::new(),
        outputs: Vec::new(),
        depends_on: Vec::new(),
        compress: None,
    }];
    pipeline::save(&web, &def).expect("save");
    std::fs::write(web.join("slow.sh"), "/bin/sleep 30\n").expect("script");
    let run = call(
        &client,
        "run_pipeline",
        json!({ "project": "web", "waitSeconds": 0 }),
    )
    .await;

    let refused = call_error(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": site }),
    )
    .await;
    assert!(refused.contains("A run is going"), "{refused}");
    assert_eq!(
        project::load(&root, &config.layout, "web")
            .expect("load")
            .site_repo,
        None
    );

    call(
        &client,
        "cancel_run",
        json!({ "project": "web", "runId": run["runId"] }),
    )
    .await;
    client.cancel().await.expect("close");
    server.stop();
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_linked_repo_is_saved_as_the_folder_itself() {
    let dir = tempfile::tempdir().expect("temp");
    let (client, server, host) = serve(session_in(dir.path(), true, |_| {})).await;
    call(
        &client,
        "create_project",
        json!({ "name": "Web", "kind": "website" }),
    )
    .await;
    let (root, config) = host.session.workspace().expect("ws");
    let site = dir.path().join("site");
    git_repo(&site);
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&site, &link).expect("link");
    call(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": link }),
    )
    .await;
    assert_eq!(
        project::load(&root, &config.layout, "web")
            .expect("load")
            .site_repo,
        Some(real(&site))
    );
    // A link into the workspace is the workspace.
    let inside = dir.path().join("inside");
    git_repo(&root);
    std::os::unix::fs::symlink(&root, &inside).expect("link");
    call_error(
        &client,
        "set_ship_settings",
        json!({ "project": "web", "siteRepo": inside }),
    )
    .await;
    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_game_settings_call_with_a_bad_value_does_not_move_the_repo() {
    let dir = tempfile::tempdir().expect("temp");
    let (client, server, host) = serve(session_in(dir.path(), true, |_| {})).await;
    call(
        &client,
        "create_project",
        json!({ "name": "Crypt", "kind": "game" }),
    )
    .await;
    let (root, config) = host.session.workspace().expect("ws");
    let good = dir.path().join("game");
    git_repo(&good);
    let load = || project::load(&root, &config.layout, "crypt").expect("load");
    let before = load();
    call_error(
        &client,
        "set_game_settings",
        json!({ "project": "crypt", "repo": good, "textureCap": 7 }),
    )
    .await;
    let after = load();
    assert_eq!(after.site_repo, before.site_repo, "the repo did not move");
    assert_eq!(
        serde_json::to_value(&after.game).expect("json"),
        serde_json::to_value(&before.game).expect("json"),
    );
    let key = project::repos::root_key(&root);
    let state = host.session.app_state();
    assert!(!state.repo_set_by_agent(&key, "crypt"), "and left no mark");
    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_ship_plan_runs_no_git_in_a_repo_an_agent_chose() {
    let dir = tempfile::tempdir().expect("temp");
    let (client, server, _host) = serve(session_in(dir.path(), true, |_| {})).await;
    call(
        &client,
        "create_project",
        json!({ "name": "Crypt", "kind": "game" }),
    )
    .await;
    // The fixture is real: a plain git status does run the filter, even with
    // fsmonitor off.
    let control = dir.path().join("control");
    let ran = repo_with_a_clean_filter(&control);
    git(
        &control,
        &["-c", "core.fsmonitor=false", "status", "--porcelain"],
    );
    assert!(ran.exists(), "git status runs a clean filter");

    // Named for one look.
    let named = dir.path().join("named");
    let marker = repo_with_a_clean_filter(&named);
    let plan = call(
        &client,
        "get_ship_plan",
        json!({ "project": "crypt", "repo": named }),
    )
    .await;
    assert_eq!(plan["repo"], real(&named).as_str());
    assert!(!marker.exists(), "a repo named for one look ran its filter");
    assert!(plan["uncommitted"].is_null(), "{plan}");

    // Saved by an agent.
    let saved = dir.path().join("saved");
    let marker = repo_with_a_clean_filter(&saved);
    call(
        &client,
        "set_game_settings",
        json!({ "project": "crypt", "repo": saved }),
    )
    .await;
    let plan = call(&client, "get_ship_plan", json!({ "project": "crypt" })).await;
    assert!(!marker.exists(), "an agent-set repo ran its filter");
    assert!(plan["uncommitted"].is_null(), "{plan}");

    // A folder the other doors refuse, this one refuses too.
    let (root, _) = _host.session.workspace().expect("ws");
    git_repo(&root);
    call_error(
        &client,
        "get_ship_plan",
        json!({ "project": "crypt", "repo": root }),
    )
    .await;
    client.cancel().await.expect("close");
    server.stop();
}
