//! The export name a By hand send asks for:
//! `maq_<project>_<asset>_v<n>_<provider>`, where `<n>` is the send's number
//! for that asset in its project (the `n` of the job id `<asset>-<n>`). A
//! file saved under it is matched to its send without a guess, whatever
//! extension the site gives it and whatever copy number the browser adds
//! ("maq_moon_crate_v2_tripo (1).glb").
//!
//! Sends handed out before the project was part of the name wait for
//! `maq_<asset>_v<n>_<provider>`. Both forms are read: a part holds only
//! lower case letters, digits and `-`, so `_` always separates, and the
//! older form has one part before `_v<n>` where the newer has two.

use maquette_model::project::latin_slug;

use super::presets::ProviderId;

const PREFIX: &str = "maq_";
const EXTENSIONS: [&str; 4] = ["glb", "gltf", "fbx", "zip"];
/// The most characters the project part of a name takes.
pub const PROJECT_PART_MAX: usize = 24;
/// The most characters the asset part of a name takes.
pub const ASSET_PART_MAX: usize = 40;
/// What stands between a cut part and its tag. An id never holds it.
const TAG_MARK: &str = "--";
/// Hex characters of a cut part's tag.
const TAG_LEN: usize = 6;

/// A file name read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportName {
    /// The project part; none for a name from before it carried one.
    pub project: Option<String>,
    /// The asset part: the asset id, unless it was cut (see [`part`]).
    pub asset: String,
    pub version: u32,
    pub provider: ProviderId,
}

impl ExportName {
    /// The name as it is written, without an extension or a copy number.
    pub fn stem(&self) -> String {
        let project = self
            .project
            .as_deref()
            .map(|p| format!("{p}_"))
            .unwrap_or_default();
        format!(
            "{PREFIX}{project}{}_v{}_{}",
            self.asset,
            self.version,
            provider_word(self.provider)
        )
    }
}

fn provider_word(service: ProviderId) -> &'static str {
    match service {
        ProviderId::Tripo => "tripo",
        ProviderId::Meshy => "meshy",
    }
}

/// The send's number: `3` for `crate-lid-3` of asset `crate-lid`.
pub fn send_number(job_id: &str, asset: &str) -> Option<u32> {
    let digits = job_id.strip_prefix(asset)?.strip_prefix('-')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok()
}

/// The `<n>` of a name: the send's number, or what follows the asset in an
/// id that has none.
fn number_text(job_id: &str, asset: &str) -> String {
    send_number(job_id, asset).map_or_else(
        || job_id.strip_prefix(asset).unwrap_or(job_id).to_string(),
        |n| n.to_string(),
    )
}

/// True for text a name can carry as one part: lower case letters, digits
/// and `-`.
fn is_part(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// True for an id that goes into a name as it is: a slug as Maquette makes
/// them, which never starts or ends with `-` and never holds [`TAG_MARK`].
fn is_plain(id: &str) -> bool {
    is_part(id) && !id.starts_with('-') && !id.ends_with('-') && !id.contains(TAG_MARK)
}

/// FNV-1a, 32 bit: the same on every computer and in every version, which
/// a name that syncs between two needs.
fn fnv1a(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193)
    })
}

/// An id as one part of a name, at most `max` characters. A slug within
/// `max` is used as it is. A longer id, or one with other characters (a
/// project folder named by hand can hold spaces, capitals or `_`), is cut:
/// the start of its slug, [`TAG_MARK`] and six hex characters of its hash,
/// so two long ids that start alike still get two names, and a cut part
/// never reads as another id.
pub fn part(id: &str, max: usize, fallback: &str) -> String {
    if is_plain(id) && id.len() <= max {
        return id.to_string();
    }
    let keep = max.saturating_sub(TAG_MARK.len() + TAG_LEN);
    let slug = latin_slug(id).unwrap_or_else(|| fallback.to_string());
    let mut head: String = slug.chars().take(keep).collect();
    while head.ends_with('-') {
        head.pop();
    }
    format!("{head}{TAG_MARK}{:06x}", fnv1a(id.as_bytes()) & 0x00ff_ffff)
}

/// The project part of a name.
pub fn project_part(project: &str) -> String {
    part(project, PROJECT_PART_MAX, "project")
}

/// The asset part of a name.
pub fn asset_part(asset: &str) -> String {
    part(asset, ASSET_PART_MAX, "asset")
}

/// The name to export as, without an extension:
/// `maq_moon-island_crate-lid_v3_tripo`.
pub fn export_stem(project: &str, job_id: &str, asset: &str, service: ProviderId) -> String {
    format!(
        "{PREFIX}{}_{}_v{}_{}",
        project_part(project),
        asset_part(asset),
        number_text(job_id, asset),
        provider_word(service)
    )
}

/// The name a send from before the project was part of it waits for:
/// `maq_crate-lid_v3_tripo`. Only for reading records that old.
pub fn legacy_stem(job_id: &str, asset: &str, service: ProviderId) -> String {
    format!(
        "{PREFIX}{asset}_v{}_{}",
        number_text(job_id, asset),
        provider_word(service)
    )
}

/// The name without its model extension, any folder and surrounding space,
/// in lower case.
fn bare(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    let lower = base.to_lowercase();
    match lower.rsplit_once('.') {
        Some((stem, ext)) if EXTENSIONS.contains(&ext) => stem.trim().to_string(),
        _ => lower,
    }
}

/// What a browser adds to a second copy: " (1)", " 2", "-3", "_4".
fn is_copy_suffix(rest: &str) -> bool {
    let trimmed = rest.trim_start_matches([' ', '-', '_']);
    let inner = trimmed
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(trimmed);
    rest.is_empty() || (!inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit()))
}

/// Reads `maq_<project>_<asset>_v<n>_<provider>` or the older
/// `maq_<asset>_v<n>_<provider>` back, or `None` for any other name.
///
/// The provider is the last word (before a copy number), so an asset or a
/// project named after a provider reads right; `_v<n>` is the last `_v`
/// before it; what is left is one part or two.
pub fn parse(name: &str) -> Option<ExportName> {
    let stem = bare(name);
    let rest = stem.strip_prefix(PREFIX)?;
    let (head, provider) = ProviderId::ALL.iter().find_map(|&service| {
        let word = format!("_{}", provider_word(service));
        let at = rest.rfind(&word)?;
        is_copy_suffix(&rest[at + word.len()..]).then_some((&rest[..at], service))
    })?;
    let (ids, digits) = head.rsplit_once("_v")?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let version = digits.parse::<u32>().ok()?;
    let (project, asset) = match ids.split_once('_') {
        Some((project, asset)) => (Some(project), asset),
        None => (None, ids),
    };
    (project.is_none_or(is_part) && is_part(asset)).then(|| ExportName {
        project: project.map(str::to_string),
        asset: asset.to_string(),
        version,
        provider,
    })
}

/// True when `file` is saved under the export name `expected`: that name,
/// in any case, with any model extension and any copy number.
pub fn is_named(file: &str, expected: &str) -> bool {
    let expected = expected.trim().to_lowercase();
    if expected.is_empty() {
        return false;
    }
    parse(file).is_some_and(|named| named.stem() == expected)
        || bare(file)
            .strip_prefix(expected.as_str())
            .is_some_and(is_copy_suffix)
}

#[cfg(test)]
mod tests;
