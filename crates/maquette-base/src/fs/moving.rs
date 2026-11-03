//! Moving a whole folder without ever losing it.
//!
//! A rename is the only move that is all or nothing. A copy and a removal
//! is used only when the rename cannot work at all (the target is on
//! another volume). Any other refusal (a file held open on Windows, a
//! permission) leaves everything where it was. Once the removal of the
//! source has started, the complete copy is never deleted: when the
//! removal stops partway both folders are kept and the error names them.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{copy_contents, rename_retrying};

/// The error inside an `io::Error` from [`move_tree`] when the folder was
/// copied to another volume but the source could not be fully removed. The
/// copy at `to` is complete; what is left at `from` is a partial duplicate.
#[derive(Debug)]
pub struct PartialMove {
    pub from: PathBuf,
    pub to: PathBuf,
    pub cause: io::Error,
}

impl fmt::Display for PartialMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the folder was copied to {} in full, but {} could not be removed completely ({}); both are kept, so delete what is left at {} once the copy looks right",
            self.to.display(),
            self.from.display(),
            self.cause,
            self.from.display()
        )
    }
}

impl Error for PartialMove {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}

/// The [`PartialMove`] inside `err`, when a move stopped there.
pub fn partial_move(err: &io::Error) -> Option<&PartialMove> {
    err.get_ref()?.downcast_ref::<PartialMove>()
}

/// True when a rename failed only because the target is on another volume:
/// `EXDEV` on Unix, `ERROR_NOT_SAME_DEVICE` on Windows.
pub fn crosses_devices(err: &io::Error) -> bool {
    const EXDEV: i32 = 18;
    const ERROR_NOT_SAME_DEVICE: i32 = 17;
    let raw = if cfg!(windows) {
        ERROR_NOT_SAME_DEVICE
    } else {
        EXDEV
    };
    err.kind() == io::ErrorKind::CrossesDevices || err.raw_os_error() == Some(raw)
}

/// Moves the folder `from` to `to` (which must not exist). A rename, tried
/// again on Windows while a file in it is held open; a copy and a removal
/// only when `to` is on another volume. See the module notes for what is
/// kept when something fails.
///
/// # Errors
///
/// Fails with nothing moved when `to` exists or the rename is refused for
/// any reason but another volume, and with a [`PartialMove`] inside when
/// the source could not be fully removed after a complete copy.
pub fn move_tree(from: &Path, to: &Path) -> io::Result<()> {
    move_tree_with(from, to, rename_retrying, remove_all)
}

fn remove_all(path: &Path) -> io::Result<()> {
    fs::remove_dir_all(path)
}

pub(super) fn move_tree_with(
    from: &Path,
    to: &Path,
    rename: impl Fn(&Path, &Path) -> io::Result<()>,
    remove: impl Fn(&Path) -> io::Result<()>,
) -> io::Result<()> {
    if to.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", to.display()),
        ));
    }
    match rename(from, to) {
        Ok(()) => return Ok(()),
        Err(err) if !crosses_devices(&err) => return Err(err),
        Err(_) => {}
    }
    // Claimed first, so the cleanup below only ever deletes our own copy.
    fs::create_dir(to)?;
    if let Err(err) = copy_contents(from, to) {
        // The source is untouched, so the half copy can go.
        let _ = fs::remove_dir_all(to);
        return Err(err);
    }
    remove(from).map_err(|cause| {
        io::Error::new(
            cause.kind(),
            PartialMove {
                from: from.to_path_buf(),
                to: to.to_path_buf(),
                cause,
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cross_device(_: &Path, _: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::CrossesDevices,
            "another volume",
        ))
    }

    fn folder(root: &Path) -> PathBuf {
        let from = root.join("a");
        fs::create_dir_all(from.join("sub")).expect("mkdir");
        fs::write(from.join("one.txt"), b"1").expect("write");
        fs::write(from.join("sub").join("two.txt"), b"22").expect("write");
        from
    }

    #[test]
    fn a_refused_rename_moves_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let from = folder(dir.path());
        let to = dir.path().join("b");
        let busy = |_: &Path, _: &Path| -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "file open"))
        };
        let err = move_tree_with(&from, &to, busy, remove_all).expect_err("busy");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert!(partial_move(&err).is_none());
        assert!(!to.exists(), "no copy was made");
        assert_eq!(super::super::dir_size(&from), 3, "the source is whole");
    }

    #[test]
    fn another_volume_copies_then_removes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let from = folder(dir.path());
        let to = dir.path().join("b");
        move_tree_with(&from, &to, cross_device, remove_all).expect("move");
        assert!(!from.exists());
        assert_eq!(
            fs::read(to.join("sub").join("two.txt")).expect("read"),
            b"22"
        );
    }

    #[test]
    fn a_removal_that_stops_partway_keeps_the_whole_copy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let from = folder(dir.path());
        let to = dir.path().join("b");
        // Deletes one file, then meets a file another app holds open.
        let stuck = |path: &Path| -> io::Result<()> {
            fs::remove_file(path.join("one.txt"))?;
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "two.txt is open",
            ))
        };
        let err = move_tree_with(&from, &to, cross_device, stuck).expect_err("stuck");
        let partial = partial_move(&err).expect("a partial move");
        assert_eq!(partial.to, to);
        assert!(err.to_string().contains(&to.display().to_string()));
        assert!(err.to_string().contains(&from.display().to_string()));
        assert_eq!(fs::read(to.join("one.txt")).expect("read"), b"1");
        assert_eq!(
            fs::read(to.join("sub").join("two.txt")).expect("read"),
            b"22"
        );
        assert!(
            from.join("sub").join("two.txt").is_file(),
            "what is left stays"
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_undeletable_file_never_costs_the_copy() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let from = folder(dir.path());
        let locked = from.join("sub");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).expect("lock");
        let to = dir.path().join("b");
        let result = move_tree_with(&from, &to, cross_device, remove_all);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("unlock");
        let err = result.expect_err("sub/two.txt cannot be deleted");
        assert!(partial_move(&err).is_some());
        assert_eq!(fs::read(to.join("one.txt")).expect("read"), b"1");
        assert_eq!(
            fs::read(to.join("sub").join("two.txt")).expect("read"),
            b"22"
        );
        assert!(locked.join("two.txt").is_file());
    }

    #[test]
    fn a_failed_copy_leaves_only_the_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let to = dir.path().join("b");
        let missing = dir.path().join("nope");
        assert!(move_tree_with(&missing, &to, cross_device, remove_all).is_err());
        assert!(!to.exists());
    }
}
