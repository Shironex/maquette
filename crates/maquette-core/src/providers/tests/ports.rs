//! The jobs reach the inbox and the bake back only through the ports the
//! app hands in: fakes here stand in for both and record what they were
//! asked.

use std::path::Path;

use super::*;
use crate::base::ports::TextureBake;
use crate::error::CoreResult;
use crate::pipeline::tools::Toolbox;
use crate::sends::{InboxAim, InboxLink, InboxPort, SendLinked, SendNotes, SendPorts};
use crate::settings::game::FolderBudget;

/// An inbox that only writes down the kinds it was told.
#[derive(Default)]
struct Recorded {
    kinds: Mutex<Vec<(String, AssetKind)>>,
}

impl InboxPort for Recorded {
    fn set_kind(
        &self,
        _root: &Path,
        _config: &WorkspaceConfig,
        file: &str,
        kind: AssetKind,
    ) -> CoreResult<()> {
        self.kinds
            .lock()
            .expect("kinds")
            .push((file.to_string(), kind));
        Ok(())
    }

    fn set_aim(
        &self,
        _root: &Path,
        _config: &WorkspaceConfig,
        _file: &str,
        _aim: Option<InboxAim>,
    ) -> CoreResult<()> {
        Ok(())
    }

    fn path_of(&self, _root: &Path, _config: &WorkspaceConfig, file: &str) -> CoreResult<String> {
        Err(crate::error::CoreError::NotFound(format!(
            "{file} in the inbox"
        )))
    }
}

/// A bake no test here reaches.
struct NoBake;

impl TextureBake for NoBake {
    fn bake(
        &self,
        _dir: &Path,
        _source: &Path,
        _target: &Path,
        _tools: &Toolbox,
    ) -> CoreResult<PathBuf> {
        panic!("nothing here bakes");
    }
}

/// An inbox model as plain data.
struct Model {
    name: String,
    link: Option<InboxLink>,
    kind: Option<AssetKind>,
}

impl SendLinked for Model {
    fn inbox_name(&self) -> &str {
        &self.name
    }

    fn link_send(&mut self, link: InboxLink) {
        self.link = Some(link);
    }

    fn apply_send(
        &mut self,
        kind: AssetKind,
        _notes: SendNotes,
        _config: &WorkspaceConfig,
        _folder: Option<&FolderBudget>,
    ) {
        self.kind = Some(kind);
    }
}

#[test]
fn a_character_handed_back_goes_through_the_inbox_port_and_links_its_model() {
    let world = World::new();
    let inbox = Arc::new(Recorded::default());
    let ports = SendPorts {
        inbox: Arc::clone(&inbox) as Arc<dyn InboxPort>,
        bake: Arc::new(NoBake),
    };
    let jobs = ProviderJobs::new(
        Arc::clone(&world.secrets) as Arc<dyn SecretStore>,
        Options::default(),
        ports,
        |_| {},
    );
    jobs.open_workspace(&world.root, &world.config);
    let mut request = world.request(Provider::ByHand, PresetVariant::Character, &["front.png"]);
    request.service = ProviderId::Meshy;
    let job = jobs.send(&request, true, false).expect("send");

    let made = world.root.join("Downloaded model.glb");
    fs::write(&made, fixture::character(false)).expect("write");
    let back = jobs.hand_back(&job.id, &made).expect("hand back");
    let file = back.inbox_file.expect("in the inbox");
    assert_eq!(
        *inbox.kinds.lock().expect("kinds"),
        [(file.clone(), AssetKind::Character)]
    );

    let mut models = [Model {
        name: file,
        link: None,
        kind: None,
    }];
    jobs.link_inbox(&mut models);
    assert_eq!(
        models[0].link.as_ref().map(|l| l.job.as_str()),
        Some(job.id.as_str())
    );
    assert_eq!(models[0].kind, Some(AssetKind::Character));
}
