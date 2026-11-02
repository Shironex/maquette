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
//! lines an undo took back. The log keeps the newest lines under
//! [`LOG_MAX_BYTES`]: past it the oldest go in one atomic rewrite. A line
//! over [`LINE_MAX_BYTES`] keeps its summary but drops its operations
//! (`trimmed`), so it can no longer be undone. Lines that do not read (a
//! write cut short, a sync conflict) are skipped.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::ops::PlanOp;
use super::summary::ChangeMark;
use super::{PLANS_DIR, is_plan_id};
use crate::error::{CoreError, CoreResult};

pub const LOG_SUFFIX: &str = ".log.jsonl";
/// Past this size the oldest lines go.
pub const LOG_MAX_BYTES: u64 = 1024 * 1024;
/// What a rewrite keeps, so it happens rarely.
pub const LOG_KEEP_BYTES: u64 = 512 * 1024;
/// A longer line keeps only its summary.
pub const LINE_MAX_BYTES: usize = 128 * 1024;

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

/// The JSON line of `entry`, its operations dropped when it is too long.
fn line_of(entry: &PlanLogEntry) -> CoreResult<String> {
    let json = |e: &PlanLogEntry| {
        serde_json::to_string(e).map_err(|err| CoreError::Invalid(format!("A log line. {err}")))
    };
    let line = json(entry)?;
    if line.len() <= LINE_MAX_BYTES {
        return Ok(line);
    }
    json(&PlanLogEntry {
        ops: Vec::new(),
        trimmed: true,
        ..entry.clone()
    })
}

/// Appends `entry` in one write, then drops the oldest lines when the log
/// passed [`LOG_MAX_BYTES`]. The canvas write lock is held by the caller,
/// so lines land in the order the writes did.
///
/// # Errors
///
/// Fails when the log cannot be written.
pub fn append(project_dir: &Path, id: &str, entry: &PlanLogEntry) -> CoreResult<()> {
    let file = path(project_dir, id)?;
    let mut line = line_of(entry)?;
    line.push('\n');
    let mut handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .map_err(|e| CoreError::io(&file, e))?;
    handle
        .write_all(line.as_bytes())
        .map_err(|e| CoreError::io(&file, e))?;
    let len = handle
        .metadata()
        .map_err(|e| CoreError::io(&file, e))?
        .len();
    drop(handle);
    if len > LOG_MAX_BYTES {
        trim(&file)?;
    }
    Ok(())
}

/// Keeps the newest whole lines that fit in [`LOG_KEEP_BYTES`].
fn trim(file: &Path) -> CoreResult<()> {
    let text = read_tail(file, LOG_MAX_BYTES * 2)?;
    let mut kept: Vec<&str> = Vec::new();
    let mut size = 0u64;
    for line in text.lines().rev().filter(|l| !l.trim().is_empty()) {
        size += u64::try_from(line.len() + 1).unwrap_or(u64::MAX);
        if size > LOG_KEEP_BYTES && !kept.is_empty() {
            break;
        }
        kept.push(line);
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
    let text = read_tail(&file, LOG_MAX_BYTES * 2)?;
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
    Ok(all
        .into_iter()
        .rev()
        .take(limit)
        .map(|e| {
            let gone = undone.contains(&e.id);
            PlanLogLine {
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

/// Moves the log beside a canvas moved to the trash, as `to` with the log
/// suffix. A canvas without a log moves alone.
pub(super) fn move_beside(project_dir: &Path, id: &str, to_stem: &Path) {
    let Ok(from) = path(project_dir, id) else {
        return;
    };
    if !from.is_file() {
        return;
    }
    let to = to_stem.with_extension("").with_extension("log.jsonl");
    if maquette_base::fs::rename_retrying(&from, &to).is_err() {
        let copied = fs::copy(&from, &to).is_ok();
        if copied {
            let _ = fs::remove_file(&from);
        }
    }
}
