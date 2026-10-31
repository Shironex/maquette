//! The contact sheet: every model's check render in one PNG, in pack
//! order, on the app's graphite. Built from the renders the checks made,
//! so it shows what the other side gets.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::error::{CoreError, CoreResult};
use crate::refs::png;

/// The edge of one tile, the check renders' size.
pub(super) const TILE: usize = crate::thumbs::THUMB_SIZE as usize;
const GAP: usize = 8;
const MAX_COLUMNS: usize = 6;
/// Graphite, as the renders' backdrop.
const BACKDROP: [u8; 3] = [0x31, 0x2f, 0x2b];

/// Columns for `n` tiles: about four wide for three high.
pub(super) fn columns(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut cols = 1;
    while cols * cols * 3 < n * 4 && cols < MAX_COLUMNS {
        cols += 1;
    }
    cols.min(n)
}

fn crc(kind: &[u8], data: &[u8]) -> u32 {
    let mut crc = flate2::Crc::new();
    crc.update(kind);
    crc.update(data);
    crc.sum()
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc(kind, data).to_be_bytes());
}

/// RGB pixels as a deflated PNG.
pub(super) fn encode(width: usize, height: usize, rgb: &[u8]) -> CoreResult<Vec<u8>> {
    let row = width * 3;
    let mut raw = Vec::with_capacity((row + 1) * height);
    for y in 0..height {
        raw.push(0);
        raw.extend_from_slice(&rgb[y * row..(y + 1) * row]);
    }
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    let deflated = z
        .write_all(&raw)
        .and_then(|()| z.finish())
        .map_err(|e| CoreError::Invalid(format!("Could not encode the contact sheet. {e}")))?;
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&u32::try_from(width).unwrap_or(0).to_be_bytes());
    ihdr.extend_from_slice(&u32::try_from(height).unwrap_or(0).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &deflated);
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}

/// Draws `tile` into the sheet with its top left at (`left`, `top`),
/// scaled to [`TILE`] by nearest pixel, alpha over the backdrop.
fn draw(sheet: &mut [u8], width: usize, left: usize, top: usize, tile: &png::Pixels) {
    for y in 0..TILE {
        for x in 0..TILE {
            let [r, g, b, a] = tile.at(x * tile.width / TILE, y * tile.height / TILE);
            let i = ((top + y) * width + left + x) * 3;
            for (c, value) in [r, g, b].into_iter().enumerate() {
                let over =
                    u16::from(value) * u16::from(a) + u16::from(BACKDROP[c]) * (255 - u16::from(a));
                sheet[i + c] = u8::try_from(over / 255).unwrap_or(255);
            }
        }
    }
}

/// Writes the sheet of `pictures` to `out`. Answers how many renders it
/// holds; a picture that cannot be read is left out, and none writes no
/// file.
///
/// # Errors
///
/// Fails when the file cannot be written.
pub(super) fn write(pictures: &[PathBuf], out: &Path) -> CoreResult<usize> {
    let tiles: Vec<png::Pixels> = pictures
        .iter()
        .filter_map(|p| std::fs::read(p).ok())
        .filter_map(|bytes| png::decode(&bytes))
        .filter(|t| t.width > 0 && t.height > 0)
        .collect();
    if tiles.is_empty() {
        return Ok(0);
    }
    let cols = columns(tiles.len());
    let rows = tiles.len().div_ceil(cols);
    let width = cols * TILE + (cols + 1) * GAP;
    let height = rows * TILE + (rows + 1) * GAP;
    let mut sheet: Vec<u8> = BACKDROP
        .iter()
        .copied()
        .cycle()
        .take(width * height * 3)
        .collect();
    for (n, tile) in tiles.iter().enumerate() {
        let left = GAP + (n % cols) * (TILE + GAP);
        let top = GAP + (n / cols) * (TILE + GAP);
        draw(&mut sheet, width, left, top, tile);
    }
    let bytes = encode(width, height, &sheet)?;
    crate::fs::write_atomic(out, &bytes).map_err(|e| CoreError::io(out, e))?;
    Ok(tiles.len())
}
