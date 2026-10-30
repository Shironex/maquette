//! Avatar specs: what a character asset needs to be written as a VRM 1.0
//! avatar beyond its model. They live in `avatar.json` next to
//! `project.json`, keyed by asset id like the material tweaks, so a new
//! model version keeps them.
//!
//! ```json
//! {
//!   "version": 1,
//!   "avatars": {
//!     "hero": {
//!       "meta": { "name": "Hero", "authors": ["Me"], "licenseUrl": "https://vrm.dev/licenses/1.0/" },
//!       "bones": { "leftEye": "Eye_L", "jaw": null },
//!       "material": "mtoon-soft",
//!       "springs": []
//!     }
//!   }
//! }
//! ```
//!
//! `meta` is the VRM meta as written, with the permissions at their VRM
//! defaults (the strictest) unless set. `bones` overrides the bone map: a
//! VRM bone name to the joint it maps to, or null to leave it unmapped.
//! A stored spec may be a draft: the meta's name and authors are only
//! required when the VRM is written. `springs` lists the hanging parts a
//! person confirmed for secondary motion ([`springs`]); it is left out when
//! empty, so older files read and write the same.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use maquette_glb::vrm::{BoneOverrides, MaterialMode, VrmMeta, is_human_bone};

mod springs;
pub use springs::{SPRING_JOINTS_MAX, SPRING_LINE_MAX, SpringSpec, validate_springs};

pub const AVATAR_FILE: &str = "avatar.json";
pub const AVATAR_VERSION: u32 = 1;
/// The longest joint name a bone override may hold.
pub const JOINT_NAME_MAX_CHARS: usize = 120;

/// One character asset's avatar.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AvatarSpec {
    pub meta: VrmMeta,
    /// VRM bone name to a joint name, or `None` to leave the bone
    /// unmapped. Bones not listed map by name.
    pub bones: BoneOverrides,
    pub material: MaterialMode,
    /// The confirmed springs, in the order they were confirmed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub springs: Vec<SpringSpec>,
}

impl AvatarSpec {
    /// Checks what a draft may already hold: the meta's text and licence
    /// URL, and overrides on VRM bone names with sensible joint names. The
    /// name and authors wait for the write ([`VrmMeta::validate`]).
    ///
    /// # Errors
    ///
    /// The first problem, as a sentence for the person or agent.
    pub fn validate(&self) -> CoreResult<()> {
        self.meta.check_draft()?;
        for (bone, joint) in &self.bones {
            if !is_human_bone(bone) {
                return Err(invalid(format!(
                    "{bone} is not a VRM humanoid bone. Use the VRM 1.0 names, such as leftUpperArm."
                )));
            }
            if let Some(joint) = joint {
                let count = joint.chars().count();
                if joint.trim().is_empty() || count > JOINT_NAME_MAX_CHARS {
                    return Err(invalid(format!(
                        "{bone}: a joint name must have 1 to {JOINT_NAME_MAX_CHARS} characters."
                    )));
                }
            }
        }
        validate_springs(&self.springs)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SceneAvatars {
    pub version: u32,
    /// Asset id to its avatar.
    pub avatars: BTreeMap<String, AvatarSpec>,
}

impl Default for SceneAvatars {
    fn default() -> Self {
        Self {
            version: AVATAR_VERSION,
            avatars: BTreeMap::new(),
        }
    }
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

pub fn path(project_dir: &Path) -> PathBuf {
    project_dir.join(AVATAR_FILE)
}

/// Reads `avatar.json`, or no avatars when the project has none.
///
/// # Errors
///
/// Fails when the file exists but cannot be read, comes from a newer
/// Maquette or holds a spec [`validate`] refuses.
pub fn load(project_dir: &Path) -> CoreResult<SceneAvatars> {
    let file = path(project_dir);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(SceneAvatars::default()),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    // The version first, so a newer file whose shape changed says so.
    let version = value.get("version").and_then(serde_json::Value::as_u64);
    if let Some(found) = version.filter(|v| *v > u64::from(AVATAR_VERSION)) {
        return Err(CoreError::UnsupportedVersion {
            found: u32::try_from(found).unwrap_or(u32::MAX),
            supported: AVATAR_VERSION,
        });
    }
    let found: SceneAvatars =
        serde_json::from_value(value).map_err(|e| CoreError::json(&file, e))?;
    validate(&found)?;
    Ok(found)
}

/// Checks every avatar, naming the asset in the message.
///
/// # Errors
///
/// The first problem, as a sentence for the person or agent.
pub fn validate(avatars: &SceneAvatars) -> CoreResult<()> {
    for (asset, spec) in &avatars.avatars {
        spec.validate().map_err(|e| match e {
            CoreError::Invalid(m) => invalid(format!("{asset}: {m}")),
            other => other,
        })?;
    }
    Ok(())
}

/// Trimmed text, no empty authors or references, and the current version.
pub fn normalized(avatars: &SceneAvatars) -> SceneAvatars {
    let trim = |s: &Option<String>| {
        s.as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
    };
    let list = |l: &[String]| -> Vec<String> {
        l.iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect()
    };
    let avatars = avatars
        .avatars
        .iter()
        .map(|(id, spec)| {
            let m = &spec.meta;
            let meta = VrmMeta {
                name: m.name.trim().to_string(),
                version: trim(&m.version),
                authors: list(&m.authors),
                copyright_information: trim(&m.copyright_information),
                contact_information: trim(&m.contact_information),
                references: list(&m.references),
                third_party_licenses: trim(&m.third_party_licenses),
                other_license_url: trim(&m.other_license_url),
                ..m.clone()
            };
            (
                id.clone(),
                AvatarSpec {
                    meta,
                    ..spec.clone()
                },
            )
        })
        .collect();
    SceneAvatars {
        version: AVATAR_VERSION,
        avatars,
    }
}

/// One writer at a time, so two saves never interleave.
static WRITES: Mutex<()> = Mutex::new(());

/// Reads `avatar.json` on disk, applies `change` and writes the result
/// atomically, so only the avatars `change` touches move. Returns the whole
/// file as written.
///
/// # Errors
///
/// Refuses, writing nothing, when the file on disk comes from a newer
/// Maquette or cannot be read (so a broken file is never overwritten), and
/// fails when `change` or [`validate`] refuses or the write fails.
pub fn update(
    project_dir: &Path,
    change: impl FnOnce(&SceneAvatars) -> CoreResult<SceneAvatars>,
) -> CoreResult<SceneAvatars> {
    let _one = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let current = load(project_dir).map_err(|err| match err {
        CoreError::UnsupportedVersion { .. } => err,
        other => invalid(format!(
            "{AVATAR_FILE} could not be read, so nothing was saved over it. Fix or remove the file, then save again. {other}"
        )),
    })?;
    let next = change(&current)?;
    validate(&next)?;
    let stored = normalized(&next);
    write_json(&path(project_dir), &stored)?;
    Ok(stored)
}

/// Sets `asset`'s avatar, or removes it with `None`.
///
/// # Errors
///
/// As [`update`].
pub fn set(project_dir: &Path, asset: &str, spec: Option<AvatarSpec>) -> CoreResult<SceneAvatars> {
    update(project_dir, |current| {
        let mut next = current.clone();
        match spec {
            Some(spec) => next.avatars.insert(asset.to_string(), spec),
            None => next.avatars.remove(asset),
        };
        Ok(next)
    })
}

#[cfg(test)]
mod tests;

/// Registers this module's IPC types (the scene's avatars) with the shell's
/// TypeScript export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types.register::<SceneAvatars>().register::<SpringSpec>()
}
