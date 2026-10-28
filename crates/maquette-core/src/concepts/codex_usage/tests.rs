use serde_json::{Value, json};

use super::*;

/// The shape codex-cli 0.159.3 answered with on a Plus account, the
/// account id left out.
fn plus_answer() -> Value {
    let snapshot = json!({
        "limitId": "codex",
        "limitName": null,
        "normalModelSlug": null,
        "primary": { "usedPercent": 16, "windowDurationMins": 300, "resetsAt": 1_793_240_965 },
        "secondary": { "usedPercent": 3, "windowDurationMins": 10080, "resetsAt": 1_793_827_765 },
        "credits": { "hasCredits": false, "unlimited": false, "balance": "0" },
        "individualLimit": null,
        "spendControlReached": false,
        "planType": "plus",
        "rateLimitReachedType": null
    });
    json!({
        "ordinaryUsageAllowed": true,
        "rateLimits": snapshot.clone(),
        "rateLimitsByLimitId": { "codex": snapshot },
        "rateLimitResetCredits": { "availableCount": 0, "credits": [] },
        "rateLimitUpsell": null
    })
}

fn limited_answer() -> Value {
    json!({
        "ordinaryUsageAllowed": false,
        "rateLimits": {
            "primary": { "usedPercent": 100, "windowDurationMins": 300, "resetsAt": 1_793_240_965 },
            "secondary": { "usedPercent": 41, "windowDurationMins": 10080, "resetsAt": 1_793_827_765 },
            "planType": "plus",
            "rateLimitReachedType": "rate_limit_reached"
        }
    })
}

#[test]
fn reads_both_windows_the_plan_and_the_credits() {
    let usage = from_response(&plus_answer(), 5.0);
    assert!(usage.known);
    assert_eq!(usage.read_at_ms, 5.0);
    let primary = usage.primary.clone().expect("primary");
    assert_eq!(primary.used_percent, 16.0);
    assert_eq!(primary.window_mins, Some(300.0));
    assert_eq!(primary.resets_at_ms, Some(1_793_240_965_000.0));
    assert_eq!(
        usage.secondary.as_ref().map(|w| w.window_mins),
        Some(Some(10080.0))
    );
    assert_eq!(usage.plan.as_deref(), Some("plus"));
    assert_eq!(usage.ordinary_usage_allowed, Some(true));
    assert_eq!(usage.reset_credits, Some(0.0));
    assert_eq!(
        usage.credits.as_ref().and_then(|c| c.balance.as_deref()),
        Some("0")
    );
    assert!(!usage.limit_active());
    assert_eq!(usage.resets_at_ms(), None);
    assert!(usage.can_run_now());
}

#[test]
fn a_full_window_is_an_active_limit_that_lifts_at_its_reset() {
    let usage = from_response(&limited_answer(), 0.0);
    assert!(usage.limit_active());
    assert!(!usage.can_run_now());
    assert_eq!(usage.limit_reached.as_deref(), Some("rate_limit_reached"));
    assert_eq!(usage.resets_at_ms(), Some(1_793_240_965_000.0));
    // A full week waits for the later reset.
    let mut week = usage.clone();
    if let Some(w) = week.secondary.as_mut() {
        w.used_percent = 100.0;
    }
    assert_eq!(week.resets_at_ms(), Some(1_793_827_765_000.0));
    // The server says no while the window is not full: the limit holds and
    // the five hour reset is the best guess.
    let mut refused = from_response(&plus_answer(), 0.0);
    refused.ordinary_usage_allowed = Some(false);
    assert!(refused.limit_active());
    assert_eq!(refused.resets_at_ms(), Some(1_793_240_965_000.0));
    assert!(!refused.can_run_now());
    // Without ordinaryUsageAllowed nothing is read into the percentages.
    let mut silent = from_response(&plus_answer(), 0.0);
    silent.ordinary_usage_allowed = None;
    assert!(!silent.can_run_now());
}

#[test]
fn shapes_it_cannot_read_are_unknown_never_errors() {
    for answer in [
        json!({}),
        json!({ "rateLimits": null }),
        json!({ "rateLimits": { "primary": { "usedPercent": "lots" } } }),
        json!([1, 2]),
        Value::Null,
    ] {
        let usage = from_response(&answer, 1.0);
        assert!(!usage.known, "{answer}");
        assert!(usage.reason.is_some());
        assert!(!usage.limit_active());
        assert!(!usage.can_run_now());
    }
    // Windows may be missing on their own.
    let bare = from_response(&json!({ "rateLimits": {} }), 1.0);
    assert!(bare.known);
    assert_eq!(bare.primary, None);
    assert!(!bare.can_run_now());
}

#[test]
fn only_a_codex_new_enough_is_asked() {
    assert!(supported(Some("0.159.3")));
    assert!(supported(Some("1.0.0")));
    assert!(supported(Some("0.130.0-alpha.1")));
    assert!(!supported(Some("0.46.0")));
    assert!(!supported(Some("unknown")));
    assert!(!supported(None));
}

#[test]
fn a_toolbox_without_codex_reads_unknown() {
    let usage = read(&Toolbox::default());
    assert!(!usage.known);
    assert!(
        usage
            .reason
            .as_deref()
            .is_some_and(|r| r.contains("not installed"))
    );
}

#[cfg(unix)]
mod fake_server {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use super::super::server::ask;

    /// A stand-in `codex` that answers the handshake, and the read with
    /// `answer` (a JSON line, or nothing to stay silent). It writes its pid
    /// next to itself.
    fn fake_codex(dir: &Path, answer: Option<&str>) -> PathBuf {
        let path = dir.join("codex");
        let pid = dir.join("pid");
        let read = answer.map_or_else(
            || "sleep 30".to_string(),
            |a| format!("printf '%s\\n' '{a}'"),
        );
        let script = format!(
            "#!/bin/sh\n[ \"$1\" = app-server ] || exit 2\necho $$ > '{}'\n\
             while IFS= read -r line; do\n  case \"$line\" in\n\
             *'\"initialize\"'*) echo 'not json'; echo '{{\"method\":\"account/updated\"}}'; echo '{{\"id\":1,\"result\":{{}}}}';;\n\
             *rateLimits/read*) {read};;\n  esac\ndone\n",
            pid.display()
        );
        std::fs::write(&path, script).expect("write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        path
    }

    fn alive(dir: &Path) -> bool {
        let Ok(pid) = std::fs::read_to_string(dir.join("pid")) else {
            return false;
        };
        std::process::Command::new("kill")
            .args(["-0", pid.trim()])
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    fn path_env() -> std::ffi::OsString {
        std::env::var_os("PATH").unwrap_or_default()
    }

    #[test]
    fn shakes_hands_skips_notifications_and_reads_the_answer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let answer = r#"{"id":2,"result":{"ordinaryUsageAllowed":true,"rateLimits":{"primary":{"usedPercent":16,"windowDurationMins":300,"resetsAt":1793240965}}}}"#;
        let codex = fake_codex(dir.path(), Some(answer));
        let usage = ask(&codex, &path_env(), Duration::from_secs(10));
        assert!(usage.known, "{:?}", usage.reason);
        assert_eq!(usage.primary.map(|w| w.used_percent), Some(16.0));
        assert!(!alive(dir.path()), "the server is stopped after the read");
    }

    #[test]
    fn an_error_answer_reads_unknown_with_its_message() {
        let dir = tempfile::tempdir().expect("tempdir");
        let answer = r#"{"id":2,"error":{"code":-32601,"message":"method not found"}}"#;
        let codex = fake_codex(dir.path(), Some(answer));
        let usage = ask(&codex, &path_env(), Duration::from_secs(10));
        assert!(!usage.known);
        assert!(
            usage
                .reason
                .as_deref()
                .is_some_and(|r| r.contains("method not found"))
        );
    }

    #[test]
    fn a_silent_server_times_out_and_is_killed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let codex = fake_codex(dir.path(), None);
        let started = Instant::now();
        let usage = ask(&codex, &path_env(), Duration::from_millis(600));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(!usage.known);
        assert!(
            usage
                .reason
                .as_deref()
                .is_some_and(|r| r.contains("did not answer"))
        );
        assert!(!alive(dir.path()), "the timed out server is gone");
    }

    /// Reads the real account through the `codex` on PATH: read only and
    /// free. `cargo test -p maquette-core -- --ignored live_codex_usage`
    #[test]
    #[ignore = "needs a signed in Codex CLI"]
    fn live_codex_usage() {
        let codex = std::process::Command::new("which")
            .arg("codex")
            .output()
            .expect("which");
        let program = String::from_utf8_lossy(&codex.stdout).trim().to_string();
        let usage = ask(Path::new(&program), &path_env(), super::super::READ_TIMEOUT);
        assert!(usage.known, "{:?}", usage.reason);
        println!(
            "5h {:?}% week {:?}% allowed {:?}",
            usage.primary.map(|w| w.used_percent),
            usage.secondary.map(|w| w.used_percent),
            usage.ordinary_usage_allowed
        );
    }

    #[test]
    fn a_missing_program_reads_unknown() {
        let usage = ask(
            Path::new("/no/such/codex"),
            &path_env(),
            Duration::from_secs(1),
        );
        assert!(!usage.known);
    }
}
