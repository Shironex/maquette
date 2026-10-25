//! Reading and writing a binary glTF: the JSON chunk as a value and the
//! binary chunk as it is, so a change to the JSON leaves every byte of the
//! buffer alone.

use serde_json::Value;

const MAGIC: &[u8; 4] = b"glTF";
const JSON_CHUNK: &[u8; 4] = b"JSON";
const BIN_CHUNK: &[u8; 4] = b"BIN\0";

/// A GLB split into its JSON and its binary chunk.
pub(super) struct Glb {
    pub(super) json: Value,
    pub(super) bin: Option<Vec<u8>>,
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
}

/// The chunk at `at`: its type and its data, and where the next one starts.
fn chunk(bytes: &[u8], at: usize) -> Option<(&[u8], &[u8], usize)> {
    let len = usize::try_from(u32_at(bytes, at)?).ok()?;
    let kind = bytes.get(at + 4..at + 8)?;
    let data = bytes.get(at + 8..at + 8 + len)?;
    Some((kind, data, at + 8 + len))
}

/// Splits a GLB, or says why it is not one.
pub(super) fn read(bytes: &[u8]) -> Result<Glb, String> {
    if bytes.get(..4) != Some(MAGIC.as_slice()) {
        return Err("It is not a binary glTF (GLB) file.".to_string());
    }
    if u32_at(bytes, 4) != Some(2) {
        return Err("Only glTF 2.0 files can be tweaked.".to_string());
    }
    let (kind, data, next) = chunk(bytes, 12).ok_or("The GLB header is cut short.")?;
    if kind != JSON_CHUNK.as_slice() {
        return Err("The GLB does not start with its JSON chunk.".to_string());
    }
    let json: Value =
        serde_json::from_slice(data).map_err(|e| format!("The GLB's JSON is broken. {e}"))?;
    let bin = chunk(bytes, next)
        .filter(|(kind, _, _)| *kind == BIN_CHUNK.as_slice())
        .map(|(_, data, _)| data.to_vec());
    Ok(Glb { json, bin })
}

fn padded(mut data: Vec<u8>, fill: u8) -> Vec<u8> {
    while !data.len().is_multiple_of(4) {
        data.push(fill);
    }
    data
}

/// Joins the chunks again, each padded to four bytes as the format asks.
pub(super) fn write(glb: &Glb) -> Result<Vec<u8>, String> {
    let text = serde_json::to_vec(&glb.json).map_err(|e| e.to_string())?;
    let text = padded(text, b' ');
    let bin = glb.bin.clone().map(|b| padded(b, 0));
    let total = 12 + 8 + text.len() + bin.as_ref().map_or(0, |b| 8 + b.len());
    let size = |n: usize| {
        u32::try_from(n)
            .map(u32::to_le_bytes)
            .map_err(|_| "The model is larger than a GLB can hold.".to_string())
    };
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&size(total)?);
    out.extend_from_slice(&size(text.len())?);
    out.extend_from_slice(JSON_CHUNK);
    out.extend_from_slice(&text);
    if let Some(bin) = &bin {
        out.extend_from_slice(&size(bin.len())?);
        out.extend_from_slice(BIN_CHUNK);
        out.extend_from_slice(bin);
    }
    Ok(out)
}
