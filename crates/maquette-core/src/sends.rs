//! What the inbox and the provider sends share, kept below both so neither
//! reaches into the other: how a model gets made, the link from an inbox
//! model to the send it came from, what that send asked for, the asset a
//! model is aimed at when no send stands behind it, and where models
//! arrive and which files count as one.
//!
//! The provider jobs reach the inbox only through the ports here: the
//! inbox implements [`InboxPort`] and [`SendLinked`] (for its items), and
//! the app hands a [`SendPorts`] to the jobs where it wires them up.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

// What a send asked for is read by the inbox checks too, which sit beside
// this module, so it lives in the base crate.
pub use maquette_base::send_notes::{SendNotes, SentMaps};

use crate::assets::AssetKind;
use crate::base::ports::TextureBake;
use crate::error::CoreResult;
use crate::settings::game::FolderBudget;
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

/// True for `.glb` and `.gltf` files, but not a sync tool's temp file or
/// conflict copy of one.
pub fn is_model(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("glb") || e.eq_ignore_ascii_case("gltf"))
        && !crate::workspace::sync::is_sync_path(path)
}

/// The inbox as the provider jobs use it.
pub trait InboxPort: Send + Sync {
    /// Records the kind of the inbox model `file`, as someone picking it
    /// does.
    ///
    /// # Errors
    ///
    /// Fails for a name outside the inbox, a missing file, or when the
    /// choice cannot be saved.
    fn set_kind(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        file: &str,
        kind: AssetKind,
    ) -> CoreResult<()>;

    /// Aims the inbox model `file` at an asset, or forgets its aim with
    /// `None`.
    ///
    /// # Errors
    ///
    /// Fails for a name outside the inbox, or when the choice cannot be
    /// saved.
    fn set_aim(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        file: &str,
        aim: Option<InboxAim>,
    ) -> CoreResult<()>;

    /// The path of the inbox model `file`, read as the inbox reads one
    /// item.
    ///
    /// # Errors
    ///
    /// Fails for a name outside the inbox or a missing file.
    fn path_of(&self, root: &Path, config: &WorkspaceConfig, file: &str) -> CoreResult<String>;
}

/// An inbox model the provider jobs link to the send it came from.
pub trait SendLinked {
    /// Its file name in the inbox.
    fn inbox_name(&self) -> &str;

    /// Links it to its send.
    fn link_send(&mut self, link: InboxLink);

    /// Takes what the send record says: its kind, unless someone picked
    /// one, and the maps and texture size it asked for. `folder` is the
    /// game folder budget of the asset it was sent for, in a game project.
    fn apply_send(
        &mut self,
        kind: AssetKind,
        notes: SendNotes,
        config: &WorkspaceConfig,
        folder: Option<&FolderBudget>,
    );
}

/// What the provider jobs reach below them through: the inbox, and the
/// Blender bake that puts a split's texture back.
#[derive(Clone)]
pub struct SendPorts {
    pub inbox: Arc<dyn InboxPort>,
    pub bake: Arc<dyn TextureBake>,
}

/// Registers this module's IPC types (how a model gets made and the link
/// to its send) with the shell's TypeScript export; `maquette_core::types`
/// folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types.register::<Provider>().register::<InboxLink>()
}
