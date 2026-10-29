//! Small filesystem helpers shared by every module.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Writes `bytes` to `path` through a temp file in the same folder, so a crash
/// never leaves a half written file behind.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let dir = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = dir.join(format!(
        ".{}.{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        nanos
    ));

    let result = write_and_rename(&tmp, path, bytes);
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn write_and_rename(tmp: &Path, path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(tmp)?;
    file.write_all(bytes)?;
    file.sync_data()?;
    drop(file);
    rename_retrying(tmp, path)
}

/// `fs::rename`, tried a few more times on Windows while the target is
/// held open: a sync tool, a virus scanner or the search indexer reading
/// the old file makes the replace fail with "access denied" for a moment.
fn rename_retrying(from: &Path, to: &Path) -> io::Result<()> {
    const WAITS_MS: [u64; 5] = [20, 50, 100, 200, 400];
    let mut result = fs::rename(from, to);
    for wait in WAITS_MS {
        match &result {
            Err(err) if cfg!(windows) && err.kind() == io::ErrorKind::PermissionDenied => {
                std::thread::sleep(std::time::Duration::from_millis(wait));
                result = fs::rename(from, to);
            }
            _ => break,
        }
    }
    result
}

/// True for a path written on any computer as absolute: `/Users/me`,
/// `\\server\share` or `C:\Users\me`. A path from macOS is not absolute to
/// Windows' own check, so this looks at the text.
pub fn looks_absolute(text: &str) -> bool {
    let bytes = text.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\');
    text.starts_with('/') || text.starts_with('\\') || drive
}

/// How a workspace file records `path`: relative to `root` with `/` between
/// folders when it is inside, so the file reads the same on every computer
/// the workspace syncs to, else the absolute path as it is.
pub fn to_workspace(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) if !rel.as_os_str().is_empty() => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        _ => path.display().to_string(),
    }
}

/// A path a workspace file recorded, on this computer: a relative one under
/// `root` (never above it), an absolute one as it is.
pub fn from_workspace(root: &Path, recorded: &str) -> PathBuf {
    if looks_absolute(recorded) {
        PathBuf::from(recorded)
    } else {
        recorded
            .split(['/', '\\'])
            .filter(|part| !part.is_empty() && *part != "." && *part != "..")
            .fold(root.to_path_buf(), |path, part| path.join(part))
    }
}

/// Expands a leading `~` to the home folder. Anything else is returned as is.
pub fn expand_home(input: &str, home: Option<&Path>) -> PathBuf {
    let Some(home) = home else {
        return PathBuf::from(input);
    };
    if input == "~" {
        return home.to_path_buf();
    }
    match input
        .strip_prefix("~/")
        .or_else(|| input.strip_prefix("~\\"))
    {
        Some(rest) => home.join(rest),
        None => PathBuf::from(input),
    }
}

/// Shortens a path under the home folder to `~/...` for display.
pub fn collapse_home(path: &Path, home: Option<&Path>) -> String {
    let Some(home) = home.filter(|h| !h.as_os_str().is_empty()) else {
        return path.display().to_string();
    };
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~{MAIN_SEPARATOR}{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// Total size in bytes of every file under `path`. Errors are skipped and
/// symlinks are not followed.
pub fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.file_type().is_symlink() {
        return 0;
    }
    if meta.is_file() {
        return meta.len();
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| dir_size(&entry.path()))
        .sum()
}

/// Removes everything inside `path` but keeps the folder. Returns the bytes
/// freed. A missing folder frees nothing.
pub fn clear_dir_contents(path: &Path) -> io::Result<u64> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(err) => return Err(err),
    };
    let mut freed = 0;
    for entry in entries {
        let child = entry?.path();
        let meta = fs::symlink_metadata(&child)?;
        if meta.is_dir() {
            freed += dir_size(&child);
            fs::remove_dir_all(&child)?;
        } else {
            if meta.is_file() {
                freed += meta.len();
            }
            fs::remove_file(&child)?;
        }
    }
    Ok(freed)
}

/// Copies the folder `from` to `to` (which must not exist), files and
/// subfolders. Symlinks are skipped.
pub fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Moves the folder `from` to `to` (which must not exist). A rename when it
/// can; otherwise (another volume, or Windows refusing to rename a folder
/// with a file open in it) a copy and then a removal. When the removal
/// fails the copy is removed again, so the folder ends up in one place.
pub fn move_tree(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", to.display()),
        ));
    }
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    if let Err(err) = copy_tree(from, to) {
        let _ = fs::remove_dir_all(to);
        return Err(err);
    }
    if let Err(err) = fs::remove_dir_all(from) {
        let _ = fs::remove_dir_all(to);
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_tree_moves_nested_folders_and_refuses_to_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let from = dir.path().join("a");
        fs::create_dir_all(from.join("sub")).expect("mkdir");
        fs::write(from.join("one.txt"), b"1").expect("write");
        fs::write(from.join("sub").join("two.txt"), b"22").expect("write");
        let to = dir.path().join("b");
        move_tree(&from, &to).expect("move");
        assert!(!from.exists());
        assert_eq!(
            fs::read(to.join("sub").join("two.txt")).expect("read"),
            b"22"
        );

        let copy = dir.path().join("c");
        copy_tree(&to, &copy).expect("copy");
        assert_eq!(dir_size(&copy), 3);
        assert!(move_tree(&to, &copy).is_err(), "c exists");
        assert!(to.join("one.txt").is_file(), "nothing moved");
    }

    #[test]
    fn workspace_paths_are_relative_inside_and_absolute_outside() {
        let root = Path::new("/ws");
        assert_eq!(
            to_workspace(root, &root.join("projects").join("studio").join("handoff")),
            "projects/studio/handoff"
        );
        assert_eq!(
            to_workspace(root, Path::new("/elsewhere/x")),
            "/elsewhere/x"
        );
        assert_eq!(
            from_workspace(root, "projects/studio/handoff"),
            root.join("projects").join("studio").join("handoff")
        );
        assert_eq!(
            from_workspace(root, "projects\\studio"),
            root.join("projects").join("studio")
        );
        assert_eq!(
            from_workspace(root, "/Users/me/x"),
            PathBuf::from("/Users/me/x")
        );
        for absolute in ["/Users/me", "C:\\Users\\me", "d:/ws", "\\\\server\\share"] {
            assert!(looks_absolute(absolute), "{absolute}");
        }
        for relative in ["projects/a", "C:", "a:b", ""] {
            assert!(!looks_absolute(relative), "{relative}");
        }
    }

    #[test]
    fn write_atomic_replaces_and_leaves_no_temp() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a.json");
        write_atomic(&path, b"one").expect("first write");
        write_atomic(&path, b"two").expect("second write");
        assert_eq!(fs::read_to_string(&path).expect("read"), "two");
        let names: Vec<_> = fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.json".to_string()]);
    }

    #[test]
    fn write_atomic_fails_in_missing_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("missing").join("a.json");
        assert!(write_atomic(&path, b"x").is_err());
    }

    #[test]
    fn expand_and_collapse_home() {
        let home = Path::new("/Users/me");
        assert_eq!(expand_home("~", Some(home)), PathBuf::from("/Users/me"));
        assert_eq!(
            expand_home("~/Maquette", Some(home)),
            PathBuf::from("/Users/me/Maquette")
        );
        assert_eq!(
            expand_home("~\\Maquette", Some(home)),
            home.join("Maquette")
        );
        assert_eq!(expand_home("/tmp/x", Some(home)), PathBuf::from("/tmp/x"));
        assert_eq!(expand_home("~other", Some(home)), PathBuf::from("~other"));
        assert_eq!(expand_home("~/x", None), PathBuf::from("~/x"));

        assert_eq!(
            collapse_home(Path::new("/Users/me/Maquette"), Some(home)),
            format!("~{MAIN_SEPARATOR}Maquette")
        );
        assert_eq!(collapse_home(Path::new("/Users/me"), Some(home)), "~");
        assert_eq!(collapse_home(Path::new("/opt/x"), Some(home)), "/opt/x");
        assert_eq!(collapse_home(Path::new("/opt/x"), None), "/opt/x");
    }

    #[test]
    fn size_and_clear() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("cache");
        fs::create_dir_all(root.join("nested")).expect("mkdir");
        fs::write(root.join("a.bin"), [0u8; 10]).expect("write");
        fs::write(root.join("nested").join("b.bin"), [0u8; 5]).expect("write");
        assert_eq!(dir_size(&root), 15);
        assert_eq!(clear_dir_contents(&root).expect("clear"), 15);
        assert!(root.is_dir());
        assert_eq!(dir_size(&root), 0);
        assert_eq!(
            clear_dir_contents(&dir.path().join("nope")).expect("missing"),
            0
        );
    }

    #[cfg(unix)]
    #[test]
    fn dir_size_skips_symlinks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = dir.path().join("outside");
        fs::create_dir(&outside).expect("mkdir");
        fs::write(outside.join("big.bin"), [0u8; 100]).expect("write");
        let root = dir.path().join("root");
        fs::create_dir(&root).expect("mkdir");
        std::os::unix::fs::symlink(&outside, root.join("link")).expect("symlink");
        assert_eq!(dir_size(&root), 0);
        clear_dir_contents(&root).expect("clear");
        assert!(outside.join("big.bin").exists());
    }
}
