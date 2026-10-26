//! What made an asset's model version, for the library's "Made with".

use std::path::Path;

use super::LibrarySource;
use crate::assets::Asset;
use crate::providers::{Provider, ProviderJob};

/// What made version `version` of `asset` and the credits it cost: the
/// send that became that version (Tripo, Meshy or By hand), else a built
/// asset's script, else an imported file. `jobs` are the project's sends.
pub fn detect(
    project_dir: &Path,
    asset: &Asset,
    version: u32,
    jobs: &[ProviderJob],
) -> (LibrarySource, Option<f64>) {
    let sent = jobs
        .iter()
        .find(|j| j.asset == asset.id && j.version == Some(version));
    if let Some(job) = sent {
        let source = match job.provider {
            Provider::Tripo => LibrarySource::Tripo,
            Provider::Meshy => LibrarySource::Meshy,
            Provider::ByHand => LibrarySource::ByHand,
        };
        return (source, job.credits.or(job.estimate));
    }
    let built = crate::built::load(project_dir)
        .map(|file| file.assets.contains_key(&asset.id))
        .unwrap_or(false);
    if built {
        return (LibrarySource::Built, None);
    }
    (LibrarySource::Imported, None)
}
