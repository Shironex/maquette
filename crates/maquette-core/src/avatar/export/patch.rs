//! The change `set_avatar` makes to one asset's avatar spec.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::invalid;
use crate::avatar::AvatarSpec;
use crate::error::CoreResult;
use crate::inbox::vrm::{BoneOverrides, MaterialMode};

/// A change to one asset's spec, as `set_avatar` takes it. Only what is
/// given changes.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AvatarPatch {
    /// Start from the workspace defaults first, before the rest applies.
    pub reset: bool,
    /// Meta fields by their VRM names; null puts one back to its default.
    pub meta: Map<String, Value>,
    /// VRM bone name to a joint name, or null to leave the bone unmapped.
    pub bones: BoneOverrides,
    /// Bones whose override is dropped, so they map by name again.
    pub clear_bones: Vec<String>,
    pub material: Option<MaterialMode>,
}

/// The meta fields a patch may name.
const META_FIELDS: [&str; 18] = [
    "name",
    "version",
    "authors",
    "copyrightInformation",
    "contactInformation",
    "references",
    "thirdPartyLicenses",
    "licenseUrl",
    "avatarPermission",
    "allowExcessivelyViolentUsage",
    "allowExcessivelySexualUsage",
    "commercialUsage",
    "allowPoliticalOrReligiousUsage",
    "allowAntisocialOrHateUsage",
    "creditNotation",
    "allowRedistribution",
    "modification",
    "otherLicenseUrl",
];

/// Applies `patch` to `current` (`defaults` when it resets).
///
/// # Errors
///
/// Refuses an unknown meta field, a value of the wrong kind, or a spec
/// [`AvatarSpec::validate`] refuses.
pub fn apply_patch(
    current: &AvatarSpec,
    defaults: &AvatarSpec,
    patch: &AvatarPatch,
) -> CoreResult<AvatarSpec> {
    let mut next = if patch.reset {
        defaults.clone()
    } else {
        current.clone()
    };
    if !patch.meta.is_empty() {
        let mut meta = match serde_json::to_value(&next.meta) {
            Ok(Value::Object(map)) => map,
            _ => Map::new(),
        };
        for (field, value) in &patch.meta {
            if !META_FIELDS.contains(&field.as_str()) {
                return Err(invalid(format!(
                    "{field} is not a VRM meta field. Use one of {}.",
                    META_FIELDS.join(", ")
                )));
            }
            if value.is_null() {
                meta.remove(field);
            } else {
                meta.insert(field.clone(), value.clone());
            }
        }
        next.meta = serde_json::from_value(Value::Object(meta))
            .map_err(|e| invalid(format!("The meta does not read: {e}.")))?;
    }
    for bone in &patch.clear_bones {
        next.bones.remove(bone);
    }
    for (bone, joint) in &patch.bones {
        next.bones.insert(bone.clone(), joint.clone());
    }
    if let Some(material) = patch.material {
        next.material = material;
    }
    next.validate()?;
    Ok(next)
}
