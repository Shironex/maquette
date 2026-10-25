//! Bringing a By hand model back: reading the file you chose (a GLB, or
//! the .zip with one GLB inside that some web studios download), and
//! finding models in the Downloads folder that belong to sends waiting
//! for one.
//!
//! Nothing here moves, renames or deletes your files. A model is read and
//! copied into the inbox; the original stays where it was.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::jobs::{JobState, Provider, ProviderJob, is_glb};
use crate::error::{CoreError, CoreResult};

/// The largest model a hand-back reads, unzipped.
pub const MAX_MODEL_BYTES: u64 = 512 * 1024 * 1024;
/// Downloads only counts files changed this long before a handoff was
/// written, for clocks that disagree a little.
const SLACK_MS: f64 = 60_000.0;
/// How many files a Downloads check lists at most, newest first.
const MAX_FILES: usize = 40;
/// Words in download names that say nothing about the object.
const NOISE: [&str; 14] = [
    "glb", "gltf", "model", "models", "mesh", "tripo", "meshy", "textured", "texture", "pbr",
    "final", "download", "export", "3d",
];

/// A model read for the inbox.
#[derive(Debug)]
pub struct Model {
    pub bytes: Vec<u8>,
    /// The GLB's name inside the zip, when it came zipped.
    pub inner: Option<String>,
}

fn is_zip(head: &[u8]) -> bool {
    head.len() >= 4 && &head[..4] == b"PK\x03\x04"
}

/// True for a GLB inside a zip worth taking: not a folder, not macOS
/// resource fork junk, not a hidden file.
fn is_model_entry(name: &str) -> bool {
    let name = name.replace('\\', "/");
    let base = name.rsplit('/').next().unwrap_or(&name);
    !name.ends_with('/')
        && !name.starts_with("__MACOSX/")
        && !base.starts_with('.')
        && base.to_ascii_lowercase().ends_with(".glb")
}

/// The GLB names inside a zip, in archive order.
fn zip_models(file: &Path) -> CoreResult<Vec<String>> {
    let reader = File::open(file).map_err(|e| CoreError::io(file, e))?;
    let archive = zip::ZipArchive::new(reader).map_err(|e| {
        CoreError::Invalid(format!("{} is not a zip I can read. {e}", file_name(file)))
    })?;
    Ok(archive
        .file_names()
        .filter(|n| is_model_entry(n))
        .map(str::to_string)
        .collect())
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

fn too_big() -> CoreError {
    CoreError::Invalid(format!(
        "That model is over {} MB, more than the inbox takes.",
        MAX_MODEL_BYTES / (1024 * 1024)
    ))
}

/// Reads the model in `file`: a GLB as it is, or the one GLB inside a zip.
pub fn read_model(file: &Path) -> CoreResult<Model> {
    let mut head = [0u8; 12];
    let n = {
        let mut f = File::open(file).map_err(|e| CoreError::io(file, e))?;
        let mut read = 0;
        while read < head.len() {
            match f.read(&mut head[read..]) {
                Ok(0) => break,
                Ok(k) => read += k,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(CoreError::io(file, e)),
            }
        }
        read
    };
    let head = &head[..n];
    if is_glb(head) {
        let size = fs::metadata(file)
            .map_err(|e| CoreError::io(file, e))?
            .len();
        if size > MAX_MODEL_BYTES {
            return Err(too_big());
        }
        let bytes = fs::read(file).map_err(|e| CoreError::io(file, e))?;
        return Ok(Model { bytes, inner: None });
    }
    if !is_zip(head) {
        return Err(CoreError::Invalid(
            "That file is not a GLB or a zip with one inside. Download the model as .glb and add it again."
                .to_string(),
        ));
    }
    let models = zip_models(file)?;
    let name = match models.as_slice() {
        [] => {
            return Err(CoreError::Invalid(format!(
                "{} has no .glb inside. Download the model as GLB and add it again.",
                file_name(file)
            )));
        }
        [one] => one.clone(),
        many => {
            return Err(CoreError::Invalid(format!(
                "{} holds {} .glb files ({}). Unzip it and add the one you want.",
                file_name(file),
                many.len(),
                many.join(", ")
            )));
        }
    };
    let reader = File::open(file).map_err(|e| CoreError::io(file, e))?;
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| {
        CoreError::Invalid(format!("{} is not a zip I can read. {e}", file_name(file)))
    })?;
    let mut entry = archive.by_name(&name).map_err(|e| {
        CoreError::Invalid(format!("Could not read {name} in {}. {e}", file_name(file)))
    })?;
    if entry.size() > MAX_MODEL_BYTES {
        return Err(too_big());
    }
    // The size in the header can lie, so the read is capped as well.
    let mut bytes = Vec::new();
    (&mut entry)
        .take(MAX_MODEL_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| CoreError::Invalid(format!("Could not unzip {name}. {e}")))?;
    if bytes.len() as u64 > MAX_MODEL_BYTES {
        return Err(too_big());
    }
    if !is_glb(&bytes) {
        return Err(CoreError::Invalid(format!(
            "{name} in {} is not a GLB file.",
            file_name(file)
        )));
    }
    Ok(Model {
        bytes,
        inner: Some(name),
    })
}

/// A model file in the Downloads folder, with the send it most likely
/// belongs to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadMatch {
    /// Absolute.
    pub path: String,
    pub name: String,
    /// The GLB inside, for a zip.
    pub inner: Option<String>,
    pub bytes: f64,
    pub modified_ms: f64,
    /// The waiting send it matches, if any. You confirm every match.
    pub job: Option<String>,
    pub asset_name: Option<String>,
    /// Why it matched, in a few words.
    pub reason: Option<String>,
}

/// What Check Downloads found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsScan {
    /// The folder that was read.
    pub folder: String,
    /// Files changed before this are left out: the oldest waiting handoff.
    pub since_ms: Option<f64>,
    /// Newest first.
    pub files: Vec<DownloadMatch>,
    /// The sends waiting for a model, for choosing a match by hand.
    pub waiting: Vec<ProviderJob>,
}

struct Found {
    path: PathBuf,
    name: String,
    inner: Option<String>,
    bytes: u64,
    modified_ms: f64,
}

fn modified_ms(meta: &fs::Metadata) -> Option<f64> {
    let at = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    #[expect(clippy::cast_precision_loss, reason = "milliseconds since 1970 fit")]
    Some(at.as_millis() as f64)
}

/// GLB and zip files directly in `dir` changed since `since_ms`, newest
/// first. A zip counts only when it holds exactly one GLB.
fn scan(dir: &Path, since_ms: f64) -> Vec<Found> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<Found> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            let lower = name.to_ascii_lowercase();
            let zipped = lower.ends_with(".zip");
            if !zipped && !lower.ends_with(".glb") {
                return None;
            }
            let meta = entry.metadata().ok().filter(fs::Metadata::is_file)?;
            let modified = modified_ms(&meta)?;
            if modified + SLACK_MS < since_ms || meta.len() == 0 {
                return None;
            }
            let inner = if zipped {
                match zip_models(&path).ok()?.as_slice() {
                    [one] => Some(one.clone()),
                    _ => return None,
                }
            } else {
                None
            };
            Some(Found {
                path,
                name,
                inner,
                bytes: meta.len(),
                modified_ms: modified,
            })
        })
        .collect();
    found.sort_by(|a, b| b.modified_ms.total_cmp(&a.modified_ms));
    found.truncate(MAX_FILES);
    found
}

fn words(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        let w = word.to_string();
        if NOISE.contains(&w.as_str()) || w.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(s, _)| s)
}

/// How well a file name fits a send, 0 to 1: the share of the asset's
/// words the name carries, or 1 for the exact return name.
fn name_score(found: &Found, job: &ProviderJob) -> f64 {
    let mut names = vec![stem(&found.name).to_string()];
    if let Some(inner) = &found.inner {
        names.push(stem(inner.rsplit('/').next().unwrap_or(inner)).to_string());
    }
    let exact = |n: &str| n.eq_ignore_ascii_case(&job.id);
    if names.iter().any(|n| exact(n)) {
        return 1.0;
    }
    let wanted = words(&job.asset_name);
    if wanted.is_empty() {
        return 0.0;
    }
    let have: Vec<String> = names.iter().flat_map(|n| words(n)).collect();
    let hits = wanted
        .iter()
        .filter(|w| {
            have.iter().any(|h| {
                h == *w || (w.len() >= 4 && h.len() >= 4 && (h.starts_with(*w) || w.starts_with(h)))
            })
        })
        .count();
    #[expect(clippy::cast_precision_loss, reason = "a handful of words")]
    let score = hits as f64 / wanted.len() as f64;
    score
}

/// Pairs files with waiting sends: best name match first, each file and
/// each send once. A name that shares no word still matches when it is
/// the only new file left for the only send left.
fn assign(files: &[Found], jobs: &[&ProviderJob]) -> Vec<Option<(usize, String)>> {
    let newer = |f: &Found, j: &ProviderJob| f.modified_ms + SLACK_MS >= j.created_at_ms;
    let mut pairs: Vec<(f64, f64, usize, usize)> = Vec::new();
    for (fi, f) in files.iter().enumerate() {
        for (ji, j) in jobs.iter().enumerate() {
            if !newer(f, j) {
                continue;
            }
            let score = name_score(f, j);
            if score >= 0.5 {
                pairs.push((score, (f.modified_ms - j.created_at_ms).abs(), fi, ji));
            }
        }
    }
    pairs.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.total_cmp(&b.1)));
    let mut out: Vec<Option<(usize, String)>> = vec![None; files.len()];
    let mut taken = vec![false; jobs.len()];
    for (score, _, fi, ji) in pairs {
        if out[fi].is_some() || taken[ji] {
            continue;
        }
        taken[ji] = true;
        let reason = if (score - 1.0).abs() < f64::EPSILON {
            format!("Named like {}", jobs[ji].asset_name)
        } else {
            format!("Name close to {}", jobs[ji].asset_name)
        };
        out[fi] = Some((ji, reason));
    }
    let free_files: Vec<usize> = (0..files.len()).filter(|&i| out[i].is_none()).collect();
    let free_jobs: Vec<usize> = (0..jobs.len()).filter(|&i| !taken[i]).collect();
    if let ([fi], [ji]) = (free_files.as_slice(), free_jobs.as_slice())
        && newer(&files[*fi], jobs[*ji])
    {
        out[*fi] = Some((*ji, "The only new model since its handoff".to_string()));
    }
    out
}

/// Looks in `dir` (the Downloads folder) for models newer than the oldest
/// of `jobs` that waits for one, and offers a match for each.
pub fn check_downloads(dir: &Path, jobs: &[ProviderJob]) -> DownloadsScan {
    let waiting: Vec<&ProviderJob> = jobs
        .iter()
        .filter(|j| j.provider == Provider::ByHand && j.state == JobState::Waiting)
        .collect();
    let since = waiting
        .iter()
        .map(|j| j.created_at_ms)
        .min_by(f64::total_cmp);
    let files = since.map_or_else(Vec::new, |since| scan(dir, since));
    let matched = assign(&files, &waiting);
    let files = files
        .into_iter()
        .zip(matched)
        .map(|(f, m)| {
            let job = m.as_ref().map(|(ji, _)| waiting[*ji]);
            #[expect(clippy::cast_precision_loss, reason = "file sizes fit")]
            DownloadMatch {
                path: f.path.display().to_string(),
                name: f.name,
                inner: f.inner,
                bytes: f.bytes as f64,
                modified_ms: f.modified_ms,
                job: job.map(|j| j.id.clone()),
                asset_name: job.map(|j| j.asset_name.clone()),
                reason: m.map(|(_, r)| r),
            }
        })
        .collect();
    DownloadsScan {
        folder: dir.display().to_string(),
        since_ms: since,
        files,
        waiting: waiting.into_iter().cloned().collect(),
    }
}

#[cfg(test)]
mod tests;
