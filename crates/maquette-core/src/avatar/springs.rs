//! Spring bones for parts that hang off the body. AI made characters
//! arrive as one skinned mesh with a humanoid rig only, so a ponytail, the
//! tips of a cape or a charm move rigidly with the head or the chest. Two
//! headless Blender steps, one script ([`SCRIPT`]), change that:
//!
//! - **Finding** ([`find`]) proposes the parts hanging off the body, each
//!   with a kind, a root bone, a centre line, a length and how sure it is,
//!   and draws them on a sheet. It changes nothing and needs no yes; it is
//!   cached per model.
//! - **Chaining** ([`chain`]) adds a chain of joints along each part a
//!   person confirmed (the avatar spec's `springs`), weights the part to it
//!   and derives colliders from the body. Only new nodes and the skin
//!   weights change. It is its own approval step, like the reweight and
//!   the rest pose: before and after sheets (the after one a sway test), a
//!   pending result, and a yes held against the exact file.
//!
//! Both read the model a VRM is written from: the approved normalised rest
//! pose, or the version's file when its rest pose is VRM's already
//! ([`SpringsInput`]). Their results live in the version's avatar folder
//! under `springs/find` and `springs/chain`. A build then writes the VRM
//! from the approved chains and adds `VRMC_springBone` with each spring's
//! settings from the spec ([`chain::spring_bones`]).

pub mod chain;
pub mod find;
mod report;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{CoreError, CoreResult};
use crate::pipeline::cache::FileHasher;

pub use report::{
    ChainBuilt, ChainReport, Chained, FoundSprings, SpringCandidate, SpringWritten, SpringsSummary,
};

/// The script, run headless on a copy of the model.
pub const SCRIPT: &str = include_str!("avatar_springs.py");
const SCRIPT_FILE: &str = "avatar_springs.py";
/// The skill's copy of [`SCRIPT`], for a person or an agent to run by hand.
pub const SKILL_COPY: &str =
    "plugins/maquette/skills/maquette-avatar-vrm/scripts/avatar_springs.py";
/// The folder inside a version's avatar folder.
pub const SPRINGS_DIR: &str = "springs";
const JOB_FILE: &str = "job.json";

/// Whether a build adds the confirmed springs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SpringsMode {
    /// Look for dangling parts once (cached), and when springs are
    /// confirmed, chain them as a step that waits for a yes of its own.
    #[default]
    Auto,
    /// Write the VRM without springs.
    Off,
}

/// `<version's avatar folder>/springs`.
pub fn work_dir(version_dir: &Path) -> PathBuf {
    version_dir.join(SPRINGS_DIR)
}

/// The model the springs read and where their results go.
#[derive(Debug, Clone, PartialEq)]
pub struct SpringsInput {
    pub project: String,
    pub asset: String,
    pub version: u32,
    /// `<version's avatar folder>/springs`.
    pub dir: PathBuf,
    /// The approved normalised model, or the version's file. Only read.
    pub glb: PathBuf,
    /// VRM bone name to joint name.
    pub bones: BTreeMap<String, String>,
    /// The workspace's avatar cache, where file hashes are remembered.
    pub cache_root: PathBuf,
}

impl SpringsInput {
    fn hash(&self) -> CoreResult<String> {
        let mut hasher = FileHasher::with_memo(&self.cache_root);
        let hash = hasher
            .hash(&self.glb)
            .map_err(|e| CoreError::io(&self.glb, e))?;
        hasher.flush();
        Ok(hash)
    }
}

/// A run's key: the parts that decide its output, each on its own line.
fn key(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

#[cfg(test)]
mod blender_tests;
#[cfg(test)]
pub(crate) mod tests;
