//! A person's yes to a new rest pose, held against the exact output it was
//! given for, and the writer path that refuses anything else.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{NormalizeState, Normalized, OUTPUT_FILE, RESULT_FILE, STEP, invalid, outputs};
use crate::error::{CoreError, CoreResult};
use crate::inbox::vrm::{self, WriteSpec, Written};
use crate::json::write_json;
use crate::pipeline::cache::{self, hash_file};
use crate::project::now_rfc3339;

pub(super) const APPROVAL_FILE: &str = "approval.json";
pub(super) const DENIAL_FILE: &str = "denial.json";

/// What `approval.json` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Approval {
    key: String,
    glb_sha256: String,
    approved_at: String,
}

fn glb_hash(path: &Path) -> CoreResult<String> {
    hash_file(path).map_err(|e| CoreError::io(path, e))
}

fn read_approval(dir: &Path) -> Option<Approval> {
    let text = fs::read_to_string(dir.join(APPROVAL_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

/// What `denial.json` holds: a person's no to one key's output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Denial {
    key: String,
    denied_at: String,
}

fn denied(dir: &Path, key: &str) -> bool {
    fs::read_to_string(dir.join(DENIAL_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<Denial>(&text).ok())
        .is_some_and(|d| d.key == key)
}

/// The approval that holds for `found`: same key, same file bytes.
fn valid_approval(dir: &Path, found: &Normalized) -> CoreResult<Option<Approval>> {
    let Some(approval) = read_approval(dir) else {
        return Ok(None);
    };
    let glb = dir.join(OUTPUT_FILE);
    let holds = approval.key == found.key && glb_hash(&glb)? == approval.glb_sha256;
    Ok(holds.then_some(approval))
}

/// The result in `dir`, with its approval state, or `None` when nothing
/// was normalised there, its outputs changed since, or an older version
/// wrote it in a shape this one cannot read.
///
/// # Errors
///
/// Fails when the result file is there but cannot be read from disk.
pub fn load(dir: &Path) -> CoreResult<Option<Normalized>> {
    let file = dir.join(RESULT_FILE);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(CoreError::io(&file, e)),
    };
    // A result an older version wrote, in a shape this one no longer reads, is a miss: the
    // next run makes it again instead of failing for the person.
    let Ok(mut found) = serde_json::from_str::<Normalized>(&text) else {
        return Ok(None);
    };
    let intact = cache::load_record(dir, STEP)
        .is_some_and(|r| cache::is_fresh(&r, &found.key, dir, &outputs()));
    if !intact {
        return Ok(None);
    }
    let approval = valid_approval(dir, &found)?;
    found.state = if approval.is_some() {
        NormalizeState::Approved
    } else if denied(dir, &found.key) {
        NormalizeState::Denied
    } else {
        NormalizeState::Pending
    };
    found.approved_at = approval.map(|a| a.approved_at);
    Ok(Some(found))
}

fn not_normalised() -> CoreError {
    invalid("Nothing normalised here yet, or its files changed since. Normalise the model again.")
}

/// Records a person's yes to the rest pose change in `dir`, for this exact
/// output.
///
/// # Errors
///
/// Fails when there is no intact result to approve.
pub fn approve(dir: &Path) -> CoreResult<Normalized> {
    let mut found = load(dir)?.ok_or_else(not_normalised)?;
    let approval = Approval {
        key: found.key.clone(),
        glb_sha256: glb_hash(&dir.join(OUTPUT_FILE))?,
        approved_at: now_rfc3339(),
    };
    write_json(&dir.join(APPROVAL_FILE), &approval)?;
    let _ = fs::remove_file(dir.join(DENIAL_FILE));
    found.state = NormalizeState::Approved;
    found.approved_at = Some(approval.approved_at);
    Ok(found)
}

/// Records a person's no to the rest pose change in `dir`: the output stays
/// unwritten until someone approves it after all, or a new run (other
/// options, another source) replaces it.
///
/// # Errors
///
/// Fails when there is no intact result to deny.
pub fn deny(dir: &Path) -> CoreResult<Normalized> {
    let mut found = load(dir)?.ok_or_else(not_normalised)?;
    let denial = Denial {
        key: found.key.clone(),
        denied_at: now_rfc3339(),
    };
    write_json(&dir.join(DENIAL_FILE), &denial)?;
    let _ = fs::remove_file(dir.join(APPROVAL_FILE));
    found.state = NormalizeState::Denied;
    found.approved_at = None;
    Ok(found)
}

/// The normalised GLB's bytes, only once a person approved them.
///
/// # Errors
///
/// Refuses a result that is pending, missing or changed since the yes.
pub fn approved_glb(dir: &Path) -> CoreResult<Vec<u8>> {
    let found = load(dir)?.ok_or_else(not_normalised)?;
    if found.state != NormalizeState::Approved {
        return Err(invalid(
            "The new rest pose waits for a person's yes. Look at the before and after sheets, then approve it.",
        ));
    }
    let glb = dir.join(OUTPUT_FILE);
    let bytes = fs::read(&glb).map_err(|e| CoreError::io(&glb, e))?;
    // The bytes read now must be the bytes approved.
    let approval = read_approval(dir).ok_or_else(not_normalised)?;
    if hex::encode(Sha256::digest(&bytes)) != approval.glb_sha256 {
        return Err(not_normalised());
    }
    Ok(bytes)
}

/// Writes the approved normalised model in `dir` as VRM 1.0.
///
/// # Errors
///
/// Refuses an unapproved result ([`approved_glb`]), and fails where the
/// writer does.
pub fn write_vrm(dir: &Path, spec: &WriteSpec) -> CoreResult<Written> {
    let bytes = approved_glb(dir)?;
    vrm::write(&bytes, spec, dir)
}
