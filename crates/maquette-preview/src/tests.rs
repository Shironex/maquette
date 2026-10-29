use super::*;
use maquette_base::site_config::SiteConfig;
use maquette_base::toolchain::HostOs;

#[test]
fn the_package_manager_comes_from_the_nearest_lockfile() {
    let dir = tempfile::tempdir().expect("dir");
    let site = dir.path().join("apps/site");
    fs_dir(&site);
    assert_eq!(package_manager(&site), "npm");
    std::fs::write(dir.path().join("pnpm-lock.yaml"), "").expect("lock");
    assert_eq!(package_manager(&site), "pnpm");
    std::fs::write(site.join("bun.lock"), "").expect("lock");
    assert_eq!(package_manager(&site), "bun");
}

fn fs_dir(path: &Path) {
    std::fs::create_dir_all(path).expect("dir");
}

#[test]
fn only_addresses_on_this_machine_count_as_local() {
    for good in [
        "http://localhost:15292/",
        "http://127.0.0.1:4321",
        "https://localhost/x?y",
        "http://[::1]:3000/",
    ] {
        assert!(is_local_url(good), "{good}");
    }
    for bad in [
        "http://example.com:15292/",
        "http://localhost.example.com/",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "http://localhost:1/ x",
        "localhost:15292",
    ] {
        assert!(!is_local_url(bad), "{bad}");
    }
}

#[test]
fn the_address_is_read_from_dev_server_output() {
    assert_eq!(
        find_local_url(
            "  \u{1b}[32m┃\u{1b}[39m Local    \u{1b}[36mhttp://localhost:15292/\u{1b}[39m"
        ),
        Some("http://localhost:15292/".into())
    );
    assert_eq!(
        find_local_url("ready at http://127.0.0.1:5173."),
        Some("http://127.0.0.1:5173".into())
    );
    assert_eq!(find_local_url("Network  http://192.168.1.4:15292/"), None);
    assert_eq!(find_local_url("watching for file changes"), None);
}

fn env_with(dir: &Path) -> ProbeEnv {
    let mut env = ProbeEnv::empty(HostOs::current());
    env.path_dirs = vec![dir.to_path_buf()];
    env
}

#[cfg(unix)]
fn executable(dir: &Path, name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

#[cfg(unix)]
#[test]
fn the_plan_finds_the_program_and_picks_a_port() {
    let bin = tempfile::tempdir().expect("bin");
    let site = tempfile::tempdir().expect("site");
    let pnpm = executable(bin.path(), "pnpm", "#!/bin/sh\n");
    std::fs::write(site.path().join("pnpm-lock.yaml"), "").expect("lock");
    std::fs::write(
        site.path().join("package.json"),
        r#"{ "scripts": { "dev": "astro dev" } }"#,
    )
    .expect("package");
    let env = env_with(bin.path());
    let found = command::choose(site.path(), &SiteConfig::legacy(), None).expect("command");
    let plan = plan(site.path(), &found, &env).expect("plan");
    assert_eq!(plan.program, pnpm);
    let port = plan.port.expect("a picked port");
    assert_eq!(plan.args, vec!["run", "dev", "--port", &port.to_string()]);
    assert_eq!(plan.url, None);
    assert_eq!(
        plan.expected_url(),
        Some(format!("http://localhost:{port}/"))
    );
    assert_eq!(plan.line, format!("pnpm run dev --port {port}"));
    assert!(plan.port_in_args);
    assert_eq!(plan.key, "pnpm run dev --port {port}");

    // A typed command gets the port only as PORT, which the tool may ignore.
    let typed = command::choose(
        site.path(),
        &SiteConfig::legacy(),
        Some("pnpm run dev:landing"),
    )
    .expect("typed");
    let plan = super::plan(site.path(), &typed, &env).expect("plan");
    assert!(plan.port.is_some());
    assert!(!plan.port_in_args);
    assert_eq!(plan.key, "pnpm run dev:landing");

    let mut config = SiteConfig::legacy();
    config.preview = Some(maquette_base::site_config::PreviewConfig {
        command: vec!["pnpm".into(), "dev".into()],
        url: Some("http://localhost:15292/".into()),
    });
    let found = command::choose(site.path(), &config, None).expect("command");
    let plan = super::plan(site.path(), &found, &env).expect("plan");
    assert_eq!(plan.args, vec!["dev".to_string()]);
    assert_eq!(plan.port, None);
    assert_eq!(plan.url.as_deref(), Some("http://localhost:15292/"));
}

#[test]
fn the_plan_refuses_paths_remote_addresses_and_missing_programs() {
    let site = tempfile::tempdir().expect("site");
    let env = env_with(site.path());
    let with = |command: &[&str], url: Option<&str>| SiteConfig {
        preview: Some(maquette_base::site_config::PreviewConfig {
            command: command.iter().map(|s| (*s).to_string()).collect(),
            url: url.map(str::to_string),
        }),
        ..SiteConfig::legacy()
    };
    let path = command::choose(site.path(), &with(&["./serve.sh"], None), None);
    assert_eq!(path.expect_err("path").code(), "invalid");
    let remote = command::choose(
        site.path(),
        &with(&["pnpm"], Some("https://example.com/")),
        None,
    );
    assert_eq!(remote.expect_err("remote").code(), "invalid");
    let found =
        command::choose(site.path(), &with(&["no-such-tool-here"], None), None).expect("command");
    let missing = plan(site.path(), &found, &env);
    assert_eq!(missing.expect_err("missing").code(), "unavailable");
}

#[test]
fn answers_only_when_something_listens() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    assert!(answers(&format!("http://127.0.0.1:{port}/")));
    drop(listener);
    // Not the freed port: under load another process may take it. No one
    // can listen on port 0, so a connect there never succeeds.
    assert!(!answers("http://127.0.0.1:0/"));
}
