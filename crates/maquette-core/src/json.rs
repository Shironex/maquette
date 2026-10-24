//! JSON file helpers shared by the config and state modules.

use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{CoreError, CoreResult};
use crate::fs::write_atomic;

/// Pretty JSON with a trailing newline, written atomically.
pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> CoreResult<()> {
    let mut text = serde_json::to_string_pretty(value).map_err(|e| CoreError::json(path, e))?;
    text.push('\n');
    write_atomic(path, text.as_bytes()).map_err(|e| CoreError::io(path, e))
}

/// Parses `text` laid over `T::default()`, so older or partial files load
/// with defaults for every missing key. Unknown keys are ignored.
///
/// Done this way instead of `#[serde(default)]` so the exported TypeScript
/// types keep every field required.
pub(crate) fn from_json_with_defaults<T>(text: &str) -> Result<T, serde_json::Error>
where
    T: Serialize + DeserializeOwned + Default,
{
    let mut base = serde_json::to_value(T::default())?;
    let overlay: Value = serde_json::from_str(text)?;
    merge(&mut base, overlay);
    serde_json::from_value(base)
}

fn merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base), Value::Object(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(&key) {
                    Some(slot) => merge(slot, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (slot, value) => *slot = value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_keeps_nested_defaults() {
        let mut base = serde_json::json!({"a": {"b": 1, "c": 2}, "d": [1]});
        merge(
            &mut base,
            serde_json::json!({"a": {"c": 3}, "d": [], "e": true}),
        );
        assert_eq!(
            base,
            serde_json::json!({"a": {"b": 1, "c": 3}, "d": [], "e": true})
        );
    }
}
