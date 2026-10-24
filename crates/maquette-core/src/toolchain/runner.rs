use std::ffi::OsStr;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Output cap per stream, so a chatty tool cannot grow memory without bound.
const MAX_CAPTURE: u64 = 1024 * 1024;
/// How long to wait for the pipes to close after the process exits. A
/// grandchild can keep a pipe open, and the probe must not hang on it.
const DRAIN_GRACE: Duration = Duration::from_secs(2);
const POLL_EVERY: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeOutput {
    pub status_ok: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProbeError {
    #[error("timed out")]
    Timeout,
    #[error("could not start: {0}")]
    Spawn(String),
}

/// Runs a tool to read its version. Tests swap in a fake so no real tool runs.
pub trait CommandRunner: Sync {
    fn run(
        &self,
        program: &Path,
        args: &[&str],
        path_env: &OsStr,
        timeout: Duration,
    ) -> Result<ProbeOutput, ProbeError>;
}

/// Runs real processes with `std::process::Command`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(
        &self,
        program: &Path,
        args: &[&str],
        path_env: &OsStr,
        timeout: Duration,
    ) -> Result<ProbeOutput, ProbeError> {
        let mut command = Command::new(program);
        command
            .args(args)
            .env("PATH", path_env)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command
            .spawn()
            .map_err(|err| ProbeError::Spawn(err.to_string()))?;
        let deadline = Instant::now() + timeout;
        let stdout = drain(child.stdout.take());
        let stderr = drain(child.stderr.take());

        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ProbeError::Timeout);
                }
                Ok(None) => thread::sleep(POLL_EVERY),
                Err(err) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ProbeError::Spawn(err.to_string()));
                }
            }
        };

        let wait = deadline
            .saturating_duration_since(Instant::now())
            .max(DRAIN_GRACE);
        Ok(ProbeOutput {
            status_ok: status.success(),
            stdout: stdout.recv_timeout(wait).unwrap_or_default(),
            stderr: stderr.recv_timeout(wait).unwrap_or_default(),
        })
    }
}

fn drain<R: Read + Send + 'static>(stream: Option<R>) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut stream) = stream {
            let mut bytes = Vec::new();
            let _ = (&mut stream).take(MAX_CAPTURE).read_to_end(&mut bytes);
            let _ = io::copy(&mut stream, &mut io::sink());
            text = String::from_utf8_lossy(&bytes).into_owned();
        }
        let _ = tx.send(text);
    });
    rx
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn system_runner_reads_output_and_status() {
        let out = SystemRunner
            .run(
                Path::new("/bin/sh"),
                &["-c", "echo hello; echo oops >&2; exit 0"],
                OsStr::new("/usr/bin:/bin"),
                Duration::from_secs(5),
            )
            .expect("runs");
        assert!(out.status_ok);
        assert_eq!(out.stdout.trim(), "hello");
        assert_eq!(out.stderr.trim(), "oops");

        let failed = SystemRunner
            .run(
                Path::new("/bin/sh"),
                &["-c", "exit 3"],
                OsStr::new("/usr/bin:/bin"),
                Duration::from_secs(5),
            )
            .expect("runs");
        assert!(!failed.status_ok);
    }

    #[test]
    fn system_runner_times_out() {
        let started = Instant::now();
        let result = SystemRunner.run(
            Path::new("/bin/sh"),
            &["-c", "sleep 5"],
            OsStr::new("/usr/bin:/bin"),
            Duration::from_millis(200),
        );
        assert_eq!(result, Err(ProbeError::Timeout));
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn system_runner_reports_spawn_failure() {
        let result = SystemRunner.run(
            Path::new("/definitely/not/here"),
            &[],
            OsStr::new(""),
            Duration::from_secs(1),
        );
        assert!(matches!(result, Err(ProbeError::Spawn(_))));
    }
}
