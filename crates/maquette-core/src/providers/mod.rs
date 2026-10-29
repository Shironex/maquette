//! Sending a picked concept or ref to a 3D service: Tripo and Meshy over
//! their REST APIs, or by hand through a handoff folder.

pub mod arrivals;
pub mod balance;
pub mod catalog;
pub mod handback;
pub mod handoff;
pub mod http;
pub mod jobs;
pub mod ledger;
pub mod meshy;
pub mod naming;
pub mod openai_images;
pub mod own_file;
pub mod presets;
pub mod pricing;
pub mod secrets;
pub mod split;
pub mod subscription;
pub mod tripo;

#[cfg(test)]
pub(crate) mod fake;
#[cfg(test)]
mod tests;

use serde::Serialize;

pub use arrivals::{Arrival, ArrivalState, Arrivals, PendingHandoff, WatchStatus, WatchTiming};
pub use balance::Balance;
pub use catalog::{ModelOption, ProviderCatalog};
pub use handback::{DownloadMatch, DownloadsScan};
pub use handoff::options::{StudioChoice, StudioMesh, StudioRig, StudioSettings};
pub use handoff::{CardRow, HandoffCard};
pub use jobs::{
    BakeBack, BakeState, BatchFailure, BatchItem, BatchRequest, BatchSent, CancelMode, Endpoints,
    InboxLink, JobPhase, JobState, MAX_BATCH, Options, Provider, ProviderEvent, ProviderJob,
    ProviderJobs, SendRequest, SplitJob, SplitRequest, SplitSource,
};
pub use ledger::{Ledger, ServiceTotals, Spend, SpendOutcome, SpendSource};
pub use presets::{
    MeshyPreset, MeshyPresets, MeshyTexture, PoseMode, PresetVariant, ProviderId, ProviderSettings,
    TripoGeometryQuality, TripoPreset, TripoPresets, TripoRigSpec, TripoTextureQuality,
    meshy_problem, tripo_problem,
};
pub use secrets::{KeyStatus, MemoryStore, SecretStore};

use http::{Api, StopFlag, Timing};

/// What Test key found.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KeyTest {
    pub service: ProviderId,
    pub ok: bool,
    /// Credits left, when the key works.
    pub balance: Option<f64>,
    pub message: String,
}

/// Checks a key with the cheapest call there is: both services have a
/// free balance endpoint.
pub fn test_key(endpoints: &Endpoints, service: ProviderId, key: &str, timing: Timing) -> KeyTest {
    let stop = StopFlag::default();
    let (base, name) = match service {
        ProviderId::Tripo => (endpoints.tripo.as_str(), tripo::SERVICE),
        ProviderId::Meshy => (endpoints.meshy.as_str(), meshy::SERVICE),
    };
    let balance = Api::new(name, base, key, timing).and_then(|api| match service {
        ProviderId::Tripo => tripo::Tripo {
            api: &api,
            stop: &stop,
        }
        .balance()
        .map(|(free, _)| free),
        ProviderId::Meshy => meshy::Meshy {
            api: &api,
            stop: &stop,
        }
        .balance(),
    });
    match balance {
        Ok(balance) => KeyTest {
            service,
            ok: true,
            balance: Some(balance),
            message: format!("The key works. {} credits left.", format_credits(balance)),
        },
        Err(error) => KeyTest {
            service,
            ok: false,
            balance: None,
            message: error.message(name),
        },
    }
}

/// 1234.5 becomes "1,234.5"; whole numbers lose the decimals.
pub fn format_credits(credits: f64) -> String {
    let rounded = (credits * 100.0).round() / 100.0;
    let whole = rounded.trunc() as u64;
    let frac = rounded - rounded.trunc();
    let grouped = presets::group(u32::try_from(whole).unwrap_or(u32::MAX));
    if frac.abs() < 0.005 {
        grouped
    } else {
        format!("{grouped}{}", &format!("{frac:.2}")[1..])
    }
}
