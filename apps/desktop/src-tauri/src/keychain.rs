//! The API keys in the OS keychain: macOS Keychain or Windows Credential
//! Manager, under the service `com.shironex.maquette`. Nothing else in
//! the app ever writes a key down.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use maquette_core::providers::secrets::SecretStore;
use maquette_core::{CoreError, CoreResult};

pub const SERVICE: &str = "com.shironex.maquette";

/// The keychain, once its store is set up.
#[derive(Default)]
pub struct KeychainStore {
    seen: SeenKeys,
}

/// What this run of the app learned about each key. macOS has no silent way to
/// ask whether a key exists: a look is a read, and an unsigned build can raise a
/// prompt on any read. So [`SecretStore::is_saved`] reads an account at most
/// once, and a refused read is remembered as unknown until a real use reads again.
#[derive(Default)]
struct SeenKeys(Mutex<HashMap<String, Option<bool>>>);

impl SeenKeys {
    fn map(&self) -> MutexGuard<'_, HashMap<String, Option<bool>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn note(&self, account: &str, saved: Option<bool>) {
        self.map().insert(account.to_string(), saved);
    }

    fn is_saved(&self, account: &str, read: impl FnOnce() -> Option<bool>) -> Option<bool> {
        if let Some(known) = self.map().get(account) {
            return *known;
        }
        // Not held over the read, which can wait on a prompt for a long time.
        let saved = read();
        self.note(account, saved);
        saved
    }
}

impl KeychainStore {
    /// Sets up the platform store. Fails on systems without one, where
    /// Settings then says keys cannot be saved.
    pub fn new() -> CoreResult<Self> {
        install()?;
        Ok(Self::default())
    }
}

#[cfg(target_os = "macos")]
fn install() -> CoreResult<()> {
    let store = apple_native_keyring_store::keychain::Store::new().map_err(unavailable)?;
    keyring_core::set_default_store(store);
    Ok(())
}

#[cfg(windows)]
fn install() -> CoreResult<()> {
    let store = windows_native_keyring_store::Store::new().map_err(unavailable)?;
    keyring_core::set_default_store(store);
    Ok(())
}

#[cfg(not(any(target_os = "macos", windows)))]
fn install() -> CoreResult<()> {
    Err(CoreError::Unavailable(
        "This system has no keychain Maquette can use for API keys.".to_string(),
    ))
}

/// The keychain's own error, without anything that could hold the key.
fn unavailable(error: keyring_core::Error) -> CoreError {
    let detail = match error {
        keyring_core::Error::NoStorageAccess(_) => "the keychain is locked or access was denied",
        keyring_core::Error::PlatformFailure(_) => "the keychain reported a failure",
        keyring_core::Error::BadEncoding(_) | keyring_core::Error::BadDataFormat(..) => {
            "the saved key could not be read"
        }
        _ => "the keychain refused the request",
    };
    CoreError::Unavailable(format!("Could not use the keychain: {detail}."))
}

fn entry(account: &str) -> CoreResult<keyring_core::Entry> {
    keyring_core::Entry::new(SERVICE, account).map_err(unavailable)
}

fn read(account: &str) -> CoreResult<Option<String>> {
    match entry(account)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(err) => Err(unavailable(err)),
    }
}

impl SecretStore for KeychainStore {
    fn get_account(&self, account: &str) -> CoreResult<Option<String>> {
        let found = read(account);
        self.seen
            .note(account, found.as_ref().ok().map(Option::is_some));
        found
    }

    fn set_account(&self, account: &str, key: &str) -> CoreResult<()> {
        entry(account)?.set_password(key).map_err(unavailable)?;
        self.seen.note(account, Some(true));
        Ok(())
    }

    fn remove_account(&self, account: &str) -> CoreResult<()> {
        match entry(account)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => {
                self.seen.note(account, Some(false));
                Ok(())
            }
            Err(err) => Err(unavailable(err)),
        }
    }

    fn is_saved(&self, account: &str) -> Option<bool> {
        self.seen
            .is_saved(account, || read(account).ok().map(|key| key.is_some()))
    }
}

/// Used when the keychain cannot be opened: every call explains why.
pub struct NoKeychain(pub String);

impl SecretStore for NoKeychain {
    fn get_account(&self, _account: &str) -> CoreResult<Option<String>> {
        Ok(None)
    }

    fn set_account(&self, _account: &str, _key: &str) -> CoreResult<()> {
        Err(CoreError::Unavailable(self.0.clone()))
    }

    fn remove_account(&self, _account: &str) -> CoreResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn asking_again_reads_the_keychain_only_once() {
        let seen = SeenKeys::default();
        let reads = Cell::new(0);
        let read = |answer| {
            reads.set(reads.get() + 1);
            answer
        };
        for _ in 0..5 {
            assert_eq!(seen.is_saved("tripo", || read(Some(true))), Some(true));
        }
        assert_eq!(reads.get(), 1);

        // A refused read stays unknown and does not ask again either.
        for _ in 0..5 {
            assert_eq!(seen.is_saved("meshy", || read(None)), None);
        }
        assert_eq!(reads.get(), 2);

        // A real use updates what it said.
        seen.note("meshy", Some(false));
        assert_eq!(seen.is_saved("meshy", || read(None)), Some(false));
        assert_eq!(reads.get(), 2);
    }
}
