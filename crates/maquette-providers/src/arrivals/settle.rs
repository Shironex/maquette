//! When a download is done: its size has stayed the same, and above zero,
//! for a quiet spell. Browsers write in bursts, and some create the final
//! name empty first, so neither the first event nor a size alone says the
//! file is whole. Time is passed in, so this is testable without sleeping.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What one look at a tracked file found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    /// Still growing, empty, or not quiet for long enough yet.
    Waiting,
    /// Whole: take it. It is no longer tracked.
    Stable,
    /// Removed or renamed away.
    Gone,
    /// Never settled within the limit.
    GaveUp,
}

#[derive(Debug)]
struct Tracked {
    size: Option<u64>,
    changed: Instant,
    first: Instant,
}

#[derive(Debug)]
pub(super) struct Settle {
    quiet: Duration,
    give_up: Duration,
    files: HashMap<PathBuf, Tracked>,
}

impl Settle {
    pub(super) fn new(quiet: Duration, give_up: Duration) -> Self {
        Self {
            quiet,
            give_up,
            files: HashMap::new(),
        }
    }

    /// Something happened to `path`: track it, or restart its quiet spell.
    pub(super) fn touch(&mut self, path: &Path, now: Instant) {
        self.files
            .entry(path.to_path_buf())
            .and_modify(|t| t.changed = now)
            .or_insert(Tracked {
                size: None,
                changed: now,
                first: now,
            });
    }

    /// The tracked paths, sorted.
    pub(super) fn paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self.files.keys().cloned().collect();
        paths.sort();
        paths
    }

    pub(super) fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Looks at `path` now that it measures `size` (`None`: not there).
    pub(super) fn observe(&mut self, path: &Path, size: Option<u64>, now: Instant) -> Step {
        let Some(tracked) = self.files.get_mut(path) else {
            return Step::Gone;
        };
        let step = if size.is_none() {
            Step::Gone
        } else if now.saturating_duration_since(tracked.first) >= self.give_up {
            Step::GaveUp
        } else if size != tracked.size {
            tracked.size = size;
            tracked.changed = now;
            Step::Waiting
        } else if size == Some(0) || now.saturating_duration_since(tracked.changed) < self.quiet {
            Step::Waiting
        } else {
            Step::Stable
        };
        if step != Step::Waiting {
            self.files.remove(path);
        }
        step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    #[test]
    fn a_file_is_stable_once_its_size_holds_for_the_quiet_spell() {
        let start = Instant::now();
        let mut s = Settle::new(Duration::from_millis(1000), Duration::from_secs(60));
        let path = Path::new("/dl/crate.glb");
        s.touch(path, start);
        assert_eq!(s.paths(), vec![path.to_path_buf()]);
        assert_eq!(s.observe(path, Some(100), at(start, 0)), Step::Waiting);
        assert_eq!(s.observe(path, Some(900), at(start, 500)), Step::Waiting);
        assert_eq!(s.observe(path, Some(900), at(start, 1200)), Step::Waiting);
        assert_eq!(s.observe(path, Some(900), at(start, 1500)), Step::Stable);
        assert!(s.is_empty(), "a stable file is handed over once");
    }

    #[test]
    fn an_empty_placeholder_never_counts_as_done() {
        let start = Instant::now();
        let mut s = Settle::new(Duration::from_millis(100), Duration::from_secs(60));
        let path = Path::new("/dl/model.glb");
        s.touch(path, start);
        assert_eq!(s.observe(path, Some(0), at(start, 0)), Step::Waiting);
        assert_eq!(s.observe(path, Some(0), at(start, 5000)), Step::Waiting);
        assert_eq!(s.observe(path, Some(42), at(start, 5100)), Step::Waiting);
        assert_eq!(s.observe(path, Some(42), at(start, 5200)), Step::Stable);
    }

    #[test]
    fn activity_restarts_the_spell_and_removed_or_stuck_files_drop_out() {
        let start = Instant::now();
        let mut s = Settle::new(Duration::from_millis(1000), Duration::from_secs(10));
        let path = Path::new("/dl/a.zip");
        s.touch(path, start);
        assert_eq!(s.observe(path, Some(5), at(start, 0)), Step::Waiting);
        s.touch(path, at(start, 900));
        assert_eq!(s.observe(path, Some(5), at(start, 1200)), Step::Waiting);
        assert_eq!(s.observe(path, Some(5), at(start, 1900)), Step::Stable);

        s.touch(path, at(start, 2000));
        assert_eq!(s.observe(path, None, at(start, 2100)), Step::Gone);
        assert_eq!(s.observe(path, Some(5), at(start, 2200)), Step::Gone);

        let slow = Path::new("/dl/slow.glb");
        s.touch(slow, start);
        for ms in (0..10_000).step_by(500) {
            assert_eq!(s.observe(slow, Some(ms), at(start, ms)), Step::Waiting);
        }
        assert_eq!(s.observe(slow, Some(1), at(start, 10_000)), Step::GaveUp);
        assert!(s.is_empty());
    }
}
