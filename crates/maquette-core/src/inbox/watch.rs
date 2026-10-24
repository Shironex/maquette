//! Watches the inbox folder for new or changed models. File system events
//! come in bursts while a download or an export writes a file, so paths are
//! debounced: a path is reported once it has been quiet for a while.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use notify::{RecursiveMode, Watcher as _};

use crate::error::{CoreError, CoreResult};

/// How long a path must stay quiet before it is reported.
pub const QUIET: Duration = Duration::from_millis(700);

/// True for `.glb` and `.gltf` files.
pub fn is_model(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("glb") || e.eq_ignore_ascii_case("gltf"))
}

/// Collects paths and hands each back once it has been quiet for `quiet`.
/// Time is passed in, so the logic is testable without sleeping.
#[derive(Debug)]
pub struct Debouncer {
    quiet: Duration,
    pending: HashMap<PathBuf, Instant>,
}

impl Debouncer {
    pub fn new(quiet: Duration) -> Self {
        Self {
            quiet,
            pending: HashMap::new(),
        }
    }

    /// Records activity on `path`, restarting its quiet period.
    pub fn push(&mut self, path: PathBuf, now: Instant) {
        self.pending.insert(path, now);
    }

    /// The paths that have been quiet long enough, sorted, and forgets them.
    pub fn ready(&mut self, now: Instant) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, at)| now.saturating_duration_since(**at) >= self.quiet)
            .map(|(p, _)| p.clone())
            .collect();
        for path in &out {
            self.pending.remove(path);
        }
        out.sort();
        out
    }

    /// When the next path will be ready.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.pending.values().min().map(|at| *at + self.quiet)
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// A running watcher. Dropping it stops the watch.
pub struct InboxWatcher {
    // Declared first so it drops first, which ends the event thread.
    watcher: Option<notify::RecommendedWatcher>,
    thread: Option<JoinHandle<()>>,
    dir: PathBuf,
}

impl InboxWatcher {
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

impl Drop for InboxWatcher {
    fn drop(&mut self) {
        drop(self.watcher.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Watches `dir` (created if missing) and calls `on_change` with the model
/// paths that changed, after they have been quiet for `quiet`.
///
/// # Errors
///
/// Fails when the folder cannot be created or watched.
pub fn watch(
    dir: &Path,
    quiet: Duration,
    on_change: impl Fn(Vec<PathBuf>) + Send + 'static,
) -> CoreResult<InboxWatcher> {
    std::fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })
    .map_err(|e| CoreError::Unavailable(format!("Could not watch the inbox. {e}")))?;
    watcher
        .watch(dir, RecursiveMode::NonRecursive)
        .map_err(|e| CoreError::Unavailable(format!("Could not watch {}. {e}", dir.display())))?;
    let thread = thread::Builder::new()
        .name("inbox-watch".to_string())
        .spawn(move || {
            let mut debouncer = Debouncer::new(quiet);
            loop {
                let wait = debouncer
                    .next_deadline()
                    .map_or(Duration::from_millis(500), |d| {
                        d.saturating_duration_since(Instant::now())
                    });
                match rx.recv_timeout(wait) {
                    Ok(Ok(event)) => {
                        let now = Instant::now();
                        for path in event.paths.into_iter().filter(|p| is_model(p)) {
                            debouncer.push(path, now);
                        }
                    }
                    Ok(Err(err)) => tracing::warn!(error = %err, "inbox watch error"),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                let ready = debouncer.ready(Instant::now());
                if !ready.is_empty() {
                    on_change(ready);
                }
            }
        })
        .map_err(|e| CoreError::Unavailable(format!("Could not start the inbox watcher. {e}")))?;
    Ok(InboxWatcher {
        watcher: Some(watcher),
        thread: Some(thread),
        dir: dir.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn bursts_collapse_into_one_report_after_the_quiet_period() {
        let start = Instant::now();
        let ms = |n| start + Duration::from_millis(n);
        let mut d = Debouncer::new(Duration::from_millis(500));
        assert!(d.is_empty());
        let a = PathBuf::from("/inbox/a.glb");
        let b = PathBuf::from("/inbox/b.glb");
        d.push(a.clone(), ms(0));
        d.push(a.clone(), ms(300));
        d.push(b.clone(), ms(100));
        assert_eq!(d.next_deadline(), Some(ms(600)));
        assert_eq!(d.ready(ms(599)), Vec::<PathBuf>::new());
        assert_eq!(d.ready(ms(600)), vec![b.clone()]);
        assert_eq!(
            d.ready(ms(700)),
            Vec::<PathBuf>::new(),
            "a was touched at 300"
        );
        d.push(a.clone(), ms(750));
        assert_eq!(d.ready(ms(1250)), vec![a]);
        assert!(d.is_empty());
        assert_eq!(d.next_deadline(), None);
    }

    #[test]
    fn only_models_count() {
        assert!(is_model(Path::new("x/Character.GLB")));
        assert!(is_model(Path::new("scene.gltf")));
        assert!(!is_model(Path::new("scene.bin")));
        assert!(!is_model(Path::new("notes")));
    }

    #[test]
    fn a_new_file_in_the_folder_is_reported_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let inbox = dir.path().join("inbox");
        let seen: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
        let sink = Arc::clone(&seen);
        let watcher = watch(&inbox, Duration::from_millis(200), move |paths| {
            sink.lock().expect("lock").extend(paths);
        })
        .expect("watch");
        assert_eq!(watcher.dir(), inbox);
        // FSEvents can deliver events from just before the watch started, so
        // give it a moment.
        thread::sleep(Duration::from_millis(300));
        std::fs::write(inbox.join("notes.txt"), b"x").expect("write");
        for _ in 0..5 {
            std::fs::write(inbox.join("crate.glb"), b"partial").expect("write");
            thread::sleep(Duration::from_millis(20));
        }
        let start = Instant::now();
        while seen.lock().expect("lock").is_empty() && start.elapsed() < Duration::from_secs(10) {
            thread::sleep(Duration::from_millis(50));
        }
        thread::sleep(Duration::from_millis(500));
        let seen = seen.lock().expect("lock").clone();
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(seen[0].ends_with("crate.glb"));
        drop(watcher);
    }
}
