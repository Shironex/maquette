//! The project's style board: `style.json` next to `project.json`, with its
//! images copied into `style/`.
//!
//! ```json
//! {
//!   "version": 1,
//!   "revision": 3,
//!   "line": "frosted glass, cool night light, chunky silhouettes",
//!   "palette": ["#1b2440", "#9fc3ff", "#e8f1ff"],
//!   "images": [
//!     { "file": "style/veil_gate_kit_02.png", "source": "concepts/veil-gate-kit/veil-gate-kit_02.png" }
//!   ],
//!   "updatedAt": "2026-10-28T09:12:00Z"
//! }
//! ```
//!
//! Every Lab direction and object ref job carries the board unless its
//! brief turns it off: the line and palette go into the prompt after the
//! direction's own text, and the images are attached after the direction's
//! references (see [`super::codex::assemble`]). Each change that alters the
//! board bumps `revision`, and a job records the revision it carried.
//!
//! The images are always copies inside the project, never links to files
//! elsewhere. A copy that leaves the board stays in `style/`, so a job that
//! still attaches it runs again the same.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::objects::check_source;
use crate::error::{CoreError, CoreResult};
use crate::json::{from_json_with_defaults, write_json};
use crate::project::now_rfc3339;

pub const STYLE_FILE: &str = "style.json";
pub const STYLE_DIR: &str = "style";
pub const STYLE_VERSION: u32 = 1;
/// A handful: enough to set a look, few enough that the brief still leads.
pub const MAX_STYLE_IMAGES: usize = 4;
pub const MAX_LINE_CHARS: usize = 200;
pub const MAX_PALETTE: usize = 6;

/// One image of the board.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StyleImage {
    /// The copy, relative to the project folder: `style/<name>`.
    pub file: String,
    /// The concept or ref it was copied from, relative to the project
    /// folder. `None` for a file added from elsewhere on disk.
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StyleBoard {
    pub version: u32,
    /// Counts the changes; 0 for a board never saved.
    pub revision: u32,
    /// One written style line, for example "frosted glass, cool night light".
    pub line: String,
    /// `#rrggbb` colours, lowercase.
    pub palette: Vec<String>,
    pub images: Vec<StyleImage>,
    pub updated_at: Option<String>,
}

impl Default for StyleBoard {
    fn default() -> Self {
        Self {
            version: STYLE_VERSION,
            revision: 0,
            line: String::new(),
            palette: Vec::new(),
            images: Vec::new(),
            updated_at: None,
        }
    }
}

impl StyleBoard {
    /// True when there is nothing to carry: no line, colour or image.
    pub fn is_empty(&self) -> bool {
        self.line.trim().is_empty() && self.palette.is_empty() && self.images.is_empty()
    }

    /// The image copies, relative to the project folder, in board order.
    pub fn files(&self) -> Vec<String> {
        self.images.iter().map(|i| i.file.clone()).collect()
    }
}

/// A change from the app or an agent. Every field left out stays.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StyleBoardChange {
    #[serde(default)]
    pub line: Option<String>,
    #[serde(default)]
    pub palette: Option<Vec<String>>,
    /// The whole new list of images, in order. Each is a board image as it
    /// is (`style/...`), a concept or ref of the project (`concepts/...`,
    /// `refs/...`), or an absolute path to an image to copy in.
    #[serde(default)]
    pub images: Option<Vec<String>>,
}

/// Serialises board writes: the app and an agent may change it at once.
static WRITES: Mutex<()> = Mutex::new(());

pub fn path(project_dir: &Path) -> PathBuf {
    project_dir.join(STYLE_FILE)
}

/// One line, inner runs of white space folded to one space.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `#rrggbb` in lowercase from `#rgb`, `rrggbb` and the like, or `None`.
pub fn normalize_colour(text: &str) -> Option<String> {
    let hex = text.trim().trim_start_matches('#').to_ascii_lowercase();
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        6 => Some(format!("#{hex}")),
        3 => Some(format!(
            "#{}",
            hex.chars().flat_map(|c| [c, c]).collect::<String>()
        )),
        _ => None,
    }
}

fn is_board_file(file: &str) -> bool {
    file.strip_prefix(STYLE_DIR)
        .and_then(|rest| rest.strip_prefix('/'))
        .is_some_and(|name| crate::workspace::is_single_segment(name) && !name.starts_with('.'))
}

/// Checks the line, the palette and the image list.
///
/// # Errors
///
/// Returns the first problem as a sentence.
pub fn validate(board: &StyleBoard) -> CoreResult<()> {
    let invalid = |m: String| Err(CoreError::Invalid(m));
    if board.version > STYLE_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: board.version,
            supported: STYLE_VERSION,
        });
    }
    if board.line.chars().count() > MAX_LINE_CHARS {
        return invalid(format!(
            "Keep the style line to {MAX_LINE_CHARS} characters or fewer."
        ));
    }
    if board.palette.len() > MAX_PALETTE {
        return invalid(format!("Keep the palette to {MAX_PALETTE} colours."));
    }
    if let Some(bad) = board
        .palette
        .iter()
        .find(|c| normalize_colour(c).as_deref() != Some(c.as_str()))
    {
        return invalid(format!("{bad:?} is not a colour. Write it as #rrggbb."));
    }
    if board.images.len() > MAX_STYLE_IMAGES {
        return invalid(format!(
            "The style board holds {MAX_STYLE_IMAGES} images at most."
        ));
    }
    if let Some(bad) = board.images.iter().find(|i| !is_board_file(&i.file)) {
        return invalid(format!(
            "{} is not an image in the project's {STYLE_DIR}/ folder.",
            bad.file
        ));
    }
    Ok(())
}

/// The project's `style.json`, or an empty board without one.
///
/// # Errors
///
/// Fails when the file exists but cannot be read or [`validate`] refuses it.
pub fn load(project_dir: &Path) -> CoreResult<StyleBoard> {
    let file = path(project_dir);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(StyleBoard::default()),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let board: StyleBoard =
        from_json_with_defaults(&text).map_err(|e| CoreError::json(&file, e))?;
    validate(&board)?;
    Ok(board)
}

/// [`load`], falling back to an empty board (and a warning) for a broken
/// file, for the views that must still show something.
pub fn load_or_default(project_dir: &Path) -> StyleBoard {
    load(project_dir).unwrap_or_else(|err| {
        tracing::warn!(error = %err, "unreadable style.json, no style board");
        StyleBoard::default()
    })
}

/// The board a new job carries: `None` when the brief turned it off, the
/// board is empty, or it cannot be read. Images whose copy is gone drop.
pub fn for_job(project_dir: &Path, use_style: bool) -> Option<StyleBoard> {
    if !use_style {
        return None;
    }
    let mut board = load_or_default(project_dir);
    board.images.retain(|i| project_dir.join(&i.file).is_file());
    (!board.is_empty()).then_some(board)
}

/// True for a PNG, JPEG or WebP by its header.
fn is_style_image(path: &Path) -> bool {
    fs::read(path)
        .ok()
        .and_then(|b| crate::inbox::images::sniff(&b))
        .is_some_and(|mime| mime != "image/ktx2")
}

/// Where an image given to the board comes from.
enum Source {
    /// Already on the board.
    Kept(StyleImage),
    /// A file of the project, relative to its folder.
    Project(String),
    /// A file outside the project, absolute.
    Outside(PathBuf),
}

/// Sorts one entry of [`StyleBoardChange::images`] into a [`Source`].
fn classify(project_dir: &Path, board: &StyleBoard, entry: &str) -> CoreResult<Source> {
    let clean = entry.trim().replace('\\', "/");
    if let Some(kept) = board
        .images
        .iter()
        .find(|i| i.file == clean || i.source.as_deref() == Some(clean.as_str()))
    {
        return Ok(Source::Kept(kept.clone()));
    }
    let given = Path::new(entry.trim());
    if given.is_absolute() {
        let inside = given
            .canonicalize()
            .ok()
            .zip(project_dir.canonicalize().ok())
            .and_then(|(file, dir)| {
                file.strip_prefix(&dir)
                    .ok()
                    .map(|rel| rel.to_string_lossy().replace('\\', "/"))
            });
        return match inside {
            Some(rel) => classify(project_dir, board, &rel),
            None if given.is_file() => Ok(Source::Outside(given.to_path_buf())),
            None => Err(CoreError::NotFound(format!(
                "the image {}",
                given.display()
            ))),
        };
    }
    Ok(Source::Project(check_source(project_dir, &clean)?))
}

/// Copies `from` into `style/`, named after it, and returns the copy's
/// path relative to the project folder. The name is one safe segment, so
/// the copy always lands inside the project.
fn copy_in(project_dir: &Path, from: &Path) -> CoreResult<String> {
    if !is_style_image(from) {
        return Err(CoreError::Invalid(format!(
            "{} is not a PNG, JPEG or WebP image.",
            from.file_name().map_or_else(
                || from.display().to_string(),
                |n| n.to_string_lossy().into_owned()
            )
        )));
    }
    let stem = from
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = from
        .extension()
        .map_or_else(|| "png".to_string(), |e| e.to_string_lossy().to_lowercase());
    let ext = if ["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) {
        ext
    } else {
        "png".to_string()
    };
    let rel = super::objects::free_name(project_dir, STYLE_DIR, &stem, &ext);
    let dir = project_dir.join(STYLE_DIR);
    fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    fs::copy(from, project_dir.join(&rel)).map_err(|e| CoreError::io(from, e))?;
    Ok(rel)
}

/// The board's new image list from a change's entries: kept ones as they
/// are, the rest copied in. Repeats drop.
fn gather(
    project_dir: &Path,
    board: &StyleBoard,
    entries: &[String],
) -> CoreResult<Vec<StyleImage>> {
    let mut out: Vec<StyleImage> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for entry in entries.iter().filter(|e| !e.trim().is_empty()) {
        let key = entry.trim().replace('\\', "/");
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let image = match classify(project_dir, board, entry)? {
            Source::Kept(image) => image,
            Source::Project(rel) => {
                if out.iter().any(|i| i.source.as_deref() == Some(&rel)) {
                    continue;
                }
                StyleImage {
                    file: copy_in(project_dir, &project_dir.join(&rel))?,
                    source: Some(rel),
                }
            }
            Source::Outside(path) => StyleImage {
                file: copy_in(project_dir, &path)?,
                source: None,
            },
        };
        if out.iter().any(|i| i.file == image.file) {
            continue;
        }
        out.push(image);
        if out.len() > MAX_STYLE_IMAGES {
            return Err(CoreError::Invalid(format!(
                "The style board holds {MAX_STYLE_IMAGES} images at most."
            )));
        }
    }
    Ok(out)
}

/// The palette cleaned: each colour as `#rrggbb`, repeats dropped.
fn clean_palette(palette: &[String]) -> CoreResult<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for raw in palette.iter().filter(|c| !c.trim().is_empty()) {
        let colour = normalize_colour(raw).ok_or_else(|| {
            CoreError::Invalid(format!(
                "{:?} is not a colour. Write it as #rrggbb.",
                raw.trim()
            ))
        })?;
        if !out.contains(&colour) {
            out.push(colour);
        }
    }
    Ok(out)
}

/// Applies `change` to the stored board and saves it. The revision goes up
/// only when the line, palette or images really changed.
///
/// # Errors
///
/// Fails when an image is outside the project and not a file, is not an
/// image, or the result does not validate or cannot be written.
pub fn update(project_dir: &Path, change: &StyleBoardChange) -> CoreResult<StyleBoard> {
    let _guard = WRITES.lock().unwrap_or_else(|p| p.into_inner());
    let before = load(project_dir)?;
    let mut board = before.clone();
    if let Some(line) = &change.line {
        board.line = one_line(line);
    }
    if let Some(palette) = &change.palette {
        board.palette = clean_palette(palette)?;
    }
    // Checked before any copy, so a refused line copies nothing.
    validate(&board)?;
    if let Some(images) = &change.images {
        board.images = gather(project_dir, &before, images)?;
    }
    validate(&board)?;
    let same = board.line == before.line
        && board.palette == before.palette
        && board.images == before.images;
    if same {
        return Ok(before);
    }
    board.version = STYLE_VERSION;
    board.revision = before.revision + 1;
    board.updated_at = Some(now_rfc3339());
    write_json(&path(project_dir), &board)?;
    Ok(board)
}

#[cfg(test)]
mod tests;
