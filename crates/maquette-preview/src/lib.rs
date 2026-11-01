//! The site's own dev server, started from the app so the person sees the
//! page with the project's models: the site's dev command ([`command`]),
//! run in the site folder in its own process group (job object on Windows)
//! and stopped when another site starts, its owner lets go or the app
//! quits ([`server`]), with a frame proxy in front so the Page view can
//! show it ([`frame`]). Nothing is built or deployed here. The Page view's
//! settings for one project ([`page`]) live here too, since they pick the
//! command the server runs.
//!
//! maquette-core re-exports this crate as `site::preview` and its `page`
//! module as `page`.

pub mod command;
pub mod frame;
pub mod page;
pub mod server;

use std::ffi::OsString;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use command::{CommandSource, DevCommand, PreviewAsk};
pub use frame::FrameProxy;
pub use server::{START_TIMEOUT, ServerOwner, ServerState, ServerStatus, SitePreview};

use maquette_base::error::{CoreError, CoreResult};
use maquette_base::toolchain::{ProbeEnv, find_on_path};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);

/// What runs and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewPlan {
    pub site: PathBuf,
    pub program: PathBuf,
    pub args: Vec<String>,
    /// From the site config; otherwise read from the output.
    pub url: Option<String>,
    /// The port Maquette picked for the server, when it picked one.
    pub port: Option<u16>,
    /// True when that port is in the command's args. A port passed only as
    /// `PORT` is a hint the dev tool may ignore (Vite and Astro do), so the
    /// address the output prints wins over it.
    pub port_in_args: bool,
    pub path_env: OsString,
    /// The command with its port, as one line for the log.
    pub line: String,
    /// The command before the port is picked: a start whose key differs
    /// from the running server's starts it afresh, so a changed dev command
    /// takes effect.
    pub key: String,
}

impl PreviewPlan {
    /// Where the server answers when its output never says: the site
    /// config's address, or the picked port on localhost.
    pub fn expected_url(&self) -> Option<String> {
        self.url
            .clone()
            .or_else(|| self.port.map(|p| format!("http://localhost:{p}/")))
    }
}

/// The package manager that runs the site's scripts: the nearest lockfile
/// from the site up (a site inside a monorepo has it at the root), npm
/// without one.
pub fn package_manager(site: &Path) -> &'static str {
    command::lockfile(site).0
}

/// A port nothing on this machine listens on right now.
///
/// # Errors
///
/// Fails when the system has no free port to give.
pub fn free_port() -> CoreResult<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| {
        CoreError::Unavailable(format!(
            "Could not find a free port for the dev server. {e}"
        ))
    })?;
    listener
        .local_addr()
        .map(|a| a.port())
        .map_err(|e| CoreError::Unavailable(format!("Could not read the free port. {e}")))
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

/// What `command` runs for `site`: the program found on the child PATH and
/// a free port when the command wants one.
///
/// # Errors
///
/// Fails when no port is free or the program is not on PATH.
pub fn plan(site: &Path, command: &DevCommand, env: &ProbeEnv) -> CoreResult<PreviewPlan> {
    let port = if command.wants_port() {
        Some(free_port()?)
    } else {
        None
    };
    let port_in_args = port.is_some() && command.argv.iter().any(|a| a.contains(command::PORT));
    let key = command.line();
    let mut argv = command::with_port(&command.argv, port);
    let line = command::line(&argv);
    let program = argv.remove(0);
    command::check_program(&program, "The dev command")?;
    let path_env = env.child_path(None);
    let mut probe = env.clone();
    probe.path_dirs = std::env::split_paths(&path_env).collect();
    let found = find_on_path(&[program.as_str()], &probe).ok_or_else(|| {
        CoreError::Unavailable(format!(
            "Could not find {program} on PATH to start the site's dev server."
        ))
    })?;
    Ok(PreviewPlan {
        site: site.to_path_buf(),
        program: found,
        args: argv,
        url: command.url.clone(),
        port,
        port_in_args,
        path_env,
        line,
        key,
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

#[cfg(test)]
mod tests;

/// Registers this crate's IPC types (the Page view's settings and the dev server's command and
/// status) with the shell's TypeScript export; `maquette_core::types` folds it in with the core's.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<page::PageSource>()
        .register::<page::PageSettings>()
        .register::<page::PageView>()
        .register::<CommandSource>()
        .register::<DevCommand>()
        .register::<PreviewAsk>()
        .register::<ServerState>()
        .register::<ServerOwner>()
        .register::<ServerStatus>()
}
