//! The API keys in the OS keychain: macOS Keychain or Windows Credential
//! Manager, under the service `com.shironex.maquette`. Nothing else in
//! the app ever writes a key down.

use maquette_core::providers::secrets::SecretStore;
use maquette_core::{CoreError, CoreResult};

pub const SERVICE: &str = "com.shironex.maquette";

/// The keychain, once its store is set up.
pub struct KeychainStore;

impl KeychainStore {
    /// Sets up the platform store. Fails on systems without one, where
    /// Settings then says keys cannot be saved.
    pub fn new() -> CoreResult<Self> {
        install()?;
        Ok(Self)
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

impl SecretStore for KeychainStore {
    fn get_account(&self, account: &str) -> CoreResult<Option<String>> {
        match entry(account)?.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(err) => Err(unavailable(err)),
        }
    }

    fn set_account(&self, account: &str, key: &str) -> CoreResult<()> {
        entry(account)?.set_password(key).map_err(unavailable)
    }

    fn remove_account(&self, account: &str) -> CoreResult<()> {
        match entry(account)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(err) => Err(unavailable(err)),
        }
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
