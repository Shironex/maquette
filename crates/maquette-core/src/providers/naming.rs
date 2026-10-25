//! The export name a By hand send asks for: `maq_<asset>_v<n>_<provider>`,
//! where `<n>` is the send's number for that asset (the `n` of the job id
//! `<asset>-<n>`). A file saved under it is matched to its send without a
//! guess, whatever extension the site gives it and whatever copy number
//! the browser adds ("maq_crate_v2_tripo (1).glb").

use super::presets::ProviderId;

const PREFIX: &str = "maq_";
const EXTENSIONS: [&str; 4] = ["glb", "gltf", "fbx", "zip"];

/// A file name read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportName {
    pub asset: String,
    pub version: u32,
    pub provider: ProviderId,
}

impl ExportName {
    /// The job id the name points at.
    pub fn job_id(&self) -> String {
        format!("{}-{}", self.asset, self.version)
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
    job_id
        .strip_prefix(asset)?
        .strip_prefix('-')?
        .parse::<u32>()
        .ok()
}

/// The name to export as, without an extension: `maq_crate-lid_v3_tripo`.
pub fn export_stem(job_id: &str, asset: &str, service: ProviderId) -> String {
    let version = send_number(job_id, asset).map_or_else(
        || job_id.strip_prefix(asset).unwrap_or(job_id).to_string(),
        |n| n.to_string(),
    );
    format!("{PREFIX}{asset}_v{version}_{}", provider_word(service))
}

/// The name without its model extension, any folder and surrounding space.
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

fn is_asset_id(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Reads `maq_<asset>_v<n>_<provider>` back, or `None` for any other name.
pub fn parse(name: &str) -> Option<ExportName> {
    let stem = bare(name);
    let rest = stem.strip_prefix(PREFIX)?;
    let (head, provider, tail) = ProviderId::ALL.iter().find_map(|&service| {
        let word = format!("_{}", provider_word(service));
        let at = rest.rfind(&word)?;
        Some((&rest[..at], service, &rest[at + word.len()..]))
    })?;
    if !is_copy_suffix(tail) {
        return None;
    }
    let (asset, version) = head.rsplit_once("_v")?;
    let version = version.parse::<u32>().ok()?;
    is_asset_id(asset).then(|| ExportName {
        asset: asset.to_string(),
        version,
        provider,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stem_carries_the_asset_the_send_number_and_the_provider() {
        assert_eq!(
            export_stem("crate-lid-3", "crate-lid", ProviderId::Tripo),
            "maq_crate-lid_v3_tripo"
        );
        assert_eq!(
            export_stem("lamp-1", "lamp", ProviderId::Meshy),
            "maq_lamp_v1_meshy"
        );
        assert_eq!(send_number("lamp-12", "lamp"), Some(12));
        assert_eq!(send_number("lamp-x", "lamp"), None);
        assert_eq!(send_number("lamps-1", "lamp"), None);
    }

    #[test]
    fn names_read_back_with_any_extension_case_and_copy_number() {
        let want = ExportName {
            asset: "crate-lid".into(),
            version: 3,
            provider: ProviderId::Tripo,
        };
        for name in [
            "maq_crate-lid_v3_tripo.glb",
            "MAQ_Crate-Lid_V3_Tripo.GLB",
            "maq_crate-lid_v3_tripo (1).glb",
            "maq_crate-lid_v3_tripo-2.zip",
            "maq_crate-lid_v3_tripo 2.fbx",
            "/Users/me/Downloads/maq_crate-lid_v3_tripo.glb",
            "C:\\Users\\me\\Downloads\\maq_crate-lid_v3_tripo.glb",
            "maq_crate-lid_v3_tripo",
        ] {
            assert_eq!(parse(name), Some(want.clone()), "{name}");
        }
        assert_eq!(want.job_id(), "crate-lid-3");
        let meshy = parse("maq_lamp_v12_meshy.glb").expect("meshy");
        assert_eq!(meshy.provider, ProviderId::Meshy);
        assert_eq!(meshy.version, 12);
    }

    #[test]
    fn other_names_are_not_read_as_exports() {
        for name in [
            "crate-lid-3.glb",
            "model (3).glb",
            "maq_crate-lid_tripo.glb",
            "maq__v3_tripo.glb",
            "maq_crate lid_v3_tripo.glb",
            "maq_crate-lid_v3_blender.glb",
            "maq_crate-lid_v3_tripo_final.glb",
            "maq_crate-lid_vx_tripo.glb",
            "tripo_crate.zip",
        ] {
            assert_eq!(parse(name), None, "{name}");
        }
    }

    #[test]
    fn a_stem_always_parses_back_to_its_send() {
        for (job, asset, service) in [
            ("a-1", "a", ProviderId::Tripo),
            ("v2-lamp-7", "v2-lamp", ProviderId::Meshy),
            ("tripo-statue-2", "tripo-statue", ProviderId::Tripo),
        ] {
            let parsed = parse(&format!("{}.glb", export_stem(job, asset, service)))
                .unwrap_or_else(|| panic!("{job}"));
            assert_eq!(parsed.job_id(), job);
            assert_eq!(parsed.provider, service);
        }
    }
}
