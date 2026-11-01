//! What a person is asked before an agent-set repo's dev command first runs.
//!
//! The approval never depends on reading the command: its key holds the
//! command line and a hash of every repo file listed here, whatever the
//! program. Reading the command only feeds the card, and only for forms whose
//! meaning is exact; anything else says so instead of guessing.

use std::fs;
use std::path::Path;

use maquette_base::fs::real_folder;
use maquette_base::hash::hash_file;
use serde::{Deserialize, Serialize};

use crate::command::{CommandSource, DevCommand, scripts};

/// The repo files that change what a dev command runs. Hashed, never kept as
/// text: `.npmrc` can hold tokens.
const CONFIG_FILES: [&str; 6] = [
    "package.json",
    ".npmrc",
    "pnpm-workspace.yaml",
    ".yarnrc.yml",
    "bunfig.toml",
    "maquette.site.json",
];

/// The listed files that can set the shell or the manager in a way the card
/// does not read: `scriptShell`, `yarnPath` and bun's own settings.
const MANAGER_FILES: [&str; 3] = ["pnpm-workspace.yaml", ".yarnrc.yml", "bunfig.toml"];

/// Words `pnpm X` or `yarn X` take as the manager's own command, not a script,
/// even when `package.json` has a script of that name.
const MANAGER_WORDS: [&str; 45] = [
    "add",
    "audit",
    "bin",
    "cache",
    "config",
    "create",
    "dedupe",
    "deploy",
    "dlx",
    "env",
    "exec",
    "explain",
    "fetch",
    "global",
    "help",
    "i",
    "import",
    "info",
    "init",
    "install",
    "licenses",
    "link",
    "list",
    "login",
    "logout",
    "ls",
    "node",
    "outdated",
    "pack",
    "patch",
    "plugin",
    "prune",
    "publish",
    "rebuild",
    "remove",
    "rm",
    "root",
    "set",
    "store",
    "unlink",
    "up",
    "update",
    "version",
    "why",
    "workspace",
];

/// The question, resolved the way the server resolves the command. A change
/// to the folder, the command or a listed file asks again. A yes trusts the
/// repo, including nested packages and the files scripts load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAsk {
    /// The repo's real folder: links resolved, so a moved link asks again.
    pub repo: String,
    /// The command as one line, before the port is picked.
    pub command: String,
    pub source: CommandSource,
    /// What the command's package script runs, only when that is certain.
    /// None means Maquette cannot tell what the command runs.
    pub script_runs: Option<String>,
    /// That script's `pre` and `post` scripts, as `predev: its command`.
    pub hooks: Vec<String>,
    /// The last `script-shell` of the repo's `.npmrc`.
    pub script_shell: Option<String>,
    /// The other files here that can change the shell or the manager.
    pub manager_config: Vec<String>,
    /// A hash of each listed file that exists.
    pub files_hash: String,
}

/// The program, with Windows' `.cmd` taken off; `.exe` and paths stay as they
/// are, so they match no manager.
fn manager(program: &str) -> &str {
    program.strip_suffix(".cmd").unwrap_or(program)
}

/// The script of an exact form: `npm run X`, `npm run-script X`, `pnpm X`,
/// `pnpm run X`, `yarn X`, `yarn run X`, `bun run X`. No flags and no other
/// words: every other command line answers None.
fn script_word(argv: &[String]) -> Option<&str> {
    let words: Vec<&str> = argv.iter().map(String::as_str).collect();
    let word = match words.as_slice() {
        [program, "run", word] if matches!(manager(program), "npm" | "pnpm" | "yarn" | "bun") => {
            *word
        }
        [program, "run-script", word] if manager(program) == "npm" => *word,
        [program, word]
            if matches!(manager(program), "pnpm" | "yarn") && !MANAGER_WORDS.contains(word) =>
        {
            *word
        }
        _ => return None,
    };
    (!word.starts_with('-')).then_some(word)
}

/// The script behind `command`. Maquette picked it itself on the package.json
/// route, adding only the port; any other route has to be an exact form.
fn script_name(command: &DevCommand) -> Option<&str> {
    match command.source {
        CommandSource::PackageJson => command.script.as_deref(),
        _ => script_word(&command.argv),
    }
}

fn script_shell(npmrc: &Path) -> Option<String> {
    let text = fs::read_to_string(npmrc).ok()?;
    text.lines().rev().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        let name = name.trim().replace('_', "-");
        (name == "script-shell").then(|| value.trim().to_string())
    })
}

/// One line per listed file that exists. A file that cannot be read still
/// counts, so it appearing or going away asks again.
fn files_hash(site: &Path) -> String {
    let lines: Vec<String> = CONFIG_FILES
        .iter()
        .filter(|name| fs::symlink_metadata(site.join(name)).is_ok())
        .map(|name| {
            let hash = hash_file(&site.join(name)).unwrap_or_else(|_| "unreadable".to_string());
            format!("{name} {hash}")
        })
        .collect();
    lines.join("\n")
}

impl PreviewAsk {
    /// The question for `command` in `site`.
    pub fn of(site: &Path, command: &DevCommand) -> Self {
        let found = scripts(site).ok().flatten().unwrap_or_default();
        let name = script_name(command).filter(|n| found.contains_key(*n));
        let script_runs = name.and_then(|n| found.get(n).cloned());
        let hooks = name.map_or_else(Vec::new, |n| {
            [format!("pre{n}"), format!("post{n}")]
                .into_iter()
                .filter_map(|hook| {
                    let body = found.get(&hook)?;
                    Some(format!("{hook}: {body}"))
                })
                .collect()
        });
        Self {
            repo: real_folder(site).to_string_lossy().into_owned(),
            command: command.line(),
            source: command.source,
            script_runs,
            hooks,
            script_shell: script_shell(&site.join(".npmrc")),
            manager_config: MANAGER_FILES
                .iter()
                .filter(|name| fs::symlink_metadata(site.join(name)).is_ok())
                .map(ToString::to_string)
                .collect(),
            files_hash: files_hash(site),
        }
    }

    /// What an approval is remembered by, with [`repo`](Self::repo). Nothing
    /// read from the command goes in.
    pub fn key(&self) -> String {
        format!("{}\n{}", self.command, self.files_hash)
    }
}

#[cfg(test)]
mod tests {
    use maquette_base::site_config::{PreviewConfig, SiteConfig};

    use super::*;
    use crate::command::{choose, parse_line};

    fn settings(line: &str) -> DevCommand {
        DevCommand {
            argv: parse_line(line).expect("line"),
            source: CommandSource::Settings,
            url: None,
            manager: None,
            script: None,
            lockfile: None,
        }
    }

    fn site(package: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("dir");
        fs::write(dir.path().join("package.json"), package).expect("package");
        dir
    }

    const SCRIPTS: &str = r#"{ "scripts": { "dev": "vite", "predev": "node a.js", "postdev": "node b.js", "install": "harmless", "exec": "harmless" } }"#;

    #[test]
    fn only_an_exact_form_shows_a_script_body() {
        let dir = site(SCRIPTS);
        for line in [
            "npm run dev",
            "npm run-script dev",
            "pnpm dev",
            "pnpm run dev",
            "yarn dev",
            "yarn run dev",
            "bun run dev",
            "npm.cmd run dev",
            "pnpm.cmd dev",
            "yarn.cmd dev",
            "bun.cmd run dev",
        ] {
            let ask = PreviewAsk::of(dir.path(), &settings(line));
            assert_eq!(ask.script_runs.as_deref(), Some("vite"), "{line}");
            assert_eq!(
                ask.hooks,
                ["predev: node a.js", "postdev: node b.js"],
                "{line}"
            );
        }
    }

    #[test]
    fn any_other_form_shows_no_script_body() {
        let dir = site(SCRIPTS);
        for line in [
            "pnpm --filter web dev",
            "pnpm -C other dev",
            "pnpm --silent run dev",
            "pnpm run --silent dev",
            "npm run dev --prefix other",
            "npm --prefix other run dev",
            "npm run dev -- --port {port}",
            "yarn workspace web dev",
            "corepack pnpm dev",
            "npx vite",
            "env A=1 pnpm dev",
            "node --run dev",
            "node server.js",
            "pnpm.exe dev",
            "npm.exe run dev",
            "npm exec dev",
            "npm dev",
            "bun dev",
            "pnpm exec",
            "pnpm install",
            "yarn exec",
            "pnpm run nope",
        ] {
            let ask = PreviewAsk::of(dir.path(), &settings(line));
            assert_eq!(ask.script_runs, None, "{line}");
            assert!(ask.hooks.is_empty(), "{line}");
        }
    }

    #[test]
    fn the_package_json_route_names_the_script_maquette_picked() {
        let dir = site(SCRIPTS);
        let config = SiteConfig::legacy();
        let command = choose(dir.path(), &config, None).expect("command");
        assert_eq!(command.source, CommandSource::PackageJson);
        let ask = PreviewAsk::of(dir.path(), &command);
        assert_eq!(ask.script_runs.as_deref(), Some("vite"));
    }

    #[test]
    fn any_listed_file_changing_asks_again_whatever_the_program() {
        for line in [
            "node server.js",
            "corepack pnpm dev",
            "pnpm --filter web dev",
            "pnpm dev",
        ] {
            let dir = site(SCRIPTS);
            let key = || PreviewAsk::of(dir.path(), &settings(line)).key();
            for name in CONFIG_FILES {
                let before = key();
                fs::write(dir.path().join(name), "one").expect("write");
                let written = key();
                assert_ne!(before, written, "{name} appearing, {line}");
                fs::write(dir.path().join(name), "two").expect("write");
                assert_ne!(written, key(), "{name} changing, {line}");
            }
        }
    }

    #[test]
    fn a_changed_script_asks_again_from_settings_and_from_the_site_config() {
        let dir = site(r#"{ "scripts": { "dev": "vite" } }"#);
        let config = SiteConfig {
            preview: Some(PreviewConfig {
                command: vec!["pnpm".into(), "dev".into()],
                url: None,
            }),
            ..SiteConfig::legacy()
        };
        let asks = |d: &Path| {
            [
                PreviewAsk::of(d, &choose(d, &config, None).expect("command")),
                PreviewAsk::of(
                    d,
                    &choose(d, &config, Some("npm run dev")).expect("command"),
                ),
            ]
        };
        let before = asks(dir.path());
        assert_eq!(before[0].source, CommandSource::SiteConfig);
        assert_eq!(before[1].source, CommandSource::Settings);
        fs::write(
            dir.path().join("package.json"),
            r#"{ "scripts": { "dev": "node evil.js" } }"#,
        )
        .expect("changed");
        for (old, new) in before.iter().zip(&asks(dir.path())) {
            assert_eq!(old.script_runs.as_deref(), Some("vite"));
            assert_eq!(new.script_runs.as_deref(), Some("node evil.js"));
            assert_ne!(old.key(), new.key());
        }
    }

    #[test]
    fn the_last_script_shell_wins_and_no_npmrc_text_reaches_the_key() {
        let dir = site(SCRIPTS);
        fs::write(
            dir.path().join(".npmrc"),
            "script-shell=/bin/first\n//registry/:_authToken=secret\n; script-shell=/bin/off\nscript-shell = /tmp/last.sh\n",
        )
        .expect("npmrc");
        let ask = PreviewAsk::of(dir.path(), &settings("pnpm dev"));
        assert_eq!(ask.script_shell.as_deref(), Some("/tmp/last.sh"));
        assert!(!ask.key().contains("secret"));
        assert!(!ask.files_hash.contains("secret"));
    }

    #[test]
    fn a_file_that_can_set_the_shell_or_manager_is_named() {
        let dir = site(SCRIPTS);
        let names = || PreviewAsk::of(dir.path(), &settings("pnpm dev")).manager_config;
        assert!(names().is_empty());
        fs::write(dir.path().join(".yarnrc.yml"), "yarnPath: evil.cjs\n").expect("yarn");
        fs::write(dir.path().join("pnpm-workspace.yaml"), "scriptShell: x\n").expect("pnpm");
        fs::write(dir.path().join("bunfig.toml"), "[run]\n").expect("bun");
        assert_eq!(
            names(),
            ["pnpm-workspace.yaml", ".yarnrc.yml", "bunfig.toml"]
        );
    }
}
