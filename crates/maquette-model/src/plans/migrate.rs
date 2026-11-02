//! Older canvas files, brought up to [`PLAN_VERSION`] as they are read. A
//! file is only migrated in memory: it is written as the current version
//! the first time a save changes something, so opening an old canvas never
//! rewrites it. A file from a newer Maquette is refused here, before its
//! shape is read, and so is never written back.
//!
//! - **1 to 2:** asset and Lab direction nodes. Every node gains `asset`
//!   and `direction` (both null for the note, picture and link a version 1
//!   file can hold).

use serde_json::{Map, Value};

use super::PLAN_VERSION;
use crate::error::{CoreError, CoreResult};

/// The node kinds a version 1 file may hold.
const V1_KINDS: [&str; 3] = ["note", "image", "link"];

fn invalid<T>(message: &str) -> CoreResult<T> {
    Err(CoreError::Invalid(message.to_string()))
}

/// The file's `version`, refused when it is missing, not a whole number
/// from 1 up, or newer than this Maquette reads.
///
/// # Errors
///
/// [`CoreError::UnsupportedVersion`] for a newer file, else
/// [`CoreError::Invalid`].
pub fn version_of(value: &Value) -> CoreResult<u32> {
    let Some(found) = value.get("version").and_then(Value::as_u64) else {
        return invalid("The canvas file has no version.");
    };
    if found > u64::from(PLAN_VERSION) {
        return Err(CoreError::UnsupportedVersion {
            found: u32::try_from(found).unwrap_or(u32::MAX),
            supported: PLAN_VERSION,
        });
    }
    match u32::try_from(found) {
        Ok(version) if version >= 1 => Ok(version),
        _ => invalid("The canvas file has no version."),
    }
}

/// `value` as a file of the current version would hold it.
///
/// # Errors
///
/// As [`version_of`], and when an old file holds what its version could
/// not (an asset node in a version 1 file).
pub fn to_current(mut value: Value) -> CoreResult<Value> {
    let mut version = version_of(&value)?;
    while version < PLAN_VERSION {
        match version {
            1 => v1_to_v2(&mut value)?,
            _ => return invalid("The canvas file has a version this Maquette never wrote."),
        }
        version += 1;
        if let Some(file) = value.as_object_mut() {
            file.insert("version".into(), Value::from(version));
        }
    }
    Ok(value)
}

fn v1_to_v2(value: &mut Value) -> CoreResult<()> {
    let Some(nodes) = value.get_mut("nodes").and_then(Value::as_array_mut) else {
        // No node list: the shape check after this says what is wrong.
        return Ok(());
    };
    for node in nodes.iter_mut().filter_map(Value::as_object_mut) {
        let kind = node.get("kind").and_then(Value::as_str).unwrap_or_default();
        if !V1_KINDS.contains(&kind) {
            return Err(CoreError::Invalid(format!(
                "The canvas file says version 1 but holds a node of kind {kind:?}, which version 1 has no room for."
            )));
        }
        add_null(node, "asset");
        add_null(node, "direction");
    }
    Ok(())
}

fn add_null(node: &mut Map<String, Value>, field: &str) {
    node.entry(field).or_insert(Value::Null);
}

#[cfg(test)]
mod tests;
