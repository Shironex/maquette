use std::path::{Path, PathBuf};

use super::process::exit_message;
use super::*;
use crate::free_port;
use maquette_base::toolchain::{ProbeEnv, find_on_path};

/// A tiny dev server in Node: prints its address the way Vite does, then
/// answers on `$PORT`.
const SERVE: &str = "const http = require('http');\
http.createServer((q, r) => r.end('ok')).listen(+process.env.PORT, '127.0.0.1', () => \
console.log('  Local  http://127.0.0.1:' + process.env.PORT + '/'));";

fn node() -> Option<PathBuf> {
    find_on_path(&["node"], &ProbeEnv::from_system())
}

fn node_plan(site: &Path, node: &Path, script: &str) -> PreviewPlan {
    PreviewPlan {
        site: site.to_path_buf(),
        program: node.to_path_buf(),
        args: vec!["-e".into(), script.into()],
        url: None,
        port: Some(free_port().expect("port")),
        port_in_args: true,
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        line: "node -e <server>".into(),
        key: "node -e <server>".into(),
    }
}

fn gone(url: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while answers(url) && Instant::now() < deadline {
        thread::sleep(POLL);
    }
    !answers(url)
}

#[test]
fn a_server_starts_reports_its_address_and_stops() {
    let Some(node) = node() else { return };
    let site = tempfile::tempdir().expect("site");
    let plan = node_plan(site.path(), &node, SERVE);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let heard = Arc::clone(&seen);
    let preview = SitePreview::new(move |s| {
        heard.lock().expect("seen").push(s.state);
    });
    let first = preview.begin(&plan, "moon-island", ServerOwner::Page);
    assert_eq!(first.state, ServerState::Starting);
    assert_eq!(first.project.as_deref(), Some("moon-island"));
    let status = preview.wait(Duration::from_secs(20));
    assert_eq!(status.state, ServerState::Running, "{status:?}");
    // The port answers as the address prints, so the server may first run
    // at the guessed localhost url and then move to the printed one.
    let printed = format!("http://127.0.0.1:{}/", plan.port.expect("port"));
    let deadline = Instant::now() + Duration::from_secs(10);
    while preview.status().url.as_deref() != Some(printed.as_str()) && Instant::now() < deadline {
        thread::sleep(POLL);
    }
    let status = preview.status();
    assert_eq!(status.url.as_deref(), Some(printed.as_str()), "{status:?}");
    assert!(!status.external);
    assert!(
        status.log.iter().any(|l| l.contains("Local")),
        "{:?}",
        status.log
    );
    // The app hears a state right after the slot takes it, so give the
    // listener a moment instead of reading it the instant the wait returns.
    let heard_running = || seen.lock().expect("seen").contains(&ServerState::Running);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !heard_running() && Instant::now() < deadline {
        thread::sleep(POLL);
    }
    assert!(heard_running());

    // A second start for the same site reuses it, and Preview on site
    // outranks the view, so the view letting go keeps it running.
    let again = preview.begin(&plan, "moon-island", ServerOwner::Pipeline);
    assert_eq!(again.state, ServerState::Running);
    assert_eq!(again.owner, Some(ServerOwner::Pipeline));
    assert!(!preview.release("moon-island", ServerOwner::Page));
    assert_eq!(preview.status().state, ServerState::Running);

    let url = status.url.expect("url");
    let frame = status.frame_url.expect("frame url");
    assert_eq!(
        again.frame_url.as_deref(),
        Some(frame.as_str()),
        "a reuse keeps the proxy"
    );
    assert!(preview.release("moon-island", ServerOwner::Pipeline));
    assert_eq!(preview.status().state, ServerState::Idle);
    assert!(gone(&url), "the server stopped");
    assert!(gone(&frame), "the frame proxy stopped");
}

/// Answers on `$PORT` without a word, and serves the site on another port
/// it prints after `delay` ms, like a typed command whose tool ignores
/// `PORT` next to one that honours it.
fn two_servers(delay: u32) -> String {
    format!(
        "const http = require('http');\
http.createServer((q, r) => r.end('port')).listen(+process.env.PORT, '127.0.0.1');\
setTimeout(() => {{ const s = http.createServer((q, r) => r.end('site'));\
s.listen(0, '127.0.0.1', () => console.log('  Local  http://127.0.0.1:' + s.address().port + '/')); }}, {delay});"
    )
}

#[test]
fn a_port_given_only_as_port_loses_to_the_printed_address() {
    let Some(node) = node() else { return };
    let site = tempfile::tempdir().expect("site");
    let mut plan = node_plan(site.path(), &node, &two_servers(600));
    plan.port_in_args = false;
    let guess = plan.expected_url().expect("guess");
    let preview = SitePreview::default();
    let url = preview
        .start(&plan, "moon-island", ServerOwner::Pipeline)
        .expect("start");
    assert_ne!(url, guess, "the printed address wins");
    assert!(url.starts_with("http://127.0.0.1:"), "{url}");
    preview.stop();
}

#[test]
fn a_server_running_at_its_port_moves_to_the_address_it_prints_later() {
    let Some(node) = node() else { return };
    let site = tempfile::tempdir().expect("site");
    let plan = node_plan(site.path(), &node, &two_servers(1500));
    let guess = plan.expected_url().expect("guess");
    let preview = SitePreview::default();
    let first = preview
        .start(&plan, "moon-island", ServerOwner::Pipeline)
        .expect("start");
    assert_eq!(
        first.replace("localhost", "127.0.0.1"),
        guess.replace("localhost", "127.0.0.1")
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while preview.status().url.as_deref() == Some(first.as_str()) && Instant::now() < deadline {
        thread::sleep(POLL);
    }
    let status = preview.status();
    let moved = status.url.expect("url");
    assert_ne!(moved, first, "{:?}", status.log);
    assert!(
        status
            .log
            .iter()
            .any(|l| l.contains("so Maquette shows that instead"))
    );
    assert!(status.frame_url.is_some());
    preview.stop();
}

#[test]
fn a_changed_dev_command_starts_afresh() {
    let Some(node) = node() else { return };
    let site = tempfile::tempdir().expect("site");
    let plan = node_plan(site.path(), &node, SERVE);
    let preview = SitePreview::default();
    let first = preview
        .start(&plan, "moon-island", ServerOwner::Page)
        .expect("first");
    let mut changed = node_plan(site.path(), &node, SERVE);
    changed.key = "node -e <another server>".into();
    let second = preview
        .start(&changed, "moon-island", ServerOwner::Page)
        .expect("second");
    assert_ne!(first, second);
    assert!(gone(&first), "the old command's server stopped");
    preview.stop();
}

#[test]
fn the_view_letting_go_stops_its_own_server() {
    let Some(node) = node() else { return };
    let site = tempfile::tempdir().expect("site");
    let plan = node_plan(site.path(), &node, SERVE);
    let preview = SitePreview::default();
    let url = preview
        .start(&plan, "moon-island", ServerOwner::Page)
        .expect("start");
    assert!(!preview.release("another-project", ServerOwner::Page));
    assert!(preview.release("moon-island", ServerOwner::Page));
    assert!(gone(&url));
}

#[test]
fn a_server_that_exits_early_says_why_and_names_a_busy_port() {
    let Some(node) = node() else { return };
    let site = tempfile::tempdir().expect("site");
    let plan = node_plan(
        site.path(),
        &node,
        "console.error('Error: listen EADDRINUSE: address already in use'); process.exit(1)",
    );
    let preview = SitePreview::default();
    let err = preview
        .start(&plan, "moon-island", ServerOwner::Page)
        .expect_err("exits");
    let text = err.to_string();
    assert!(text.contains("stopped before it answered"), "{text}");
    assert!(text.contains("is in use by another program"), "{text}");
    assert!(text.contains("EADDRINUSE"), "{text}");
    let status = preview.status();
    assert_eq!(status.state, ServerState::Failed);
    assert!(status.message.is_some());
}

#[test]
fn a_missing_program_fails_without_a_process() {
    let site = tempfile::tempdir().expect("site");
    let plan = PreviewPlan {
        site: site.path().to_path_buf(),
        program: site.path().join("no-such-dev-tool"),
        args: Vec::new(),
        url: None,
        port: None,
        port_in_args: false,
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        line: "no-such-dev-tool".into(),
        key: "no-such-dev-tool".into(),
    };
    let preview = SitePreview::default();
    preview.begin(&plan, "moon-island", ServerOwner::Agent);
    let status = preview.wait(Duration::from_secs(10));
    assert_eq!(status.state, ServerState::Failed, "{status:?}");
}

#[test]
fn a_server_already_answering_is_reused_and_never_stopped() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let url = format!("http://127.0.0.1:{port}/");
    let site = tempfile::tempdir().expect("site");
    let plan = PreviewPlan {
        site: site.path().to_path_buf(),
        program: site.path().join("never-runs"),
        args: Vec::new(),
        url: Some(url.clone()),
        port: None,
        port_in_args: false,
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        line: "never-runs".into(),
        key: "never-runs".into(),
    };
    let preview = SitePreview::default();
    let status = preview.begin(&plan, "moon-island", ServerOwner::Page);
    assert_eq!(status.state, ServerState::Running);
    assert!(status.external);
    assert_eq!(status.url.as_deref(), Some(url.as_str()));
    // The frame loads it through the frame proxy, which ends with it.
    let frame = status.frame_url.expect("frame url");
    assert_ne!(frame, url);
    assert!(frame.starts_with("http://127.0.0.1:"), "{frame}");
    assert!(answers(&frame));
    preview.stop();
    assert_eq!(preview.status().frame_url, None);
    assert!(gone(&frame), "the frame proxy stopped with the server");
    assert!(answers(&url), "the other program keeps running");
}

#[test]
fn a_server_off_this_machine_shows_without_the_frame_proxy() {
    let mut slot = Slot::default();
    let old = slot.run_at("http://192.168.1.20:3000/".to_string());
    assert!(old.is_none());
    assert_eq!(slot.status.state, ServerState::Running);
    assert_eq!(slot.status.frame_url, None);
    assert!(
        slot.status
            .log
            .iter()
            .any(|l| l.contains("without its frame proxy")),
        "{:?}",
        slot.status.log
    );
}

#[test]
fn a_start_for_another_site_stops_the_first() {
    let Some(node) = node() else { return };
    let one = tempfile::tempdir().expect("one");
    let two = tempfile::tempdir().expect("two");
    let preview = SitePreview::default();
    let first = preview
        .start(
            &node_plan(one.path(), &node, SERVE),
            "one",
            ServerOwner::Page,
        )
        .expect("first");
    let second = preview
        .start(
            &node_plan(two.path(), &node, SERVE),
            "two",
            ServerOwner::Page,
        )
        .expect("second");
    assert!(gone(&first), "the first site's server stopped");
    assert!(answers(&second));
    assert_eq!(preview.status().project.as_deref(), Some("two"));
    preview.stop();
}

#[cfg(unix)]
#[test]
fn the_watchdog_ends_the_server_when_the_app_end_of_the_pipe_closes() {
    use super::process::spawn;
    let Some(node) = node() else { return };
    let site = tempfile::tempdir().expect("site");
    let plan = node_plan(site.path(), &node, SERVE);
    let url = plan.expected_url().expect("url");
    let mut proc_ = spawn(&plan).expect("spawn");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !answers(&url) && Instant::now() < deadline {
        thread::sleep(POLL);
    }
    assert!(answers(&url), "the server came up");
    // As if the app died: its end of stdin closes, nothing is killed.
    proc_._stdin = None;
    assert!(gone(&url), "the watchdog ended the server");
    let _ = proc_.child.wait();
}

#[test]
fn the_exit_message_keeps_the_last_lines() {
    let plan = PreviewPlan {
        site: PathBuf::from("/site"),
        program: PathBuf::from("pnpm"),
        args: Vec::new(),
        url: Some("http://localhost:15292/".into()),
        port: None,
        port_in_args: false,
        path_env: std::ffi::OsString::new(),
        line: "pnpm dev".into(),
        key: "pnpm dev".into(),
    };
    let log: Vec<String> = std::iter::once("$ pnpm dev".to_string())
        .chain((0..20).map(|i| format!("line {i}")))
        .collect();
    let message = exit_message(false, &log, &plan);
    assert!(message.starts_with("The site's dev server stopped."));
    assert!(message.contains("line 19"));
    assert!(!message.contains("line 7\n"));
    let busy = exit_message(true, &["listen EADDRINUSE".to_string()], &plan);
    assert!(busy.contains("Port 15292 is in use"), "{busy}");
}
