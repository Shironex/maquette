use maquette_core::providers::{ProviderId, secrets};
use serde_json::{Value, json};

use crate::mcp::host::Host;

/// Whether each key is saved. None when the store could not say, which is not
/// the same as no key.
pub(super) struct Keys {
    pub(super) tripo: Option<bool>,
    pub(super) meshy: Option<bool>,
    pub(super) openai: Option<bool>,
    /// A look still waits on a Keychain prompt, so the nulls mean "answer it".
    pub(super) prompt: bool,
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

    /// The `keys` of get_started's answer, with a note while a Keychain
    /// prompt waits for a person.
    pub(super) fn value(&self) -> Value {
        let mut out = json!({ "tripo": self.tripo, "meshy": self.meshy, "openai": self.openai,
                              "unreadable": self.unreadable() });
        if self.prompt && self.unreadable() {
            out["note"] = json!(PROMPT_NOTE);
        }
        out
    }
}

/// What get_started says while a key read waits on a Keychain prompt.
pub(super) const PROMPT_NOTE: &str = "A Keychain prompt from Maquette is waiting on this computer, so the keys read as unknown (null). Ask the person to answer it (Always Allow keeps it from coming back), then call get_started again.";

/// Reads through [`SecretStore::is_saved`], so asking again and again never
/// raises a Keychain prompt, and a read waiting on one is given up on after a
/// few seconds (null) instead of holding the answer.
pub(super) fn read_keys(host: &dyn Host) -> Keys {
    let store = host.studio().secrets();
    Keys {
        tripo: store.is_saved(secrets::account(ProviderId::Tripo)),
        meshy: store.is_saved(secrets::account(ProviderId::Meshy)),
        openai: store.is_saved(secrets::OPENAI_ACCOUNT),
        prompt: store.prompt_waiting(),
    }
}
