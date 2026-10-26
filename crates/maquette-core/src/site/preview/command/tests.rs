use super::super::super::PreviewConfig;
use super::*;

fn site_with(package: Option<&str>, lock: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("site");
    if let Some(text) = package {
        std::fs::write(dir.path().join("package.json"), text).expect("package");
    }
    if let Some(name) = lock {
        std::fs::write(dir.path().join(name), "").expect("lock");
    }
    dir
}

#[test]
fn a_typed_command_splits_on_spaces_and_keeps_quotes() {
    assert_eq!(
        parse_line("pnpm  run dev --port {port}").expect("parse"),
        vec!["pnpm", "run", "dev", "--port", "{port}"]
    );
    assert_eq!(
        parse_line(r#"npm run "dev site" -- --host '127.0.0.1'"#).expect("quotes"),
        vec!["npm", "run", "dev site", "--", "--host", "127.0.0.1"]
    );
    // Nothing is expanded: there is no shell.
    assert_eq!(
        parse_line("pnpm dev $HOME `id` ;rm").expect("literal"),
        vec!["pnpm", "dev", "$HOME", "`id`", ";rm"]
    );
    assert_eq!(
        parse_line(r#"pnpm """#).expect("empty arg"),
        vec!["pnpm", ""]
    );
    for bad in [
        "",
        "   ",
        "pnpm \"dev",
        "./node_modules/.bin/vite",
        "C:\\tools\\pnpm dev",
    ] {
        assert_eq!(parse_line(bad).expect_err(bad).code(), "invalid", "{bad}");
    }
    let long = format!("pnpm {}", "x".repeat(COMMAND_MAX_CHARS));
    assert_eq!(parse_line(&long).expect_err("long").code(), "invalid");
}

#[test]
fn the_package_json_dev_script_runs_with_the_lockfile_manager_and_a_port() {
    let site = site_with(
        Some(r#"{ "scripts": { "build": "astro build", "dev": "astro dev" } }"#),
        Some("pnpm-lock.yaml"),
    );
    let found = choose(site.path(), &SiteConfig::legacy(), None).expect("choose");
    assert_eq!(found.source, CommandSource::PackageJson);
    assert_eq!(found.argv, vec!["pnpm", "run", "dev", "--port", PORT]);
    assert_eq!(found.manager.as_deref(), Some("pnpm"));
    assert_eq!(found.script.as_deref(), Some("dev"));
    assert_eq!(found.lockfile.as_deref(), Some("pnpm-lock.yaml"));
    assert!(found.wants_port());
    assert_eq!(found.line(), "pnpm run dev --port {port}");
}

#[test]
fn npm_passes_the_port_after_a_double_dash_and_start_is_the_fallback() {
    let site = site_with(Some(r#"{ "scripts": { "start": "next start" } }"#), None);
    let found = choose(site.path(), &SiteConfig::legacy(), None).expect("choose");
    assert_eq!(
        found.argv,
        vec!["npm", "run", "start", "--", "--port", PORT]
    );
    assert_eq!(found.lockfile, None);
    let yarn = site_with(
        Some(r#"{ "scripts": { "serve": "eleventy --serve", "start": "x" } }"#),
        Some("yarn.lock"),
    );
    let found = choose(yarn.path(), &SiteConfig::legacy(), None).expect("yarn");
    assert_eq!(found.argv[..3], ["yarn", "run", "start"]);
}

#[test]
fn a_site_config_address_needs_no_port() {
    let site = site_with(
        Some(r#"{ "scripts": { "dev": "astro dev" } }"#),
        Some("bun.lock"),
    );
    let config = SiteConfig {
        preview: Some(PreviewConfig {
            command: Vec::new(),
            url: Some("http://localhost:15292/".into()),
        }),
        ..SiteConfig::legacy()
    };
    let found = choose(site.path(), &config, None).expect("choose");
    assert_eq!(found.argv, vec!["bun", "run", "dev"]);
    assert_eq!(found.url.as_deref(), Some("http://localhost:15292/"));
    assert!(!found.wants_port());

    let config = SiteConfig {
        preview: Some(PreviewConfig {
            command: vec!["pnpm".into(), "dev".into()],
            url: Some("http://localhost:15292/".into()),
        }),
        ..SiteConfig::legacy()
    };
    let found = choose(site.path(), &config, None).expect("config");
    assert_eq!(found.source, CommandSource::SiteConfig);
    assert_eq!(found.argv, vec!["pnpm", "dev"]);
}

#[test]
fn the_project_setting_wins_over_the_site() {
    let site = site_with(Some(r#"{ "scripts": { "dev": "astro dev" } }"#), None);
    let config = SiteConfig {
        preview: Some(PreviewConfig {
            command: vec!["pnpm".into(), "dev".into()],
            url: None,
        }),
        ..SiteConfig::legacy()
    };
    let found = choose(
        site.path(),
        &config,
        Some("  pnpm astro dev --port {port} "),
    )
    .expect("ok");
    assert_eq!(found.source, CommandSource::Settings);
    assert_eq!(found.argv, vec!["pnpm", "astro", "dev", "--port", PORT]);
    assert!(found.wants_port());
    assert_eq!(
        with_port(&found.argv, Some(5173)),
        vec!["pnpm", "astro", "dev", "--port", "5173"]
    );
    // A blank setting falls back to the site.
    let fallback = choose(site.path(), &config, Some("  ")).expect("blank");
    assert_eq!(fallback.source, CommandSource::SiteConfig);
}

#[test]
fn a_site_without_a_dev_script_says_how_to_set_one() {
    let none = site_with(None, None);
    let err = choose(none.path(), &SiteConfig::legacy(), None).expect_err("no package");
    assert!(err.to_string().contains("no package.json"), "{err}");
    let empty = site_with(Some(r#"{ "scripts": { "build": "astro build" } }"#), None);
    let err = choose(empty.path(), &SiteConfig::legacy(), None).expect_err("no script");
    assert!(err.to_string().contains("dev, start, serve"), "{err}");
    assert!(err.to_string().contains("Settings > Project"), "{err}");
    let broken = site_with(Some("{ not json"), None);
    assert_eq!(
        choose(broken.path(), &SiteConfig::legacy(), None)
            .expect_err("json")
            .code(),
        "json"
    );
}

#[test]
fn a_line_quotes_args_with_spaces() {
    assert_eq!(
        line(&["npm".into(), "run".into(), "dev site".into()]),
        "npm run \"dev site\""
    );
}
