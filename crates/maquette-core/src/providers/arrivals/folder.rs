//! Watching one folder (Downloads) for new model files, through `notify`
//! (FSEvents on macOS, ReadDirectoryChangesW on Windows). Only reads: a
//! file is reported once its size settles, and nothing in the folder is
//! ever created, moved or deleted.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use notify::event::ModifyKind;
use notify::{EventKind, RecursiveMode, Watcher as _};

use super::settle::{Settle, Step};
use crate::error::{CoreError, CoreResult};

/// The file types a web studio exports a model as.
pub const WATCHED: [&str; 3] = ["glb", "fbx", "zip"];

/// How often tracked files are measured, and how long one must hold still.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchTiming {
    pub tick: Duration,
    pub quiet: Duration,
    pub give_up: Duration,
}

impl WatchTiming {
    pub const APP: Self = Self {
        tick: Duration::from_millis(500),
        quiet: Duration::from_millis(1500),
        give_up: Duration::from_secs(30 * 60),
    };
}

/// True for a `.glb`, `.fbx` or `.zip` that is not hidden. Partial downloads
/// (`.crdownload`, `.part`, `.download`) end in something else.
pub fn is_watched(path: &Path) -> bool {
    let hidden = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_none_or(|n| n.starts_with('.'));
    !hidden
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| WATCHED.iter().any(|w| e.eq_ignore_ascii_case(w)))
}

/// A running watch. Dropping it stops the watch and its thread.
pub(super) struct FolderWatch {
    // Declared first so it drops first, which ends the event thread.
    watcher: Option<notify::RecommendedWatcher>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for FolderWatch {
    fn drop(&mut self) {
        drop(self.watcher.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn size_of(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()
        .filter(std::fs::Metadata::is_file)
        .map(|m| m.len())
}

/// Watches `dir` (not created when missing) and calls `on_stable` with each
/// model file once it has settled.
/// Changes that can mean new bytes. Reads, removals and metadata (a
/// quarantine flag, a permission change) never do.
fn counts(kind: &EventKind) -> bool {
    !matches!(
        kind,
        EventKind::Access(_) | EventKind::Remove(_) | EventKind::Modify(ModifyKind::Metadata(_))
    )
}

pub(super) fn watch_folder(
    dir: &Path,
    timing: WatchTiming,
    on_stable: impl Fn(PathBuf) + Send + 'static,
) -> CoreResult<FolderWatch> {
    if !dir.is_dir() {
        return Err(CoreError::NotFound(format!("the folder {}", dir.display())));
    }
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })
    .map_err(|e| CoreError::Unavailable(format!("Could not watch for downloads. {e}")))?;
    watcher
        .watch(dir, RecursiveMode::NonRecursive)
        .map_err(|e| CoreError::Unavailable(format!("Could not watch {}. {e}", dir.display())))?;
    let thread = thread::Builder::new()
        .name("downloads-watch".to_string())
        .spawn(move || {
            let mut settle = Settle::new(timing.quiet, timing.give_up);
            loop {
                // Nothing to settle: sleep until the folder changes.
                let next = if settle.is_empty() {
                    rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
                } else {
                    rx.recv_timeout(timing.tick)
                };
                match next {
                    Ok(Ok(event)) => {
                        if counts(&event.kind) {
                            let now = Instant::now();
                            for path in event.paths.iter().filter(|p| is_watched(p)) {
                                settle.touch(path, now);
                            }
                        }
                    }
                    Ok(Err(err)) => tracing::warn!(error = %err, "downloads watch error"),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                if settle.is_empty() {
                    continue;
                }
                let now = Instant::now();
                for path in settle.paths() {
                    if settle.observe(&path, size_of(&path), now) == Step::Stable {
                        on_stable(path);
                    }
                }
            }
        })
        .map_err(|e| CoreError::Unavailable(format!("Could not start the downloads watch. {e}")))?;
    Ok(FolderWatch {
        watcher: Some(watcher),
        thread: Some(thread),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn only_model_exports_are_watched() {
        for yes in [
            "a.glb",
            "B.GLB",
            "c.fbx",
            "d.zip",
            "/dl/maq_x_v1_tripo (1).glb",
        ] {
            assert!(is_watched(Path::new(yes)), "{yes}");
        }
        for no in [
            "a.glb.crdownload",
            "a.glb.part",
            "a.glb.download",
            ".hidden.glb",
            "notes.txt",
            "scene.gltf",
            "noext",
        ] {
            assert!(!is_watched(Path::new(no)), "{no}");
        }
    }

    #[test]
    fn a_new_export_is_reported_once_after_it_settles_and_left_in_place() {
        let dir = tempfile::tempdir().expect("tempdir");
        let seen: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
        let sink = Arc::clone(&seen);
        let timing = WatchTiming {
            tick: Duration::from_millis(50),
            quiet: Duration::from_millis(300),
            give_up: Duration::from_secs(30),
        };
        let watch = watch_folder(dir.path(), timing, move |path| {
            sink.lock().expect("lock").push(path);
        })
        .expect("watch");
        // FSEvents can deliver events from just before the watch started.
        thread::sleep(Duration::from_millis(300));
        std::fs::write(dir.path().join("notes.txt"), b"x").expect("write");
        let file = dir.path().join("maq_lamp_v1_tripo.glb");
        for n in 1..=4 {
            std::fs::write(&file, vec![b'g'; n * 100]).expect("write");
            thread::sleep(Duration::from_millis(40));
        }
        let start = Instant::now();
        while seen.lock().expect("lock").is_empty() && start.elapsed() < Duration::from_secs(10) {
            thread::sleep(Duration::from_millis(50));
        }
        thread::sleep(Duration::from_millis(600));
        drop(watch);
        let seen = seen.lock().expect("lock").clone();
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(seen[0].ends_with("maq_lamp_v1_tripo.glb"));
        assert_eq!(std::fs::metadata(&file).expect("still there").len(), 400);
    }

    #[test]
    fn a_missing_folder_is_an_error_and_is_not_created() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("Downloads");
        assert!(watch_folder(&missing, WatchTiming::APP, |_| {}).is_err());
        assert!(!missing.exists());
    }
}
