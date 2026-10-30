//! A person's yes to a new rest pose, held against the exact output it was
//! given for, and the writer path that refuses anything else.

use std::path::Path;

use super::{NormalizeState, Normalized, OUTPUT_FILE, RESULT_FILE, STEP, outputs};
use crate::avatar::gate::{Gate, Gated};
use crate::error::CoreResult;
use crate::inbox::vrm::{self, WriteSpec, Written};

pub(super) const GATE: Gate = Gate {
    result: RESULT_FILE,
    step: STEP,
    output: OUTPUT_FILE,
    outputs,
    missing: "Nothing normalised here yet, or its files changed since. Normalise the model again.",
    waiting: "The new rest pose waits for a person's yes. Look at the before and after sheets, then approve it.",
};

impl Gated for Normalized {
    fn key(&self) -> &str {
        &self.key
    }

    fn decided(&mut self, state: NormalizeState, approved_at: Option<String>) {
        self.state = state;
        self.approved_at = approved_at;
    }
}

/// The result in `dir`, with its approval state, or `None` when nothing
/// was normalised there, its outputs changed since, or an older version
/// wrote it in a shape this one cannot read.
///
/// # Errors
///
/// Fails when the result file is there but cannot be read from disk.
pub fn load(dir: &Path) -> CoreResult<Option<Normalized>> {
    GATE.load(dir)
}

/// Records a person's yes to the rest pose change in `dir`, for this exact
/// output.
///
/// # Errors
///
/// Fails when there is no intact result to approve.
pub fn approve(dir: &Path) -> CoreResult<Normalized> {
    GATE.approve(dir)
}

/// Records a person's no to the rest pose change in `dir`: the output stays
/// unwritten until someone approves it after all, or a new run (other
/// options, another source) replaces it.
///
/// # Errors
///
/// Fails when there is no intact result to deny.
pub fn deny(dir: &Path) -> CoreResult<Normalized> {
    GATE.deny(dir)
}

/// The normalised GLB's bytes, only once a person approved them.
///
/// # Errors
///
/// Refuses a result that is pending, missing or changed since the yes.
pub fn approved_glb(dir: &Path) -> CoreResult<Vec<u8>> {
    GATE.approved_bytes::<Normalized>(dir)
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
