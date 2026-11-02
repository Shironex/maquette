//! Opens things outside the app: a provider's web studio or the site's
//! local preview in the system browser, and a folder or file in Finder or
//! Explorer. Only addresses the app knows (the provider sites, the preview
//! server's local address) and paths it resolved itself get here; nothing
//! typed in the web view is opened as is.
//!
//! The program is spawned with an args array, never through a shell.

use std::path::Path;
use std::process::Command;

/// Opens an `https://` address in the default browser.
pub fn open_url(url: &str) -> std::io::Result<()> {
    if !url.starts_with("https://") || url.chars().any(char::is_whitespace) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "only https addresses open in the browser",
        ));
    }
    browser(url)
}

/// The provider web studios a By hand send may open: a plain link to the
/// site, nothing typed in the web view, and nothing more than opening it.
pub const PROVIDER_SITES: [&str; 2] = ["https://studio.tripo3d.ai", "https://www.meshy.ai"];

/// True for one of [`PROVIDER_SITES`] or a page under it.
pub fn is_provider_site(url: &str) -> bool {
    PROVIDER_SITES.iter().any(|site| {
        url.strip_prefix(site)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

/// Opens a provider's web studio in the default browser, when the address
/// is on the allowlist.
pub fn open_provider_site(url: &str) -> std::io::Result<()> {
    if !is_provider_site(url) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "only the Tripo and Meshy studios open from a handoff",
        ));
    }
    open_url(url)
}

/// The project's own pages About may open: the repo (releases, issue forms,
/// source) and the docs site.
pub const PROJECT_LINKS: [&str; 2] = [
    "https://github.com/Shironex/maquette",
    "https://shironex.github.io/maquette",
];

/// Where the first run sends someone to get a missing tool.
pub const TOOL_SITES: [&str; 1] = ["https://www.blender.org"];

/// True for one of [`PROJECT_LINKS`] or [`TOOL_SITES`], or a page under it.
pub fn is_project_link(url: &str) -> bool {
    PROJECT_LINKS.iter().chain(TOOL_SITES.iter()).any(|base| {
        url.strip_prefix(base)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

/// Opens one of the project's pages in the default browser, when the
/// address is on the allowlist.
pub fn open_project_link(url: &str) -> std::io::Result<()> {
    if !is_project_link(url) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "only the Maquette repo, its docs and blender.org open from the app",
        ));
    }
    open_url(url)
}

/// Opens an address on this machine (`http://localhost:15292/`) in the
/// default browser: the site's preview server.
pub fn open_local_url(url: &str) -> std::io::Result<()> {
    if !maquette_core::site::preview::is_local_url(url) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "only addresses on this machine open as a preview",
        ));
    }
    browser(url)
}

/// Hands an address the callers checked to the default browser.
fn browser(url: &str) -> std::io::Result<()> {
    let mut command = if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(windows) {
        // explorer.exe hands an address to the default browser.
        Command::new("explorer")
    } else {
        Command::new("xdg-open")
    };
    command.arg(url).spawn().map(|_| ())
}

/// Opens a folder in the file manager.
pub fn open_folder(folder: &Path) -> std::io::Result<()> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    Command::new(program).arg(folder).spawn().map(|_| ())
}

/// Shows a file selected in its folder, so it can be dragged from there
/// (into a browser upload, for example).
pub fn reveal_file(file: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // explorer wants `/select,"<path>"` as one raw argument; a Windows
        // path cannot hold a quote, so this cannot break out of it.
        Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", file.display()))
            .spawn()
            .map(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg("-R").arg(file).spawn().map(|_| ())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        open_folder(file.parent().unwrap_or(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_addresses_open() {
        for bad in [
            "file:///etc/passwd",
            "http://example.com",
            "https://a b",
            "javascript:x",
        ] {
            assert!(open_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn only_the_provider_studios_open_from_a_handoff() {
        assert!(is_provider_site("https://studio.tripo3d.ai"));
        assert!(is_provider_site("https://www.meshy.ai/workspace"));
        for service in maquette_core::providers::ProviderId::ALL {
            let site = maquette_core::providers::handoff::site(service);
            assert!(is_provider_site(site), "{site}");
        }
        for bad in [
            "https://studio.tripo3d.ai.example.com",
            "https://www.meshy.ai@evil.example",
            "http://www.meshy.ai",
            "https://example.com",
            "https://studio.tripo3d.aiX/",
        ] {
            assert!(!is_provider_site(bad), "{bad}");
            assert!(open_provider_site(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn only_the_project_pages_open_from_about() {
        assert!(is_project_link("https://github.com/Shironex/maquette"));
        assert!(is_project_link(
            "https://github.com/Shironex/maquette/issues/new?template=bug_report.yml&version=0.1.0"
        ));
        assert!(is_project_link("https://shironex.github.io/maquette/"));
        assert!(is_project_link("https://www.blender.org/download/"));
        for bad in [
            "https://github.com/Shironex/maquette-fork",
            "https://github.com/Shironex/other",
            "https://github.com/Shironex/maquette@evil.example",
            "http://github.com/Shironex/maquette",
            "https://shironex.github.io/maquettes",
            "https://www.blender.org.example.com/",
            "https://www.blender.org@evil.example",
            "https://www.blender.org:443@evil.example/",
            "https://www.blender.org:8443/",
            "https://www.blender.org\\@evil.example/",
            "https://github.com/Shironex/maquette:x@evil.example/",
            "https://shironex.github.io/maquette\\@evil.example/",
            "https://example.com/",
        ] {
            assert!(!is_project_link(bad), "{bad}");
            assert!(open_project_link(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn only_addresses_on_this_machine_open_as_a_preview() {
        for bad in [
            "https://example.com/",
            "http://localhost.example.com/",
            "file:///etc/passwd",
            "http://localhost:1/ x",
        ] {
            assert!(open_local_url(bad).is_err(), "{bad}");
        }
    }
}
