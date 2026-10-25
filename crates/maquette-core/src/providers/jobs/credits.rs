//! Credits: what a send should cost before it goes, the ledger of what
//! sends spent and the balance each provider reports.

use super::{BatchRequest, ProviderJobs, SendRequest};
use crate::error::{CoreError, CoreResult};
use crate::providers::balance::{self, Balance};
use crate::providers::ledger::{self, Ledger};
use crate::providers::presets::{ProviderId, ProviderSettings};
use crate::providers::pricing;

/// The expected credits of one send, on its own settings or the saved preset.
fn send_estimate(providers: &ProviderSettings, request: &SendRequest) -> Option<f64> {
    let service = request.provider.service()?;
    let tripo = (service == ProviderId::Tripo).then(|| {
        request
            .tripo
            .clone()
            .unwrap_or_else(|| providers.tripo(request.variant).clone())
    });
    let meshy = (service == ProviderId::Meshy).then(|| {
        request
            .meshy
            .clone()
            .unwrap_or_else(|| providers.meshy(request.variant).clone())
    });
    pricing::estimate(
        request.provider,
        request.variant,
        tripo.as_ref(),
        meshy.as_ref(),
    )
}

impl ProviderJobs {
    /// What a send should cost, from the price lists; `None` for By hand.
    /// Checks nothing else, so the panel can show it while it is edited.
    pub fn estimate(&self, request: &SendRequest) -> CoreResult<Option<f64>> {
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        Ok(send_estimate(&workspace.config.providers, request))
    }

    /// What a whole batch should cost; `None` for By hand.
    pub fn estimate_batch(&self, request: &BatchRequest) -> CoreResult<Option<f64>> {
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        let providers = &workspace.config.providers;
        let each: Vec<Option<f64>> = request
            .items
            .iter()
            .map(|item| send_estimate(providers, &request.send_request(item)))
            .collect();
        Ok(each.iter().copied().sum())
    }

    /// The open workspace's credits ledger.
    pub fn ledger(&self) -> CoreResult<Ledger> {
        let root = self
            .root()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        ledger::read(&root)
    }

    /// Every provider's balance, asked over the network with the saved
    /// keys. Call it off the UI thread.
    pub fn balances(&self) -> Vec<Balance> {
        let options = self.options();
        balance::balances(
            &options.endpoints,
            self.secrets().as_ref(),
            |service| match service {
                ProviderId::Tripo => options.tripo,
                ProviderId::Meshy => options.meshy,
            },
        )
    }
}
