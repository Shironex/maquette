//! The pack service: runs each pack on a thread of its own, one model at a
//! time, keeps the running packs' records in memory (written to disk after
//! every change) and tells the app through a callback. A person may decide
//! on a model, stop, zip what passes or go on from where a pack stopped
//! while other models still run.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::decide::{PackDecision, apply, settle_closed, stopped_at};
use super::finish::{ZipChoice, finish};
use super::plan::PackPlan;
use super::record::{self, MARKER, PackItem, PackItemState, PackRecord, PackState, RECORD_VERSION};
use super::run::{run_pack, settle};
use super::{PackRequest, presets};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::CompressDefaults;
use crate::pipeline::runner::PipelineRunner;
use crate::pipeline::tools::Toolbox;
use crate::project::now_rfc3339;
use crate::toolchain::ToolId;

/// The runner lane a pack's compress runs use.
pub(super) const LANE: &str = "packs";

/// What the app hears: a pack's record after every change.
pub type PackEvent = PackRecord;

pub(super) struct Active {
    pub(super) record: PackRecord,
    pub(super) stop: bool,
    /// Runs only this model (a decision on a stopped pack), then stops
    /// again with the rest still queued.
    pub(super) only: Option<String>,
    pub(super) zip: Option<ZipChoice>,
    /// The last model ran and the pack is being finished: nothing else
    /// may change it now.
    pub(super) closing: bool,
}

pub(super) struct Inner {
    pub(super) runner: PipelineRunner,
    pub(super) on_change: Box<dyn Fn(&PackRecord) + Send + Sync>,
    pub(super) active: Mutex<HashMap<String, Active>>,
}

impl Inner {
    pub(super) fn active(&self) -> MutexGuard<'_, HashMap<String, Active>> {
        self.active.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn tell(&self, root: &Path, record: &PackRecord) {
        if let Err(err) = record::save(root, record) {
            tracing::warn!(error = %err, pack = %record.id, "could not save a pack record");
        }
        (self.on_change)(record);
    }
}

/// Everything a pack's thread needs besides the record.
#[derive(Clone)]
pub(super) struct Drive {
    pub(super) root: PathBuf,
    pub(super) tools: Toolbox,
    pub(super) defaults: CompressDefaults,
}

/// The pack service. Cheap to clone.
#[derive(Clone)]
pub struct Packs {
    inner: Arc<Inner>,
}

fn stamp() -> String {
    now_rfc3339()
        .chars()
        .filter(char::is_ascii_digit)
        .take(14)
        .collect()
}

impl Packs {
    pub fn new(
        runner: PipelineRunner,
        on_change: impl Fn(&PackRecord) + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                runner,
                on_change: Box::new(on_change),
                active: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Starts a pack of `plan`'s models; answers its record at once.
    ///
    /// # Errors
    ///
    /// Fails without Blender, for bad settings, and for an out folder the
    /// caller may not write or that holds files Maquette did not write.
    pub fn start(
        &self,
        root: &Path,
        tools: Toolbox,
        defaults: CompressDefaults,
        request: &PackRequest,
        plan: PackPlan,
        from_anywhere: bool,
    ) -> CoreResult<PackRecord> {
        let settings = request.settings.checked()?;
        let (dir, zip) =
            record::out_dir(root, request.out.as_deref(), &settings.name, from_anywhere)?;
        if !tools.has(ToolId::Blender) {
            return Err(CoreError::Unavailable(
                "Blender is not installed, and a pack needs it to write and check each model."
                    .to_string(),
            ));
        }
        if dir.join(MARKER).is_file() {
            fs::remove_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
        }
        fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
        let id = format!("{}-{}", settings.name, stamp());
        let marker = dir.join(MARKER);
        crate::fs::write_atomic(&marker, id.as_bytes()).map_err(|e| CoreError::io(&marker, e))?;
        let items = plan
            .models
            .into_iter()
            .map(|m| {
                let budget = settings.budget(m.kind);
                PackItem::new(m, budget)
            })
            .collect();
        let record = PackRecord {
            version: RECORD_VERSION,
            id: id.clone(),
            name: settings.name.clone(),
            label: plan.label,
            source: request.source.clone(),
            settings,
            dir: dir.display().to_string(),
            zip: Some(zip.display().to_string()),
            state: PackState::Running,
            items,
            created_at: now_rfc3339(),
            finished_at: None,
            bytes: None,
            message: None,
            preset: request.preset.clone(),
        };
        if let Some(preset) = &request.preset {
            presets::mark_used(root, preset);
        }
        let drive = Drive {
            root: root.to_path_buf(),
            tools,
            defaults,
        };
        self.launch(&drive, record.clone(), None);
        Ok(record)
    }

    /// Puts `record` in the running set and starts its thread. With `only`
    /// set, just that model runs and the pack then stops.
    fn launch(&self, drive: &Drive, mut record: PackRecord, only: Option<String>) {
        record.state = PackState::Running;
        record.message = None;
        self.inner.tell(&drive.root, &record);
        self.inner.active().insert(
            record.id.clone(),
            Active {
                record: record.clone(),
                stop: false,
                only,
                zip: None,
                closing: false,
            },
        );
        let inner = Arc::clone(&self.inner);
        let drive = drive.clone();
        std::thread::spawn(move || run_pack(&inner, &drive, &record.id));
    }

    /// A pack's record: the running one's from memory, else from disk.
    ///
    /// # Errors
    ///
    /// Fails for an unknown pack.
    pub fn get(&self, root: &Path, id: &str) -> CoreResult<PackRecord> {
        if let Some(active) = self.inner.active().get(id) {
            return Ok(active.record.clone());
        }
        record::load(root, id).map(settle_closed)
    }

    /// The newest packs; one the app closed on reads as stopped.
    pub fn recent(&self, root: &Path) -> Vec<PackRecord> {
        let active = self.inner.active();
        record::recent(root)
            .into_iter()
            .map(|r| match active.get(&r.id) {
                Some(a) => a.record.clone(),
                None => settle_closed(r),
            })
            .collect()
    }

    /// Applies a person's decision on one model.
    ///
    /// # Errors
    ///
    /// Fails for an unknown pack or model, a raise that is no raise and a
    /// pack that is already done.
    pub fn decide(
        &self,
        root: &Path,
        tools: Toolbox,
        defaults: CompressDefaults,
        id: &str,
        decision: &PackDecision,
    ) -> CoreResult<PackRecord> {
        let mut active = self.inner.active();
        if let Some(running) = active.get_mut(id) {
            closing(running)?;
            apply(&mut running.record, decision)?;
            let record = running.record.clone();
            drop(active);
            self.inner.tell(root, &record);
            return Ok(record);
        }
        drop(active);
        let mut record = settle_closed(record::load(root, id)?);
        if record.state == PackState::Done {
            return Err(CoreError::Invalid(
                "This pack is done. Start a new one to change it.".to_string(),
            ));
        }
        let stopped = record.state == PackState::Stopped;
        apply(&mut record, decision)?;
        let drive = Drive {
            root: root.to_path_buf(),
            tools,
            defaults,
        };
        if record
            .items
            .iter()
            .any(|i| i.state == PackItemState::Queued)
        {
            if stopped {
                // A stopped pack stays stopped: a keep only records the
                // answer, a raise or again runs that one model and the
                // rest waits for go on.
                if matches!(decision, PackDecision::Keep { .. }) {
                    record.message = Some(stopped_at(&record, "Stopped"));
                    self.inner.tell(root, &record);
                    return Ok(record);
                }
                self.launch(&drive, record.clone(), Some(decision.key().to_string()));
            } else {
                self.launch(&drive, record.clone(), None);
            }
            return self.get(root, id);
        }
        settle(&self.inner, &drive, &mut record);
        Ok(record)
    }

    /// Finishes the pack with `choice`: at once when nothing runs, else
    /// once the model running now is done (the models still queued are left
    /// out).
    ///
    /// # Errors
    ///
    /// Fails for an unknown pack, one already done, or when nothing can go
    /// in it.
    pub fn zip(&self, root: &Path, id: &str, choice: ZipChoice) -> CoreResult<PackRecord> {
        let mut active = self.inner.active();
        if let Some(running) = active.get_mut(id) {
            closing(running)?;
            running.zip = Some(choice);
            return Ok(running.record.clone());
        }
        drop(active);
        let mut record = settle_closed(record::load(root, id)?);
        if record.state == PackState::Done {
            return Err(CoreError::Invalid("This pack is done already.".to_string()));
        }
        zip_now(&self.inner, root, &mut record, choice)?;
        Ok(record)
    }

    /// Stops the pack once the model running now is done.
    ///
    /// # Errors
    ///
    /// Fails for a pack that is not running.
    pub fn stop(&self, id: &str) -> CoreResult<PackRecord> {
        let mut active = self.inner.active();
        let running = active
            .get_mut(id)
            .ok_or_else(|| CoreError::Invalid("This pack is not running.".to_string()))?;
        closing(running)?;
        running.stop = true;
        Ok(running.record.clone())
    }

    /// Goes on with a stopped pack from the first model that did not run.
    ///
    /// # Errors
    ///
    /// Fails for an unknown pack, a running one, one with nothing left to
    /// run, and without Blender.
    pub fn resume(
        &self,
        root: &Path,
        tools: Toolbox,
        defaults: CompressDefaults,
        id: &str,
    ) -> CoreResult<PackRecord> {
        if self.inner.active().contains_key(id) {
            return Err(CoreError::Invalid(
                "This pack is running already.".to_string(),
            ));
        }
        if !tools.has(ToolId::Blender) {
            return Err(CoreError::Unavailable(
                "Blender is not installed, and a pack needs it.".to_string(),
            ));
        }
        let mut record = settle_closed(record::load(root, id)?);
        for item in &mut record.items {
            if item.state == PackItemState::Running {
                item.state = PackItemState::Queued;
            }
        }
        if !record
            .items
            .iter()
            .any(|i| i.state == PackItemState::Queued)
        {
            return Err(CoreError::Invalid(
                "Every model of this pack has run.".to_string(),
            ));
        }
        fs::create_dir_all(&record.dir).map_err(|e| CoreError::io(Path::new(&record.dir), e))?;
        let drive = Drive {
            root: root.to_path_buf(),
            tools,
            defaults,
        };
        self.launch(&drive, record.clone(), None);
        Ok(self.get(root, id).unwrap_or(record))
    }

    /// Blocks until no pack runs. For tests and the CLI.
    pub fn wait(&self) {
        while !self.inner.active().is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
    }
}

fn closing(active: &Active) -> CoreResult<()> {
    if active.closing {
        return Err(CoreError::Invalid(
            "The last model ran and the pack is being finished. Try again in a moment.".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn zip_now(
    inner: &Inner,
    root: &Path,
    record: &mut PackRecord,
    choice: ZipChoice,
) -> CoreResult<()> {
    record.state = PackState::Zipping;
    inner.tell(root, record);
    match finish(record, choice) {
        Ok(()) => {
            inner.tell(root, record);
            Ok(())
        }
        Err(err) => {
            record.state = PackState::Waiting;
            record.message = Some(err.to_string());
            inner.tell(root, record);
            Err(err)
        }
    }
}
