//! The site's own preview server, started from the app so the person sees
//! what Ship wrote: the site's dev command, run in the site folder in its
//! own process group (job object on Windows), stopped when another site
//! starts or the app quits. Nothing is built or deployed here.

use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use process_wrap::std::{ChildWrapper, CommandWrap};

use super::SiteConfig;
use crate::error::{CoreError, CoreResult};
use crate::toolchain::{ProbeEnv, find_on_path};

/// How long a server gets to answer after it starts.
pub const START_TIMEOUT: Duration = Duration::from_secs(90);
const POLL: Duration = Duration::from_millis(250);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
/// Lines of output kept for the error when the server stops early.
const TAIL_LINES: usize = 12;

/// What runs and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewPlan {
    pub site: PathBuf,
    pub program: PathBuf,
    pub args: Vec<String>,
    /// From the site config; otherwise read from the output.
    pub url: Option<String>,
    pub path_env: OsString,
}

/// The package manager that runs the site's scripts: the nearest lockfile
/// from the site up (a site inside a monorepo has it at the root), npm
/// without one.
pub fn package_manager(site: &Path) -> &'static str {
    const LOCKS: [(&str, &str); 5] = [
        ("pnpm-lock.yaml", "pnpm"),
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
    ];
    site.ancestors()
        .find_map(|dir| {
            LOCKS
                .iter()
                .find(|(file, _)| dir.join(file).is_file())
                .map(|(_, manager)| *manager)
        })
        .unwrap_or("npm")
}

/// True for an `http://` or `https://` address on this machine:
/// `localhost`, `127.0.0.1` or `[::1]`, with any port and path.
pub fn is_local_url(url: &str) -> bool {
    let Some(rest) = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
    else {
        return false;
    };
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let host_port = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = match host_port.strip_prefix('[') {
        Some(v6) => v6.split(']').next().map(|h| format!("[{h}]")),
        None => host_port.split(':').next().map(str::to_string),
    };
    matches!(host.as_deref(), Some("localhost" | "127.0.0.1" | "[::1]"))
}

/// The first local address in a line of server output, without colour codes
/// or trailing punctuation.
pub fn find_local_url(line: &str) -> Option<String> {
    let mut plain = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // An ANSI escape: skip to its final letter.
            for d in chars.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            plain.push(c);
        }
    }
    plain.split_whitespace().find_map(|word| {
        let start = word.find("http")?;
        let url = word[start..].trim_end_matches([',', '.', ')', ']', '"', '\'']);
        is_local_url(url).then(|| url.to_string())
    })
}

/// What `config` runs for `site`, with the program found on the child PATH.
///
/// # Errors
///
/// Fails when the config names a path instead of a program, its address is
/// not on this machine or the program is not on PATH.
pub fn plan(site: &Path, config: &SiteConfig, env: &ProbeEnv) -> CoreResult<PreviewPlan> {
    let preview = config.preview.clone().unwrap_or_default();
    let mut command = preview.command;
    if command.is_empty() {
        command = vec![package_manager(site).into(), "run".into(), "dev".into()];
    }
    let program = command.remove(0);
    if program.contains(['/', '\\']) {
        return Err(CoreError::Invalid(format!(
            "{}: the preview command must name a program on PATH, like pnpm, not {program}.",
            super::CONFIG_FILE
        )));
    }
    if let Some(url) = preview.url.as_deref()
        && !is_local_url(url)
    {
        return Err(CoreError::Invalid(format!(
            "{}: the preview url must be on this machine (localhost), not {url}.",
            super::CONFIG_FILE
        )));
    }
    let path_env = env.child_path(None);
    let mut probe = env.clone();
    probe.path_dirs = std::env::split_paths(&path_env).collect();
    let found = find_on_path(&[program.as_str()], &probe).ok_or_else(|| {
        CoreError::Unavailable(format!(
            "Could not find {program} on PATH to start the site's preview."
        ))
    })?;
    Ok(PreviewPlan {
        site: site.to_path_buf(),
        program: found,
        args: command,
        url: preview.url,
        path_env,
    })
}

/// True when something accepts connections at `url`'s host and port.
pub fn answers(url: &str) -> bool {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host_port = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let default_port = if url.starts_with("https://") { 443 } else { 80 };
    let target = if host_port
        .rsplit_once(':')
        .is_some_and(|(_, p)| p.parse::<u16>().is_ok())
    {
        host_port.to_string()
    } else {
        format!("{host_port}:{default_port}")
    };
    target.to_socket_addrs().is_ok_and(|mut addrs| {
        addrs.any(|addr| TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).is_ok())
    })
}

struct Running {
    site: PathBuf,
    child: Box<dyn ChildWrapper>,
    url: Arc<Mutex<Option<String>>>,
    tail: Arc<Mutex<Vec<String>>>,
}

impl Drop for Running {
    /// A server never outlives the app that started it.
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn read_output<R: Read + Send + 'static>(
    stream: R,
    url: Arc<Mutex<Option<String>>>,
    tail: Arc<Mutex<Vec<String>>>,
) {
    thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            let mut found = lock(&url);
            if found.is_none() {
                *found = find_local_url(&line);
            }
            drop(found);
            let mut kept = lock(&tail);
            kept.push(line);
            let extra = kept.len().saturating_sub(TAIL_LINES);
            kept.drain(..extra);
        }
    });
}

/// The one preview server the app runs.
#[derive(Default)]
pub struct SitePreview {
    running: Mutex<Option<Running>>,
}

impl SitePreview {
    /// Starts the plan's server, or reuses one that already answers, and
    /// returns its address once it accepts connections. Blocks up to
    /// [`START_TIMEOUT`].
    ///
    /// # Errors
    ///
    /// Fails when the program cannot start, stops before it answers or does
    /// not answer in time.
    pub fn start(&self, plan: &PreviewPlan) -> CoreResult<String> {
        if let Some(url) = plan.url.as_deref().filter(|u| answers(u)) {
            return Ok(url.to_string());
        }
        let mut slot = lock(&self.running);
        let reuse = slot
            .as_mut()
            .is_some_and(|r| r.site == plan.site && matches!(r.child.try_wait(), Ok(None)));
        if !reuse {
            // Dropping the old one stops it.
            *slot = None;
            *slot = Some(spawn(plan)?);
        }
        let (url, tail) = slot
            .as_ref()
            .map(|r| (Arc::clone(&r.url), Arc::clone(&r.tail)))
            .ok_or_else(|| CoreError::Invalid("The preview did not start.".into()))?;
        drop(slot);
        self.wait(plan, &url, &tail)
    }

    fn wait(
        &self,
        plan: &PreviewPlan,
        url: &Mutex<Option<String>>,
        tail: &Mutex<Vec<String>>,
    ) -> CoreResult<String> {
        let started = Instant::now();
        while started.elapsed() < START_TIMEOUT {
            let address = plan.url.clone().or_else(|| lock(url).clone());
            if let Some(address) = address.filter(|a| answers(a)) {
                return Ok(address);
            }
            let exited = lock(&self.running)
                .as_mut()
                .is_none_or(|r| !matches!(r.child.try_wait(), Ok(None)));
            if exited {
                *lock(&self.running) = None;
                let last = lock(tail).join("\n");
                return Err(CoreError::Unavailable(format!(
                    "The site's preview stopped before it answered.\n{last}"
                )));
            }
            thread::sleep(POLL);
        }
        Err(CoreError::Unavailable(format!(
            "The site's preview did not answer within {} seconds.",
            START_TIMEOUT.as_secs()
        )))
    }

    /// Stops the server the app started, if any.
    pub fn stop(&self) {
        if let Some(mut running) = lock(&self.running).take() {
            let _ = running.child.start_kill();
            let _ = running.child.wait();
        }
    }
}

fn spawn(plan: &PreviewPlan) -> CoreResult<Running> {
    let mut command = CommandWrap::with_new(&plan.program, |cmd| {
        cmd.args(&plan.args)
            .current_dir(&plan.site)
            .env("PATH", &plan.path_env)
            .env("BROWSER", "none")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
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
        CoreError::Unavailable(format!(
            "Could not start {} for the site's preview. {e}",
            plan.program.display()
        ))
    })?;
    let url = Arc::new(Mutex::new(None));
    let tail = Arc::new(Mutex::new(Vec::new()));
    if let Some(out) = child.stdout().take() {
        read_output(out, Arc::clone(&url), Arc::clone(&tail));
    }
    if let Some(err) = child.stderr().take() {
        read_output(err, Arc::clone(&url), Arc::clone(&tail));
    }
    Ok(Running {
        site: plan.site.clone(),
        child,
        url,
        tail,
    })
}

#[cfg(test)]
mod tests;
