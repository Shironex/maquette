//! Spawning one program call: its own process group or job object, the
//! output pipes read line by line, cancel and timeout.

use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use process_wrap::std::CommandWrap;

use super::execution::Execution;
use super::{POLL, lock};
use crate::pipeline::StepDef;
use crate::pipeline::history::{self, LogStream};
use crate::pipeline::progress::Tracker;
use crate::pipeline::tools::ResolvedTool;

/// How long to keep reading pipes after the process exits.
const DRAIN_GRACE: Duration = Duration::from_secs(2);
const MAX_LINE_CHARS: usize = 4000;

impl Execution {
    pub(super) fn spawn(
        &mut self,
        step: &StepDef,
        tool: &ResolvedTool,
        args: &[String],
        root: &Path,
        tracker: &mut Tracker,
    ) -> Result<i32, SpawnError> {
        let path_env: OsString = {
            let mut dirs: Vec<PathBuf> = tool
                .program
                .parent()
                .map(Path::to_path_buf)
                .into_iter()
                .collect();
            dirs.extend(std::env::split_paths(&self.setup.tools.path_env));
            std::env::join_paths(dirs).unwrap_or_else(|_| self.setup.tools.path_env.clone())
        };
        let stdin_text = self.options.stdin.clone();
        let timeout = self.options.timeout;
        let mut command = CommandWrap::with_new(&tool.program, |cmd| {
            cmd.args(args)
                .current_dir(root)
                .env("PATH", &path_env)
                .env("PYTHONUNBUFFERED", "1")
                .stdin(if stdin_text.is_some() {
                    Stdio::piped()
                } else {
                    Stdio::null()
                })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        });
        #[cfg(unix)]
        {
            command.wrap(process_wrap::std::ProcessGroup::leader());
        }
        #[cfg(windows)]
        {
            use windows::Win32::System::Threading::CREATE_NO_WINDOW;
            command.wrap(process_wrap::std::CreationFlags(CREATE_NO_WINDOW));
            command.wrap(process_wrap::std::JobObject);
        }
        let mut child = command.spawn().map_err(|e| {
            SpawnError::Start(format!("Could not start {}. {e}", tool.program.display()))
        })?;
        // The input goes in on its own thread, so a program that prints a
        // lot before it reads never blocks on a full pipe.
        if let (Some(text), Some(mut input)) = (stdin_text, child.stdin().take()) {
            thread::spawn(move || {
                use std::io::Write as _;
                let _ = input.write_all(text.as_bytes());
            });
        }
        let started = Instant::now();
        let mut timed_out = false;
        let (tx, rx) = mpsc::channel::<(LogStream, String)>();
        let readers = [
            child
                .stdout()
                .take()
                .map(|s| read_lines(s, LogStream::Stdout, tx.clone())),
            child
                .stderr()
                .take()
                .map(|s| read_lines(s, LogStream::Stderr, tx.clone())),
        ];
        drop(tx);
        *lock(&self.child) = Some(child);
        let step_id = step.id.clone();
        let mut exit: Option<i32> = None;
        let mut exited_at: Option<Instant> = None;
        loop {
            match rx.recv_timeout(POLL) {
                Ok((stream, text)) => {
                    let level = history::classify(&text);
                    let moved = tracker.line(&text);
                    self.log(Some(&step_id), stream, level, text);
                    if moved {
                        self.report_progress(tracker);
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) if exit.is_some() => break,
                Err(RecvTimeoutError::Disconnected | RecvTimeoutError::Timeout) => {}
            }
            if exit.is_none() {
                let mut slot = lock(&self.child);
                if let Some(child) = slot.as_mut() {
                    if self.cancel.load(Ordering::SeqCst) {
                        let _ = child.start_kill();
                    }
                    if !timed_out && timeout.is_some_and(|limit| started.elapsed() > limit) {
                        timed_out = true;
                        let _ = child.start_kill();
                    }
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            exit = Some(status.code().unwrap_or(-1));
                            exited_at = Some(Instant::now());
                        }
                        Ok(None) => {}
                        Err(err) => {
                            let _ = child.start_kill();
                            drop(slot);
                            *lock(&self.child) = None;
                            return Err(SpawnError::Start(format!(
                                "Lost track of the process. {err}"
                            )));
                        }
                    }
                }
            }
            if exited_at.is_some_and(|t| t.elapsed() > DRAIN_GRACE) {
                break;
            }
        }
        *lock(&self.child) = None;
        for reader in readers.into_iter().flatten() {
            if reader.is_finished() {
                let _ = reader.join();
            }
        }
        if self.cancel.load(Ordering::SeqCst) {
            return Err(SpawnError::Cancelled);
        }
        if let Some(limit) = timeout.filter(|_| timed_out) {
            return Err(SpawnError::TimedOut(limit));
        }
        Ok(exit.unwrap_or(-1))
    }
}

/// One program call: an optional log label and its args.
pub(super) type Invocation = (Option<String>, Vec<String>);

pub(super) enum SpawnError {
    Start(String),
    Cancelled,
    TimedOut(Duration),
}

/// "0.4s", "45s", "2m", "1m 30s".
pub(super) fn human_duration(d: Duration) -> String {
    let secs = d.as_secs();
    match (secs / 60, secs % 60) {
        (0, _) if d.subsec_millis() > 0 => format!("{:.1}s", d.as_secs_f64()),
        (0, s) => format!("{s}s"),
        (m, 0) => format!("{m}m"),
        (m, s) => format!("{m}m {s}s"),
    }
}

pub(super) fn program_name(tool: &ResolvedTool) -> String {
    tool.program
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| tool.program.display().to_string())
}

/// Reads `stream` line by line (lossy UTF-8, `\r` progress lines split too)
/// and sends each to `tx`.
fn read_lines<R: Read + Send + 'static>(
    stream: R,
    kind: LogStream,
    tx: mpsc::Sender<(LogStream, String)>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let text = String::from_utf8_lossy(&buf);
                    for part in text.split('\r') {
                        let line = part.trim_end_matches(['\n', '\r']);
                        if line.trim().is_empty() {
                            continue;
                        }
                        let line: String = line.chars().take(MAX_LINE_CHARS).collect();
                        if tx.send((kind, line)).is_err() {
                            return;
                        }
                    }
                }
            }
        }
    })
}
