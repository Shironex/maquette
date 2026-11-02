//! A canvas's change log: `plans/<id>.log.jsonl` beside the canvas file,
//! one JSON line per write by a person or an agent, appended and never
//! edited, so a sync tool sends only the new lines and the canvas file
//! stays small and readable.
//!
//! ```json
//! {"id":"c-mh2k1x9a001","at":"2026-11-02T12:40:00.000Z","by":"Research agent","run":"r-1240",
//!  "summary":"Added note: Dusk light","mark":"add","ops":[{"op":"add-node","node":{...}}],
//!  "undoes":[],"trimmed":false}
//! ```
//!
//! `by` is null for a person. `ops` are the operations as written
//! ([`super::ops`]), which is what an undo turns around; `undoes` names the
//! lines an undo took back. Lines that do not read (a write cut short, a
//! sync conflict) are skipped.
//!
//! What stays undoable, and for how long:
//!
//! - An agent's line that changed a person's content (their node or link,
//!   or an agent node they wrote in last) is the only place their earlier
//!   version lives, so it is a keeper ([`keep`]): rotation never drops it
//!   until it is released, by an undo (the person's or the agent's, which
//!   writes its line even when nothing is left to undo) or by the person's
//!   Keep. An agent write is refused when its line would be over
//!   [`LINE_MAX_BYTES`] (split it into smaller calls) or when it changes a
//!   person's content and the keepers would pass [`KEEP_MAX_BYTES`] (the
//!   person undoes or keeps agent changes first). Undo lines are never
//!   keepers and are never refused for room.
//! - Every other line, a person's own edits and agents' changes to their
//!   own nodes, stays undoable while it is among the newest lines: past
//!   [`LOG_MAX_BYTES`] one atomic rewrite keeps the keepers and the newest
//!   [`LOG_KEEP_BYTES`] of the rest. A person's own line over
//!   [`LINE_MAX_BYTES`] keeps its summary but drops its operations
//!   (`trimmed`); the canvas holds what it wrote.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::ops::PlanOp;
use super::summary::ChangeMark;

mod keep;
use super::{PLANS_DIR, is_plan_id};
use crate::error::{CoreError, CoreResult};
pub use keep::{is_keeper, keeper_bytes, protected_node, released_ids};

pub const LOG_SUFFIX: &str = ".log.jsonl";
/// Past this size the oldest lines go.
pub const LOG_MAX_BYTES: u64 = 1024 * 1024;
/// What a rewrite keeps, so it happens rarely.
pub const LOG_KEEP_BYTES: u64 = 512 * 1024;
/// The longest line. A person's longer line keeps only its summary; an
/// agent's longer write is refused. Room for one node's longest text (20 000
/// characters, up to 4 bytes each) before and after.
pub const LINE_MAX_BYTES: usize = 256 * 1024;
/// The most the keepers (agents' changes to a person's content not undone
/// yet) may hold; past it agents cannot change a person's content.
pub const KEEP_MAX_BYTES: u64 = 4 * 1024 * 1024;
/// The most of the log that is read: the keepers and the rest together.
const READ_MAX_BYTES: u64 = KEEP_MAX_BYTES + LOG_MAX_BYTES * 2;

/// One write to a canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanLogEntry {
    pub id: String,
    /// RFC 3339.
    pub at: String,
    /// The agent's name; `None` for a person.
    pub by: Option<String>,
    /// An agent's run, grouping several calls.
    pub run: Option<String>,
    pub summary: String,
    pub mark: ChangeMark,
    #[serde(default)]
    pub ops: Vec<PlanOp>,
    /// The lines this one undid.
    #[serde(default)]
    pub undoes: Vec<String>,
    /// The agent lines a person kept (accepted), which releases them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keeps: Vec<String>,
    /// The operations were too big to keep.
    #[serde(default)]
    pub trimmed: bool,
}

/// A log line as the Changes panel and agents read it: no operations.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanLogLine {
    pub id: String,
    pub at: String,
    pub by: Option<String>,
    pub run: Option<String>,
    pub summary: String,
    pub mark: ChangeMark,
    /// A later line undid it.
    pub undone: bool,
    /// It can be undone: it changed nodes or edges, kept its operations
    /// and was not undone yet.
    pub undoable: bool,
    /// An agent's change to a person's content the log still keeps for
    /// undo: Keep accepts it and frees that room.
    pub keepable: bool,
    /// A person kept (accepted) it.
    pub kept: bool,
}

static SEQ: AtomicU32 = AtomicU32::new(0);

fn base36(mut n: u128) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    loop {
        out.push(DIGITS[usize::try_from(n % 36).unwrap_or(0)]);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// A short id that sorts by when it was made: the prefix, the time in
/// milliseconds and a counter, all base 36, as the view makes node ids.
pub fn new_id(prefix: &str) -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let seq = SEQ.fetch_add(1, Ordering::Relaxed) % (36 * 36 * 36);
    format!("{prefix}-{}{:0>3}", base36(ms), base36(u128::from(seq)))
}

/// `<project>/plans/<id>.log.jsonl`.
pub fn path(project_dir: &Path, id: &str) -> CoreResult<PathBuf> {
    if !is_plan_id(id) {
        return Err(CoreError::NotFound(format!("the canvas {id}")));
    }
    Ok(project_dir
        .join(PLANS_DIR)
        .join(format!("{id}{LOG_SUFFIX}")))
}

fn json(entry: &PlanLogEntry) -> CoreResult<String> {
    serde_json::to_string(entry).map_err(|err| CoreError::Invalid(format!("A log line. {err}")))
}

/// How long `entry`'s line is, in bytes.
///
/// # Errors
///
/// Fails when it does not serialize.
pub fn line_len(entry: &PlanLogEntry) -> CoreResult<usize> {
    Ok(json(entry)?.len())
}

/// The JSON line of `entry`, its operations dropped when it is too long
/// (then true).
fn line_of(entry: &PlanLogEntry) -> CoreResult<(String, bool)> {
    let line = json(entry)?;
    if line.len() <= LINE_MAX_BYTES {
        return Ok((line, false));
    }
    let cut = json(&PlanLogEntry {
        ops: Vec::new(),
        trimmed: true,
        ..entry.clone()
    })?;
    Ok((cut, true))
}

/// The bytes the keepers would hold once `entry` (a line `len` bytes long)
/// is appended to the canvas's log.
///
/// # Errors
///
/// Fails when the log cannot be read.
pub fn held_after(
    project_dir: &Path,
    id: &str,
    entry: &PlanLogEntry,
    len: usize,
) -> CoreResult<u64> {
    keep::held_after(&path(project_dir, id)?, entry, len)
}

/// Whether the file is missing, empty or ends with a line break, so the next
/// line starts on its own (a write cut short leaves half a line).
fn ends_whole(file: &Path) -> CoreResult<bool> {
    let mut handle = match fs::File::open(file) {
        Ok(handle) => handle,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(true),
        Err(err) => return Err(CoreError::io(file, err)),
    };
    let len = handle.metadata().map_err(|e| CoreError::io(file, e))?.len();
    if len == 0 {
        return Ok(true);
    }
    handle
        .seek(SeekFrom::Start(len - 1))
        .map_err(|e| CoreError::io(file, e))?;
    let mut last = [0u8; 1];
    handle
        .read_exact(&mut last)
        .map_err(|e| CoreError::io(file, e))?;
    Ok(last[0] == b'\n')
}

/// Appends `entry` in one write, then drops the oldest lines when the lines
/// that are not keepers passed [`LOG_MAX_BYTES`]. The canvas write lock is held by the caller,
/// so lines land in the order the writes did.
///
/// # Errors
///
/// Fails when the log cannot be written.
pub fn append(project_dir: &Path, id: &str, entry: &PlanLogEntry) -> CoreResult<()> {
    let file = path(project_dir, id)?;
    let before = keep::stamp(&file);
    let (mut line, cut) = line_of(entry)?;
    line.push('\n');
    if !ends_whole(&file)? {
        line.insert(0, '\n');
    }
    let mut handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .map_err(|e| CoreError::io(&file, e))?;
    handle
        .write_all(line.as_bytes())
        .map_err(|e| CoreError::io(&file, e))?;
    drop(handle);
    if keep::noted(&file, entry, !cut, line.len(), before)? > LOG_MAX_BYTES {
        let trimmed = trim(&file);
        keep::forget(&file);
        trimmed?;
    }
    Ok(())
}

/// Once the other lines pass [`LOG_MAX_BYTES`], keeps every keeper
/// ([`is_keeper`]) and the newest other lines that fit in
/// [`LOG_KEEP_BYTES`]; lines that do not read go.
fn trim(file: &Path) -> CoreResult<()> {
    let text = read_tail(file, READ_MAX_BYTES)?;
    let parsed: Vec<(&str, PlanLogEntry)> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok().map(|e| (l, e)))
        .collect();
    let all: Vec<PlanLogEntry> = parsed.iter().map(|(_, e)| e.clone()).collect();
    let undone = released_ids(&all);
    let rest: u64 = parsed
        .iter()
        .filter(|(_, e)| !is_keeper(e, &undone))
        .map(|(l, _)| u64::try_from(l.len() + 1).unwrap_or(u64::MAX))
        .sum();
    if rest <= LOG_MAX_BYTES {
        // The keepers alone make the file long: nothing may go yet.
        return Ok(());
    }
    let mut kept: Vec<&str> = Vec::new();
    let mut size = 0u64;
    for (line, entry) in parsed.iter().rev() {
        if is_keeper(entry, &undone) {
            kept.push(line);
            continue;
        }
        size += u64::try_from(line.len() + 1).unwrap_or(u64::MAX);
        if size <= LOG_KEEP_BYTES || kept.is_empty() {
            kept.push(line);
        }
    }
    kept.reverse();
    let mut out = kept.join("\n");
    out.push('\n');
    maquette_base::fs::write_atomic(file, out.as_bytes()).map_err(|e| CoreError::io(file, e))
}

/// The last `max` bytes of the file, from the first whole line in them.
fn read_tail(file: &Path, max: u64) -> CoreResult<String> {
    let mut handle = match fs::File::open(file) {
        Ok(handle) => handle,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(String::new()),
        Err(err) => return Err(CoreError::io(file, err)),
    };
    let len = handle.metadata().map_err(|e| CoreError::io(file, e))?.len();
    let start = len.saturating_sub(max);
    handle
        .seek(SeekFrom::Start(start))
        .map_err(|e| CoreError::io(file, e))?;
    let mut bytes = Vec::new();
    handle
        .take(max)
        .read_to_end(&mut bytes)
        .map_err(|e| CoreError::io(file, e))?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if start == 0 {
        return Ok(text);
    }
    // The first line was cut by the seek.
    Ok(text
        .split_once('\n')
        .map(|(_, rest)| rest.to_string())
        .unwrap_or_default())
}

/// Every line of the log that reads, oldest first.
///
/// # Errors
///
/// Fails when the log exists but cannot be read.
pub fn entries(project_dir: &Path, id: &str) -> CoreResult<Vec<PlanLogEntry>> {
    let file = path(project_dir, id)?;
    let text = read_tail(&file, READ_MAX_BYTES)?;
    Ok(text
        .lines()
        .filter_map(|line| serde_json::from_str::<PlanLogEntry>(line).ok())
        .collect())
}

/// The ids some line undid.
pub fn undone_ids(entries: &[PlanLogEntry]) -> BTreeSet<String> {
    entries
        .iter()
        .flat_map(|e| e.undoes.iter().cloned())
        .collect()
}

/// The newest `limit` lines, newest first, without their operations.
///
/// # Errors
///
/// As [`entries`].
pub fn lines(project_dir: &Path, id: &str, limit: usize) -> CoreResult<Vec<PlanLogLine>> {
    let all = entries(project_dir, id)?;
    let undone = undone_ids(&all);
    let released = released_ids(&all);
    let kept: BTreeSet<&String> = all.iter().flat_map(|e| e.keeps.iter()).collect();
    let flags: Vec<(bool, bool)> = all
        .iter()
        .map(|e| (is_keeper(e, &released), kept.contains(&e.id)))
        .collect();
    Ok(all
        .into_iter()
        .zip(flags)
        .rev()
        .take(limit)
        .map(|(e, (keepable, kept))| {
            let gone = undone.contains(&e.id);
            PlanLogLine {
                keepable,
                kept,
                undoable: !gone && !e.trimmed && !e.ops.is_empty(),
                undone: gone,
                id: e.id,
                at: e.at,
                by: e.by,
                run: e.run,
                summary: e.summary,
                mark: e.mark,
            }
        })
        .collect())
}

/// Moves the canvas's log into the trash beside where the canvas goes
/// (`to_stem` with the log suffix). A canvas without a log has nothing to
/// move. Returns where it went, so a canvas move that fails can put it back.
///
/// # Errors
///
/// Fails when the log stays: a new canvas with the same id would inherit
/// it, so the remove must not go on.
pub(super) fn move_beside(
    project_dir: &Path,
    id: &str,
    to_stem: &Path,
) -> CoreResult<Option<PathBuf>> {
    let from = path(project_dir, id)?;
    if !from.is_file() {
        return Ok(None);
    }
    let to = to_stem.with_extension("").with_extension("log.jsonl");
    keep::forget(&from);
    if maquette_base::fs::rename_retrying(&from, &to).is_ok() {
        return Ok(Some(to));
    }
    fs::copy(&from, &to).map_err(|e| CoreError::io(&from, e))?;
    fs::remove_file(&from).map_err(|e| CoreError::io(&from, e))?;
    Ok(Some(to))
}
