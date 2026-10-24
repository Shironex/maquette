//! Where the API keys live. The shell gives the core the OS keychain
//! (macOS Keychain, Windows Credential Manager); tests use a store in
//! memory. A key never goes into a config file, a log or an event: the UI
//! only ever sees its last four characters.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

use super::presets::ProviderId;
use crate::error::{CoreError, CoreResult};

pub trait SecretStore: Send + Sync {
    fn get(&self, service: ProviderId) -> CoreResult<Option<String>>;
    fn set(&self, service: ProviderId, key: &str) -> CoreResult<()>;
    fn remove(&self, service: ProviderId) -> CoreResult<()>;
}

/// Keys in memory, for tests and mock runs.
#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<ProviderId, String>>);

impl MemoryStore {
    fn keys(&self) -> std::sync::MutexGuard<'_, HashMap<ProviderId, String>> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, service: ProviderId) -> CoreResult<Option<String>> {
        Ok(self.keys().get(&service).cloned())
    }

    fn set(&self, service: ProviderId, key: &str) -> CoreResult<()> {
        self.keys().insert(service, key.to_string());
        Ok(())
    }

    fn remove(&self, service: ProviderId) -> CoreResult<()> {
        self.keys().remove(&service);
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

/// Trims a pasted key and refuses anything that cannot be one.
pub fn clean_key(service: ProviderId, key: &str) -> CoreResult<String> {
    let key = key.trim();
    if key.is_empty() {
        return Err(CoreError::Invalid(format!(
            "Paste a {} API key first.",
            service.label()
        )));
    }
    if key.len() > 512 || key.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(CoreError::Invalid(format!(
            "That does not look like a {} API key.",
            service.label()
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
