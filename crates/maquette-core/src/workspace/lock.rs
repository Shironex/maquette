//! The workspace lock: `.maquette-lock.json` in the workspace root says
//! which computer has the workspace open. A workspace shared between
//! computers by a sync tool should be open on one of them at a time, or
//! both write the same files and the sync tool keeps conflict copies.
//!
//! The app writes the lock when it opens a workspace, refreshes its
//! heartbeat about once a minute and removes it on a clean exit. A lock
//! whose heartbeat is older than [`FRESH_MS`] is stale (that app crashed or
//! the computer slept) and is taken over without asking; a fresh one from
//! another computer is a warning. The lock syncs like any other file, so the
//! other computer sees it; every scan skips it.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;

pub const LOCK_FILE: &str = ".maquette-lock.json";
const LOCK_VERSION: u32 = 1;
/// A heartbeat younger than this means the other app is still running.
pub const FRESH_MS: f64 = 3.0 * 60.0 * 1000.0;
/// How often the open app refreshes its heartbeat.
pub const HEARTBEAT_MS: u64 = 60 * 1000;

/// Who holds the workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLock {
    pub version: u32,
    /// The computer's name, as people know it.
    pub host: String,
    /// A random id made once per computer, so a renamed computer is still
    /// itself. Empty in a lock an older app wrote.
    #[serde(default)]
    pub machine: String,
    /// `macos`, `windows` or `linux`.
    pub os: String,
    pub app_version: String,
    pub pid: u32,
    /// Milliseconds since the epoch, when the app opened the workspace.
    pub opened_at_ms: f64,
    /// Milliseconds since the epoch, refreshed about once a minute.
    pub heartbeat_ms: f64,
}

/// This computer and app, as a lock names them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub host: String,
    pub machine: String,
    pub os: String,
    pub app_version: String,
    pub pid: u32,
}

impl Holder {
    /// True when `lock` was written on this computer, by any run of the app.
    pub fn is_same_machine(&self, lock: &WorkspaceLock) -> bool {
        if !lock.machine.is_empty() && !self.machine.is_empty() {
            return lock.machine == self.machine;
        }
        lock.host.eq_ignore_ascii_case(&self.host)
    }

    fn lock(&self, opened_at_ms: f64, now_ms: f64) -> WorkspaceLock {
        WorkspaceLock {
            version: LOCK_VERSION,
            host: self.host.clone(),
            machine: self.machine.clone(),
            os: self.os.clone(),
            app_version: self.app_version.clone(),
            pid: self.pid,
            opened_at_ms,
            heartbeat_ms: now_ms,
        }
    }
}

/// What opening the workspace finds.
#[derive(Debug, Clone, PartialEq)]
pub enum LockCheck {
    /// No lock, or one that cannot be read.
    Free,
    /// This computer's own lock (this run or an earlier one).
    Ours(WorkspaceLock),
    /// Another computer's lock with an old heartbeat.
    Stale(WorkspaceLock),
    /// Another computer has the workspace open right now.
    Held(WorkspaceLock),
}

/// The lock in `root`, if there is a readable one.
pub fn read(root: &Path) -> Option<WorkspaceLock> {
    let text = fs::read_to_string(root.join(LOCK_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Looks at the lock without changing it.
pub fn check(root: &Path, me: &Holder, now_ms: f64) -> LockCheck {
    let Some(lock) = read(root) else {
        return LockCheck::Free;
    };
    if me.is_same_machine(&lock) {
        LockCheck::Ours(lock)
    } else if (now_ms - lock.heartbeat_ms).abs() < FRESH_MS {
        LockCheck::Held(lock)
    } else {
        LockCheck::Stale(lock)
    }
}

/// Writes this computer's lock, keeping the open time of its own earlier
/// lock.
///
/// # Errors
///
/// Fails when the file cannot be written.
pub fn claim(root: &Path, me: &Holder, now_ms: f64) -> CoreResult<WorkspaceLock> {
    let opened = match read(root) {
        Some(lock) if me.is_same_machine(&lock) && lock.pid == me.pid => lock.opened_at_ms,
        _ => now_ms,
    };
    let lock = me.lock(opened, now_ms);
    write_json(&root.join(LOCK_FILE), &lock)?;
    Ok(lock)
}

/// Refreshes the heartbeat. Writes nothing when another computer took the
/// workspace over meanwhile (they chose Open anyway), and says so.
///
/// # Errors
///
/// Fails when the file cannot be written.
pub fn beat(root: &Path, me: &Holder, now_ms: f64) -> CoreResult<bool> {
    match read(root) {
        Some(lock) if !me.is_same_machine(&lock) => Ok(false),
        _ => claim(root, me, now_ms).map(|_| true),
    }
}

/// Removes the lock when it is this computer's. Another computer's lock
/// stays.
///
/// # Errors
///
/// Fails when the file is there but cannot be removed.
pub fn release(root: &Path, me: &Holder) -> CoreResult<()> {
    let path = root.join(LOCK_FILE);
    match read(root) {
        Some(lock) if !me.is_same_machine(&lock) => return Ok(()),
        None if !path.exists() => return Ok(()),
        _ => {}
    }
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(CoreError::io(&path, err)),
    }
}

/// The computer's name: the one people know it by on macOS, the Windows
/// computer name, else the host name.
pub fn host_name() -> String {
    if let Ok(name) = std::env::var("COMPUTERNAME")
        && !name.trim().is_empty()
    {
        return name.trim().to_string();
    }
    #[cfg(target_os = "macos")]
    if let Some(name) = command_line("scutil", &["--get", "ComputerName"]) {
        return name;
    }
    #[cfg(not(windows))]
    if let Some(name) = command_line("hostname", &[]) {
        return name;
    }
    std::env::var("HOSTNAME")
        .ok()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "this computer".to_string())
}

#[cfg(not(windows))]
fn command_line(program: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holder(host: &str, machine: &str, pid: u32) -> Holder {
        Holder {
            host: host.into(),
            machine: machine.into(),
            os: "macos".into(),
            app_version: "0.1.0".into(),
            pid,
        }
    }

    const NOW: f64 = 1_800_000_000_000.0;

    #[test]
    fn a_missing_or_unreadable_lock_is_free() {
        let dir = tempfile::tempdir().expect("tempdir");
        let me = holder("studio-mac", "m1", 10);
        assert_eq!(check(dir.path(), &me, NOW), LockCheck::Free);
        fs::write(dir.path().join(LOCK_FILE), "not json").expect("write");
        assert_eq!(check(dir.path(), &me, NOW), LockCheck::Free);
    }

    #[test]
    fn another_computers_lock_is_held_while_fresh_then_stale() {
        let dir = tempfile::tempdir().expect("tempdir");
        let other = holder("studio-pc", "m2", 20);
        claim(dir.path(), &other, NOW).expect("claim");
        let me = holder("studio-mac", "m1", 10);
        assert!(matches!(
            check(dir.path(), &me, NOW + 60_000.0),
            LockCheck::Held(l) if l.host == "studio-pc"
        ));
        assert!(matches!(
            check(dir.path(), &me, NOW + FRESH_MS + 1.0),
            LockCheck::Stale(_)
        ));
    }

    #[test]
    fn the_same_machine_is_ours_even_after_a_rename_or_restart() {
        let dir = tempfile::tempdir().expect("tempdir");
        claim(dir.path(), &holder("old-name", "m1", 10), NOW).expect("claim");
        let renamed = holder("new-name", "m1", 11);
        assert!(matches!(
            check(dir.path(), &renamed, NOW),
            LockCheck::Ours(_)
        ));
        // A lock with no machine id (an older app) matches by host name.
        let mut lock = read(dir.path()).expect("lock");
        lock.machine.clear();
        write_json(&dir.path().join(LOCK_FILE), &lock).expect("write");
        assert!(matches!(
            check(dir.path(), &holder("OLD-NAME", "m9", 12), NOW),
            LockCheck::Ours(_)
        ));
    }

    #[test]
    fn beat_keeps_the_open_time_and_stops_after_a_take_over() {
        let dir = tempfile::tempdir().expect("tempdir");
        let me = holder("studio-mac", "m1", 10);
        claim(dir.path(), &me, NOW).expect("claim");
        assert!(beat(dir.path(), &me, NOW + 60_000.0).expect("beat"));
        let lock = read(dir.path()).expect("lock");
        assert_eq!(lock.opened_at_ms, NOW);
        assert_eq!(lock.heartbeat_ms, NOW + 60_000.0);

        let other = holder("studio-pc", "m2", 20);
        claim(dir.path(), &other, NOW + 90_000.0).expect("take over");
        assert!(!beat(dir.path(), &me, NOW + 120_000.0).expect("beat"));
        assert_eq!(read(dir.path()).expect("lock").host, "studio-pc");
    }

    #[test]
    fn release_removes_only_this_computers_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let me = holder("studio-mac", "m1", 10);
        let other = holder("studio-pc", "m2", 20);
        release(dir.path(), &me).expect("nothing to release");
        claim(dir.path(), &other, NOW).expect("claim");
        release(dir.path(), &me).expect("release");
        assert!(dir.path().join(LOCK_FILE).is_file(), "another's lock stays");
        claim(dir.path(), &me, NOW).expect("claim");
        release(dir.path(), &me).expect("release");
        assert!(!dir.path().join(LOCK_FILE).exists());
    }

    #[test]
    fn the_host_name_is_never_empty() {
        assert!(!host_name().trim().is_empty());
    }
}
