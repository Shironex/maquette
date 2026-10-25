use super::*;
use crate::toolchain::HostOs;

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
fn the_plan_runs_the_configured_program_or_the_package_manager() {
    let bin = tempfile::tempdir().expect("bin");
    let site = tempfile::tempdir().expect("site");
    let pnpm = executable(bin.path(), "pnpm", "#!/bin/sh\n");
    std::fs::write(site.path().join("pnpm-lock.yaml"), "").expect("lock");
    let env = env_with(bin.path());
    let plan = plan(site.path(), &SiteConfig::legacy(), &env).expect("plan");
    assert_eq!(plan.program, pnpm);
    assert_eq!(plan.args, vec!["run".to_string(), "dev".to_string()]);
    assert_eq!(plan.url, None);

    let mut config = SiteConfig::legacy();
    config.preview = Some(super::super::PreviewConfig {
        command: vec!["pnpm".into(), "dev".into()],
        url: Some("http://localhost:15292/".into()),
    });
    let plan = super::plan(site.path(), &config, &env).expect("plan");
    assert_eq!(plan.args, vec!["dev".to_string()]);
    assert_eq!(plan.url.as_deref(), Some("http://localhost:15292/"));
}

#[test]
fn the_plan_refuses_paths_remote_addresses_and_missing_programs() {
    let site = tempfile::tempdir().expect("site");
    let env = env_with(site.path());
    let with = |command: &[&str], url: Option<&str>| SiteConfig {
        preview: Some(super::super::PreviewConfig {
            command: command.iter().map(|s| (*s).to_string()).collect(),
            url: url.map(str::to_string),
        }),
        ..SiteConfig::legacy()
    };
    let path = plan(site.path(), &with(&["./serve.sh"], None), &env);
    assert_eq!(path.expect_err("path").code(), "invalid");
    let remote = plan(
        site.path(),
        &with(&["pnpm"], Some("https://example.com/")),
        &env,
    );
    assert_eq!(remote.expect_err("remote").code(), "invalid");
    let missing = plan(site.path(), &with(&["no-such-tool-here"], None), &env);
    assert_eq!(missing.expect_err("missing").code(), "unavailable");
}

#[test]
fn answers_only_when_something_listens() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    assert!(answers(&format!("http://127.0.0.1:{port}/")));
    drop(listener);
    assert!(!answers(&format!("http://127.0.0.1:{port}/")));
}

#[cfg(unix)]
#[test]
fn a_server_starts_reports_its_address_and_stops() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    let bin = tempfile::tempdir().expect("bin");
    let site = tempfile::tempdir().expect("site");
    // A tiny server: prints its address, then listens with Python if there
    // is one, else the test only checks the early exit below.
    let python = ["python3", "python"]
        .into_iter()
        .find(|p| find_on_path(&[*p], &ProbeEnv::from_system()).is_some());
    let Some(python) = python else { return };
    let script = format!(
        "#!/bin/sh\necho \"  Local  http://127.0.0.1:{port}/\"\nexec {python} -m http.server {port} --bind 127.0.0.1\n"
    );
    let program = executable(bin.path(), "fake-dev", &script);
    let plan = PreviewPlan {
        site: site.path().to_path_buf(),
        program,
        args: Vec::new(),
        url: None,
        path_env: std::env::var_os("PATH").unwrap_or_default(),
    };
    let preview = SitePreview::default();
    let url = preview.start(&plan).expect("start");
    assert_eq!(url, format!("http://127.0.0.1:{port}/"));
    // A second start reuses the running server.
    assert_eq!(preview.start(&plan).expect("again"), url);
    preview.stop();
    let deadline = Instant::now() + Duration::from_secs(5);
    while answers(&url) && Instant::now() < deadline {
        thread::sleep(POLL);
    }
    assert!(!answers(&url), "the server stopped");
}

#[cfg(unix)]
#[test]
fn a_server_that_exits_early_says_why() {
    let bin = tempfile::tempdir().expect("bin");
    let site = tempfile::tempdir().expect("site");
    let program = executable(
        bin.path(),
        "broken-dev",
        "#!/bin/sh\necho 'no astro here' >&2\nexit 1\n",
    );
    let plan = PreviewPlan {
        site: site.path().to_path_buf(),
        program,
        args: Vec::new(),
        url: None,
        path_env: std::env::var_os("PATH").unwrap_or_default(),
    };
    let err = SitePreview::default().start(&plan).expect_err("exits");
    assert!(err.to_string().contains("no astro here"), "{err}");
}
