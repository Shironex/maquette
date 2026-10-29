//! The inbox behind the provider jobs' ports: the inbox itself answers the
//! inbox port, its items take the links to their sends, and
//! [`send_ports`] hands the jobs both ports with the Blender bake.

use std::path::Path;
use std::sync::Arc;

use super::{InboxAim, InboxItem};
use crate::assets::AssetKind;
use crate::error::CoreResult;
use crate::game::FolderBudget;
use crate::parts::bake_back::Bake;
use crate::sends::{InboxLink, InboxPort, SendLinked, SendNotes, SendPorts};
use crate::workspace::WorkspaceConfig;

/// The workspace's inbox, as the provider jobs reach it.
pub struct Inbox;

impl InboxPort for Inbox {
    fn set_kind(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        file: &str,
        kind: AssetKind,
    ) -> CoreResult<()> {
        super::set_kind(root, config, file, kind).map(|_| ())
    }

    fn set_aim(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        file: &str,
        aim: Option<InboxAim>,
    ) -> CoreResult<()> {
        super::set_aim(root, config, file, aim)
    }

    fn path_of(&self, root: &Path, config: &WorkspaceConfig, file: &str) -> CoreResult<String> {
        super::item(root, config, file).map(|item| item.file)
    }
}

impl SendLinked for InboxItem {
    fn inbox_name(&self) -> &str {
        &self.id
    }

    fn link_send(&mut self, link: InboxLink) {
        self.link = Some(link);
    }

    fn apply_send(
        &mut self,
        kind: AssetKind,
        notes: SendNotes,
        config: &WorkspaceConfig,
        folder: Option<&FolderBudget>,
    ) {
        super::apply_send(self, kind, notes, config, folder);
    }
}

/// The ports the provider jobs are built with: this inbox and the Blender
/// bake back.
pub fn send_ports() -> SendPorts {
    SendPorts {
        inbox: Arc::new(Inbox),
        bake: Arc::new(Bake),
    }
}
