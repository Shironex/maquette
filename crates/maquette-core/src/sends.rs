//! What the inbox and the provider sends share, kept below both so neither
//! reaches into the other: how a model gets made, the link from an inbox
//! model to the send it came from, what that send asked for, the asset a
//! model is aimed at when no send stands behind it, and where models
//! arrive.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// What a send asked for is read by the inbox checks too, which sit beside
// this module, so it lives in the base crate.
pub use maquette_base::send_notes::{SendNotes, SentMaps};

use crate::settings::presets::ProviderId;
use crate::workspace::WorkspaceConfig;

/// How a model gets made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Tripo,
    Meshy,
    ByHand,
}

impl Provider {
    pub fn service(self) -> Option<ProviderId> {
        match self {
            Self::Tripo => Some(ProviderId::Tripo),
            Self::Meshy => Some(ProviderId::Meshy),
            Self::ByHand => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Tripo => "Tripo",
            Self::Meshy => "Meshy",
            Self::ByHand => "By hand",
        }
    }
}

/// An inbox model that came from a send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InboxLink {
    pub job: String,
    pub project: String,
    pub asset: String,
    pub asset_name: String,
    pub provider: Provider,
    /// The send it redoes, when it came from a Redo.
    #[serde(default)]
    pub redo_of: Option<String>,
}

/// The asset an inbox model goes to by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InboxAim {
    pub project: String,
    pub asset: String,
    pub asset_name: String,
}

/// The workspace's inbox folder, where models arrive.
pub fn inbox_dir(root: &Path, config: &WorkspaceConfig) -> PathBuf {
    root.join(&config.layout.inbox)
}

/// Registers this module's IPC types (how a model gets made and the link
/// to its send) with the shell's TypeScript export; `maquette_core::types`
/// folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types.register::<Provider>().register::<InboxLink>()
}
