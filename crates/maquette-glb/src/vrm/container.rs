//! The GLB container, taken apart into its JSON and BIN chunks and put back
//! together, plus the small JSON edits the VRM writer makes on the way.

use serde_json::{Value, json};

use maquette_base::error::{CoreError, CoreResult};

pub(super) fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

/// The JSON and BIN chunks of a GLB. A GLB without a BIN chunk gives an
/// empty one.
pub(super) fn split(bytes: &[u8]) -> CoreResult<(Value, Vec<u8>)> {
    let word = |at: usize| -> Option<usize> {
        let b: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
        usize::try_from(u32::from_le_bytes(b)).ok()
    };
    if bytes.get(..4) != Some(b"glTF".as_slice()) || word(4) != Some(2) {
        return Err(invalid("A VRM is written onto a binary glTF (.glb) model."));
    }
    let bad = || invalid("This .glb is not well formed.");
    let json_len = word(12).ok_or_else(bad)?;
    if bytes.get(16..20) != Some(b"JSON".as_slice()) {
        return Err(bad());
    }
    let text = bytes.get(20..20 + json_len).ok_or_else(bad)?;
    let json: Value = serde_json::from_slice(text)
        .map_err(|e| invalid(format!("The model's glTF JSON does not read. {e}")))?;
    let at = 20 + json_len;
    let bin = match word(at) {
        Some(len) if bytes.get(at + 4..at + 8) == Some(b"BIN\0".as_slice()) => {
            bytes.get(at + 8..at + 8 + len).ok_or_else(bad)?.to_vec()
        }
        _ => Vec::new(),
    };
    Ok((json, bin))
}

/// A GLB from its JSON and BIN chunks, each padded to four bytes as the
/// container asks: the JSON with spaces, the BIN with zeros.
pub(super) fn join(json: &Value, mut bin: Vec<u8>) -> CoreResult<Vec<u8>> {
    let mut text = serde_json::to_vec(json).map_err(|e| invalid(e.to_string()))?;
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let bin_chunk = if bin.is_empty() { 0 } else { 8 + bin.len() };
    let total = 12 + 8 + text.len() + bin_chunk;
    let word = |n: usize| -> CoreResult<[u8; 4]> {
        u32::try_from(n)
            .map(u32::to_le_bytes)
            .map_err(|_| invalid("The VRM would pass the 4 GB a .glb can hold."))
    };
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&word(total)?);
    out.extend_from_slice(&word(text.len())?);
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    if !bin.is_empty() {
        out.extend_from_slice(&word(bin.len())?);
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&bin);
    }
    Ok(out)
}

/// Appends `item` to the array at `key`, making it when missing, and
/// returns its index.
pub(super) fn push(json: &mut Value, key: &str, item: Value) -> usize {
    if !json.get(key).is_some_and(Value::is_array) {
        json[key] = json!([]);
    }
    json[key].as_array_mut().map_or(0, |list| {
        list.push(item);
        list.len() - 1
    })
}

/// Lists `name` in `extensionsUsed` once.
pub(super) fn use_extension(json: &mut Value, name: &str) {
    let listed = json
        .get("extensionsUsed")
        .and_then(Value::as_array)
        .is_some_and(|list| list.iter().any(|v| v == name));
    if !listed {
        push(json, "extensionsUsed", json!(name));
    }
}

/// Sets `extensions.<name>` on `target`, keeping its other extensions.
pub(super) fn set_extension(target: &mut Value, name: &str, value: Value) {
    if !target.get("extensions").is_some_and(Value::is_object) {
        target["extensions"] = json!({});
    }
    target["extensions"][name] = value;
}

/// Snaps node scales within `noise` of 1 on every axis to exactly 1.
/// Returns how many nodes changed.
pub(super) fn snap_scales(json: &mut Value, noise: f64) -> usize {
    let Some(nodes) = json.get_mut("nodes").and_then(Value::as_array_mut) else {
        return 0;
    };
    let mut snapped = 0;
    for node in nodes {
        let Some(scale) = node.get_mut("scale").and_then(Value::as_array_mut) else {
            continue;
        };
        let values: Vec<f64> = scale.iter().filter_map(Value::as_f64).collect();
        let near = values.len() == 3 && values.iter().all(|v| (v - 1.0).abs() <= noise);
        if near && values.iter().any(|v| *v != 1.0) {
            *scale = vec![json!(1.0), json!(1.0), json!(1.0)];
            snapped += 1;
        }
    }
    snapped
}

/// Appends `bytes` to the BIN chunk as a new buffer view on buffer 0, four
/// byte aligned, and returns the view's index.
pub(super) fn append_view(json: &mut Value, bin: &mut Vec<u8>, bytes: &[u8]) -> CoreResult<usize> {
    let first = json.get("buffers").and_then(|b| b.get(0)).cloned();
    if first.as_ref().is_some_and(|b| b.get("uri").is_some()) {
        return Err(invalid(
            "The model's first buffer is an outside file, so the thumbnail has nowhere to go.",
        ));
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let offset = bin.len();
    bin.extend_from_slice(bytes);
    if first.is_none() {
        push(json, "buffers", json!({}));
    }
    json["buffers"][0]["byteLength"] = json!(bin.len());
    Ok(push(
        json,
        "bufferViews",
        json!({ "buffer": 0, "byteOffset": offset, "byteLength": bytes.len() }),
    ))
}
