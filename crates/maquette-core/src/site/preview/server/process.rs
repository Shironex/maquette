//! The dev server's process: spawned in its own process group under the
//! watchdog (a job object on Windows), its output read line by line into
//! the log, and the sentence that says why it stopped.

use std::io::{BufRead, BufReader, Read};
use std::process::{ChildStdin, Stdio};
use std::sync::Arc;
use std::thread;

use process_wrap::std::{ChildWrapper, CommandWrap};

use super::{LOG_LINES, Shared, lock};
use crate::error::{CoreError, CoreResult};
use crate::site::preview::{PreviewPlan, find_local_url};

/// Lines of output quoted when the server stops early.
const TAIL_LINES: usize = 12;

/// Ends the dev tool's process group when the app's end of stdin closes,
/// and passes on the tool's exit otherwise. The tool and its args arrive
/// as `"$@"`, never as text in the script.
#[cfg(unix)]
const WATCHDOG: &str = r#"exec 3<&0
"$@" 3<&- &
child=$!
{ read -r _ <&3; kill -TERM 0; } &
wait "$child"
code=$?
trap '' TERM
kill -TERM 0 2>/dev/null
exit "$code""#;

pub(super) struct Proc {
    pub(super) child: Box<dyn ChildWrapper>,
    /// Held open for the watchdog; dropping it ends the server on unix.
    pub(super) _stdin: Option<ChildStdin>,
}

impl Drop for Proc {
    /// A server never outlives the app that started it.
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// Why the server stopped, with its last lines and a word on a busy port.
pub(super) fn exit_message(early: bool, log: &[String], plan: &PreviewPlan) -> String {
    let head = if early {
        "The site's dev server stopped before it answered."
    } else {
        "The site's dev server stopped."
    };
    let tail: Vec<&str> = log
        .iter()
        .skip(log.len().saturating_sub(TAIL_LINES))
        .map(String::as_str)
        .filter(|l| !l.starts_with("$ "))
        .collect();
    let busy = tail.iter().any(|l| {
        let l = l.to_lowercase();
        l.contains("eaddrinuse") || l.contains("address already in use")
    });
    let port = plan
        .port
        .map(|p| p.to_string())
        .or_else(|| plan.url.as_deref().and_then(port_of))
        .unwrap_or_else(|| "its port".to_string());
    let mut message = head.to_string();
    if busy {
        message.push_str(&format!(
            " Port {port} is in use by another program: stop it, or change the port in the dev command."
        ));
    }
    if !tail.is_empty() {
        message.push('\n');
        message.push_str(&tail.join("\n"));
    }
    message
}

fn port_of(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let host_port = rest.split(['/', '?', '#']).next()?;
    host_port.rsplit_once(':').map(|(_, p)| p.to_string())
}

pub(super) fn read_output<R: Read + Send + 'static>(
    stream: R,
    shared: Arc<Shared>,
    generation: u64,
) {
    thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            let mut slot = lock(&shared);
            if slot.generation != generation {
                return;
            }
            if slot.found.is_none() {
                slot.found = find_local_url(&line);
            }
            slot.status.log.push(line);
            let extra = slot.status.log.len().saturating_sub(LOG_LINES);
            slot.status.log.drain(..extra);
            slot.dirty = true;
        }
    });
}

pub(super) fn spawn(plan: &PreviewPlan) -> CoreResult<Proc> {
    #[cfg(unix)]
    let mut command = CommandWrap::with_new("/bin/sh", |cmd| {
        cmd.arg("-c")
            .arg(WATCHDOG)
            .arg("maquette-dev")
            .arg(&plan.program)
            .args(&plan.args);
    });
    #[cfg(windows)]
    let mut command = CommandWrap::with_new(&plan.program, |cmd| {
        cmd.args(&plan.args);
    });
    let cmd = command.command_mut();
    cmd.current_dir(&plan.site)
        .env("PATH", &plan.path_env)
        .env("BROWSER", "none")
        .env("NO_COLOR", "1")
        .env("FORCE_COLOR", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(port) = plan.port {
        cmd.env("PORT", port.to_string());
    }
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
        CoreError::Unavailable(format!(
            "Could not start {} for the site's dev server. {e}",
            plan.program.display()
        ))
    })?;
    let stdin = child.stdin().take();
    Ok(Proc {
        child,
        _stdin: stdin,
    })
}
