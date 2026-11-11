//! How a thumbnail is lit. Every model gets the studio but a built asset,
//! whose flat palette colours are judged by eye: it gets the page light of
//! [`RIG_SCRIPT`], the one preview rig the standard views share
//! ([`crate::views`]), so a colour reads on its card as it will on the page.
//!
//! A page light render is cached under a name that carries a key of the
//! rig's text. A changed rig renders the built assets' thumbnails again the
//! next time they are asked for and leaves every studio render where it is.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{CoreError, CoreResult};
use crate::pipeline::cache::FileHasher;

/// The page light: the sun and the even fill the Bake light step bakes in,
/// shown through the page's tone mapping. Its own docs say where each number
/// comes from.
pub const RIG_SCRIPT: &str = include_str!("preview_rig.py");
pub const RIG_FILE: &str = "preview_rig.py";
/// Renders a palette as a swatch sheet in the page light: the rig's
/// reference picture, which the `swatch_sheet` example makes again.
pub const SWATCH_SCRIPT: &str = include_str!("swatch_sheet.py");
pub const SWATCH_FILE: &str = "swatch_sheet.py";

/// How a thumbnail is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThumbLook {
    /// A warm key, a warm fill and a cool rim: every model but a built one.
    #[default]
    Studio,
    /// The page light of [`RIG_SCRIPT`]: a built asset.
    Page,
}

impl ThumbLook {
    /// What `render_thumb.py` is told besides the model and the picture.
    pub fn args(self) -> &'static [&'static str] {
        match self {
            ThumbLook::Studio => &[],
            ThumbLook::Page => &["--look", "page"],
        }
    }
}

/// A short key of a rig's text, the same on every platform's line ends.
pub fn rig_key(rig: &str) -> String {
    let digest = Sha256::digest(rig.replace("\r\n", "\n").as_bytes());
    hex::encode(&digest[..4])
}

/// The picture's name for a model of content `hash`: the studio's is the
/// hash alone, as it always was, and the page light's adds the rig's key.
pub fn png_name(hash: &str, look: ThumbLook, rig: &str) -> String {
    let short = &hash[..hash.len().min(32)];
    match look {
        ThumbLook::Studio => format!("{short}.png"),
        ThumbLook::Page => format!("{short}-page-{}.png", rig_key(rig)),
    }
}

/// `<dir>/<name>` for the content of `file` in `look` (see [`png_name`]).
/// Hashes are memoised by size and modified time in `<dir>/hashes.json`.
///
/// # Errors
///
/// Fails when the model cannot be read.
pub fn png_for_look(dir: &Path, file: &Path, look: ThumbLook) -> CoreResult<PathBuf> {
    let mut hasher = FileHasher::with_memo(dir);
    let hash = hasher.hash(file).map_err(|e| CoreError::io(file, e))?;
    hasher.flush();
    Ok(dir.join(png_name(&hash, look, RIG_SCRIPT)))
}

/// The look an asset's thumbnail gets: the page light when a script of the
/// project builds it, the studio otherwise (and when `built.json` cannot be
/// read).
pub fn look_of(project_dir: &Path, asset: &str) -> ThumbLook {
    match crate::built::load(project_dir) {
        Ok(built) if built.assets.contains_key(asset) => ThumbLook::Page,
        _ => ThumbLook::Studio,
    }
}
