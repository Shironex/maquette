//! Finishing a pack: the README, the manifest and the contact sheet next to
//! the model folders, then the zip. Only the models that pass (and, when a
//! person says so, the ones with a warning) go in; the folders of the rest
//! are taken out, so the folder and the zip hold the same pack.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::licence::{self, MIXAMO_NOTE};
use super::model_run::WORK_DIR;
use super::record::{MARKER, PackItem, PackItemState, PackRecord, PackState};
use super::{PackCompression, PackSettings, sheet};
use crate::assets::AssetKind;
use crate::error::{CoreError, CoreResult};
use crate::project::now_rfc3339;

/// Which models a finish takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ZipChoice {
    /// Only the models that pass or were kept.
    Passing,
    /// Those and the ones with a warning, kept as they are.
    All,
}

pub(super) const README: &str = "README.txt";
pub(super) const MANIFEST: &str = "manifest.json";
pub(super) const SHEET: &str = "contact-sheet.png";

fn takes(item: &PackItem, choice: ZipChoice) -> bool {
    !item.files.is_empty()
        && (item.state.passes() || (choice == ZipChoice::All && item.state == PackItemState::Warn))
}

/// "115,673".
fn with_commas(n: u32) -> String {
    let digits = n.to_string();
    let mut text = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

fn budgets_line(settings: &PackSettings) -> String {
    [AssetKind::Prop, AssetKind::Character, AssetKind::Scene]
        .into_iter()
        .map(|kind| {
            let label = super::kind_label(kind).to_lowercase();
            match settings.budget(kind) {
                Some(t) => format!("{label} {}", with_commas(t)),
                None => format!("{label} as they are"),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The README a pack carries.
pub(super) fn readme(record: &PackRecord, items: &[&PackItem]) -> String {
    let s = &record.settings;
    let mut text = format!("{}, {} models\n\n", record.name, items.len());
    let texture = if s.texture == 0 {
        "textures as they were".to_string()
    } else {
        format!("textures up to {} px", s.texture)
    };
    text += &format!(
        "Format    {}, Y up, {}\n",
        s.format.label(),
        if s.real_size {
            "metres, real size"
        } else {
            "metres"
        }
    );
    if s.compression == PackCompression::Web {
        text += "          GLB files compressed with meshopt and KTX2\n";
    }
    text += &format!(
        "Pivot     {}\n",
        if s.pivot_bottom {
            "bottom centre, on the floor"
        } else {
            "as each model was made"
        }
    );
    text += &format!("Budget    {}\n", budgets_line(s));
    text += &format!(
        "Material  {}, {texture}\n\n",
        if s.one_material {
            "named body (body_2 and on for more)"
        } else {
            "as each model was made"
        }
    );
    text += "Import: drop the folder in. Keep each model's files together in its folder:\n";
    text += "an .obj reads its .mtl, and the .mtl its .png, by name, from the same folder.\n";
    if s.format.obj() && s.rigged_glb {
        text += "A rigged model also has a .glb with its bones, since OBJ cannot hold a rig.\n";
    }
    text += "\nWhere each model came from and whose terms apply is in manifest.json.\n";
    if items.iter().any(|i| i.clips_left_out > 0 || i.model.mixamo) {
        text += &format!("\n{MIXAMO_NOTE}\n");
    }
    text += "\nMade with Maquette.\n";
    text
}

/// The manifest a pack carries.
pub(super) fn manifest(record: &PackRecord, items: &[&PackItem]) -> serde_json::Value {
    let s = &record.settings;
    let models: Vec<serde_json::Value> = items
        .iter()
        .map(|i| {
            let m = &i.model;
            let prefix = format!("{}/", m.key);
            let files: Vec<&str> = i
                .files
                .iter()
                .map(|f| f.strip_prefix(&prefix).unwrap_or(f))
                .collect();
            let from = match (&m.entry, &m.project, &m.asset) {
                (Some(entry), _, _) => json!({ "library": entry, "version": m.version }),
                (None, Some(project), Some(asset)) => {
                    json!({ "project": project, "asset": asset, "version": m.version })
                }
                _ => serde_json::Value::Null,
            };
            json!({
                "name": m.name,
                "key": m.key,
                "kind": m.kind,
                "tris": i.triangles,
                "trisBefore": m.triangles(),
                "height_m": i.height_m.map(|h| (f64::from(h) * 1000.0).round() / 1000.0),
                "files": files,
                "from": from,
                "source": licence::source_label(m.source),
                "licence": m.licence,
                "clipsLeftOut": i.clips_left_out,
                "check": i.state,
                "note": i.note,
            })
        })
        .collect();
    let mut notes = Vec::new();
    if items.iter().any(|i| i.clips_left_out > 0 || i.model.mixamo) {
        notes.push(MIXAMO_NOTE);
    }
    json!({
        "pack": record.name,
        "made": now_rfc3339(),
        "maquette": env!("CARGO_PKG_VERSION"),
        "up": "Y",
        "units": "m",
        "realSize": s.real_size,
        "pivot": if s.pivot_bottom { "bottom-centre" } else { "as-made" },
        "format": s.format,
        "compression": s.compression,
        "textureMax": (s.texture > 0).then_some(s.texture),
        "models": models,
        "notes": notes,
    })
}

/// Every file under `dir` but dot files and dot folders, relative, sorted.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(folder) = stack.pop() {
        let Ok(read) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in read.filter_map(Result::ok) {
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(rel) = path.strip_prefix(dir) {
                found.push(rel.to_path_buf());
            }
        }
    }
    found.sort();
    found
}

/// Now, as the zip's file times (UTC, to the second).
fn zip_time() -> zip::DateTime {
    let now = now_rfc3339();
    let num =
        |at: std::ops::Range<usize>| now.get(at).and_then(|s| s.parse::<u16>().ok()).unwrap_or(0);
    let small = |at: std::ops::Range<usize>| u8::try_from(num(at)).unwrap_or(0);
    zip::DateTime::from_date_and_time(
        num(0..4),
        small(5..7),
        small(8..10),
        small(11..13),
        small(14..16),
        small(17..19),
    )
    .unwrap_or_default()
}

fn zip_error(e: impl std::fmt::Display) -> CoreError {
    CoreError::Invalid(format!("Could not write the zip. {e}"))
}

/// Zips `dir` as `<name>/...` into `zip`, written whole or not at all.
pub(super) fn write_zip(dir: &Path, name: &str, zip: &Path) -> CoreResult<f64> {
    let partial = zip.with_extension("zip.part");
    let file = fs::File::create(&partial).map_err(|e| CoreError::io(&partial, e))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip_time());
    for rel in files_under(dir) {
        let inside = format!("{name}/{}", rel.to_string_lossy().replace('\\', "/"));
        let bytes = fs::read(dir.join(&rel)).map_err(|e| CoreError::io(dir.join(&rel), e))?;
        writer.start_file(inside, options).map_err(zip_error)?;
        writer.write_all(&bytes).map_err(zip_error)?;
    }
    writer.finish().map_err(zip_error)?;
    fs::rename(&partial, zip).map_err(|e| CoreError::io(zip, e))?;
    #[expect(clippy::cast_precision_loss, reason = "zip sizes stay below 2^53")]
    Ok(fs::metadata(zip).map_or(0.0, |m| m.len() as f64))
}

/// Writes the README, manifest and contact sheet, takes out the models the
/// choice leaves, and zips the folder when the settings ask. The record
/// comes back done.
///
/// # Errors
///
/// Fails when a file cannot be written; the record then says why.
pub fn finish(record: &mut PackRecord, choice: ZipChoice) -> CoreResult<()> {
    let dir = PathBuf::from(&record.dir);
    let left: Vec<String> = record
        .items
        .iter()
        .filter(|i| !takes(i, choice))
        .map(|i| i.model.key.clone())
        .collect();
    for key in &left {
        let folder = dir.join(key);
        if folder.is_dir() {
            fs::remove_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
        }
    }
    let items: Vec<&PackItem> = record.items.iter().filter(|i| takes(i, choice)).collect();
    if items.is_empty() {
        return Err(CoreError::Invalid(
            "No model is ready to go in the pack. Export the failed ones again first.".to_string(),
        ));
    }
    let s = record.settings.clone();
    if s.readme {
        let path = dir.join(README);
        crate::fs::write_atomic(&path, readme(record, &items).as_bytes())
            .map_err(|e| CoreError::io(&path, e))?;
    }
    if s.manifest {
        crate::json::write_json(&dir.join(MANIFEST), &manifest(record, &items))?;
    }
    if s.contact_sheet {
        let pictures: Vec<PathBuf> = items
            .iter()
            .filter_map(|i| i.picture.as_ref().map(PathBuf::from))
            .collect();
        sheet::write(&pictures, &dir.join(SHEET))?;
    }
    let work = dir.join(WORK_DIR);
    if work.exists() {
        fs::remove_dir_all(&work).map_err(|e| CoreError::io(&work, e))?;
    }
    let marker = dir.join(MARKER);
    crate::fs::write_atomic(&marker, record.id.as_bytes())
        .map_err(|e| CoreError::io(&marker, e))?;
    record.bytes = Some(if s.zip {
        let zip = PathBuf::from(record.zip.clone().unwrap_or_default());
        write_zip(&dir, &record.name, &zip)?
    } else {
        record.zip = None;
        #[expect(clippy::cast_precision_loss, reason = "pack sizes stay below 2^53")]
        let total = files_under(&dir)
            .iter()
            .map(|f| fs::metadata(dir.join(f)).map_or(0.0, |m| m.len() as f64))
            .sum();
        total
    });
    record.state = PackState::Done;
    record.finished_at = Some(now_rfc3339());
    record.message = (!left.is_empty()).then(|| {
        format!(
            "{} model{} left out of the pack.",
            left.len(),
            if left.len() == 1 { "" } else { "s" }
        )
    });
    Ok(())
}
