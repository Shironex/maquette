//! What each provider says is left: Tripo's `GET account/balance` (credits
//! left, and those held by running tasks) and Meshy's `GET balance`. Both
//! are free calls. A service without a saved key is skipped, not called.

use serde::Serialize;

use super::http::{Api, StopFlag, Timing};
use super::jobs::Endpoints;
use super::presets::ProviderId;
use super::secrets::SecretStore;
use super::{meshy, tripo};
use crate::pipeline::history::now_ms;

/// One service's balance, or why there is none.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    pub service: ProviderId,
    /// A key is saved for this service.
    pub key: bool,
    /// Credits left to spend.
    pub credits: Option<f64>,
    /// Tripo: credits held by tasks still running.
    pub held: Option<f64>,
    pub error: Option<String>,
    pub checked_at_ms: f64,
}

/// Asks one service for its balance with `key`.
pub fn balance(endpoints: &Endpoints, service: ProviderId, key: &str, timing: Timing) -> Balance {
    let stop = StopFlag::default();
    let (base, name) = match service {
        ProviderId::Tripo => (endpoints.tripo.as_str(), tripo::SERVICE),
        ProviderId::Meshy => (endpoints.meshy.as_str(), meshy::SERVICE),
    };
    let found = Api::new(name, base, key, timing).and_then(|api| match service {
        ProviderId::Tripo => tripo::Tripo {
            api: &api,
            stop: &stop,
        }
        .balance()
        .map(|(credits, held)| (credits, Some(held))),
        ProviderId::Meshy => meshy::Meshy {
            api: &api,
            stop: &stop,
        }
        .balance()
        .map(|credits| (credits, None)),
    });
    let (credits, held, error) = match found {
        Ok((credits, held)) => (Some(credits), held, None),
        Err(err) => (None, None, Some(err.message(name))),
    };
    Balance {
        service,
        key: true,
        credits,
        held,
        error,
        checked_at_ms: now_ms(),
    }
}

/// Every service's balance, in [`ProviderId::ALL`] order.
pub fn balances(
    endpoints: &Endpoints,
    secrets: &dyn SecretStore,
    timing: impl Fn(ProviderId) -> Timing,
) -> Vec<Balance> {
    ProviderId::ALL
        .iter()
        .map(|&service| match secrets.get(service) {
            Ok(Some(key)) => balance(endpoints, service, &key, timing(service)),
            Ok(None) => Balance {
                service,
                key: false,
                credits: None,
                held: None,
                error: None,
                checked_at_ms: now_ms(),
            },
            Err(err) => Balance {
                service,
                key: false,
                credits: None,
                held: None,
                error: Some(err.to_string()),
                checked_at_ms: now_ms(),
            },
        })
        .collect()
}
