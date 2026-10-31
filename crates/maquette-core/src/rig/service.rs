//! The rig doctor service: reads a case at once, runs its pose test and fix
//! on a background thread, and tells the app about every change.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use super::run::{Run, check_can_run};
use super::{
    JointMoveState, RigRecord, RigRun, RigSource, RigStarter, RigState, RigStep, RigStop,
    RigStopKind, read, record,
};
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::now_ms;
use crate::pipeline::tools::Toolbox;
use crate::workspace::WorkspaceConfig;

type OnChange = Box<dyn Fn(&RigRecord) + Send + Sync>;

pub(super) struct Inner {
    pub(super) runner: Arc<dyn ScriptRunner + Send + Sync>,
    pub(super) on_change: OnChange,
    active: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

/// The service. Cheap to clone.
#[derive(Clone)]
pub struct RigDoctors {
    inner: Arc<Inner>,
}

/// A case's id: the name in lowercase words, with the version or `inbox`.
fn case_id(name: &str, source: &RigSource) -> String {
    let words: Vec<String> = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    let tail = source
        .version
        .map_or_else(|| "inbox".to_string(), |v| format!("v{v}"));
    format!("{}-{tail}", words.join("-"))
}

impl RigDoctors {
    pub fn new(
        runner: Arc<dyn ScriptRunner + Send + Sync>,
        on_change: impl Fn(&RigRecord) + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                runner,
                on_change: Box::new(on_change),
                active: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub(super) fn active(&self) -> MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.inner
            .active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(super) fn publish(&self, record: &mut RigRecord) -> CoreResult<()> {
        record.updated_ms = now_ms();
        record::save(record)?;
        (self.inner.on_change)(record);
        Ok(())
    }

    /// Opens the case of a model: reads it (no Blender) and writes nothing
    /// but the record. An open case of the same model comes back as it is.
    ///
    /// # Errors
    ///
    /// Fails when the file is not a `.glb` glTF model.
    pub fn open(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        name: &str,
        file: &Path,
        source: RigSource,
        starter: RigStarter,
    ) -> CoreResult<RigRecord> {
        let id = case_id(name, &source);
        if let Some(found) = record::load(root, config, &id)?.filter(|r| {
            r.input == file.display().to_string()
                && !matches!(r.state, RigState::Discarded | RigState::Failed)
        }) {
            let mut found = found;
            if starter == RigStarter::Agent && found.started_by != starter {
                found.started_by = starter;
                self.publish(&mut found)?;
            }
            return Ok(found);
        }
        let dir = record::work_dir(root, config, &id);
        let mut case = record::blank(id, name.to_string(), &dir, file, source);
        case.started_by = starter;
        let (model, route, issues) = read::read_model(file)?;
        case.model = model;
        case.route = route;
        case.state = if route.is_none() || issues.iter().any(|i| !i.ok) {
            RigState::Refused
        } else {
            RigState::Ready
        };
        case.issues = issues;
        case.step = if case.source.version.is_some() {
            RigStep::Before
        } else {
            RigStep::Accept
        };
        self.publish(&mut case)?;
        Ok(case)
    }

    /// The Inbox model was accepted: the case goes on from that version.
    ///
    /// # Errors
    ///
    /// Fails for an unknown case.
    pub fn accepted(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        id: &str,
        project: &str,
        asset: &str,
        (version, file): (u32, &Path),
    ) -> CoreResult<RigRecord> {
        let mut case = self.get(root, config, id)?;
        case.source = RigSource {
            inbox: None,
            project: Some(project.to_string()),
            asset: Some(asset.to_string()),
            version: Some(version),
        };
        case.input = file.display().to_string();
        if case.step == RigStep::Accept {
            case.step = RigStep::Before;
        }
        self.publish(&mut case)?;
        Ok(case)
    }

    /// Case `id`.
    ///
    /// # Errors
    ///
    /// Fails for an unknown case.
    pub fn get(&self, root: &Path, config: &WorkspaceConfig, id: &str) -> CoreResult<RigRecord> {
        let mut found = record::load(root, config, id)?
            .ok_or_else(|| CoreError::Invalid(format!("No rig doctor case {id}.")))?;
        // A run the app closed on did not finish.
        if found.state.running() && !self.active().contains_key(id) {
            found.state = if found.draft.is_some() && found.after.is_some() {
                // Only the walk clip proof runs with a finished draft.
                RigState::Fixed
            } else if found.before.is_some() {
                RigState::Tested
            } else {
                RigState::Ready
            };
            found.progress = None;
            found.error = Some("The app closed during this run. Run it again.".to_string());
        }
        Ok(found)
    }

    /// Starts a pose test or a fix; answers at once. It works on a copy.
    ///
    /// # Errors
    ///
    /// Fails for an unknown case, a refused one, one still in the Inbox, a
    /// run already going, a run out of order, and a third round.
    pub fn run(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        tools: Toolbox,
        id: &str,
        what: RigRun,
    ) -> CoreResult<RigRecord> {
        let mut case = self.get(root, config, id)?;
        check_can_run(&case, what)?;
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.active();
            if active.contains_key(id) {
                return Err(CoreError::Invalid(
                    "This rig doctor case is already running.".to_string(),
                ));
            }
            active.insert(id.to_string(), Arc::clone(&cancel));
        }
        case.state = if what == RigRun::PoseTest {
            RigState::Testing
        } else {
            RigState::Fixing
        };
        match what {
            RigRun::PoseTest => case.step = RigStep::Before,
            RigRun::Fix | RigRun::Round2 => case.step = RigStep::Fix,
            RigRun::WalkProof => {}
        }
        case.error = None;
        case.stop = None;
        match what {
            RigRun::PoseTest => case.stages.clear(),
            RigRun::Fix | RigRun::Round2 => case.round += 1,
            RigRun::WalkProof => {}
        }
        case.walk_proof = None;
        self.publish(&mut case)?;
        let dir = PathBuf::from(&case.dir);
        let answer = case.clone();
        let run = Run::new(case, Arc::clone(&self.inner), tools, cancel, dir);
        let doctors = self.clone();
        let key = id.to_string();
        thread::spawn(move || {
            let outcome = run.go(what);
            doctors.active().remove(&key);
            run.finish(outcome);
        });
        Ok(answer)
    }

    /// Asks a running case to stop after the script it is in.
    ///
    /// # Errors
    ///
    /// Fails when the case is not running.
    pub fn cancel(&self, id: &str) -> CoreResult<()> {
        let active = self.active();
        let flag = active
            .get(id)
            .ok_or_else(|| CoreError::Invalid("This case is not running.".to_string()))?;
        flag.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// A person's answer to the joint move. There is no agent path to it.
    ///
    /// # Errors
    ///
    /// Fails for an unknown case or one with no joint move asked.
    pub fn decide_joints(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        id: &str,
        approve: bool,
    ) -> CoreResult<RigRecord> {
        let mut case = self.get(root, config, id)?;
        let draft = case.draft.clone();
        let change = case
            .joint_move
            .as_mut()
            .filter(|m| m.state == JointMoveState::Asked && case.state == RigState::Fixed)
            .ok_or_else(|| CoreError::Invalid("No joint move waits for an answer.".to_string()))?;
        change.state = if approve {
            JointMoveState::Approved
        } else {
            JointMoveState::Declined
        };
        let moved = change.draft.clone();
        let (limb, joints) = (change.limb.clone(), change.joints);
        if approve {
            case.draft = Some(moved);
            for defect in case.defects.iter_mut().filter(|d| !d.fixed) {
                defect.fixed = true;
                defect.outcome = Some("joints moved".to_string());
            }
            for check in case.checks.iter_mut() {
                if check.title.starts_with("Bones and rest pose") {
                    check.title = "Bones kept, rest pose changed by your yes".to_string();
                    check.detail = format!("{joints} joints of the {limb} moved onto the mesh");
                }
            }
        } else {
            case.draft = draft;
        }
        self.publish(&mut case)?;
        Ok(case)
    }

    /// The draft became `version` of its asset.
    ///
    /// # Errors
    ///
    /// Fails for an unknown case.
    pub fn landed(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        id: &str,
        version: u32,
    ) -> CoreResult<RigRecord> {
        let mut case = self.get(root, config, id)?;
        case.state = RigState::Landed;
        case.step = RigStep::Land;
        case.landed = Some(version);
        self.publish(&mut case)?;
        Ok(case)
    }

    /// Holds the draft: a check got worse on its way to a version.
    ///
    /// # Errors
    ///
    /// Fails for an unknown case.
    pub fn held(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        id: &str,
        why: String,
    ) -> CoreResult<RigRecord> {
        let mut case = self.get(root, config, id)?;
        case.state = RigState::Stopped;
        case.stop = Some(RigStop {
            kind: RigStopKind::Worse,
            title: "A check got worse after the fix".to_string(),
            detail: why,
            row: None,
        });
        self.publish(&mut case)?;
        Ok(case)
    }

    /// Drops the draft. The version it came from is as it was.
    ///
    /// # Errors
    ///
    /// Fails for an unknown or running case.
    pub fn discard(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        id: &str,
    ) -> CoreResult<RigRecord> {
        let mut case = self.get(root, config, id)?;
        if case.state.running() {
            return Err(CoreError::Invalid(
                "Cancel the run before discarding its draft.".to_string(),
            ));
        }
        for file in [case.draft.take()].into_iter().flatten() {
            let _ = fs::remove_file(file);
        }
        case.state = RigState::Discarded;
        case.joint_move = None;
        self.publish(&mut case)?;
        Ok(case)
    }
}
