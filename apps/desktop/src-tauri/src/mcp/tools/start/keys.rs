use maquette_core::providers::{ProviderId, secrets};

use crate::mcp::host::Host;

/// Whether each key is saved. None when the store could not say, which is not
/// the same as no key.
pub(super) struct Keys {
    pub(super) tripo: Option<bool>,
    pub(super) meshy: Option<bool>,
    pub(super) openai: Option<bool>,
}

impl Keys {
    /// A paid 3D provider's key is known to be saved. A store that could not
    /// say is not a yes: `next` would offer providers that then fail.
    pub(super) fn any_saved(&self) -> bool {
        self.tripo == Some(true) || self.meshy == Some(true)
    }

    /// The store could not say for at least one key (null above): `next` does
    /// not offer paid providers on it.
    pub(super) fn unreadable(&self) -> bool {
        [self.tripo, self.meshy, self.openai].contains(&None)
    }
}

/// Reads through [`SecretStore::is_saved`], so asking again and again never
/// raises a Keychain prompt.
pub(super) fn read_keys(host: &dyn Host) -> Keys {
    let store = host.studio().secrets();
    Keys {
        tripo: store.is_saved(secrets::account(ProviderId::Tripo)),
        meshy: store.is_saved(secrets::account(ProviderId::Meshy)),
        openai: store.is_saved(secrets::OPENAI_ACCOUNT),
    }
}
