//! A person's yes or no to a derived avatar output (a reweighted or a
//! normalised model), held against the exact file it was given for. Each
//! step keeps its result, its approval and its denial in its own folder.

use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::normalize::NormalizeState;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::pipeline::cache::{self, hash_file};
use crate::project::now_rfc3339;

pub(crate) const APPROVAL_FILE: &str = "approval.json";
pub(crate) const DENIAL_FILE: &str = "denial.json";

/// A step's result, as its `result.json` holds it.
pub(crate) trait Gated: DeserializeOwned {
    /// The run's cache key; a yes or a no holds for this key only.
    fn key(&self) -> &str;
    fn decided(&mut self, state: NormalizeState, approved_at: Option<String>);
}

/// Where one step keeps its files, and what it says when they are missing.
pub(crate) struct Gate {
    pub(crate) result: &'static str,
    /// The step cache record's name.
    pub(crate) step: &'static str,
    /// The file a yes is held against.
    pub(crate) output: &'static str,
    pub(crate) outputs: fn() -> Vec<String>,
    /// Nothing intact to decide.
    pub(crate) missing: &'static str,
    /// The output waits for a yes.
    pub(crate) waiting: &'static str,
}

/// What `approval.json` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Approval {
    key: String,
    glb_sha256: String,
    approved_at: String,
}

/// What `denial.json` holds: a person's no to one key's output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Denial {
    key: String,
    denied_at: String,
}

fn glb_hash(path: &Path) -> CoreResult<String> {
    hash_file(path).map_err(|e| CoreError::io(path, e))
}

fn read_approval(dir: &Path) -> Option<Approval> {
    let text = fs::read_to_string(dir.join(APPROVAL_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

fn denied(dir: &Path, key: &str) -> bool {
    fs::read_to_string(dir.join(DENIAL_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<Denial>(&text).ok())
        .is_some_and(|d| d.key == key)
}

impl Gate {
    fn missing(&self) -> CoreError {
        CoreError::Invalid(self.missing.to_string())
    }

    /// The approval that holds for `key`: same key, same file bytes.
    fn valid_approval(&self, dir: &Path, key: &str) -> CoreResult<Option<Approval>> {
        let Some(approval) = read_approval(dir) else {
            return Ok(None);
        };
        let holds = approval.key == key && glb_hash(&dir.join(self.output))? == approval.glb_sha256;
        Ok(holds.then_some(approval))
    }

    /// The result in `dir`, with its approval state, or `None` when nothing
    /// ran there, its outputs changed since, or an older version wrote it in
    /// a shape this one cannot read.
    ///
    /// # Errors
    ///
    /// Fails when the result file is there but cannot be read from disk.
    pub(crate) fn load<T: Gated>(&self, dir: &Path) -> CoreResult<Option<T>> {
        let file = dir.join(self.result);
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(CoreError::io(&file, e)),
        };
        // A result an older version wrote, in a shape this one no longer reads, is a miss: the
        // next run makes it again instead of failing for the person.
        let Ok(mut found) = serde_json::from_str::<T>(&text) else {
            return Ok(None);
        };
        let intact = cache::load_record(dir, self.step)
            .is_some_and(|r| cache::is_fresh(&r, found.key(), dir, &(self.outputs)()));
        if !intact {
            return Ok(None);
        }
        let approval = self.valid_approval(dir, found.key())?;
        let state = if approval.is_some() {
            NormalizeState::Approved
        } else if denied(dir, found.key()) {
            NormalizeState::Denied
        } else {
            NormalizeState::Pending
        };
        found.decided(state, approval.map(|a| a.approved_at));
        Ok(Some(found))
    }

    /// Records a person's yes to the output in `dir`, for this exact file.
    ///
    /// # Errors
    ///
    /// Fails when there is no intact result to approve.
    pub(crate) fn approve<T: Gated>(&self, dir: &Path) -> CoreResult<T> {
        let mut found: T = self.load(dir)?.ok_or_else(|| self.missing())?;
        let approval = Approval {
            key: found.key().to_string(),
            glb_sha256: glb_hash(&dir.join(self.output))?,
            approved_at: now_rfc3339(),
        };
        write_json(&dir.join(APPROVAL_FILE), &approval)?;
        let _ = fs::remove_file(dir.join(DENIAL_FILE));
        found.decided(NormalizeState::Approved, Some(approval.approved_at));
        Ok(found)
    }

    /// Records a person's no to the output in `dir`: it stays unused until
    /// someone approves it after all, or a new run replaces it.
    ///
    /// # Errors
    ///
    /// Fails when there is no intact result to deny.
    pub(crate) fn deny<T: Gated>(&self, dir: &Path) -> CoreResult<T> {
        let mut found: T = self.load(dir)?.ok_or_else(|| self.missing())?;
        let denial = Denial {
            key: found.key().to_string(),
            denied_at: now_rfc3339(),
        };
        write_json(&dir.join(DENIAL_FILE), &denial)?;
        let _ = fs::remove_file(dir.join(APPROVAL_FILE));
        found.decided(NormalizeState::Denied, None);
        Ok(found)
    }

    /// The output's bytes, only once a person approved them.
    ///
    /// # Errors
    ///
    /// Refuses a result that is pending, missing or changed since the yes.
    pub(crate) fn approved_bytes<T: Gated>(&self, dir: &Path) -> CoreResult<Vec<u8>> {
        let found: T = self.load(dir)?.ok_or_else(|| self.missing())?;
        let approval = self
            .valid_approval(dir, found.key())?
            .ok_or_else(|| CoreError::Invalid(self.waiting.to_string()))?;
        let file = dir.join(self.output);
        let bytes = fs::read(&file).map_err(|e| CoreError::io(&file, e))?;
        // The bytes read now must be the bytes approved.
        if hex::encode(Sha256::digest(&bytes)) != approval.glb_sha256 {
            return Err(self.missing());
        }
        Ok(bytes)
    }
}
