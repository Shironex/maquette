//! Images someone already has, added to a project's `refs/`. A ref can be in
//! use by another asset or a past send, so a file there is never replaced.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use super::{MAX_NAME_TRIES, check_drop, file_name};
use crate::concepts::REFS_DIR;
use crate::error::{CoreError, CoreResult};

/// The file's name, with the extension its header says when its own says
/// something else or nothing.
fn ref_name(path: &Path, extension: &str) -> String {
    let own = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    let fits = match own.as_deref() {
        Some("jpeg") => extension == "jpg",
        Some(own) => own == extension,
        None => false,
    };
    if fits {
        return file_name(path);
    }
    let stem = path
        .file_stem()
        .map_or_else(|| "image".into(), |s| s.to_string_lossy().into_owned());
    format!("{stem}.{extension}")
}

/// The name in `names` that is `wanted` to a case insensitive disk: the exact
/// one when there is one.
fn entry_like(names: impl Iterator<Item = String>, wanted: &str) -> Option<String> {
    let lower = wanted.to_lowercase();
    let mut like = None;
    for name in names {
        if name == wanted {
            return Some(name);
        }
        if name.to_lowercase() == lower {
            like = Some(name);
        }
    }
    like
}

/// `front.png`, then `front-2.png` and on.
fn numbered(name: &str, n: u32) -> String {
    if n == 1 {
        return name.to_string();
    }
    match name.rsplit_once('.') {
        Some((stem, extension)) => format!("{stem}-{n}.{extension}"),
        None => format!("{name}-{n}"),
    }
}

/// Both are at most an image's limit: `from` was checked and `there` has its size.
fn same_bytes(from: &Path, there: &Path) -> io::Result<bool> {
    let (a, b) = (fs::metadata(from)?, fs::metadata(there)?);
    Ok(b.is_file() && a.len() == b.len() && fs::read(from)? == fs::read(there)?)
}

fn listing(folder: &Path) -> CoreResult<Vec<String>> {
    let entries = fs::read_dir(folder).map_err(|e| CoreError::io(folder, e))?;
    Ok(entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect())
}

/// The name `from` ends up under in `refs`, and whether this call wrote it:
/// the first free name, or the one already holding the same bytes.
fn place(from: &Path, refs: &Path, name: &str) -> CoreResult<(String, bool)> {
    for n in 1..=MAX_NAME_TRIES {
        let candidate = numbered(name, n);
        // A case insensitive disk would answer for this name with another's
        // file, so every disk treats them as one and keeps the real name.
        if let Some(real) = entry_like(listing(refs)?.into_iter(), &candidate) {
            let there = refs.join(&real);
            if same_bytes(from, &there).map_err(|e| CoreError::io(&there, e))? {
                return Ok((real, false));
            }
            continue;
        }
        let to = refs.join(&candidate);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&to)
        {
            Ok(mut out) => {
                let copied =
                    fs::File::open(from).and_then(|mut input| io::copy(&mut input, &mut out));
                if let Err(error) = copied {
                    let _ = fs::remove_file(&to);
                    return Err(CoreError::io(from, error));
                }
                return Ok((candidate, true));
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                if same_bytes(from, &to).map_err(|e| CoreError::io(&to, e))? {
                    return Ok((candidate, false));
                }
            }
            Err(error) => return Err(CoreError::io(&to, error)),
        }
    }
    Err(CoreError::Invalid(format!(
        "{MAX_NAME_TRIES} other refs are already called {name}. Rename the file and add it again."
    )))
}

/// Copies images into `refs/` and returns their final paths relative to the
/// project, each once: an image already there under its name is reused, and
/// one that differs gets a free name beside it. Every file is checked before
/// anything is written, and a failed copy takes back what this call wrote.
pub fn import_refs(project_dir: &Path, paths: &[PathBuf]) -> CoreResult<Vec<String>> {
    let extensions = check_drop(paths)?;
    let refs = project_dir.join(REFS_DIR);
    fs::create_dir_all(&refs).map_err(|e| CoreError::io(&refs, e))?;
    let mut out = Vec::with_capacity(paths.len());
    let mut written = Vec::new();
    for (from, extension) in paths.iter().zip(extensions) {
        match place(from, &refs, &ref_name(from, extension)) {
            Ok((name, wrote)) => {
                if wrote {
                    written.push(refs.join(&name));
                }
                let path = format!("{REFS_DIR}/{name}");
                if !out.contains(&path) {
                    out.push(path);
                }
            }
            Err(error) => {
                for file in &written {
                    let _ = fs::remove_file(file);
                }
                return Err(error);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::entry_like;

    /// A case sensitive disk cannot show the real name a case insensitive one
    /// answers with, so this runs the rule on a listing.
    #[test]
    fn the_real_name_of_an_entry_wins_over_the_name_asked_for() {
        let names = ["Front.png".to_string(), "back.png".to_string()];
        assert_eq!(
            entry_like(names.iter().cloned(), "front.png"),
            Some(names[0].clone())
        );
        assert_eq!(
            entry_like(names.iter().cloned(), "back.png"),
            Some(names[1].clone())
        );
        assert_eq!(entry_like(names.iter().cloned(), "left.png"), None);
    }

    #[test]
    fn an_exact_name_wins_when_a_disk_holds_both_cases() {
        let names = ["Front.png".to_string(), "front.png".to_string()];
        assert_eq!(
            entry_like(names.iter().cloned(), "front.png"),
            Some(names[1].clone())
        );
    }
}
