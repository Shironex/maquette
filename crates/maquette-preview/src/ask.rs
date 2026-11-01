//! What a person is asked before an agent-set repo's dev command first runs.

use std::fs;
use std::path::Path;

use maquette_base::fs::real_folder;
use maquette_base::hash::hash_file;
use serde::{Deserialize, Serialize};

use crate::command::{CommandSource, DevCommand, scripts};

/// The programs that run a package script.
const MANAGERS: [&str; 4] = ["npm", "pnpm", "yarn", "bun"];

/// The question, resolved the way the server resolves the command. Its
/// [`key`](Self::key) holds everything named here, so a change to any of it
/// asks again. A yes trusts the repo's contents: files a script loads are
/// not covered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAsk {
    /// The repo's real folder: links resolved, so a moved link asks again.
    pub repo: String,
    /// The command as one line, before the port is picked.
    pub command: String,
    pub source: CommandSource,
    /// When the command runs a package script, what that script runs.
    pub script_runs: Option<String>,
    /// The script's `pre` and `post` scripts, as `predev: its command`.
    pub hooks: Vec<String>,
    /// The `script-shell` of the repo's `.npmrc`, which runs every script.
    pub script_shell: Option<String>,
    /// Hashes of files that change what runs: `.npmrc` (never its text, it
    /// can hold tokens), and `package.json` when a package manager runs
    /// something no script here names.
    pub files_hash: Option<String>,
}

/// The script a package manager's command line runs: `npm run dev`,
/// `pnpm dev`, `pnpm run dev`, `yarn dev`, `bun run dev`. Flags are skipped.
fn script_word(argv: &[String]) -> Option<&str> {
    let mut words = argv.iter().skip(1).map(String::as_str);
    let mut next = || words.find(|w| !w.starts_with('-'));
    match next()? {
        "run" | "run-script" => next(),
        word => Some(word),
    }
}

fn runs_a_manager(argv: &[String]) -> bool {
    argv.first().is_some_and(|program| {
        let name = program.to_lowercase();
        let name = name.strip_suffix(".cmd").unwrap_or(&name);
        MANAGERS.contains(&name)
    })
}

fn script_shell(npmrc: &Path) -> Option<String> {
    let text = fs::read_to_string(npmrc).ok()?;
    text.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == "script-shell").then(|| value.trim().to_string())
    })
}

impl PreviewAsk {
    /// The question for `command` in `site`.
    pub fn of(site: &Path, command: &DevCommand) -> Self {
        let manager = runs_a_manager(&command.argv);
        let found = scripts(site)
            .ok()
            .flatten()
            .filter(|_| manager)
            .unwrap_or_default();
        let name = script_word(&command.argv).filter(|n| found.contains_key(*n));
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
        let npmrc = site.join(".npmrc");
        let mut hashes = Vec::new();
        if let Ok(hash) = hash_file(&npmrc) {
            hashes.push(format!("npmrc {hash}"));
        }
        if manager
            && script_runs.is_none()
            && let Ok(hash) = hash_file(&site.join("package.json"))
        {
            hashes.push(format!("package.json {hash}"));
        }
        Self {
            repo: real_folder(site).to_string_lossy().into_owned(),
            command: command.line(),
            source: command.source,
            script_runs,
            hooks,
            script_shell: script_shell(&npmrc),
            files_hash: (!hashes.is_empty()).then(|| hashes.join(" ")),
        }
    }

    /// What an approval is remembered by, with [`repo`](Self::repo).
    pub fn key(&self) -> String {
        let mut parts = vec![self.command.clone()];
        parts.extend(self.script_runs.clone());
        parts.extend(self.hooks.iter().cloned());
        parts.extend(self.files_hash.clone());
        parts.join("\n")
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

    const SCRIPTS: &str =
        r#"{ "scripts": { "dev": "vite", "predev": "node a.js", "postdev": "node b.js" } }"#;

    #[test]
    fn every_way_to_name_a_script_shows_its_body() {
        let dir = site(SCRIPTS);
        for line in [
            "npm run dev",
            "npm run-script dev",
            "pnpm dev",
            "pnpm run dev",
            "yarn dev",
            "yarn run dev",
            "bun run dev",
            "npm run dev -- --port {port}",
            "pnpm --silent run dev",
        ] {
            let ask = PreviewAsk::of(dir.path(), &settings(line));
            assert_eq!(ask.script_runs.as_deref(), Some("vite"), "{line}");
            assert_eq!(
                ask.hooks,
                ["predev: node a.js", "postdev: node b.js"],
                "{line}"
            );
        }
        let plain = PreviewAsk::of(dir.path(), &settings("node server.js"));
        assert_eq!(plain.script_runs, None);
        assert!(plain.hooks.is_empty());
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
        let from_config = |d: &Path| choose(d, &config, None).expect("command");
        let from_settings = |d: &Path| choose(d, &config, Some("npm run dev")).expect("command");
        let before = [
            PreviewAsk::of(dir.path(), &from_config(dir.path())),
            PreviewAsk::of(dir.path(), &from_settings(dir.path())),
        ];
        assert_eq!(before[0].source, CommandSource::SiteConfig);
        assert_eq!(before[1].source, CommandSource::Settings);
        fs::write(
            dir.path().join("package.json"),
            r#"{ "scripts": { "dev": "node evil.js" } }"#,
        )
        .expect("changed");
        let after = [
            PreviewAsk::of(dir.path(), &from_config(dir.path())),
            PreviewAsk::of(dir.path(), &from_settings(dir.path())),
        ];
        for (old, new) in before.iter().zip(&after) {
            assert_eq!(old.script_runs.as_deref(), Some("vite"));
            assert_eq!(new.script_runs.as_deref(), Some("node evil.js"));
            assert_ne!(old.key(), new.key());
        }
    }

    #[test]
    fn a_changed_pre_or_post_script_asks_again() {
        let dir = site(r#"{ "scripts": { "dev": "vite" } }"#);
        let key = || PreviewAsk::of(dir.path(), &settings("pnpm dev")).key();
        let first = key();
        fs::write(
            dir.path().join("package.json"),
            r#"{ "scripts": { "dev": "vite", "postdev": "node evil.js" } }"#,
        )
        .expect("post");
        assert_ne!(first, key());
    }

    #[test]
    fn a_changed_npmrc_asks_again_without_its_text_in_the_key() {
        let dir = site(r#"{ "scripts": { "dev": "vite" } }"#);
        let ask = || PreviewAsk::of(dir.path(), &settings("pnpm dev"));
        let first = ask().key();
        fs::write(
            dir.path().join(".npmrc"),
            "//registry/:_authToken=secret\nscript-shell = /tmp/evil.sh\n",
        )
        .expect("npmrc");
        let second = ask();
        assert_ne!(first, second.key());
        assert_eq!(second.script_shell.as_deref(), Some("/tmp/evil.sh"));
        assert!(!second.key().contains("secret"));
    }

    #[test]
    fn a_manager_running_something_unnamed_is_tied_to_the_whole_package_file() {
        let dir = site(r#"{ "scripts": { "dev": "vite" } }"#);
        let key = || PreviewAsk::of(dir.path(), &settings("pnpm --filter web dev")).key();
        let first = key();
        fs::write(dir.path().join("package.json"), r#"{ "scripts": {} }"#).expect("changed");
        assert_ne!(first, key());
    }
}
