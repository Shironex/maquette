//! Where the API keys live. The shell gives the core the OS keychain
//! (macOS Keychain, Windows Credential Manager); tests use a store in
//! memory. A key never goes into a config file, a log or an event: the UI
//! only ever sees its last four characters.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

use super::presets::ProviderId;
use crate::error::{CoreError, CoreResult};

/// Keys by keychain account: the 3D services' under [`account`], other
/// keys (OpenAI's, [`OPENAI_ACCOUNT`]) under their own account name.
pub trait SecretStore: Send + Sync {
    fn get_account(&self, account: &str) -> CoreResult<Option<String>>;
    fn set_account(&self, account: &str, key: &str) -> CoreResult<()>;
    fn remove_account(&self, account: &str) -> CoreResult<()>;
    /// Whether a key is saved, or None when the store could not say. Meant
    /// for asking often: a store whose reads can prompt answers from memory.
    fn is_saved(&self, account: &str) -> Option<bool> {
        self.get_account(account).ok().map(|key| key.is_some())
    }

    fn get(&self, service: ProviderId) -> CoreResult<Option<String>> {
        self.get_account(account(service))
    }
    fn set(&self, service: ProviderId, key: &str) -> CoreResult<()> {
        self.set_account(account(service), key)
    }
    fn remove(&self, service: ProviderId) -> CoreResult<()> {
        self.remove_account(account(service))
    }
}

/// Keys in memory, for tests and mock runs.
#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl MemoryStore {
    fn keys(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl SecretStore for MemoryStore {
    fn get_account(&self, account: &str) -> CoreResult<Option<String>> {
        Ok(self.keys().get(account).cloned())
    }

    fn set_account(&self, account: &str, key: &str) -> CoreResult<()> {
        self.keys().insert(account.to_string(), key.to_string());
        Ok(())
    }

    fn remove_account(&self, account: &str) -> CoreResult<()> {
        self.keys().remove(account);
        Ok(())
    }
}

/// The keychain account name for a service's key.
pub fn account(service: ProviderId) -> &'static str {
    match service {
        ProviderId::Tripo => "tripo-api-key",
        ProviderId::Meshy => "meshy-api-key",
    }
}

/// The keychain account of the OpenAI API key the Lab's image API uses.
pub const OPENAI_ACCOUNT: &str = "openai-api-key";

/// Trims a pasted key and refuses anything that cannot be one.
pub fn clean_key(service: ProviderId, key: &str) -> CoreResult<String> {
    clean_named_key(service.label(), key)
}

/// [`clean_key`] for a key named `label` ("OpenAI").
pub fn clean_named_key(label: &str, key: &str) -> CoreResult<String> {
    let key = key.trim();
    if key.is_empty() {
        return Err(CoreError::Invalid(format!(
            "Paste a {label} API key first."
        )));
    }
    if key.len() > 512 || key.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(CoreError::Invalid(format!(
            "That does not look like a {label} API key."
        )));
    }
    Ok(key.to_string())
}

/// The last four characters, which is all the UI shows.
pub fn tail(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    let start = chars.len().saturating_sub(4);
    chars[start..].iter().collect()
}

/// What Settings shows for one key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatus {
    pub service: ProviderId,
    pub saved: bool,
    /// The last four characters of the saved key.
    pub tail: Option<String>,
}

pub fn status(store: &dyn SecretStore, service: ProviderId) -> CoreResult<KeyStatus> {
    let key = store.get(service)?;
    Ok(KeyStatus {
        service,
        saved: key.is_some(),
        tail: key.as_deref().map(tail),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_cleaned_and_only_their_tail_shows() {
        let store = MemoryStore::default();
        assert!(clean_key(ProviderId::Tripo, "  ").is_err());
        assert!(clean_key(ProviderId::Tripo, "tsk abc").is_err());
        let key = clean_key(ProviderId::Tripo, " tsk_0123456789abcd \n").expect("clean");
        store.set(ProviderId::Tripo, &key).expect("set");
        let shown = status(&store, ProviderId::Tripo).expect("status");
        assert_eq!(shown.tail.as_deref(), Some("abcd"));
        assert!(
            !serde_json::to_string(&shown)
                .expect("json")
                .contains("0123")
        );
        assert!(!status(&store, ProviderId::Meshy).expect("status").saved);
        store.remove(ProviderId::Tripo).expect("remove");
        assert!(!status(&store, ProviderId::Tripo).expect("status").saved);
        assert_eq!(tail("ab"), "ab");
    }
}
