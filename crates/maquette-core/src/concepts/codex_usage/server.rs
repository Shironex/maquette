//! One conversation with `codex app-server`: start it, shake hands, ask
//! for the rate limits, and stop it whatever happened.

use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use process_wrap::std::{ChildWrapper, CommandWrap};
use serde_json::{Value, json};

use super::{CodexUsage, from_response};
use crate::pipeline::history::now_ms;

const INITIALIZE: u64 = 1;
const READ: u64 = 2;

/// The running server. Dropping it kills it with its whole process group
/// (job object on Windows) and reaps it, so no read leaves one behind.
struct Server(Box<dyn ChildWrapper>);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.start_kill();
        let _ = self.0.wait();
    }
}

/// Asks the app server at `program` for the usage, giving up after
/// `timeout`. Every failure reads as unknown, with the reason.
pub(super) fn ask(program: &Path, path_env: &OsStr, timeout: Duration) -> CodexUsage {
    let started = now_ms();
    match talk(program, path_env, timeout) {
        Ok(result) => from_response(&result, now_ms()),
        Err(reason) => {
            tracing::info!(%reason, "the Codex usage read gave no answer");
            CodexUsage::unknown(reason, started)
        }
    }
}

/// The PATH the server gets: its own folder first (an npm shim finds its
/// `node` there), then the toolbox's.
fn child_path(program: &Path, path_env: &OsStr) -> std::ffi::OsString {
    let mut dirs: Vec<PathBuf> = program
        .parent()
        .map(Path::to_path_buf)
        .into_iter()
        .collect();
    dirs.extend(std::env::split_paths(path_env));
    std::env::join_paths(dirs).unwrap_or_else(|_| path_env.to_os_string())
}

fn spawn(program: &Path, path_env: &OsStr) -> Result<Server, String> {
    let path = child_path(program, path_env);
    let mut command = CommandWrap::with_new(program, |cmd| {
        cmd.arg("app-server")
            .current_dir(std::env::temp_dir())
            .env("PATH", &path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
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
    command
        .spawn()
        .map(Server)
        .map_err(|e| format!("Could not start Codex's app server. {e}"))
}

fn talk(program: &Path, path_env: &OsStr, timeout: Duration) -> Result<Value, String> {
    let deadline = Instant::now() + timeout;
    let mut server = spawn(program, path_env)?;
    let output = server
        .0
        .stdout()
        .take()
        .ok_or("Codex's app server has no output.")?;
    // Declared after the server, so it closes first when the read ends.
    let mut input = server
        .0
        .stdin()
        .take()
        .ok_or("Codex's app server has no input.")?;
    let lines = read_lines(output);
    send(
        &mut input,
        &json!({
            "id": INITIALIZE,
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "maquette",
                    "title": "Maquette",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        }),
    )?;
    answer(&lines, INITIALIZE, deadline, timeout)?;
    send(&mut input, &json!({ "method": "initialized" }))?;
    send(
        &mut input,
        &json!({ "id": READ, "method": "account/rateLimits/read" }),
    )?;
    answer(&lines, READ, deadline, timeout)
}

/// Reads the server's lines on a thread of its own; it ends when the
/// server is killed and its output closes.
fn read_lines(output: impl std::io::Read + Send + 'static) -> Receiver<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(output).lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    rx
}

fn send(input: &mut ChildStdin, message: &Value) -> Result<(), String> {
    let mut line = message.to_string();
    line.push('\n');
    input
        .write_all(line.as_bytes())
        .and_then(|()| input.flush())
        .map_err(|e| format!("Codex's app server stopped listening. {e}"))
}

/// True when `message` answers request `id` (a number, or its text).
fn answers(message: &Value, id: u64) -> bool {
    match message.get("id") {
        Some(Value::Number(n)) => n.as_u64() == Some(id),
        Some(Value::String(s)) => s.trim() == id.to_string(),
        _ => false,
    }
}

/// Waits for the answer to request `id`, skipping notifications and lines
/// that are not JSON. An error answer is the failure, with its message.
pub(super) fn answer(
    lines: &Receiver<String>,
    id: u64,
    deadline: Instant,
    timeout: Duration,
) -> Result<Value, String> {
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let line = match lines.recv_timeout(left) {
            Ok(line) => line,
            Err(RecvTimeoutError::Timeout) => {
                return Err(format!(
                    "Codex's app server did not answer within {} s.",
                    timeout.as_secs()
                ));
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err("Codex's app server stopped before it answered.".into());
            }
        };
        let Ok(message) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if !answers(&message, id) {
            continue;
        }
        if let Some(error) = message.get("error") {
            let text = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("no message");
            return Err(format!("Codex's app server refused the request: {text}"));
        }
        return message
            .get("result")
            .cloned()
            .ok_or_else(|| "Codex's app server answered without a result.".into());
    }
}
