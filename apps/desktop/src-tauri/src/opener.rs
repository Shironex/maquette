//! Opens things outside the app: a provider's web studio in the system
//! browser, and a folder or file in Finder or Explorer. Only addresses the
//! app knows (the provider sites) and paths it resolved itself get here;
//! nothing typed in the web view is opened as is.
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
}
