//! The API keys in the OS keychain: macOS Keychain or Windows Credential
//! Manager, under the service `com.shironex.maquette`. Nothing else in
//! the app ever writes a key down.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::time::Duration;

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
///
/// A read waiting on a Keychain prompt blocks until a person answers it, so it
/// runs on its own thread and is given up on after a few seconds: the answer is
/// then unknown, and while that read still waits no other read starts, so
/// asking again never piles up threads or prompts. Once the person answers,
/// the read notes what it found.
#[derive(Default)]
struct SeenKeys(Arc<Mutex<Seen>>);

#[derive(Default)]
struct Seen {
    known: HashMap<String, Option<bool>>,
    /// The account a read given up on still waits for.
    pending: Option<String>,
}

/// How long a look at a key may wait before it reads as unknown.
const READ_WAIT: Duration = Duration::from_secs(3);

impl SeenKeys {
    fn seen(&self) -> MutexGuard<'_, Seen> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn note(&self, account: &str, saved: Option<bool>) {
        self.seen().known.insert(account.to_string(), saved);
    }

    /// A read still waits, most likely on a prompt only a person can answer.
    fn waiting(&self) -> bool {
        self.seen().pending.is_some()
    }

    fn is_saved(
        &self,
        account: &str,
        wait: Duration,
        read: impl FnOnce() -> Option<bool> + Send + 'static,
    ) -> Option<bool> {
        {
            let mut seen = self.seen();
            if let Some(known) = seen.known.get(account) {
                return *known;
            }
            if seen.pending.is_some() {
                return None;
            }
            seen.pending = Some(account.to_string());
        }
        // Not held over the read, which can wait on a prompt for a long time.
        let (tx, rx) = mpsc::channel();
        let shared = Arc::clone(&self.0);
        let owner = account.to_string();
        std::thread::spawn(move || {
            let saved = read();
            let mut seen = shared.lock().unwrap_or_else(PoisonError::into_inner);
            seen.pending = None;
            // A real use that answered meanwhile knows better.
            seen.known.entry(owner).or_insert(saved);
            drop(seen);
            let _ = tx.send(saved);
        });
        rx.recv_timeout(wait).ok().flatten()
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
        let owned = account.to_string();
        self.seen.is_saved(account, READ_WAIT, move || {
            read(&owned).ok().map(|key| key.is_some())
        })
    }

    fn prompt_waiting(&self) -> bool {
        self.seen.waiting()
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    const WAIT: Duration = Duration::from_secs(5);

    #[test]
    fn asking_again_reads_the_keychain_only_once() {
        let seen = SeenKeys::default();
        let reads = Arc::new(AtomicUsize::new(0));
        let read = |answer: Option<bool>| {
            let reads = Arc::clone(&reads);
            move || {
                reads.fetch_add(1, Ordering::SeqCst);
                answer
            }
        };
        for _ in 0..5 {
            assert_eq!(seen.is_saved("tripo", WAIT, read(Some(true))), Some(true));
        }
        assert_eq!(reads.load(Ordering::SeqCst), 1);

        // A refused read stays unknown and does not ask again either.
        for _ in 0..5 {
            assert_eq!(seen.is_saved("meshy", WAIT, read(None)), None);
        }
        assert_eq!(reads.load(Ordering::SeqCst), 2);

        // A real use updates what it said.
        seen.note("meshy", Some(false));
        assert_eq!(seen.is_saved("meshy", WAIT, read(None)), Some(false));
        assert_eq!(reads.load(Ordering::SeqCst), 2);
        assert!(!seen.waiting());
    }

    #[test]
    fn a_read_waiting_on_a_prompt_reads_as_unknown_and_starts_no_other() {
        let seen = SeenKeys::default();
        let reads = Arc::new(AtomicUsize::new(0));
        let (answer, prompt) = mpsc::channel::<bool>();
        let counted = Arc::clone(&reads);
        let blocked = move || {
            counted.fetch_add(1, Ordering::SeqCst);
            prompt.recv().ok()
        };
        let short = Duration::from_millis(50);
        assert_eq!(seen.is_saved("tripo", short, blocked), None, "gave up");
        assert!(seen.waiting());
        // Asking again, for that key or another, starts no read while it waits.
        for account in ["tripo", "meshy", "openai"] {
            let counted = Arc::clone(&reads);
            let again = move || {
                counted.fetch_add(1, Ordering::SeqCst);
                Some(true)
            };
            assert_eq!(seen.is_saved(account, short, again), None, "{account}");
        }
        assert_eq!(reads.load(Ordering::SeqCst), 1);

        // The person answers: the read notes what it found.
        answer.send(true).expect("answer");
        let deadline = std::time::Instant::now() + WAIT;
        while seen.waiting() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!seen.waiting());
        assert_eq!(seen.is_saved("tripo", short, || None), Some(true));
        assert_eq!(reads.load(Ordering::SeqCst), 1);
    }
}
