//! What the web view may read through the asset protocol: the workspace,
//! and the one dot folder under it that holds pictures it shows. On Unix the
//! scope's wildcards never match a dot file or folder, which keeps a
//! project's `.git` and `.env` out of it, so a dot folder must be named.

use std::path::Path;

use maquette_core::packs::{EXPORTS_DIR, RECORDS_DIR};
use tauri::{Manager, Runtime};

/// Opens `root` to the asset protocol.
pub(crate) fn open_workspace<R: Runtime>(app: &impl Manager<R>, root: &Path) -> tauri::Result<()> {
    let scope = app.asset_protocol_scope();
    scope.allow_directory(root, true)?;
    // Each pack's check renders live here, beside its record.
    scope.allow_directory(root.join(EXPORTS_DIR).join(RECORDS_DIR), true)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// The real scope of a real (mock runtime) app.
    #[test]
    fn the_scope_serves_pack_pictures_and_keeps_other_dot_files_out() {
        let dir = tempfile::tempdir().expect("dir");
        let root = dir.path();
        let file = |rel: &[&str]| {
            let path = rel.iter().fold(root.to_path_buf(), |p, part| p.join(part));
            fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            fs::write(&path, b"x").expect("file");
            path
        };
        let thumb = file(&["library", "crate", "thumb.png"]);
        let picture = file(&[EXPORTS_DIR, RECORDS_DIR, "pack-1", "crate.png"]);
        let record = file(&[EXPORTS_DIR, RECORDS_DIR, "pack-1.json"]);
        let sheet = file(&[EXPORTS_DIR, "pack", "contact-sheet.png"]);
        let secret = file(&["projects", "game", ".env"]);
        let git = file(&["projects", "game", ".git", "config"]);
        let outside = tempfile::tempdir().expect("outside");
        let other = outside.path().join("other.png");
        fs::write(&other, b"x").expect("other");

        let app = tauri::test::mock_app();
        let bare = app.asset_protocol_scope();
        bare.allow_directory(root, true).expect("allow");
        if cfg!(unix) {
            assert!(
                !bare.is_allowed(&picture),
                "the workspace alone refuses a dot folder, the bug this guards"
            );
        }

        open_workspace(&app, root).expect("open");
        let scope = app.asset_protocol_scope();
        for served in [&thumb, &picture, &record, &sheet] {
            assert!(scope.is_allowed(served), "{}", served.display());
        }
        assert!(!scope.is_allowed(&other), "nothing outside the workspace");
        if cfg!(unix) {
            assert!(!scope.is_allowed(&secret), "a project's .env stays out");
            assert!(!scope.is_allowed(&git), "a project's .git stays out");
        }
    }
}
