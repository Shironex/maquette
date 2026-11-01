//! The repo an agent names: where it may point Ship, and that a refused or
//! bad call leaves everything as it was. Temp folders only.

use maquette_core::{pipeline, project};
use serde_json::{Value, json};

use super::start::{serve, session_in};
use super::support::{call, call_error};

fn git_repo(path: &std::path::Path) {
    std::fs::create_dir_all(path.join(".git")).expect("repo");
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
    assert_eq!(repo(), Some(good.display().to_string()));
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
            Some(first.display().to_string())
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
