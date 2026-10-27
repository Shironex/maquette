//! The app's own updates, through `tauri-plugin-updater`, driven from here
//! rather than from the web view: the web view calls the `updater_*`
//! commands and follows the `UpdaterUpdate` event, and has no permission to
//! call the plugin itself.
//!
//! The updater stays off unless every gate is open, and it never fails a
//! start. [`mode`] decides once at launch:
//!
//! - no public key in `plugins.updater.pubkey` (the case until the first
//!   public release): [`UpdaterMode::Unreleased`];
//! - a debug build (`pnpm tauri dev`): [`UpdaterMode::DevBuild`];
//! - `MAQUETTE_NO_UPDATER=1` (tests, scripted runs): [`UpdaterMode::TurnedOff`].
//!
//! Only [`UpdaterMode::On`] registers the plugin and schedules checks; every
//! other mode answers the commands with "off" and never touches the network.

mod service;
#[cfg(test)]
mod tests;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;

pub use service::Updates;
pub(crate) use service::schedule;

/// The variable that turns the updater off for one run.
pub const NO_UPDATER_ENV: &str = "MAQUETTE_NO_UPDATER";

/// The first check waits this long after launch, so it never competes with
/// the start.
pub const FIRST_CHECK: Duration = Duration::from_secs(10);

/// Then a check every six hours: sessions are long, and hourly is noise.
pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// Whether this build updates itself, and why not when it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum UpdaterMode {
    /// Checks run, and an update can be downloaded and installed.
    On,
    /// No update key is configured yet: updates start with the first
    /// public release.
    Unreleased,
    /// A development build, which never replaces itself.
    DevBuild,
    /// Turned off for this run with `MAQUETTE_NO_UPDATER`.
    TurnedOff,
}

impl UpdaterMode {
    pub fn is_on(self) -> bool {
        self == Self::On
    }
}

/// Decides the mode from the configured key, the build and the environment.
pub fn mode(pubkey: &str, debug_build: bool, turned_off: bool) -> UpdaterMode {
    if pubkey.trim().is_empty() {
        UpdaterMode::Unreleased
    } else if debug_build {
        UpdaterMode::DevBuild
    } else if turned_off {
        UpdaterMode::TurnedOff
    } else {
        UpdaterMode::On
    }
}

/// True when `MAQUETTE_NO_UPDATER` holds anything but empty, `0` or `false`.
pub fn turned_off_by(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        let v = v.trim();
        !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false")
    })
}

/// What `plugins.updater` in `tauri.conf.json` says: the key and the first
/// endpoint. Missing parts read as empty, never as an error.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedConfig {
    pub pubkey: String,
    pub endpoint: Option<String>,
}

impl FeedConfig {
    pub fn from_plugin_config(updater: Option<&serde_json::Value>) -> Self {
        let Some(updater) = updater else {
            return Self::default();
        };
        Self {
            pubkey: updater
                .get("pubkey")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            endpoint: updater
                .get("endpoints")
                .and_then(serde_json::Value::as_array)
                .and_then(|list| list.first())
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
        }
    }
}

/// The mode of this process, from the app's config.
pub fn mode_for(feed: &FeedConfig) -> UpdaterMode {
    let env = std::env::var(NO_UPDATER_ENV).ok();
    mode(
        &feed.pubkey,
        cfg!(debug_assertions),
        turned_off_by(env.as_deref()),
    )
}

/// Whether a failed check only means that no release is out for this
/// machine yet, rather than that something is broken.
///
/// That is the plugin's answer while the feed is missing (the repo is
/// private, or no release was published: GitHub answers 404 and the plugin
/// says it found no valid release JSON) and while a new release lists other
/// platforms but not this one yet (the assets are still uploading). Neither
/// is worth an error in the UI: the app simply has nothing to offer.
pub fn is_release_pending(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("could not fetch a valid release json")
        || message.contains("in the response `platforms` object")
        || (message.contains("404") && message.contains("latest.json"))
}

/// One step of an update, as the web view hears it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UpdaterEvent {
    /// A check started.
    Checking,
    /// A newer version is out.
    Available { update: UpdateInfo },
    /// This is the newest version.
    Current,
    /// No release is out for this machine yet (see [`is_release_pending`]).
    Pending,
    /// Bytes of the update arrived. `total` is unknown when the server did
    /// not say.
    Progress { downloaded: f64, total: Option<f64> },
    /// The update is downloaded and checked, ready to install.
    Ready { update: UpdateInfo },
    /// A check, download or install failed.
    Failed { message: String },
}

impl UpdaterEvent {
    /// A failure, or [`UpdaterEvent::Pending`] when it only means no release
    /// is out yet.
    pub fn failed(message: impl Into<String>) -> Self {
        let message = message.into();
        if is_release_pending(&message) {
            Self::Pending
        } else {
            Self::Failed { message }
        }
    }
}

/// A release the feed offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    /// The release notes the feed carries, as written.
    pub notes: Option<String>,
    /// The day it was published, `YYYY-MM-DD`.
    pub date: Option<String>,
}

/// What the About section shows about updates before any check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterStatus {
    pub mode: UpdaterMode,
    /// The feed address, when one is configured.
    pub endpoint: Option<String>,
    /// The update found by the last check, if any.
    pub available: Option<UpdateInfo>,
    /// True once an update is downloaded and waits for a restart.
    pub ready: bool,
}
