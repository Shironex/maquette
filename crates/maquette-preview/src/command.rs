//! Which command starts a site's dev server, in this order: the project's
//! own choice (Settings > Project, kept in `page.json`), the `preview` of
//! the site's `maquette.site.json`, or what the site's `package.json` and
//! nearest lockfile say: the package manager runs the first of the `dev`,
//! `start` and `serve` scripts.
//!
//! A command is a program on PATH and its args, never a shell line, so a
//! value the person typed is passed to the program as it is. `{port}` in an
//! arg is the port Maquette picked; a command read from `package.json` gets
//! `--port {port}` unless the site config names its address.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use maquette_base::error::{CoreError, CoreResult};
use maquette_base::site_config::{CONFIG_FILE, SiteConfig};

/// The scripts a dev server is looked for in, in order.
pub const DEV_SCRIPTS: [&str; 3] = ["dev", "start", "serve"];
/// Stands for the port Maquette picked, in a command's args.
pub const PORT: &str = "{port}";
/// The longest command Settings accepts, in characters.
pub const COMMAND_MAX_CHARS: usize = 400;

const LOCKS: [(&str, &str); 5] = [
    ("pnpm-lock.yaml", "pnpm"),
    ("bun.lock", "bun"),
    ("bun.lockb", "bun"),
    ("yarn.lock", "yarn"),
    ("package-lock.json", "npm"),
];

/// Where a dev command came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum CommandSource {
    /// The project's Settings.
    Settings,
    /// The site's `maquette.site.json`.
    SiteConfig,
    /// The site's `package.json` scripts and lockfile.
    PackageJson,
}

/// The command that starts a site's dev server, before the port is picked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DevCommand {
    /// The program, then its args; `{port}` stays in place.
    pub argv: Vec<String>,
    pub source: CommandSource,
    /// The address the site config names, when it names one.
    pub url: Option<String>,
    /// For a command read from `package.json`: the manager and script.
    pub manager: Option<String>,
    pub script: Option<String>,
    /// The lockfile the manager came from; none means npm by default.
    pub lockfile: Option<String>,
}

impl DevCommand {
    /// The command as one line, for the log and the UI.
    pub fn line(&self) -> String {
        line(&self.argv)
    }

    /// True when a port is picked for it: `{port}` is in an arg, or nothing
    /// says where the server answers.
    pub fn wants_port(&self) -> bool {
        self.url.is_none() || self.argv.iter().any(|a| a.contains(PORT))
    }
}

/// What a person is asked before an agent-set repo's dev command first
/// runs, resolved the way the server resolves it. Its [`key`](Self::key)
/// holds both the folder's command and what a package script runs, so a
/// changed path, command or script asks again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAsk {
    /// The repo's real folder: links resolved, so a moved link asks again.
    pub repo: String,
    /// The command as one line, before the port is picked.
    pub command: String,
    pub source: CommandSource,
    /// For a package script, what the script itself runs.
    pub script_runs: Option<String>,
}

impl PreviewAsk {
    /// The question for `command` in `site`.
    pub fn of(site: &Path, command: &DevCommand) -> Self {
        let repo = fs::canonicalize(site).unwrap_or_else(|_| site.to_path_buf());
        let script_runs = command.script.as_ref().and_then(|name| {
            let found = scripts(site).ok().flatten()?;
            found.get(name).cloned()
        });
        Self {
            repo: repo.to_string_lossy().into_owned(),
            command: command.line(),
            source: command.source,
            script_runs,
        }
    }

    /// What an approval is remembered by, with [`repo`](Self::repo).
    pub fn key(&self) -> String {
        match &self.script_runs {
            Some(runs) => format!("{}\n{runs}", self.command),
            None => self.command.clone(),
        }
    }
}

/// `argv` as one line, quoting args with spaces.
pub fn line(argv: &[String]) -> String {
    argv.iter()
        .map(|a| {
            if a.is_empty() || a.contains(char::is_whitespace) {
                format!("\"{a}\"")
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The manager of the nearest lockfile from the site up (a site inside a
/// monorepo has it at the root) and that lockfile's name; npm without one.
pub fn lockfile(site: &Path) -> (&'static str, Option<&'static str>) {
    site.ancestors()
        .find_map(|dir| {
            LOCKS
                .iter()
                .find(|(file, _)| dir.join(file).is_file())
                .map(|(file, manager)| (*manager, Some(*file)))
        })
        .unwrap_or(("npm", None))
}

/// Splits a command the person typed into a program and its args: spaces
/// separate, double or single quotes keep spaces in an arg. Nothing is
/// expanded; there is no shell.
///
/// # Errors
///
/// Fails on an empty or overlong command, an unclosed quote, or a program
/// given as a path.
pub fn parse_line(text: &str) -> CoreResult<Vec<String>> {
    if text.chars().count() > COMMAND_MAX_CHARS {
        return Err(CoreError::Invalid(format!(
            "Keep the dev command to {COMMAND_MAX_CHARS} characters or fewer."
        )));
    }
    let mut argv = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    for c in text.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                started = true;
            }
            None if c.is_whitespace() => {
                if started {
                    argv.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            None => {
                current.push(c);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return Err(CoreError::Invalid(
            "The dev command has a quote that is never closed.".into(),
        ));
    }
    if started {
        argv.push(current);
    }
    let Some(program) = argv.first() else {
        return Err(CoreError::Invalid("The dev command is empty.".into()));
    };
    check_program(program, "the dev command")?;
    Ok(argv)
}

/// A program must be a name found on PATH, like pnpm, never a path.
pub(super) fn check_program(program: &str, what: &str) -> CoreResult<()> {
    if program.contains(['/', '\\']) || program.trim().is_empty() {
        return Err(CoreError::Invalid(format!(
            "{what} must name a program on PATH, like pnpm, not {program}."
        )));
    }
    Ok(())
}

/// The site's `package.json` scripts, name to command line.
///
/// # Errors
///
/// Fails when the file exists but is not readable JSON.
pub fn scripts(site: &Path) -> CoreResult<Option<BTreeMap<String, String>>> {
    let file = site.join("package.json");
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    let found = value
        .get("scripts")
        .and_then(serde_json::Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    Ok(Some(found))
}

/// The dev command for `site`: `custom` (from Settings) first, then the
/// site config's, then the package manager and the first dev script.
///
/// # Errors
///
/// Fails when the custom or configured command is not valid, the site
/// config names an address that is not on this machine, or the site has no
/// `package.json` or no dev script to run.
pub fn choose(site: &Path, config: &SiteConfig, custom: Option<&str>) -> CoreResult<DevCommand> {
    let preview = config.preview.clone().unwrap_or_default();
    if let Some(url) = preview.url.as_deref()
        && !super::is_local_url(url)
    {
        return Err(CoreError::Invalid(format!(
            "{CONFIG_FILE}: the preview url must be on this machine (localhost), not {url}."
        )));
    }
    if let Some(text) = custom.map(str::trim).filter(|t| !t.is_empty()) {
        return Ok(DevCommand {
            argv: parse_line(text)?,
            source: CommandSource::Settings,
            url: None,
            manager: None,
            script: None,
            lockfile: None,
        });
    }
    if let Some(program) = preview.command.first() {
        check_program(program, &format!("{CONFIG_FILE}: the preview command"))?;
        return Ok(DevCommand {
            argv: preview.command.clone(),
            source: CommandSource::SiteConfig,
            url: preview.url,
            manager: None,
            script: None,
            lockfile: None,
        });
    }
    let found = scripts(site)?.ok_or_else(|| {
        CoreError::Invalid(
            "The site repo has no package.json, so there is no dev script to run. Set the dev command in Settings > Project."
                .into(),
        )
    })?;
    let script = DEV_SCRIPTS
        .into_iter()
        .find(|s| found.contains_key(*s))
        .ok_or_else(|| {
            CoreError::Invalid(format!(
                "The site's package.json has no {} script. Set the dev command in Settings > Project.",
                DEV_SCRIPTS.join(", ")
            ))
        })?;
    let (manager, lock) = lockfile(site);
    let mut argv = vec![manager.to_string(), "run".into(), script.to_string()];
    if preview.url.is_none() {
        // npm keeps args after `--` for the script; the others pass them on.
        if manager == "npm" {
            argv.push("--".into());
        }
        argv.extend(["--port".to_string(), PORT.to_string()]);
    }
    Ok(DevCommand {
        argv,
        source: CommandSource::PackageJson,
        url: preview.url,
        manager: Some(manager.to_string()),
        script: Some(script.to_string()),
        lockfile: lock.map(str::to_string),
    })
}

/// `argv` with `{port}` replaced by `port`.
pub fn with_port(argv: &[String], port: Option<u16>) -> Vec<String> {
    argv.iter()
        .map(|a| match port {
            Some(p) => a.replace(PORT, &p.to_string()),
            None => a.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
