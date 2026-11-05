//! A GLB's `asset.extras`: read from the JSON chunk alone, and written by
//! rewriting only that chunk, so a mark travels with the file itself.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde_json::{Map, Value};

use maquette_base::error::{CoreError, CoreResult};

const MAGIC: &[u8; 4] = b"glTF";
const JSON_CHUNK: u32 = 0x4E4F_534A;
const HEADER: usize = 12;
const CHUNK_HEADER: usize = 8;
/// A JSON chunk bigger than this is no model this reads extras from.
const MAX_JSON: u32 = 64 * 1024 * 1024;

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    let four: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(four))
}

/// The JSON chunk's length, when `head` (the first 20 bytes) opens a GLB.
fn json_len(head: &[u8]) -> Option<u32> {
    if head.get(..4)? != MAGIC || word(head, HEADER + 4)? != JSON_CHUNK {
        return None;
    }
    word(head, HEADER).filter(|len| *len <= MAX_JSON)
}

/// `asset.extras[key]` of the GLB at `path`, reading only its header and
/// JSON chunk; none for a file that is no GLB or has no such extra.
pub fn asset_extra(path: &Path, key: &str) -> Option<Value> {
    let mut file = File::open(path).ok()?;
    let mut head = [0u8; HEADER + CHUNK_HEADER];
    file.read_exact(&mut head).ok()?;
    let len = usize::try_from(json_len(&head)?).ok()?;
    let mut json = vec![0u8; len];
    file.read_exact(&mut json).ok()?;
    let doc: Value = serde_json::from_slice(&json).ok()?;
    doc.get("asset")?.get("extras")?.get(key).cloned()
}

/// `bytes`, a GLB, with `asset.extras[key]` set to `value`; every other
/// chunk is kept byte for byte.
///
/// # Errors
///
/// Fails when `bytes` is no GLB with a JSON chunk first.
pub fn with_asset_extra(bytes: &[u8], key: &str, value: Value) -> CoreResult<Vec<u8>> {
    let bad = || CoreError::Invalid("The model is not a GLB with a JSON chunk.".to_string());
    let len = json_len(bytes).ok_or_else(bad)?;
    let start = HEADER + CHUNK_HEADER;
    let end = start + usize::try_from(len).map_err(|_| bad())?;
    let mut doc: Value =
        serde_json::from_slice(bytes.get(start..end).ok_or_else(bad)?).map_err(|_| bad())?;
    let asset = doc
        .as_object_mut()
        .ok_or_else(bad)?
        .entry("asset")
        .or_insert_with(|| Value::Object(Map::new()));
    let extras = asset
        .as_object_mut()
        .ok_or_else(bad)?
        .entry("extras")
        .or_insert_with(|| Value::Object(Map::new()));
    if !extras.is_object() {
        *extras = Value::Object(Map::new());
    }
    if let Some(map) = extras.as_object_mut() {
        map.insert(key.to_string(), value);
    }
    let mut json = serde_json::to_vec(&doc).map_err(|_| bad())?;
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let rest = &bytes[end..];
    let total = HEADER + CHUNK_HEADER + json.len() + rest.len();
    let as_u32 = |n: usize| u32::try_from(n).map_err(|_| bad());
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&bytes[..8]);
    out.extend_from_slice(&as_u32(total)?.to_le_bytes());
    out.extend_from_slice(&as_u32(json.len())?.to_le_bytes());
    out.extend_from_slice(&JSON_CHUNK.to_le_bytes());
    out.extend_from_slice(&json);
    out.extend_from_slice(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_extra_is_written_into_the_json_chunk_and_read_back() {
        let model = crate::fixture::clean_prop();
        let marked = with_asset_extra(&model, "mark", json!({ "a": 1 })).expect("marked");
        assert_eq!(word(&marked, 8).expect("length") as usize, marked.len());
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("m.glb");
        std::fs::write(&file, &marked).expect("write");
        assert_eq!(asset_extra(&file, "mark"), Some(json!({ "a": 1 })));
        assert_eq!(asset_extra(&file, "other"), None);
        // The model still reads as the same model.
        let before = crate::analyse::analyse_bytes(&model, dir.path()).expect("before");
        let after = crate::analyse::analyse_bytes(&marked, dir.path()).expect("after");
        assert_eq!(before.triangles, after.triangles);
    }

    #[test]
    fn a_file_that_is_no_glb_has_no_extra_and_is_not_marked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("m.glb");
        std::fs::write(&file, b"not a model at all").expect("write");
        assert_eq!(asset_extra(&file, "mark"), None);
        assert!(with_asset_extra(b"nope", "mark", json!(true)).is_err());
    }
}
