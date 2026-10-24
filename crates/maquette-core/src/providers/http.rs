//! The blocking HTTP client the provider calls share.

use std::sync::OnceLock;

/// reqwest is built without a bundled TLS backend choice, so the process
/// picks one: ring, which builds the same on macOS and Windows. Safe to
/// call any number of times; the first call wins.
pub fn ensure_crypto() {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        // Err means another provider is already installed, which is fine.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
